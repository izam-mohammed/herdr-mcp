//! The hub: one long-lived `events.subscribe` connection, read by a task
//! that feeds the ring. herdr allows one request per connection except
//! `events.subscribe`, which keeps streaming (docs/external/herdr-socket-api.md).
use super::ring::{Filter, Ring};
use crate::error::Failure;
use crate::herdr::HerdrClient;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines};
use tokio::net::UnixStream;
use tokio::net::unix::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::Notify;

/// Every type that needs only `type` (docs/external/herdr-socket-api.md).
const LIFECYCLE: &[&str] = &[
    "workspace.created",
    "workspace.updated",
    "workspace.metadata_updated",
    "workspace.renamed",
    "workspace.moved",
    "workspace.reordered",
    "workspace.closed",
    "workspace.focused",
    "worktree.created",
    "worktree.opened",
    "worktree.removed",
    "tab.created",
    "tab.closed",
    "tab.focused",
    "tab.renamed",
    "tab.moved",
    "pane.created",
    "pane.closed",
    "pane.updated",
    "pane.focused",
    "pane.moved",
    "pane.exited",
    "pane.agent_detected",
    "layout.updated",
];

pub struct EventHub {
    ring: Mutex<Ring>,
    notify: Notify,
    connected: AtomicBool,
    panes_watched: AtomicUsize,
}

pub struct Status {
    pub connected: bool,
    pub buffered: usize,
    pub capacity: usize,
    pub last_seq: u64,
    pub last_event_ms: Option<u64>,
    pub lost: u64,
    pub panes_watched: usize,
}

impl EventHub {
    /// A hub with no connection, for tests and for `start`.
    #[must_use]
    pub fn new(capacity: usize) -> Arc<Self> {
        Arc::new(Self {
            ring: Mutex::new(Ring::new(capacity)),
            notify: Notify::new(),
            connected: AtomicBool::new(false),
            panes_watched: AtomicUsize::new(0),
        })
    }

    /// Start the subscription task. Needs a running tokio runtime.
    #[must_use]
    pub fn start(socket: PathBuf, capacity: usize) -> Arc<Self> {
        let hub = Self::new(capacity);
        let client = HerdrClient::new(socket.clone(), std::time::Duration::from_secs(10), false);
        tokio::spawn(run(hub.clone(), client, socket));
        hub
    }

    pub fn ring(&self) -> MutexGuard<'_, Ring> {
        self.ring
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// A wait registers before it checks the ring, so no push is missed.
    pub fn notify(&self) -> &Notify {
        &self.notify
    }

    #[must_use]
    pub fn status(&self) -> Status {
        let ring = self.ring();
        Status {
            connected: self.connected.load(Ordering::Relaxed),
            buffered: ring.len(),
            capacity: ring.capacity(),
            last_seq: ring.last_seq(),
            last_event_ms: ring.last_event_ms,
            lost: ring.lost,
            panes_watched: self.panes_watched.load(Ordering::Relaxed),
        }
    }

    fn push(&self, name: &str, data: Value) {
        self.ring().push(name, data);
        self.notify.notify_waiters();
    }

    /// Wait for the first matching event after `since`, up to `timeout`.
    pub async fn wait(
        &self,
        since: Option<u64>,
        filter: &Filter,
        timeout: std::time::Duration,
    ) -> (Option<Value>, u64) {
        let deadline = tokio::time::Instant::now() + timeout;
        let since = since.unwrap_or_else(|| self.ring().last_seq());
        loop {
            let notified = self.notify.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            if let Some(e) = self.ring().first_after(since, filter) {
                return (Some(e.to_json()), e.seq);
            }
            tokio::select! {
                () = &mut notified => {}
                () = tokio::time::sleep_until(deadline) => {
                    return (None, self.ring().last_seq().max(since));
                }
            }
        }
    }
}

struct Conn {
    lines: Lines<BufReader<OwnedReadHalf>>,
    /// Held open: closing our write side may end the stream.
    _write: OwnedWriteHalf,
}

fn bad(code: &str, message: impl Into<String>) -> Failure {
    Failure::new(code, message)
}

/// Connect, subscribe and wait for `subscription_started`.
async fn subscribe(socket: &Path, subscriptions: &[Value]) -> Result<Conn, Failure> {
    let io = |e: std::io::Error| bad("herdr_unreachable", format!("event stream: {e}"));
    let stream = UnixStream::connect(socket).await.map_err(io)?;
    let (read, mut write) = stream.into_split();
    let request = json!({"id": "sub", "method": "events.subscribe", "params": {"subscriptions": subscriptions}});
    write
        .write_all(format!("{request}\n").as_bytes())
        .await
        .map_err(io)?;
    let mut lines = BufReader::new(read).lines();
    let first = tokio::time::timeout(std::time::Duration::from_secs(5), lines.next_line())
        .await
        .map_err(|_| bad("timeout", "herdr did not start the subscription"))?
        .map_err(io)?
        .ok_or_else(|| bad("herdr_unreachable", "herdr closed the event stream"))?;
    let reply: Value = serde_json::from_str(&first)
        .map_err(|e| bad("invalid_reply", format!("subscription reply: {e}")))?;
    if let Some(e) = reply.get("error") {
        let text = |k| e.get(k).and_then(Value::as_str).unwrap_or_default();
        return Err(bad(text("code"), text("message")));
    }
    Ok(Conn {
        lines,
        _write: write,
    })
}

/// Lifecycle types plus one `pane.agent_status_changed` per pane: herdr
/// requires a `pane_id` on that type (docs/external/herdr-socket-api.md).
fn subscriptions(panes: &[String]) -> Vec<Value> {
    LIFECYCLE
        .iter()
        .map(|t| json!({"type": t}))
        .chain(
            panes
                .iter()
                .map(|p| json!({"type": "pane.agent_status_changed", "pane_id": p})),
        )
        .collect()
}

/// Pane ids now, sorted. Also what validates the set: herdr rejects a
/// whole request that names a missing pane.
async fn list_panes(client: &HerdrClient) -> Result<Vec<String>, Failure> {
    let result = client.call("pane.list", json!({})).await?;
    let mut panes: Vec<String> = result["panes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|p| p["pane_id"].as_str().map(str::to_owned))
        .collect();
    panes.sort();
    Ok(panes)
}

/// List the panes and subscribe to them; a pane closed in between is
/// rejected by herdr, so retry once with a fresh list.
async fn open(client: &HerdrClient, socket: &Path) -> Result<(Conn, Vec<String>), Failure> {
    let mut last = None;
    for _ in 0..2 {
        let panes = list_panes(client).await?;
        match subscribe(socket, &subscriptions(&panes)).await {
            Ok(conn) => return Ok((conn, panes)),
            Err(e) => last = Some(e),
        }
    }
    Err(last.unwrap_or_else(|| bad("invalid_reply", "no subscription attempt")))
}

const BACKOFF_MIN: std::time::Duration = std::time::Duration::from_millis(100);
const BACKOFF_MAX: std::time::Duration = std::time::Duration::from_secs(5);
/// Wait this long after a pane event for more, so a burst is one rebuild.
const REBUILD_DELAY: std::time::Duration = std::time::Duration::from_millis(50);
/// How long the old subscription is read after the new one has started.
const DRAIN_IDLE: std::time::Duration = std::time::Duration::from_millis(150);
/// How long a line seen on the old subscription suppresses its twin on the new.
const DUP_WINDOW: std::time::Duration = std::time::Duration::from_secs(1);

/// Lines read from the old subscription during a rebuild, so their copies on
/// the new one are dropped.
#[derive(Default)]
struct Dups(Vec<(tokio::time::Instant, String)>);

impl Dups {
    fn take(&mut self, line: &str) -> bool {
        let now = tokio::time::Instant::now();
        self.0.retain(|(t, _)| now.duration_since(*t) < DUP_WINDOW);
        self.0.iter().position(|(_, l)| l == line).is_some_and(|i| {
            self.0.remove(i);
            true
        })
    }
}

enum Line {
    Event,
    /// herdr says this subscriber fell behind and closed the subscription.
    Lost,
    /// A pane appeared or went: the set to watch changed.
    PaneSetChanged,
}

fn handle(hub: &EventHub, line: &str) -> Option<Line> {
    let v: Value = serde_json::from_str(line).ok()?;
    let name = v.get("event").and_then(Value::as_str)?;
    if name == "events_lost" {
        return Some(Line::Lost);
    }
    hub.push(name, v.get("data").cloned().unwrap_or(Value::Null));
    Some(match name {
        "pane_created" | "pane.created" | "pane_closed" | "pane.closed" => Line::PaneSetChanged,
        _ => Line::Event,
    })
}

/// What a resubscribe tells the reader about the world now, from
/// `session.snapshot` on its own connection (herdr's advice after a loss).
async fn snapshot_detail(client: &HerdrClient) -> Value {
    let Ok(result) = client.call("session.snapshot", json!({})).await else {
        return json!({"snapshot": null});
    };
    let snap = &result["snapshot"];
    let count = |k: &str| snap[k].as_array().map_or(0, Vec::len);
    json!({"snapshot": {
        "focused_workspace_id": snap["focused_workspace_id"],
        "focused_tab_id": snap["focused_tab_id"],
        "focused_pane_id": snap["focused_pane_id"],
        "workspaces": count("workspaces"),
        "tabs": count("tabs"),
        "panes": count("panes"),
    }})
}

async fn run(hub: Arc<EventHub>, client: HerdrClient, socket: PathBuf) {
    let mut backoff = BACKOFF_MIN;
    // Why the last stream ended, until the next one has started and left a gap marker.
    let mut ended: Option<String> = None;
    loop {
        if let Ok((conn, panes)) = open(&client, &socket).await {
            backoff = BACKOFF_MIN;
            if let Some(reason) = ended.take() {
                let detail = snapshot_detail(&client).await;
                hub.ring().push_gap(&reason, &detail);
                hub.notify.notify_waiters();
            }
            hub.panes_watched.store(panes.len(), Ordering::Relaxed);
            hub.connected.store(true, Ordering::Relaxed);
            let reason = stream(&hub, &client, &socket, conn, panes).await;
            hub.connected.store(false, Ordering::Relaxed);
            hub.ring().lost += 1;
            ended = Some(reason);
            tokio::time::sleep(BACKOFF_MIN).await;
        } else {
            tokio::time::sleep(backoff).await;
            backoff = (backoff * 2).min(BACKOFF_MAX);
        }
    }
}

/// Read the stream until it ends. A pane created or closed rebuilds it.
async fn stream(
    hub: &EventHub,
    client: &HerdrClient,
    socket: &Path,
    mut conn: Conn,
    mut panes: Vec<String>,
) -> String {
    let mut dups = Dups::default();
    let mut rebuild_at: Option<tokio::time::Instant> = None;
    loop {
        let next = match rebuild_at {
            Some(at) => tokio::time::timeout_at(at, conn.lines.next_line()).await,
            None => Ok(conn.lines.next_line().await),
        };
        match next {
            Err(_) => {
                rebuild_at = None;
                match rebuild(hub, client, socket, &mut conn, &mut panes, &mut dups).await {
                    // A pane came or went while the old stream drained: go again.
                    Ok(true) => rebuild_at = Some(tokio::time::Instant::now()),
                    Ok(false) => {}
                    Err(e) => {
                        return format!("pane set rejected or herdr unreachable ({})", e.code);
                    }
                }
            }
            Ok(Ok(Some(line))) => {
                if dups.take(&line) {
                    continue;
                }
                match handle(hub, &line) {
                    Some(Line::Lost) => return "events_lost: the subscriber fell behind".into(),
                    Some(Line::PaneSetChanged) => {
                        rebuild_at.get_or_insert(tokio::time::Instant::now() + REBUILD_DELAY);
                    }
                    Some(Line::Event) | None => {}
                }
            }
            Ok(Ok(None) | Err(_)) => return "connection closed".into(),
        }
    }
}

/// Subscribe to the new pane set, then close the old subscription, reading
/// what it still holds first so nothing is lost and nothing doubled.
async fn rebuild(
    hub: &EventHub,
    client: &HerdrClient,
    socket: &Path,
    conn: &mut Conn,
    panes: &mut Vec<String>,
    dups: &mut Dups,
) -> Result<bool, Failure> {
    let (new, now) = open(client, socket).await?;
    if now == *panes {
        // Nothing changed after all; keep the old one (the new drops here).
        return Ok(false);
    }
    let mut old = std::mem::replace(conn, new);
    *panes = now;
    hub.panes_watched.store(panes.len(), Ordering::Relaxed);
    let mut again = false;
    while let Ok(Ok(Some(line))) = tokio::time::timeout(DRAIN_IDLE, old.lines.next_line()).await {
        let seen = handle(hub, &line);
        again |= matches!(seen, Some(Line::PaneSetChanged));
        if seen.is_some() {
            dups.0.push((tokio::time::Instant::now(), line));
        }
    }
    Ok(again)
}
