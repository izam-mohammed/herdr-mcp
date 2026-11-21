//! Plan 13 Phase 2: the six harmless additions against a fake herdr: method and
//! params sent, herdr's limits checked first, the selection text capped, and
//! the read-only list.
mod support;
use serde_json::{Value, json};
use support::{FakeHerdr, Mcp, Opts};

const FIXTURES: &str = r#"{"method":"pane.scroll","result":{"type":"ok"}}
{"method":"pane.report_metadata","result":{"type":"ok"}}
{"method":"agent.view.set","result":{"type":"ok"}}
{"method":"agent.view.clear","result":{"type":"ok"}}
{"method":"workspace.move_block","result":{"type":"ok"}}
{"method":"pane.selection.read","params":{"pane_id":"wA:p2","anchor":{"row":0,"col":0},"cursor":{"row":1,"col":5}},"result":{"type":"pane_selection","selection":{"text":"hello"}}}
{"method":"pane.selection.read","params":{"pane_id":"wA:p3","anchor":{"row":0,"col":0},"cursor":{"row":1,"col":5}},"result":{"type":"pane_selection","selection":{"text":"@@BIG@@"}}}"#;

const ENV: &[(&str, &str)] = &[
    ("HERDR_ENV", "1"),
    ("HERDR_PANE_ID", "wA:p1"),
    ("HERDR_MCP_TOOLS", "panes,agents,workspaces"),
];

async fn start(tag: &str) -> (FakeHerdr, Mcp) {
    let big = format!("{}END", "x".repeat(50_000));
    let fake = FakeHerdr::start_with(tag, false, &FIXTURES.replace("@@BIG@@", &big));
    let mcp = Mcp::start(
        &fake.socket,
        &Opts {
            args: &[],
            env: ENV,
        },
    )
    .await;
    (fake, mcp)
}

fn sc(r: &Value) -> &Value {
    &r["result"]["structuredContent"]
}

#[tokio::test]
async fn each_tool_sends_its_method_and_params() {
    let (fake, mut mcp) = start("l13-send").await;
    let cases = [
        (
            "herdr_pane_scroll",
            json!({"pane_id":"wA:p2","offset_from_bottom":12}),
            "pane.scroll",
            json!({"pane_id":"wA:p2","offset_from_bottom":12}),
        ),
        (
            "herdr_pane_report_metadata",
            json!({"pane_id":"wA:p2","source":"s","title":"t","tokens":{"a-1":"x","b":null},"ttl_ms":5}),
            "pane.report_metadata",
            json!({"pane_id":"wA:p2","source":"s","title":"t","tokens":{"a-1":"x","b":null},"ttl_ms":5}),
        ),
        (
            "herdr_agent_view_set",
            json!({"source":"s","filter":{"op":"eq","field":"status","value":"blocked"},"sort":[{"field":"attention"}]}),
            "agent.view.set",
            json!({"source":"s","filter":{"op":"eq","field":"status","value":"blocked"},"sort":[{"field":"attention"}]}),
        ),
        (
            "herdr_agent_view_clear",
            json!({}),
            "agent.view.clear",
            json!({}),
        ),
        (
            "herdr_agent_view_clear",
            json!({"source":"s"}),
            "agent.view.clear",
            json!({"source":"s"}),
        ),
        (
            "herdr_workspace_move_block",
            json!({"workspace_ids":["w1","w2"],"before_workspace_id":"w3"}),
            "workspace.move_block",
            json!({"workspace_ids":["w1","w2"],"before_workspace_id":"w3"}),
        ),
        (
            "herdr_workspace_move_block",
            json!({"workspace_ids":["w1"]}),
            "workspace.move_block",
            json!({"workspace_ids":["w1"]}),
        ),
    ];
    for (tool, args, method, params) in cases {
        let r = mcp.call(tool, args).await;
        assert_eq!(r["result"]["isError"], false, "{tool}: {r}");
        let seen = fake.requests();
        let last = seen.last().expect("a request");
        assert_eq!(last["method"], method);
        assert_eq!(last["params"], params, "{tool}");
    }
}

#[tokio::test]
async fn selection_read_passes_text_and_caps_it() {
    let (_fake, mut mcp) = start("l13-sel").await;
    let args =
        |pane: &str| json!({"pane_id":pane,"anchor":{"row":0,"col":0},"cursor":{"row":1,"col":5}});
    let r = mcp.call("herdr_pane_selection_read", args("wA:p2")).await;
    assert_eq!(sc(&r)["selection"]["text"], "hello");
    assert!(sc(&r).get("truncated").is_none());
    let r = mcp.call("herdr_pane_selection_read", args("wA:p3")).await;
    assert_eq!(sc(&r)["truncated"], true);
    let text = sc(&r)["selection"]["text"].as_str().expect("text");
    assert_eq!(text.chars().count(), 40_000);
    assert!(text.ends_with("END"));
}

#[tokio::test]
async fn pane_metadata_limits_are_checked_before_the_call() {
    let (fake, mut mcp) = start("l13-meta").await;
    let many: serde_json::Map<String, Value> =
        (0..17).map(|i| (format!("k{i}"), json!("v"))).collect();
    let bad = [
        json!({"pane_id":"wA:p2","source":"s","tokens":many}),
        json!({"pane_id":"wA:p2","source":"s","tokens":{"bad key":"v"}}),
        json!({"pane_id":"wA:p2","source":"s","tokens":{"":"v"}}),
        json!({"pane_id":"wA:p2","source":"s","ttl_ms":0}),
        json!({"pane_id":"wA:p2","source":"s","ttl_ms":86_400_001}),
    ];
    for args in bad {
        let r = mcp.call("herdr_pane_report_metadata", args).await;
        assert_eq!(r["result"]["isError"], true, "{r}");
        assert_eq!(sc(&r)["code"], "invalid_params");
    }
    assert!(fake.requests().is_empty(), "nothing reached herdr");
}

#[tokio::test]
async fn read_only_keeps_only_the_selection_read() {
    let fake = FakeHerdr::start_with("l13-ro", false, FIXTURES);
    let mut mcp = Mcp::start(
        &fake.socket,
        &Opts {
            args: &["--read-only"],
            env: ENV,
        },
    )
    .await;
    let names = mcp.tool_names().await;
    assert!(names.contains(&"herdr_pane_selection_read".to_owned()));
    for gone in [
        "herdr_pane_scroll",
        "herdr_pane_report_metadata",
        "herdr_agent_view_set",
        "herdr_agent_view_clear",
        "herdr_workspace_move_block",
    ] {
        assert!(!names.contains(&gone.to_owned()), "{gone} in read-only");
    }
}
