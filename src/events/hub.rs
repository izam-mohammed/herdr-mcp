//! The hub: one long-lived `events.subscribe` connection, read by a task
//! that feeds the ring. herdr allows one request per connection except
//! `events.subscribe`, which keeps streaming (docs/external/herdr-socket-api.md).
use super::ring::{Filter, Ring};
use crate::error::Failure;
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
        tokio::spawn(run(hub.clone(), socket));
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

fn lifecycle() -> Vec<Value> {
    LIFECYCLE.iter().map(|t| json!({"type": t})).collect()
}

const BACKOFF_MIN: std::time::Duration = std::time::Duration::from_millis(100);
const BACKOFF_MAX: std::time::Duration = std::time::Duration::from_secs(5);

async fn run(hub: Arc<EventHub>, socket: PathBuf) {
    let mut backoff = BACKOFF_MIN;
    loop {
        if let Ok(mut conn) = subscribe(&socket, &lifecycle()).await {
            backoff = BACKOFF_MIN;
            hub.connected.store(true, Ordering::Relaxed);
            while let Ok(Some(line)) = conn.lines.next_line().await {
                if let Ok(v) = serde_json::from_str::<Value>(&line)
                    && let Some(name) = v.get("event").and_then(Value::as_str)
                {
                    hub.push(name, v.get("data").cloned().unwrap_or(Value::Null));
                }
            }
            hub.connected.store(false, Ordering::Relaxed);
        } else {
            tokio::time::sleep(backoff).await;
            backoff = (backoff * 2).min(BACKOFF_MAX);
        }
    }
}
