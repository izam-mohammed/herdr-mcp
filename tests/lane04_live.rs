//! Plan 04 live test against an isolated `herdr-mcp-test-04` session, only
//! with `HERDR_MCP_LIVE=1`. A stub executable named `claude` stands in for a
//! real agent; herdr detects it by process name. Never touches the default
//! session.
mod support;
use serde_json::{Value, json};
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use support::{Mcp, Opts};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

const SESSION: &str = "herdr-mcp-test-04";

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

fn ok(r: &Value) -> &Value {
    assert_eq!(r["result"]["isError"], false, "{r}");
    &r["result"]["structuredContent"]
}

/// The pane's shell may still be starting, and herdr refuses a rename while
/// startup is pending: retry both until they settle.
async fn start_and_rename(mcp: &mut Mcp, pane: &str) {
    let mut started = Value::Null;
    for _ in 0..50 {
        started = mcp
            .call(
                "herdr_agent_start",
                json!({"name":"stub","kind":"claude","pane_id":pane,"timeout_ms":10000}),
            )
            .await;
        if started["result"]["isError"] == false {
            break;
        }
        assert_eq!(
            started["result"]["structuredContent"]["code"],
            "agent_pane_busy"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert_eq!(ok(&started)["type"], "agent_started");

    let mut renamed = false;
    for _ in 0..50 {
        let r = mcp
            .call(
                "herdr_agent_rename",
                json!({"target": pane, "name": "stubby"}),
            )
            .await;
        if r["result"]["isError"] == false {
            renamed = true;
            break;
        }
        assert_eq!(
            r["result"]["structuredContent"]["code"],
            "agent_launch_pending"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(renamed, "rename never settled");
}

#[tokio::test]
async fn agents_start_read_wait_rename_and_the_own_agent_is_refused() {
    if std::env::var("HERDR_MCP_LIVE").as_deref() != Ok("1") {
        eprintln!("skipped: set HERDR_MCP_LIVE=1");
        return;
    }
    let dir = std::env::temp_dir().join(format!("herdr-mcp-lane04-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("stub dir");
    let stub = dir.join("claude");
    std::fs::write(
        &stub,
        "#!/bin/sh\necho stub agent ready\nwhile read l; do echo \"got: $l\"; done\n",
    )
    .expect("stub");
    Command::new("chmod")
        .arg("+x")
        .arg(&stub)
        .status()
        .expect("chmod");

    let session = TestSession::start().await;
    let ws = raw(
        &session.socket,
        "workspace.create",
        json!({"label":"live","focus":true}),
    )
    .await
    .expect("workspace.create");
    let pane = ws["root_pane"]["pane_id"]
        .as_str()
        .expect("pane")
        .to_owned();
    raw(
        &session.socket,
        "pane.send_input",
        json!({"pane_id": pane, "text": format!("export PATH={}:$PATH", dir.display()), "keys": ["Enter"]}),
    )
    .await
    .expect("export PATH");

    let home = std::env::var("HOME").expect("HOME");
    let mut mcp = Mcp::start(
        &session.socket,
        &Opts {
            args: &["--session", SESSION],
            env: &[
                ("HERDR_ENV", "1"),
                ("HERDR_PANE_ID", &pane),
                ("HOME", &home),
            ],
        },
    )
    .await;

    // Range check happens before herdr.
    let bad = mcp
        .call(
            "herdr_agent_start",
            json!({"name":"stub","kind":"claude","pane_id":pane,"timeout_ms":3000}),
        )
        .await;
    assert_eq!(
        bad["result"]["structuredContent"]["code"],
        "invalid_agent_timeout"
    );

    start_and_rename(&mut mcp, &pane).await;
    let got = mcp
        .call("herdr_agent_get", json!({"target":"stubby"}))
        .await;
    assert_eq!(ok(&got)["agent"]["pane_id"], pane.as_str());
    ok(&mcp
        .call("herdr_agent_explain", json!({"target":"stubby"}))
        .await);
    ok(&mcp
        .call("herdr_agent_focus", json!({"target":"stubby"}))
        .await);

    let read = mcp
        .call("herdr_agent_read", json!({"target":"stubby","lines":20}))
        .await;
    assert!(
        ok(&read)["read"]["text"]
            .as_str()
            .is_some_and(|t| t.contains("stub agent ready")),
        "{read}"
    );

    let waited = mcp
        .call(
            "herdr_agent_wait",
            json!({"target":"stubby","timeout_ms":1500}),
        )
        .await;
    assert_eq!(waited["result"]["isError"], false, "{waited}");

    // The server runs "inside" that pane, so this agent is its own.
    for tool in ["herdr_agent_prompt", "herdr_agent_send_keys"] {
        let args = if tool == "herdr_agent_prompt" {
            json!({"target":"stubby","text":"x"})
        } else {
            json!({"target":"stubby","keys":["Enter"]})
        };
        let r = mcp.call(tool, args).await;
        assert_eq!(r["result"]["structuredContent"]["code"], "own_agent", "{r}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}
