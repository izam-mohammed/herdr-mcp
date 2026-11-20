//! Plan 02 live tests against an isolated `herdr-mcp-test-02` session. They run only
//! with `HERDR_MCP_LIVE=1` and never touch the default session: every herdr
//! command here names the test session and starts with `HERDR_*` unset.
mod support;
use serde_json::{Value, json};
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use support::{Mcp, Opts};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

const SESSION: &str = "herdr-mcp-test-02";

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
    server: Child,
    socket: std::path::PathBuf,
}

impl TestSession {
    async fn start() -> Self {
        let home = std::env::var("HOME").expect("HOME");
        let socket = format!("{home}/.config/herdr/sessions/{SESSION}/herdr.sock").into();
        let server = herdr()
            .arg("server")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("start herdr server");
        let session = Self { server, socket };
        for _ in 0..100 {
            if raw(&session.socket, "ping", json!({})).await.is_ok() {
                return session;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        panic!("the isolated herdr server did not come up");
    }
}

impl Drop for TestSession {
    fn drop(&mut self) {
        let _ = herdr().args(["server", "stop"]).output();
        let _ = self.server.wait();
        let _ = Command::new("herdr")
            .args(["session", "delete", SESSION])
            .output();
        // The next test must not start while this server is still going down.
        for _ in 0..100 {
            if !self.socket.exists() {
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
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

async fn session_with_own_pane() -> (TestSession, String, String) {
    let session = TestSession::start().await;
    let ws = raw(
        &session.socket,
        "workspace.create",
        json!({"label":"home","focus":true}),
    )
    .await
    .expect("workspace.create");
    let pane = ws["root_pane"]["pane_id"]
        .as_str()
        .expect("pane")
        .to_owned();
    let ws_id = ws["workspace"]["workspace_id"]
        .as_str()
        .expect("ws")
        .to_owned();
    (session, pane, ws_id)
}

async fn mcp_in(session: &TestSession, pane: &str) -> Mcp {
    let home = std::env::var("HOME").expect("HOME");
    Mcp::start(
        &session.socket,
        &Opts {
            args: &["--session", SESSION],
            env: &[("HERDR_ENV", "1"), ("HERDR_PANE_ID", pane), ("HOME", &home)],
        },
    )
    .await
}

fn is_refused(r: &Value) -> bool {
    r["result"]["isError"] == true && r["result"]["structuredContent"]["code"] == "self_target"
}

async fn create_move_and_close_workspaces_and_tabs_and_never_the_own_ones() {
    let (session, pane, home_ws) = session_with_own_pane().await;
    let mut mcp = mcp_in(&session, &pane).await;

    let created = mcp
        .call(
            "herdr_workspace_create",
            json!({"label":"sandbox","focus":false}),
        )
        .await;
    let sandbox = ok(&created)["workspace"]["workspace_id"]
        .as_str()
        .expect("workspace id")
        .to_owned();
    ok(&mcp
        .call(
            "herdr_workspace_rename",
            json!({"workspace_id":sandbox,"label":"sandbox2"}),
        )
        .await);
    let got = mcp
        .call("herdr_workspace_get", json!({"workspace_id":sandbox}))
        .await;
    assert_eq!(ok(&got)["workspace"]["label"], "sandbox2");
    ok(&mcp
        .call(
            "herdr_workspace_move",
            json!({"workspace_id":sandbox,"insert_index":0}),
        )
        .await);
    ok(&mcp
        .call("herdr_workspace_focus", json!({"workspace_id":sandbox}))
        .await);
    ok(&mcp
        .call(
            "herdr_workspace_report_metadata",
            json!({"workspace_id":sandbox,"source":"claude","tokens":{"k":"v"},"ttl_ms":60000}),
        )
        .await);

    let t2 = mcp
        .call(
            "herdr_tab_create",
            json!({"workspace_id":sandbox,"label":"two"}),
        )
        .await;
    let t2 = ok(&t2)["tab"]["tab_id"].as_str().expect("tab").to_owned();
    let moved = mcp
        .call("herdr_tab_move", json!({"tab_id":t2,"insert_index":0}))
        .await;
    assert_eq!(ok(&moved)["tabs"][0]["tab_id"], t2.as_str());
    ok(&mcp.call("herdr_tab_close", json!({"tab_id":t2})).await);
    ok(&mcp
        .call("herdr_workspace_close", json!({"workspace_id":sandbox}))
        .await);
    let list = mcp.call("herdr_workspace_list", json!({})).await;
    let ids: Vec<&Value> = ok(&list)["workspaces"]
        .as_array()
        .expect("workspaces")
        .iter()
        .map(|w| &w["workspace_id"])
        .collect();
    assert_eq!(ids, [home_ws.as_str()]);

    // The own tab and workspace are refused; nothing is closed.
    let own_tab = format!("{home_ws}:t1");
    let r = mcp.call("herdr_tab_close", json!({"tab_id":own_tab})).await;
    assert!(is_refused(&r), "{r}");
    let r = mcp
        .call("herdr_workspace_close", json!({"workspace_id":home_ws}))
        .await;
    assert!(is_refused(&r), "{r}");
    let list = mcp.call("herdr_workspace_list", json!({})).await;
    assert_eq!(ok(&list)["workspaces"].as_array().expect("ws").len(), 1);
}

async fn a_moved_own_pane_is_still_protected_in_its_new_workspace() {
    let (session, pane, home_ws) = session_with_own_pane().await;
    let mut mcp = mcp_in(&session, &pane).await;
    let term = raw(
        &session.socket,
        "pane.current",
        json!({"caller_pane_id":pane}),
    )
    .await
    .expect("pane.current")["pane"]["terminal_id"]
        .as_str()
        .expect("terminal id")
        .to_owned();

    // The first close attempt captures the terminal id.
    let r = mcp
        .call("herdr_tab_close", json!({"tab_id":format!("{home_ws}:t1")}))
        .await;
    assert!(is_refused(&r), "{r}");

    // Move the pane to a new workspace: it gets a new public pane id.
    raw(
        &session.socket,
        "pane.move",
        json!({"pane_id":pane,"destination":{"type":"new_workspace","label":"moved","tab_label":"m"},"focus":false}),
    )
    .await
    .expect("pane.move");
    let panes = raw(&session.socket, "pane.list", json!({}))
        .await
        .expect("pane.list");
    let mine = panes["panes"]
        .as_array()
        .expect("panes")
        .iter()
        .find(|p| p["terminal_id"] == term.as_str())
        .expect("own pane still listed");
    let (new_ws, new_tab) = (
        mine["workspace_id"].as_str().expect("ws").to_owned(),
        mine["tab_id"].as_str().expect("tab").to_owned(),
    );
    assert_ne!(new_ws, home_ws, "the pane did move");

    let r = mcp
        .call("herdr_workspace_close", json!({"workspace_id":new_ws}))
        .await;
    assert!(is_refused(&r), "{r}");
    let r = mcp.call("herdr_tab_close", json!({"tab_id":new_tab})).await;
    assert!(is_refused(&r), "{r}");
    let list = mcp.call("herdr_workspace_list", json!({})).await;
    let found = ok(&list)["workspaces"]
        .as_array()
        .expect("ws")
        .iter()
        .any(|w| w["workspace_id"] == new_ws.as_str());
    assert!(found, "the protected workspace is still there");
}

/// One test, so the two scenarios never share the session in parallel.
#[tokio::test]
async fn workspaces_and_tabs_live() {
    if !live() {
        eprintln!("skipped: set HERDR_MCP_LIVE=1");
        return;
    }
    create_move_and_close_workspaces_and_tabs_and_never_the_own_ones().await;
    a_moved_own_pane_is_still_protected_in_its_new_workspace().await;
}
