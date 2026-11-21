//! Tool behaviour against a fake herdr: methods and params sent, results
//! passed through, the contract's errors, and every `## Security` row.
mod support;
use serde_json::{Value, json};
use support::{FakeHerdr, Mcp, Opts};

/// Every group registered (the default). Used where a test lists tools.
const INSIDE_ALL: &[(&str, &str)] = &[("HERDR_ENV", "1"), ("HERDR_PANE_ID", "wA:p1")];
/// The plan 01 groups only, so the events hub sends no traffic of its own
/// and request counts are exactly the tool's.
const INSIDE: &[(&str, &str)] = &[
    ("HERDR_ENV", "1"),
    ("HERDR_PANE_ID", "wA:p1"),
    ("HERDR_MCP_TOOLS", "session,workspaces,tabs,agents"),
];

fn fixture(method: &str) -> Value {
    include_str!("fixtures/herdr.jsonl")
        .lines()
        .map(|l| serde_json::from_str::<Value>(l).expect("fixture"))
        .find(|f| f["method"] == method && f.get("params").is_none())
        .expect("fixture for method")["result"]
        .clone()
}

async fn inside(fake: &FakeHerdr) -> Mcp {
    Mcp::start(
        &fake.socket,
        &Opts {
            args: &[],
            env: INSIDE,
        },
    )
    .await
}

const ALL: &[&str] = &[
    "herdr_agent_explain",
    "herdr_agent_focus",
    "herdr_agent_get",
    "herdr_agent_list",
    "herdr_agent_prompt",
    "herdr_agent_read",
    "herdr_agent_rename",
    "herdr_agent_send_keys",
    "herdr_agent_start",
    "herdr_agent_wait",
    "herdr_events_recent",
    "herdr_events_status",
    "herdr_events_wait",
    "herdr_layout_apply",
    "herdr_layout_export",
    "herdr_layout_set_split_ratio",
    "herdr_notify",
    "herdr_pane_clear",
    "herdr_pane_close",
    "herdr_pane_focus",
    "herdr_pane_get",
    "herdr_pane_layout",
    "herdr_pane_list",
    "herdr_pane_move",
    "herdr_pane_process_info",
    "herdr_pane_read",
    "herdr_pane_rename",
    "herdr_pane_resize",
    "herdr_pane_run",
    "herdr_pane_send_keys",
    "herdr_pane_send_text",
    "herdr_pane_split",
    "herdr_pane_swap",
    "herdr_pane_wait_output",
    "herdr_pane_zoom",
    "herdr_snapshot",
    "herdr_status",
    "herdr_tab_close",
    "herdr_tab_create",
    "herdr_tab_focus",
    "herdr_tab_get",
    "herdr_tab_list",
    "herdr_tab_move",
    "herdr_tab_rename",
    "herdr_whoami",
    "herdr_workspace_close",
    "herdr_workspace_create",
    "herdr_workspace_focus",
    "herdr_workspace_get",
    "herdr_workspace_list",
    "herdr_workspace_move",
    "herdr_workspace_rename",
    "herdr_workspace_report_metadata",
    "herdr_worktree_create",
    "herdr_worktree_list",
    "herdr_worktree_open",
    "herdr_worktree_remove",
];
const READS: &[&str] = &[
    "herdr_agent_explain",
    "herdr_agent_get",
    "herdr_agent_list",
    "herdr_agent_read",
    "herdr_agent_wait",
    "herdr_events_recent",
    "herdr_events_status",
    "herdr_events_wait",
    "herdr_layout_export",
    "herdr_pane_get",
    "herdr_pane_layout",
    "herdr_pane_list",
    "herdr_pane_process_info",
    "herdr_pane_read",
    "herdr_pane_wait_output",
    "herdr_snapshot",
    "herdr_status",
    "herdr_tab_get",
    "herdr_tab_list",
    "herdr_whoami",
    "herdr_workspace_get",
    "herdr_workspace_list",
    "herdr_worktree_list",
];

#[tokio::test]
async fn inside_herdr_lists_every_tool_with_annotations() {
    let fake = FakeHerdr::start("list", false);
    let mut mcp = Mcp::start(
        &fake.socket,
        &Opts {
            args: &[],
            env: INSIDE_ALL,
        },
    )
    .await;
    assert_eq!(mcp.tool_names().await, ALL);
    let r = mcp.request("tools/list", json!({})).await;
    for t in r["result"]["tools"].as_array().unwrap() {
        let a = &t["annotations"];
        for hint in [
            "readOnlyHint",
            "destructiveHint",
            "idempotentHint",
            "openWorldHint",
        ] {
            assert!(a[hint].is_boolean(), "{} lacks {hint}", t["name"]);
        }
        if !t["name"].as_str().unwrap().starts_with("herdr_pane_") {
            assert_eq!(a["openWorldHint"], false);
        }
    }
}

#[tokio::test]
async fn instructions_say_output_is_untrusted() {
    let fake = FakeHerdr::start("instr", false);
    let mut mcp = inside(&fake).await;
    let init = mcp
        .request(
            "initialize",
            json!({"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"t","version":"0"}}),
        )
        .await;
    let text = init["result"]["instructions"].as_str().unwrap();
    assert!(text.contains("untrusted"), "{text}");
}

#[tokio::test]
async fn outside_herdr_registers_no_write_tool() {
    let fake = FakeHerdr::start("outside", false);
    let mut mcp = Mcp::start(
        &fake.socket,
        &Opts {
            args: &[],
            env: &[],
        },
    )
    .await;
    assert_eq!(mcp.tool_names().await, READS);
}

#[tokio::test]
async fn allow_outside_herdr_registers_writes() {
    let fake = FakeHerdr::start("allow", false);
    let mut mcp = Mcp::start(
        &fake.socket,
        &Opts {
            args: &["--allow-outside-herdr"],
            env: &[],
        },
    )
    .await;
    assert_eq!(mcp.tool_names().await, ALL);
}

#[tokio::test]
async fn read_only_flag_lists_reads_only() {
    let fake = FakeHerdr::start("ro", false);
    let mut mcp = Mcp::start(
        &fake.socket,
        &Opts {
            args: &["--read-only"],
            env: INSIDE_ALL,
        },
    )
    .await;
    assert_eq!(mcp.tool_names().await, READS);
    let r = mcp.call("herdr_tab_create", json!({})).await;
    assert!(
        r["error"].is_object(),
        "an unregistered tool is refused: {r}"
    );
    // The events hub talks to herdr on its own; the refused write must not.
    assert!(fake.requests().iter().all(|r| r["method"] != "tab.create"));
}

#[tokio::test]
async fn tools_flag_registers_only_the_groups_named() {
    let fake = FakeHerdr::start("groups", false);
    let mut mcp = Mcp::start(
        &fake.socket,
        &Opts {
            args: &["--tools", "agents,session"],
            env: INSIDE,
        },
    )
    .await;
    assert_eq!(
        mcp.tool_names().await,
        [
            "herdr_agent_explain",
            "herdr_agent_focus",
            "herdr_agent_get",
            "herdr_agent_list",
            "herdr_agent_prompt",
            "herdr_agent_read",
            "herdr_agent_rename",
            "herdr_agent_send_keys",
            "herdr_agent_start",
            "herdr_agent_wait",
            "herdr_snapshot",
            "herdr_status",
            "herdr_whoami"
        ]
    );
}

#[tokio::test]
async fn each_tool_sends_its_method_and_returns_the_result_unchanged() {
    let cases: &[(&str, Value, &str, Value)] = &[
        ("herdr_status", json!({}), "ping", json!({})),
        ("herdr_snapshot", json!({}), "session.snapshot", json!({})),
        (
            "herdr_workspace_list",
            json!({}),
            "workspace.list",
            json!({}),
        ),
        ("herdr_tab_list", json!({}), "tab.list", json!({})),
        (
            "herdr_tab_list",
            json!({"workspace_id":"wA"}),
            "tab.list",
            json!({"workspace_id":"wA"}),
        ),
        (
            "herdr_tab_get",
            json!({"tab_id":"wA:t1"}),
            "tab.get",
            json!({"tab_id":"wA:t1"}),
        ),
        (
            "herdr_tab_focus",
            json!({"tab_id":"wA:t1"}),
            "tab.focus",
            json!({"tab_id":"wA:t1"}),
        ),
        (
            "herdr_tab_create",
            json!({"workspace_id":"wA","label":"scratch","focus":true}),
            "tab.create",
            json!({"workspace_id":"wA","label":"scratch","focus":true}),
        ),
        ("herdr_tab_create", json!({}), "tab.create", json!({})),
        (
            "herdr_tab_rename",
            json!({"tab_id":"wA:t1","label":"renamed"}),
            "tab.rename",
            json!({"tab_id":"wA:t1","label":"renamed"}),
        ),
        ("herdr_agent_list", json!({}), "agent.list", json!({})),
    ];
    for (i, (tool, args, method, params)) in cases.iter().enumerate() {
        let fake = FakeHerdr::start(&format!("each{i}"), false);
        let mut mcp = inside(&fake).await;
        let r = mcp.call(tool, args.clone()).await;
        assert_eq!(r["result"]["isError"], false, "{tool}: {r}");
        assert_eq!(r["result"]["structuredContent"], fixture(method), "{tool}");
        assert!(r["result"]["content"][0]["text"].as_str().unwrap().len() < 80);
        let seen = fake.requests();
        assert_eq!(seen.len(), 1, "{tool}");
        assert_eq!(seen[0]["method"], *method, "{tool}");
        assert_eq!(seen[0]["params"], *params, "{tool}");
    }
}

#[tokio::test]
async fn whoami_reports_env_and_the_current_pane() {
    let fake = FakeHerdr::start("who", false);
    let mut mcp = inside(&fake).await;
    let r = mcp.call("herdr_whoami", json!({})).await;
    let s = &r["result"]["structuredContent"];
    assert_eq!(s["pane_id"], "wA:p1");
    assert_eq!(s["read_only"], false);
    assert_eq!(s["pane"], fixture("pane.current"));
    assert_eq!(
        fake.requests()[0]["params"],
        json!({"caller_pane_id":"wA:p1"})
    );
}

#[tokio::test]
async fn a_herdr_error_becomes_the_contract_shape() {
    let fake = FakeHerdr::start("err", false);
    let mut mcp = inside(&fake).await;
    let r = mcp.call("herdr_tab_get", json!({"tab_id":"wZ:t9"})).await;
    assert_eq!(r["result"]["isError"], true, "{r}");
    assert_eq!(
        r["result"]["structuredContent"],
        json!({"code":"not_found","message":"tab wZ:t9 not found"})
    );
    assert_eq!(
        r["result"]["content"][0]["text"],
        "herdr not_found: tab wZ:t9 not found"
    );
}

#[tokio::test]
async fn a_wrong_typed_argument_is_refused_before_herdr_sees_it() {
    let fake = FakeHerdr::start("typed", false);
    let mut mcp = inside(&fake).await;
    let r = mcp.call("herdr_tab_get", json!({"tab_id": 5})).await;
    assert!(
        r["error"].is_object() || r["result"]["isError"] == true,
        "{r}"
    );
    let r = mcp
        .call("herdr_tab_rename", json!({"tab_id":"wA:t1"}))
        .await;
    assert!(
        r["error"].is_object() || r["result"]["isError"] == true,
        "{r}"
    );
    assert!(
        fake.requests().is_empty(),
        "herdr saw {:?}",
        fake.requests()
    );
}

#[tokio::test]
async fn stderr_has_no_reply_body_without_debug() {
    let fake = FakeHerdr::start("quiet", false);
    let mut mcp = inside(&fake).await;
    mcp.call("herdr_tab_list", json!({})).await;
    let err = mcp.finish().await;
    assert!(err.contains("tab.list"), "method is logged: {err}");
    assert!(!err.contains("pane_count"), "reply body leaked: {err}");
    assert!(!err.contains("agent_status"), "reply body leaked: {err}");

    let fake = FakeHerdr::start("loud", false);
    let mut mcp = Mcp::start(
        &fake.socket,
        &Opts {
            args: &["--debug"],
            env: INSIDE,
        },
    )
    .await;
    mcp.call("herdr_tab_list", json!({})).await;
    let err = mcp.finish().await;
    assert!(err.contains("pane_count"), "--debug logs payloads: {err}");
}

#[tokio::test]
async fn a_hung_socket_gives_timeout() {
    let fake = FakeHerdr::start("hang", true);
    let mut mcp = Mcp::start(
        &fake.socket,
        &Opts {
            args: &[],
            env: &[("HERDR_ENV", "1"), ("HERDR_MCP_TIMEOUT_MS", "300")],
        },
    )
    .await;
    let r = mcp.call("herdr_tab_list", json!({})).await;
    assert_eq!(r["result"]["isError"], true, "{r}");
    assert_eq!(r["result"]["structuredContent"]["code"], "timeout");
    assert!(
        r["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .starts_with("herdr timeout: ")
    );
}

#[tokio::test]
async fn an_absent_socket_gives_herdr_unreachable() {
    let socket = support::socket_path("absent");
    let _ = std::fs::remove_file(&socket);
    let mut mcp = Mcp::start(
        &socket,
        &Opts {
            args: &[],
            env: INSIDE,
        },
    )
    .await;
    let r = mcp.call("herdr_status", json!({})).await;
    assert_eq!(r["result"]["isError"], true);
    assert_eq!(
        r["result"]["structuredContent"]["code"],
        "herdr_unreachable"
    );
}

#[tokio::test]
async fn snapshot_filters_and_summarises() {
    let fake = FakeHerdr::start("snap", false);
    let mut mcp = inside(&fake).await;

    let r = mcp
        .call("herdr_snapshot", json!({"workspace_id":"wA"}))
        .await;
    let s = &r["result"]["structuredContent"]["snapshot"];
    assert_eq!(
        s["workspaces"],
        json!([{"workspace_id":"wA","label":"main"}])
    );
    assert_eq!(s["panes"].as_array().unwrap().len(), 1);
    assert_eq!(s["agents"], json!([]));

    let r = mcp.call("herdr_snapshot", json!({"tab_id":"wB:t1"})).await;
    let s = &r["result"]["structuredContent"]["snapshot"];
    assert_eq!(s["agents"], json!([{"pane_id":"wB:p1","agent":"claude"}]));

    let r = mcp.call("herdr_snapshot", json!({"summary":true})).await;
    let s = &r["result"]["structuredContent"];
    assert_eq!(s["type"], "session_snapshot_summary");
    assert_eq!(s["snapshot"]["counts"]["panes"], 2);
    assert_eq!(s["snapshot"]["panes"], json!(["wA:p1", "wB:p1"]));
}

#[tokio::test]
async fn a_big_snapshot_is_summarised_by_default_and_says_so() {
    let tabs: Vec<Value> = (0..600)
        .map(|i| json!({"tab_id":format!("wA:t{i}"),"workspace_id":"wA","label":"x".repeat(30)}))
        .collect();
    let big = json!({"method":"session.snapshot","result":{"type":"session_snapshot",
        "snapshot":{"protocol":22,"workspaces":[{"workspace_id":"wA"}],"tabs":tabs,
        "panes":[],"layouts":[],"agents":[]}}});
    let fake = FakeHerdr::start_with("big", false, &big.to_string());
    let mut mcp = inside(&fake).await;

    let r = mcp.call("herdr_snapshot", json!({})).await;
    let s = &r["result"]["structuredContent"];
    assert_eq!(s["type"], "session_snapshot_summary", "{r}");
    assert!(s["note"].as_str().unwrap().contains("20000"));
    assert_eq!(s["snapshot"]["counts"]["tabs"], 600);
    assert!(
        r["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("summary")
    );

    let r = mcp.call("herdr_snapshot", json!({"summary":false})).await;
    assert_eq!(r["result"]["structuredContent"]["type"], "session_snapshot");
}
