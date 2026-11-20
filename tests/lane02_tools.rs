//! Plan 02: workspace and tab tools against a fake herdr: methods and
//! params, self-target refusals, metadata limits, gating and the log line.
mod support;
use serde_json::{Value, json};
use support::{FakeHerdr, Mcp, Opts};

const FIXTURES: &str = include_str!("fixtures/lane02.jsonl");
const INSIDE: &[(&str, &str)] = &[
    ("HERDR_ENV", "1"),
    ("HERDR_PANE_ID", "wA:p1"),
    ("HERDR_MCP_TOOLS", "session,workspaces,tabs"),
];

fn fake(tag: &str, extra: &str) -> FakeHerdr {
    let all = format!("{extra}\n{FIXTURES}");
    FakeHerdr::start_with(tag, false, all.trim())
}

async fn start(f: &FakeHerdr, args: &[&str]) -> Mcp {
    Mcp::start(&f.socket, &Opts { args, env: INSIDE }).await
}

/// Requests other than the start-up capture of this server's own pane.
fn calls(f: &FakeHerdr) -> Vec<Value> {
    f.requests()
        .into_iter()
        .filter(|r| r["method"] != "pane.current")
        .collect()
}

fn methods(f: &FakeHerdr) -> Vec<String> {
    f.requests()
        .iter()
        .map(|r| r["method"].as_str().unwrap().to_owned())
        .collect()
}

fn is_close(r: &Value) -> bool {
    r["method"] == "tab.close" || r["method"] == "workspace.close"
}

fn refused(r: &Value) -> bool {
    r["result"]["isError"] == true && r["result"]["structuredContent"]["code"] == "self_target"
}

#[tokio::test]
async fn each_new_tool_sends_its_method_and_passes_the_result_through() {
    let cases: &[(&str, Value, &str, Value)] = &[
        (
            "herdr_workspace_get",
            json!({"workspace_id":"wA"}),
            "workspace.get",
            json!({"workspace_id":"wA"}),
        ),
        (
            "herdr_workspace_focus",
            json!({"workspace_id":"wA"}),
            "workspace.focus",
            json!({"workspace_id":"wA"}),
        ),
        (
            "herdr_workspace_create",
            json!({"label":"sandbox","cwd":"/x","env":{"A":"b"},"focus":true,"source_workspace_id":"wA"}),
            "workspace.create",
            json!({"label":"sandbox","cwd":"/x","env":{"A":"b"},"focus":true,"source_workspace_id":"wA"}),
        ),
        (
            "herdr_workspace_create",
            json!({}),
            "workspace.create",
            json!({}),
        ),
        (
            "herdr_workspace_rename",
            json!({"workspace_id":"wA","label":"x"}),
            "workspace.rename",
            json!({"workspace_id":"wA","label":"x"}),
        ),
        (
            "herdr_workspace_move",
            json!({"workspace_id":"wA","insert_index":0}),
            "workspace.move",
            json!({"workspace_id":"wA","insert_index":0}),
        ),
        (
            "herdr_workspace_report_metadata",
            json!({"workspace_id":"wA","source":"claude","tokens":{"k_1":"v"},"ttl_ms":1000,"seq":3}),
            "workspace.report_metadata",
            json!({"workspace_id":"wA","source":"claude","tokens":{"k_1":"v"},"ttl_ms":1000,"seq":3}),
        ),
        (
            "herdr_tab_move",
            json!({"tab_id":"wA:t1","insert_index":1}),
            "tab.move",
            json!({"tab_id":"wA:t1","insert_index":1}),
        ),
    ];
    for (i, (tool, args, method, params)) in cases.iter().enumerate() {
        let f = fake(&format!("l2each{i}"), "");
        let mut mcp = start(&f, &[]).await;
        let r = mcp.call(tool, args.clone()).await;
        assert_eq!(r["result"]["isError"], false, "{tool}: {r}");
        let want = FIXTURES
            .lines()
            .map(|l| serde_json::from_str::<Value>(l).unwrap())
            .find(|x| x["method"] == *method && x.get("params").is_none())
            .unwrap()["result"]
            .clone();
        assert_eq!(r["result"]["structuredContent"], want, "{tool}");
        let seen = calls(&f);
        assert_eq!(seen.len(), 1, "{tool}");
        assert_eq!(seen[0]["method"], *method);
        assert_eq!(seen[0]["params"], *params, "{tool}");
    }
}

#[tokio::test]
async fn closes_are_destructive_and_other_writes_are_not() {
    let f = fake("l2ann", "");
    let mut mcp = start(&f, &[]).await;
    let r = mcp.request("tools/list", json!({})).await;
    for t in r["result"]["tools"].as_array().unwrap() {
        let name = t["name"].as_str().unwrap();
        let close = name.ends_with("_close");
        assert_eq!(t["annotations"]["destructiveHint"], close, "{name}");
        assert!(
            t.get("_meta").is_none_or(Value::is_null),
            "{name} asks without the flag: {t}"
        );
    }
}

#[tokio::test]
async fn confirm_destructive_marks_only_the_closes() {
    let f = fake("l2conf", "");
    let mut mcp = start(&f, &["--confirm-destructive"]).await;
    let r = mcp.request("tools/list", json!({})).await;
    for t in r["result"]["tools"].as_array().unwrap() {
        let name = t["name"].as_str().unwrap();
        let want = name.ends_with("_close");
        let got = t["_meta"]["anthropic/requiresUserInteraction"] == true;
        assert_eq!(got, want, "{name}: {t}");
    }
}

#[tokio::test]
async fn read_only_mode_drops_the_new_writes() {
    let f = fake("l2ro", "");
    let mut mcp = start(&f, &["--read-only"]).await;
    let names = mcp.tool_names().await;
    for gone in [
        "herdr_workspace_close",
        "herdr_workspace_create",
        "herdr_workspace_move",
        "herdr_tab_close",
        "herdr_tab_move",
    ] {
        assert!(!names.iter().any(|n| n == gone), "{gone} registered");
    }
    assert!(names.iter().any(|n| n == "herdr_workspace_get"));
    let r = mcp.call("herdr_tab_close", json!({"tab_id":"wA:t2"})).await;
    assert!(r["error"].is_object(), "{r}");
    assert!(f.requests().is_empty());
}

#[tokio::test]
async fn own_tab_and_workspace_are_refused_and_others_close() {
    let f = fake("l2self", "");
    let mut mcp = start(&f, &[]).await;

    let r = mcp.call("herdr_tab_close", json!({"tab_id":"wA:t1"})).await;
    assert!(refused(&r), "{r}");
    let r = mcp
        .call("herdr_workspace_close", json!({"workspace_id":"wA"}))
        .await;
    assert!(refused(&r), "{r}");
    assert!(
        !methods(&f)
            .iter()
            .any(|m| m == "tab.close" || m == "workspace.close")
    );

    // A neighbour tab, another workspace: fine.
    let r = mcp.call("herdr_tab_close", json!({"tab_id":"wA:t2"})).await;
    assert_eq!(r["result"]["isError"], false, "{r}");
    let r = mcp
        .call("herdr_workspace_close", json!({"workspace_id":"wB"}))
        .await;
    assert_eq!(r["result"]["isError"], false, "{r}");
    let sent: Vec<_> = f.requests().into_iter().filter(is_close).collect();
    assert_eq!(sent.len(), 2);
    assert_eq!(sent[0]["params"], json!({"tab_id":"wA:t2"}));
    assert_eq!(sent[1]["params"], json!({"workspace_id":"wB"}));
}

#[tokio::test]
async fn allow_self_lets_the_close_through_without_extra_params() {
    let f = fake("l2allow", "");
    let mut mcp = start(&f, &[]).await;
    let r = mcp
        .call(
            "herdr_tab_close",
            json!({"tab_id":"wA:t1","allow_self":true}),
        )
        .await;
    assert_eq!(r["result"]["isError"], false, "{r}");
    let r = mcp
        .call(
            "herdr_workspace_close",
            json!({"workspace_id":"wA","allow_self":true}),
        )
        .await;
    assert_eq!(r["result"]["isError"], false, "{r}");
    let closes: Vec<_> = f.requests().into_iter().filter(is_close).collect();
    assert_eq!(closes[0]["params"], json!({"tab_id":"wA:t1"}));
    assert_eq!(closes[1]["params"], json!({"workspace_id":"wA"}));
}

#[tokio::test]
async fn the_refusal_reads_clearly() {
    let f = fake("l2text", "");
    let mut mcp = start(&f, &[]).await;
    let r = mcp.call("herdr_tab_close", json!({"tab_id":"wA:t1"})).await;
    let text = r["result"]["content"][0]["text"].as_str().unwrap();
    assert!(
        text.starts_with("herdr self_target: tab wA:t1 holds"),
        "{text}"
    );
    assert!(text.contains("allow_self"), "{text}");
}

#[tokio::test]
async fn a_moved_own_pane_protects_its_new_workspace() {
    // The same terminal now sits in wB:t1: wB is protected, wA is not.
    let moved = r#"{"method":"pane.list","result":{"type":"pane_list","panes":[{"pane_id":"wB:p9","terminal_id":"term_me","workspace_id":"wB","tab_id":"wB:t1"}]}}"#;
    let f = fake("l2moved", moved);
    let mut mcp = start(&f, &[]).await;
    let r = mcp
        .call("herdr_workspace_close", json!({"workspace_id":"wB"}))
        .await;
    assert!(refused(&r), "{r}");
    let r = mcp.call("herdr_tab_close", json!({"tab_id":"wB:t1"})).await;
    assert!(refused(&r), "{r}");
    let r = mcp
        .call("herdr_workspace_close", json!({"workspace_id":"wA"}))
        .await;
    assert_eq!(r["result"]["isError"], false, "{r}");
}

#[tokio::test]
async fn a_failed_pane_list_refuses_the_close() {
    let down = r#"{"method":"pane.list","error":{"code":"internal","message":"boom"}}"#;
    let f = fake("l2down", down);
    let mut mcp = start(&f, &[]).await;
    let r = mcp.call("herdr_tab_close", json!({"tab_id":"wA:t2"})).await;
    assert_eq!(r["result"]["isError"], true, "{r}");
    assert!(!methods(&f).iter().any(|m| m == "tab.close"));
}

#[tokio::test]
async fn no_terminal_id_for_the_own_pane_refuses_the_close() {
    let bare = r#"{"method":"pane.current","params":{"caller_pane_id":"wA:p1"},"result":{"type":"pane_current","pane":{"pane_id":"wA:p1"}}}"#;
    let f = fake("l2bare", bare);
    let mut mcp = start(&f, &[]).await;
    let r = mcp.call("herdr_tab_close", json!({"tab_id":"wA:t2"})).await;
    assert!(refused(&r), "{r}");
    assert!(!methods(&f).iter().any(|m| m == "tab.close"));
}

#[tokio::test]
async fn a_gone_own_pane_has_nothing_to_protect() {
    let gone = r#"{"method":"pane.list","result":{"type":"pane_list","panes":[{"pane_id":"wB:p1","terminal_id":"term_3","workspace_id":"wB","tab_id":"wB:t1"}]}}"#;
    let f = fake("l2gone", gone);
    let mut mcp = start(&f, &[]).await;
    let r = mcp.call("herdr_tab_close", json!({"tab_id":"wA:t1"})).await;
    assert_eq!(r["result"]["isError"], false, "{r}");
}

#[tokio::test]
async fn outside_herdr_there_is_no_self() {
    let f = fake("l2out", "");
    let mut mcp = Mcp::start(
        &f.socket,
        &Opts {
            args: &["--allow-outside-herdr"],
            env: &[("HERDR_MCP_TOOLS", "session,workspaces,tabs")],
        },
    )
    .await;
    let r = mcp.call("herdr_tab_close", json!({"tab_id":"wA:t1"})).await;
    assert_eq!(r["result"]["isError"], false, "{r}");
    assert_eq!(methods(&f), ["tab.close"]);
}

#[tokio::test]
async fn a_group_close_error_passes_through_and_close_group_only_when_set() {
    let f = fake("l2group", "");
    let mut mcp = start(&f, &[]).await;
    let r = mcp
        .call("herdr_workspace_close", json!({"workspace_id":"wW"}))
        .await;
    assert_eq!(
        r["result"]["structuredContent"]["code"], "workspace_group_close_required",
        "{r}"
    );
    let close: Vec<_> = f
        .requests()
        .into_iter()
        .filter(|r| r["method"] == "workspace.close")
        .collect();
    assert_eq!(close[0]["params"], json!({"workspace_id":"wW"}));

    mcp.call(
        "herdr_workspace_close",
        json!({"workspace_id":"wW","close_group":true}),
    )
    .await;
    let close: Vec<_> = f
        .requests()
        .into_iter()
        .filter(|r| r["method"] == "workspace.close")
        .collect();
    assert_eq!(
        close[1]["params"],
        json!({"workspace_id":"wW","close_group":true})
    );
}

#[tokio::test]
async fn metadata_limits_are_refused_before_herdr() {
    let f = fake("l2meta", "");
    let mut mcp = start(&f, &[]).await;
    let many: serde_json::Map<String, Value> =
        (0..17).map(|i| (format!("k{i}"), json!("v"))).collect();
    let bad = [
        json!({"workspace_id":"wA","source":"s","tokens":many}),
        json!({"workspace_id":"wA","source":"s","tokens":{"bad key":"v"}}),
        json!({"workspace_id":"wA","source":"s","tokens":{"":"v"}}),
        json!({"workspace_id":"wA","source":"s","tokens":{"k".repeat(33):"v"}}),
        json!({"workspace_id":"wA","source":"s","tokens":{"k":"v"},"ttl_ms":0}),
        json!({"workspace_id":"wA","source":"s","tokens":{"k":"v"},"ttl_ms":86_400_001}),
    ];
    for args in bad {
        let r = mcp
            .call("herdr_workspace_report_metadata", args.clone())
            .await;
        assert_eq!(r["result"]["isError"], true, "{args}: {r}");
        assert_eq!(r["result"]["structuredContent"]["code"], "invalid_params");
    }
    assert!(calls(&f).is_empty(), "{:?}", f.requests());
    let edge: serde_json::Map<String, Value> =
        (0..16).map(|i| (format!("{i:0>32}"), json!("v"))).collect();
    let ok = mcp
        .call(
            "herdr_workspace_report_metadata",
            json!({"workspace_id":"wA","source":"s","tokens":edge}),
        )
        .await;
    assert_eq!(ok["result"]["isError"], false, "{ok}");
    let ok = mcp
        .call(
            "herdr_workspace_report_metadata",
            json!({"workspace_id":"wA","source":"s","tokens":{"A-z_9":"v"},"ttl_ms":86_400_000}),
        )
        .await;
    assert_eq!(ok["result"]["isError"], false, "{ok}");
}

#[tokio::test]
async fn the_log_names_the_close_but_not_pane_text() {
    let f = fake("l2log", "");
    let mut mcp = start(&f, &[]).await;
    mcp.call("herdr_workspace_close", json!({"workspace_id":"wB"}))
        .await;
    let err = mcp.finish().await;
    assert!(err.contains("workspace.close"), "{err}");
    assert!(!err.contains("term_3"), "pane.list body leaked: {err}");
}
