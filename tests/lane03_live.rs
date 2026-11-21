//! Plan 03 live tests against an isolated `herdr-mcp-test-03` session. They run only
//! with `HERDR_MCP_LIVE=1` and never touch the default session: every herdr
//! command here names the test session and starts with `HERDR_*` unset.
mod support;
use serde_json::{Value, json};
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use support::{Mcp, Opts};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

const SESSION: &str = "herdr-mcp-test-03";

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

fn text_of(v: &Value) -> &str {
    v.as_str().expect("string")
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn run_wait_read_arrange_and_close_in_a_real_herdr() {
    if !live() {
        eprintln!("skipped: set HERDR_MCP_LIVE=1");
        return;
    }
    let session = TestSession::start().await;
    let ws = raw(
        &session.socket,
        "workspace.create",
        json!({"label":"live03","focus":true}),
    )
    .await
    .expect("workspace.create");
    let own = text_of(&ws["root_pane"]["pane_id"]).to_owned();
    let tab = text_of(&ws["tab"]["tab_id"]).to_owned();

    let home = std::env::var("HOME").expect("HOME");
    let mut mcp = Mcp::start(
        &session.socket,
        &Opts {
            args: &["--session", SESSION],
            env: &[("HERDR_ENV", "1"), ("HERDR_PANE_ID", &own), ("HOME", &home)],
        },
    )
    .await;

    // Run in a new pane to the right, wait for the printed result, read it back.
    // The typed command holds `$((1+1))`, so only its output holds `herdr-2`.
    let run = mcp
        .call(
            "herdr_pane_run",
            json!({"command":"echo herdr-$((1+1))","wait":{"match":{"type":"substring","value":"herdr-2"},"timeout_ms":15000}}),
        )
        .await;
    let run = ok(&run);
    let pane = text_of(&run["pane_id"]).to_owned();
    assert_ne!(pane, own);
    assert!(
        run["matched"]["read"]["text"]
            .as_str()
            .unwrap()
            .contains("herdr-2"),
        "{run}"
    );
    assert!(run.get("timed_out").is_none());

    let read = mcp
        .call("herdr_pane_read", json!({"pane_id": pane, "lines": 20}))
        .await;
    assert!(text_of(&ok(&read)["read"]["text"]).contains("herdr-2"));

    // A wait that never matches ends with timed_out, not an error.
    let none = mcp
        .call(
            "herdr_pane_wait_output",
            json!({"pane_id": pane, "match":{"type":"substring","value":"never-printed-xyz"},"timeout_ms":500}),
        )
        .await;
    assert_eq!(ok(&none)["timed_out"], true);

    // A bad regex is herdr's own error.
    let bad = mcp
        .call(
            "herdr_pane_wait_output",
            json!({"pane_id": pane, "match":{"type":"regex","value":"("},"timeout_ms":500}),
        )
        .await;
    assert_eq!(bad["result"]["isError"], true);
    assert_eq!(bad["result"]["structuredContent"]["code"], "invalid_regex");

    // Typing: text then keys.
    ok(&mcp
        .call(
            "herdr_pane_send_text",
            json!({"pane_id": pane, "text":"echo typed-$((2+3))"}),
        )
        .await);
    ok(&mcp
        .call(
            "herdr_pane_send_keys",
            json!({"pane_id": pane, "keys":["enter"]}),
        )
        .await);
    let w = mcp
        .call(
            "herdr_pane_wait_output",
            json!({"pane_id": pane, "match":{"type":"substring","value":"typed-5"},"timeout_ms":15000}),
        )
        .await;
    assert!(ok(&w).get("timed_out").is_none(), "{w}");

    // Inspect and arrange.
    let list = mcp.call("herdr_pane_list", json!({"tab_id": tab})).await;
    assert_eq!(ok(&list)["panes"].as_array().unwrap().len(), 2);
    ok(&mcp.call("herdr_pane_get", json!({"pane_id": pane})).await);
    let layout = mcp
        .call(
            "herdr_pane_layout",
            json!({"pane_id": pane, "direction":"left"}),
        )
        .await;
    assert_eq!(ok(&layout)["neighbor"]["pane_id"], own.as_str());
    assert_eq!(ok(&layout)["edges"]["right"], true);
    let info = mcp
        .call("herdr_pane_process_info", json!({"pane_id": pane}))
        .await;
    assert!(ok(&info)["process_info"]["foreground_processes"].is_array());
    ok(&mcp
        .call(
            "herdr_pane_rename",
            json!({"pane_id": pane, "label":"worker"}),
        )
        .await);
    ok(&mcp
        .call(
            "herdr_pane_focus",
            json!({"direction":"left","pane_id": pane}),
        )
        .await);
    ok(&mcp.call("herdr_pane_focus", json!({"pane_id": pane})).await);
    ok(&mcp.call("herdr_pane_zoom", json!({"pane_id": pane})).await);
    ok(&mcp.call("herdr_pane_zoom", json!({"pane_id": pane})).await);
    ok(&mcp
        .call(
            "herdr_pane_resize",
            json!({"pane_id": pane, "direction":"left", "amount": 0.05}),
        )
        .await);
    ok(&mcp
        .call(
            "herdr_pane_swap",
            json!({"source_pane_id": pane, "target_pane_id": own}),
        )
        .await);
    ok(&mcp.call("herdr_pane_clear", json!({"pane_id": pane})).await);

    // The own pane is refused, before herdr is asked.
    for (tool, args) in [
        ("herdr_pane_send_text", json!({"pane_id": own, "text":"x"})),
        ("herdr_pane_close", json!({"pane_id": own})),
        ("herdr_pane_clear", json!({"pane_id": own})),
    ] {
        let r = mcp.call(tool, args).await;
        assert_eq!(
            r["result"]["structuredContent"]["code"], "self_target",
            "{tool}"
        );
    }

    // Split, move to a new tab, then close.
    let split = mcp
        .call(
            "herdr_pane_split",
            json!({"direction":"down","target_pane_id": pane}),
        )
        .await;
    let extra = text_of(&ok(&split)["pane"]["pane_id"]).to_owned();
    let moved = mcp
        .call(
            "herdr_pane_move",
            json!({"pane_id": extra, "destination":{"type":"new_tab","workspace_id": text_of(&ws["workspace"]["workspace_id"]),"label":"moved"}}),
        )
        .await;
    let moved_id = text_of(&ok(&moved)["move_result"]["pane"]["pane_id"]).to_owned();
    ok(&mcp
        .call("herdr_pane_close", json!({"pane_id": moved_id}))
        .await);
    ok(&mcp.call("herdr_pane_close", json!({"pane_id": pane})).await);
    let left = raw(&session.socket, "pane.list", json!({}))
        .await
        .expect("pane.list");
    assert!(
        left["panes"]
            .as_array()
            .unwrap()
            .iter()
            .all(|p| p["pane_id"] != pane.as_str())
    );
}
