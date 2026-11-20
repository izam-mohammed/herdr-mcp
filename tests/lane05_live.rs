//! Plan 05 live tests against an isolated `herdr-mcp-test-05` session. They
//! run only with `HERDR_MCP_LIVE=1` and never touch the default session:
//! every herdr command names the test session and starts with `HERDR_*` unset.
mod support;
use serde_json::{Value, json};
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use support::{Mcp, Opts};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

const SESSION: &str = "herdr-mcp-test-05";

fn herdr() -> Command {
    let mut cmd = Command::new("herdr");
    for (k, _) in std::env::vars() {
        if k.starts_with("HERDR_") {
            cmd.env_remove(k);
        }
    }
    cmd.args(["--session", SESSION]);
    cmd
}

/// The isolated server; stopped and deleted on drop, pass or fail.
struct TestSession {
    server: Option<Child>,
    socket: std::path::PathBuf,
}

impl TestSession {
    fn socket_path() -> std::path::PathBuf {
        let home = std::env::var("HOME").expect("HOME");
        format!("{home}/.config/herdr/sessions/{SESSION}/herdr.sock").into()
    }

    async fn start() -> Self {
        let mut session = Self {
            server: None,
            socket: Self::socket_path(),
        };
        session.boot().await;
        session
    }

    async fn boot(&mut self) {
        self.server = Some(
            herdr()
                .arg("server")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .expect("start herdr server"),
        );
        for _ in 0..100 {
            if raw(&self.socket, "ping", json!({})).await.is_ok() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        panic!("the isolated herdr server did not come up");
    }

    /// Stop the server (not the session's files), as an update or crash would.
    fn stop_server(&mut self) {
        let _ = herdr().args(["server", "stop"]).output();
        if let Some(mut s) = self.server.take() {
            let _ = s.wait();
        }
    }
}

impl Drop for TestSession {
    fn drop(&mut self) {
        self.stop_server();
        let _ = Command::new("herdr")
            .args(["session", "delete", SESSION])
            .output();
    }
}

/// One raw request, outside the server under test.
async fn raw(socket: &std::path::Path, method: &str, params: Value) -> Result<Value, String> {
    let mut s = tokio::net::UnixStream::connect(socket)
        .await
        .map_err(|e| e.to_string())?;
    let line = format!("{}\n", json!({"id":"t","method":method,"params":params}));
    s.write_all(line.as_bytes())
        .await
        .map_err(|e| e.to_string())?;
    let mut reply = String::new();
    BufReader::new(s)
        .read_line(&mut reply)
        .await
        .map_err(|e| e.to_string())?;
    let v: Value = serde_json::from_str(&reply).map_err(|e| e.to_string())?;
    v.get("result").cloned().ok_or(reply)
}

fn live() -> bool {
    std::env::var("HERDR_MCP_LIVE").is_ok_and(|v| v == "1")
}

fn ok(r: &Value) -> &Value {
    assert_eq!(r["result"]["isError"], false, "{r}");
    &r["result"]["structuredContent"]
}

async fn mcp(session: &TestSession) -> Mcp {
    let home = std::env::var("HOME").expect("HOME");
    Mcp::start(
        &session.socket,
        &Opts {
            args: &["--session", SESSION],
            env: &[
                ("HERDR_ENV", "1"),
                ("HERDR_MCP_TOOLS", "events,tabs"),
                ("HOME", &home),
            ],
        },
    )
    .await
}

/// Poll `herdr_events_recent` from `since` until `want` matches an event.
async fn find_event(mcp: &mut Mcp, since: u64, want: impl Fn(&Value) -> bool) -> Value {
    for _ in 0..100 {
        let r = mcp
            .call("herdr_events_recent", json!({"since": since, "limit": 500}))
            .await;
        if let Some(e) = ok(&r)["events"]
            .as_array()
            .and_then(|a| a.iter().find(|e| want(e)))
        {
            return e.clone();
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("the event never showed up");
}

async fn status(mcp: &mut Mcp) -> Value {
    ok(&mcp.call("herdr_events_status", json!({})).await).clone()
}

async fn wait_connected(mcp: &mut Mcp) {
    for _ in 0..200 {
        if status(mcp).await["connected"] == true {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("the hub never connected");
}

#[tokio::test]
async fn a_tab_created_in_the_session_shows_up_in_recent_and_wait() {
    if !live() {
        eprintln!("skipped: set HERDR_MCP_LIVE=1");
        return;
    }
    let session = TestSession::start().await;
    raw(
        &session.socket,
        "workspace.create",
        json!({"label":"live","focus":true}),
    )
    .await
    .expect("workspace.create");
    let mut mcp = mcp(&session).await;
    wait_connected(&mut mcp).await;
    let before = status(&mut mcp).await["last_seq"].as_u64().expect("seq");

    // A wait started before the change ends with it.
    let creator = {
        let socket = session.socket.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(500)).await;
            raw(&socket, "tab.create", json!({"label":"scratch"}))
                .await
                .expect("tab.create")
        })
    };
    let result = mcp
        .call(
            "herdr_events_wait",
            json!({"types": ["tab_created"], "timeout_ms": 10000}),
        )
        .await;
    let got = ok(&result);
    assert_eq!(got["timed_out"], false, "{got}");
    let created = creator.await.expect("tab.create task");
    assert_eq!(
        got["event"]["data"]["tab"]["tab_id"],
        created["tab"]["tab_id"]
    );

    let e = find_event(&mut mcp, before, |e| e["event"] == "tab_created").await;
    assert_eq!(e["data"]["tab"]["label"], "scratch");
    let st = status(&mut mcp).await;
    assert_eq!(st["connected"], true);
    assert!(st["last_event_at"].is_u64());
}
