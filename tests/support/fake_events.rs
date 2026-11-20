//! A fake herdr with an event stream (plan 05). `events.subscribe` keeps its
//! connection open and receives what `emit` sends to a matching subscription;
//! `pane.list` answers from `set_panes`; a subscription naming an unknown
//! pane is rejected like herdr does. Every request is recorded.
#![allow(dead_code)]
use crate::support::{bind, socket_path};
use serde_json::{Value, json};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};

struct Sub {
    tx: UnboundedSender<Option<String>>,
    /// `(dotted type, pane_id)` per subscription entry.
    entries: Vec<(String, Option<String>)>,
}

#[derive(Default)]
struct State {
    panes: Vec<String>,
    subs: Vec<Sub>,
    requests: Vec<Value>,
    /// Reject this many subscribe requests with `pane_not_found`.
    reject: u32,
    snapshots: u32,
}

#[derive(Clone)]
pub struct FakeEvents {
    pub socket: PathBuf,
    state: Arc<Mutex<State>>,
}

impl FakeEvents {
    pub fn start(tag: &str, panes: &[&str]) -> FakeEvents {
        let socket = socket_path(tag);
        let listener = bind(&socket);
        let state = Arc::new(Mutex::new(State {
            panes: panes.iter().map(|p| (*p).to_owned()).collect(),
            ..State::default()
        }));
        let st = state.clone();
        tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                tokio::spawn(serve(stream, st.clone()));
            }
        });
        FakeEvents { socket, state }
    }

    fn state(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().expect("state")
    }

    pub fn set_panes(&self, panes: &[&str]) {
        self.state().panes = panes.iter().map(|p| (*p).to_owned()).collect();
    }

    pub fn reject_next_subscribes(&self, n: u32) {
        self.state().reject = n;
    }

    /// Subscribe requests received so far, as their `subscriptions` arrays.
    pub fn subscribes(&self) -> Vec<Value> {
        self.state()
            .requests
            .iter()
            .filter(|r| r["method"] == "events.subscribe")
            .map(|r| r["params"]["subscriptions"].clone())
            .collect()
    }

    pub fn methods(&self) -> Vec<String> {
        self.state()
            .requests
            .iter()
            .map(|r| r["method"].as_str().unwrap_or_default().to_owned())
            .collect()
    }

    pub fn live_subscriptions(&self) -> usize {
        self.state()
            .subs
            .iter()
            .filter(|s| !s.tx.is_closed())
            .count()
    }

    /// Send an event to every live subscription with a matching entry.
    /// `name` is herdr's dotted type (`tab.created`); the line carries `snake`.
    pub fn emit(&self, dotted: &str, snake: &str, data: &Value) {
        let pane = data["pane_id"]
            .as_str()
            .or(data["pane"]["pane_id"].as_str());
        let line = json!({"event": snake, "data": data}).to_string();
        for sub in &self.state().subs {
            let hit = sub
                .entries
                .iter()
                .any(|(t, p)| t == dotted && (p.is_none() || p.as_deref() == pane));
            if hit {
                let _ = sub.tx.send(Some(line.clone()));
            }
        }
    }

    /// Say events were lost, then close, as herdr does for a slow subscriber.
    pub fn lose(&self) {
        let line = json!({"event":"events_lost","data":{"type":"events_lost"}}).to_string();
        for sub in self.state().subs.drain(..) {
            let _ = sub.tx.send(Some(line.clone()));
            let _ = sub.tx.send(None);
        }
    }

    /// Close every event connection without a word.
    pub fn drop_subscribers(&self) {
        for sub in self.state().subs.drain(..) {
            let _ = sub.tx.send(None);
        }
    }

    pub fn snapshots(&self) -> u32 {
        self.state().snapshots
    }

    /// Wait until `n` subscriptions are live.
    pub async fn wait_subscribed(&self, n: usize) {
        for _ in 0..200 {
            if self.live_subscriptions() >= n {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        panic!("no {n} live subscriptions; saw {:?}", self.subscribes());
    }
}

async fn serve(stream: tokio::net::UnixStream, st: Arc<Mutex<State>>) {
    let mut stream = BufReader::new(stream);
    let mut line = String::new();
    if stream.read_line(&mut line).await.unwrap_or(0) == 0 {
        return;
    }
    let req: Value = serde_json::from_str(&line).expect("request json");
    let mut rx = None;
    let reply = {
        let mut s = st.lock().expect("state");
        s.requests.push(req.clone());
        match req["method"].as_str().unwrap_or_default() {
            "pane.list" => {
                let panes: Vec<Value> = s.panes.iter().map(|p| json!({"pane_id": p})).collect();
                json!({"id": req["id"], "result": {"type":"pane_list","panes": panes}})
            }
            "session.snapshot" => {
                s.snapshots += 1;
                let panes: Vec<Value> = s.panes.iter().map(|p| json!({"pane_id": p})).collect();
                json!({"id": req["id"], "result": {"type":"session_snapshot","snapshot":{
                    "focused_workspace_id":"w1","focused_tab_id":"w1:t1","focused_pane_id":"w1:p1",
                    "workspaces":[{"workspace_id":"w1"}],"tabs":[{"tab_id":"w1:t1"}],"panes":panes}}})
            }
            "events.subscribe" => {
                let entries: Vec<(String, Option<String>)> = req["params"]["subscriptions"]
                    .as_array()
                    .map_or_else(Vec::new, |a| {
                        a.iter()
                            .map(|e| {
                                (
                                    e["type"].as_str().unwrap_or_default().to_owned(),
                                    e["pane_id"].as_str().map(str::to_owned),
                                )
                            })
                            .collect()
                    });
                let missing = entries
                    .iter()
                    .find_map(|(_, p)| p.as_ref().filter(|p| !s.panes.contains(p)));
                if s.reject > 0 || missing.is_some() {
                    s.reject = s.reject.saturating_sub(1);
                    let m = missing.cloned().unwrap_or_default();
                    json!({"id": req["id"], "error": {"code":"pane_not_found","message": format!("pane {m} not found")}, "close": true})
                } else {
                    let (tx, r) = unbounded_channel();
                    s.subs.push(Sub { tx, entries });
                    rx = Some(r);
                    json!({"id": req["id"], "result": {"type":"subscription_started"}})
                }
            }
            _ => {
                json!({"id": req["id"], "error": {"code":"unsupported_method","message":"no such method"}})
            }
        }
    };
    let close = reply.get("close").is_some();
    let mut reply = reply;
    if let Some(o) = reply.as_object_mut() {
        o.remove("close");
    }
    let _ = stream
        .get_mut()
        .write_all(format!("{reply}\n").as_bytes())
        .await;
    if close {
        return;
    }
    if let Some(mut rx) = rx {
        let mut ignored = String::new();
        loop {
            tokio::select! {
                msg = rx.recv() => {
                    let Some(Some(line)) = msg else { return };
                    if stream.get_mut().write_all(format!("{line}\n").as_bytes()).await.is_err() {
                        return;
                    }
                }
                // The client hung up: the subscription ends with it.
                n = stream.read_line(&mut ignored) => {
                    if n.unwrap_or(0) == 0 {
                        return;
                    }
                }
            }
        }
    }
}
