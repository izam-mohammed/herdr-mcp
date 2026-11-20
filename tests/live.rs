//! Live tests against an isolated `herdr-mcp-test` session. They run only
//! with `HERDR_MCP_LIVE=1` and never touch the default session: every herdr
//! command here names the test session and starts with `HERDR_*` unset.
mod support;
use serde_json::{Value, json};
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use support::{Mcp, Opts};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

const SESSION: &str = "herdr-mcp-test";

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

#[tokio::test]
async fn tabs_create_rename_focus_and_list_agree_with_the_snapshot() {
    if !live() {
        eprintln!("skipped: set HERDR_MCP_LIVE=1");
        return;
    }
    let session = TestSession::start().await;
    let ws = raw(
        &session.socket,
        "workspace.create",
        json!({"label":"live","focus":true}),
    )
    .await
    .expect("workspace.create");
    let ws_id = ws["workspace"]["workspace_id"]
        .as_str()
        .expect("workspace id")
        .to_owned();

    // The flag, not HERDR_SOCKET_PATH, picks the session.
    let home = std::env::var("HOME").expect("HOME");
    let mut mcp = Mcp::start(
        &session.socket,
        &Opts {
            args: &["--session", SESSION],
            env: &[("HERDR_ENV", "1"), ("HOME", &home)],
        },
    )
    .await;

    let status = mcp.call("herdr_status", json!({})).await;
    assert_eq!(ok(&status)["protocol"], 22);

    let created = mcp
        .call(
            "herdr_tab_create",
            json!({"workspace_id": ws_id, "label": "scratch"}),
        )
        .await;
    let tab_id = ok(&created)["tab"]["tab_id"]
        .as_str()
        .expect("tab id")
        .to_owned();

    ok(&mcp
        .call(
            "herdr_tab_rename",
            json!({"tab_id": tab_id, "label": "renamed"}),
        )
        .await);
    ok(&mcp.call("herdr_tab_focus", json!({"tab_id": tab_id})).await);

    let list = mcp
        .call("herdr_tab_list", json!({"workspace_id": ws_id}))
        .await;
    let tab = ok(&list)["tabs"]
        .as_array()
        .expect("tabs")
        .iter()
        .find(|t| t["tab_id"] == tab_id.as_str())
        .cloned()
        .expect("created tab is listed");
    assert_eq!(tab["label"], "renamed");
    assert_eq!(tab["focused"], true);

    let get = mcp.call("herdr_tab_get", json!({"tab_id": tab_id})).await;
    assert_eq!(ok(&get)["tab"]["label"], "renamed");

    let snap = mcp.call("herdr_snapshot", json!({})).await;
    let snap = &ok(&snap)["snapshot"];
    assert_eq!(snap["focused_tab_id"], tab_id.as_str());
    let in_snap = snap["tabs"]
        .as_array()
        .expect("snapshot tabs")
        .iter()
        .find(|t| t["tab_id"] == tab_id.as_str())
        .expect("snapshot lists the tab");
    assert_eq!(in_snap["label"], "renamed");

    let workspaces = mcp.call("herdr_workspace_list", json!({})).await;
    assert!(
        !ok(&workspaces)["workspaces"]
            .as_array()
            .expect("workspaces")
            .is_empty()
    );
    ok(&mcp.call("herdr_agent_list", json!({})).await);
}
