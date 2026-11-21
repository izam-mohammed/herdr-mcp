//! Plan 13 Phase 4 live test against an isolated `herdr-mcp-test-13` session: the admin
//! tools that only read (`herdr_config_check`, `herdr_session_list`, `herdr_channel_show`,
//! `herdr_machine_list`, `herdr_integration_list`, `herdr_server_agent_manifests`) answer
//! over a real herdr. Runs only with `HERDR_MCP_LIVE=1`; it never stops, deletes or
//! changes anything but its own session, which the guard stops and deletes at the end.
//! Every herdr command here names the test session and starts with `HERDR_*` unset.
mod support;
use serde_json::{Value, json};
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use support::{Mcp, Opts};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

const SESSION: &str = "herdr-mcp-test-13";

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

#[tokio::test]
async fn admin_reads_over_a_real_herdr() {
    if !live() {
        eprintln!("skipped: set HERDR_MCP_LIVE=1");
        return;
    }
    let session = TestSession::start().await;
    let home = std::env::var("HOME").expect("HOME");
    let path = std::env::var("PATH").expect("PATH");
    let mut mcp = Mcp::start(
        &session.socket,
        &Opts {
            args: &["--session", SESSION, "--tools", "admin"],
            env: &[("HERDR_ENV", "1"), ("HOME", &home), ("PATH", &path)],
        },
    )
    .await;
    let sc = |r: &Value| r["result"]["structuredContent"].clone();

    let r = mcp.call("herdr_config_check", json!({})).await;
    assert_eq!(r["result"]["isError"], false, "{r}");
    assert_eq!(sc(&r)["exit"], 0, "{r}");
    assert!(sc(&r)["stdout"].as_str().unwrap().contains("ok"), "{r}");
    eprintln!("config check: {}", sc(&r)["stdout"]);

    let r = mcp.call("herdr_session_list", json!({})).await;
    assert_eq!(sc(&r)["exit"], 0, "{r}");
    let listed: Value = serde_json::from_str(sc(&r)["stdout"].as_str().unwrap()).expect("json");
    let ours = listed["sessions"]
        .as_array()
        .expect("sessions")
        .iter()
        .find(|s| s["name"] == SESSION)
        .unwrap_or_else(|| panic!("{SESSION} not listed: {listed}"));
    assert_eq!(ours["running"], true);
    eprintln!("session list names {SESSION}, running");

    let r = mcp.call("herdr_channel_show", json!({})).await;
    assert_eq!(sc(&r)["exit"], 0, "{r}");
    let channel = sc(&r)["stdout"].as_str().unwrap().trim().to_owned();
    assert!(
        ["stable", "preview"].contains(&channel.as_str()),
        "{channel}"
    );
    eprintln!("channel: {channel}");

    let r = mcp.call("herdr_machine_list", json!({})).await;
    assert_eq!(sc(&r)["exit"], 0, "{r}");
    serde_json::from_str::<Value>(sc(&r)["stdout"].as_str().unwrap()).expect("machine json");

    let r = mcp.call("herdr_integration_list", json!({})).await;
    assert_eq!(r["result"]["isError"], false, "{r}");
    assert!(sc(&r)["integrations"].is_array(), "{r}");

    let r = mcp.call("herdr_server_agent_manifests", json!({})).await;
    assert_eq!(r["result"]["isError"], false, "{r}");
    eprintln!("integration list and agent manifests answered");
    mcp.finish().await;
}
