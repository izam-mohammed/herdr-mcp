//! Plan 07 live tests against an isolated `herdr-mcp-test-07` session. They run only
//! with `HERDR_MCP_LIVE=1` and never touch the default session: every herdr
//! command here names the test session and starts with `HERDR_*` unset.
mod support;
use serde_json::{Value, json};
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use support::{Mcp, Opts};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

const SESSION: &str = "herdr-mcp-test-07";

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

fn contents(r: &Value) -> Value {
    let c = &r["result"]["contents"][0];
    assert_eq!(c["mimeType"], "application/json", "{r}");
    serde_json::from_str(c["text"].as_str().expect("text")).expect("json text")
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn resources_and_prompts_over_a_real_herdr() {
    if !live() {
        eprintln!("skipped: set HERDR_MCP_LIVE=1");
        return;
    }
    let session = TestSession::start().await;
    let ws = raw(
        &session.socket,
        "workspace.create",
        json!({"label":"live07","focus":true}),
    )
    .await
    .expect("workspace.create");
    let wid = ws["workspace"]["workspace_id"]
        .as_str()
        .expect("ws")
        .to_owned();
    let tab = ws["tab"]["tab_id"].as_str().expect("tab").to_owned();
    let pane = ws["root_pane"]["pane_id"]
        .as_str()
        .expect("pane")
        .to_owned();
    raw(
        &session.socket,
        "pane.send_text",
        json!({"pane_id": pane, "text": "echo live-$((3+4))\n"}),
    )
    .await
    .expect("send_text");

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

    let list = mcp.request("resources/list", json!({})).await;
    let uris: Vec<&str> = list["result"]["resources"]
        .as_array()
        .expect("resources")
        .iter()
        .filter_map(|r| r["uri"].as_str())
        .collect();
    assert!(uris.contains(&"herdr://session") && uris.contains(&"herdr://events/recent"));

    let snap = contents(
        &mcp.request("resources/read", json!({"uri":"herdr://session"}))
            .await,
    );
    assert!(snap["snapshot"]["workspaces"].is_array(), "{snap}");
    let list = contents(
        &mcp.request("resources/read", json!({"uri":"herdr://workspaces"}))
            .await,
    );
    assert!(list["workspaces"].to_string().contains(&wid), "{list}");
    let got = contents(
        &mcp.request(
            "resources/read",
            json!({"uri": format!("herdr://workspace/{wid}")}),
        )
        .await,
    );
    assert_eq!(got["workspace"]["label"], "live07", "{got}");
    let got = contents(
        &mcp.request(
            "resources/read",
            json!({"uri": format!("herdr://tab/{tab}")}),
        )
        .await,
    );
    assert_eq!(got["tab"]["tab_id"], tab.as_str(), "{got}");
    let agents = contents(
        &mcp.request("resources/read", json!({"uri":"herdr://agents"}))
            .await,
    );
    assert!(agents["agents"].is_array(), "{agents}");
    let events = contents(
        &mcp.request("resources/read", json!({"uri":"herdr://events/recent"}))
            .await,
    );
    assert!(events["events"].is_array(), "{events}");

    // The shell needs a moment to run the typed command.
    let mut out = String::new();
    for _ in 0..50 {
        let r = mcp
            .request(
                "resources/read",
                json!({"uri": format!("herdr://pane/{pane}/output")}),
            )
            .await;
        out = contents(&r)["read"]["text"]
            .as_str()
            .unwrap_or_default()
            .to_owned();
        if out.contains("live-7") {
            break;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    assert!(out.contains("live-7"), "{out}");

    let missing = mcp
        .request("resources/read", json!({"uri":"herdr://workspace/wNOPE"}))
        .await;
    assert_eq!(missing["error"]["code"], -32002, "{missing}");

    let p = mcp
        .request("prompts/get", json!({"name":"dashboard"}))
        .await;
    assert!(
        p["result"]["messages"][0]["content"]["text"]
            .as_str()
            .unwrap()
            .contains("herdr_snapshot")
    );
    mcp.finish().await;
}
