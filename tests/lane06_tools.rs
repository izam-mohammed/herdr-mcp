//! Plan 06: worktree, layout and notify tools against a fake herdr: methods
//! and params, `force` and `trust_repository` only when asked, own-target
//! refusals, annotations and gating.
mod support;
use serde_json::{Value, json};
use support::{FakeHerdr, Mcp, Opts};

const FIXTURES: &str = include_str!("fixtures/lane06.jsonl");
const INSIDE: &[(&str, &str)] = &[
    ("HERDR_ENV", "1"),
    ("HERDR_PANE_ID", "wA:p1"),
    ("HERDR_MCP_TOOLS", "worktrees,layout,notify"),
];

fn fake(tag: &str) -> FakeHerdr {
    FakeHerdr::start_with(tag, false, FIXTURES.trim())
}

async fn start(f: &FakeHerdr, args: &[&str]) -> Mcp {
    Mcp::start(&f.socket, &Opts { args, env: INSIDE }).await
}

/// Requests other than the capture of this server's own pane.
fn calls(f: &FakeHerdr) -> Vec<Value> {
    f.requests()
        .into_iter()
        .filter(|r| r["method"] != "pane.current" && r["method"] != "pane.list")
        .collect()
}

fn refused(r: &Value) -> bool {
    r["result"]["isError"] == true && r["result"]["structuredContent"]["code"] == "self_target"
}

const ROOT: fn() -> Value = || {
    json!({"type":"split","direction":"right","ratio":0.5,
        "first":{"type":"pane","cwd":"/x"},"second":{"type":"pane"}})
};

#[tokio::test]
async fn each_tool_sends_its_method_and_only_the_params_given() {
    let cases: Vec<(&str, Value, &str, Value)> = vec![
        ("herdr_worktree_list", json!({}), "worktree.list", json!({})),
        (
            "herdr_worktree_list",
            json!({"workspace_id":"wA","cwd":"/r","trust_repository":true}),
            "worktree.list",
            json!({"workspace_id":"wA","cwd":"/r","trust_repository":true}),
        ),
        (
            "herdr_worktree_create",
            json!({"cwd":"/r","branch":"b","base":"main","path":"/p","label":"l","focus":true}),
            "worktree.create",
            json!({"cwd":"/r","branch":"b","base":"main","path":"/p","label":"l","focus":true}),
        ),
        (
            "herdr_worktree_open",
            json!({"workspace_id":"wA","branch":"b"}),
            "worktree.open",
            json!({"workspace_id":"wA","branch":"b"}),
        ),
        (
            "herdr_worktree_remove",
            json!({"workspace_id":"wB"}),
            "worktree.remove",
            json!({"workspace_id":"wB"}),
        ),
        (
            "herdr_worktree_remove",
            json!({"workspace_id":"wB","force":true,"trust_repository":true}),
            "worktree.remove",
            json!({"workspace_id":"wB","force":true,"trust_repository":true}),
        ),
        (
            "herdr_layout_export",
            json!({"tab_id":"wA:t2"}),
            "layout.export",
            json!({"tab_id":"wA:t2"}),
        ),
        (
            "herdr_layout_apply",
            json!({"root":ROOT()}),
            "layout.apply",
            json!({"root":ROOT(),"focus":false}),
        ),
        (
            "herdr_layout_apply",
            json!({"root":ROOT(),"tab_id":"wA:t2","tab_label":"x","focus":true}),
            "layout.apply",
            json!({"root":ROOT(),"tab_id":"wA:t2","tab_label":"x","focus":true}),
        ),
        (
            "herdr_layout_set_split_ratio",
            json!({"path":[false,true],"ratio":0.25,"tab_id":"wA:t2"}),
            "layout.set_split_ratio",
            json!({"path":[false,true],"ratio":0.25,"tab_id":"wA:t2"}),
        ),
        (
            "herdr_notify",
            json!({"title":"tests passed in w3"}),
            "notification.show",
            json!({"title":"tests passed in w3"}),
        ),
        (
            "herdr_notify",
            json!({"title":"t","body":"b","position":"bottom-right","sound":"done"}),
            "notification.show",
            json!({"title":"t","body":"b","position":"bottom-right","sound":"done"}),
        ),
    ];
    for (i, (tool, args, method, params)) in cases.iter().enumerate() {
        let f = fake(&format!("l6each{i}"));
        let mut mcp = start(&f, &[]).await;
        let r = mcp.call(tool, args.clone()).await;
        assert_eq!(r["result"]["isError"], false, "{tool}: {r}");
        let seen = calls(&f);
        assert_eq!(seen.len(), 1, "{tool}");
        assert_eq!(seen[0]["method"], *method);
        assert_eq!(seen[0]["params"], *params, "{tool}");
        for absent in ["force", "trust_repository"] {
            if args.get(absent).is_none() {
                assert!(
                    seen[0]["params"].get(absent).is_none(),
                    "{tool} sent {absent}"
                );
            }
        }
    }
}

#[tokio::test]
async fn only_remove_and_apply_are_destructive_and_reads_are_reads() {
    let f = fake("l6ann");
    let mut mcp = start(&f, &["--confirm-destructive"]).await;
    let r = mcp.request("tools/list", json!({})).await;
    let tools = r["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 8);
    for t in tools {
        let name = t["name"].as_str().unwrap();
        let destructive = name == "herdr_worktree_remove" || name == "herdr_layout_apply";
        assert_eq!(t["annotations"]["destructiveHint"], destructive, "{name}");
        assert_eq!(
            t["_meta"]["anthropic/requiresUserInteraction"] == true,
            destructive,
            "{name}"
        );
        let read = name == "herdr_worktree_list" || name == "herdr_layout_export";
        assert_eq!(t["annotations"]["readOnlyHint"], read, "{name}");
    }
}

#[tokio::test]
async fn read_only_mode_keeps_only_list_and_export() {
    let f = fake("l6ro");
    let mut mcp = start(&f, &["--read-only"]).await;
    assert_eq!(
        mcp.tool_names().await,
        ["herdr_layout_export", "herdr_worktree_list"]
    );
    let r = mcp.call("herdr_notify", json!({"title":"x"})).await;
    assert!(r["error"].is_object(), "{r}");
    assert!(f.requests().is_empty());
}

#[tokio::test]
async fn own_workspace_and_tab_are_refused_and_others_go_through() {
    let f = fake("l6self");
    let mut mcp = start(&f, &[]).await;
    let r = mcp
        .call("herdr_worktree_remove", json!({"workspace_id":"wA"}))
        .await;
    assert!(refused(&r), "{r}");
    let r = mcp
        .call(
            "herdr_layout_apply",
            json!({"root":ROOT(),"tab_id":"wA:t1"}),
        )
        .await;
    assert!(refused(&r), "{r}");
    assert!(calls(&f).is_empty());

    let r = mcp
        .call("herdr_worktree_remove", json!({"workspace_id":"wB"}))
        .await;
    assert_eq!(r["result"]["isError"], false, "{r}");
    let r = mcp
        .call(
            "herdr_layout_apply",
            json!({"root":ROOT(),"tab_id":"wA:t2"}),
        )
        .await;
    assert_eq!(r["result"]["isError"], false, "{r}");
    // A new tab replaces nothing, so it is not refused, even in the own workspace.
    let r = mcp
        .call(
            "herdr_layout_apply",
            json!({"root":ROOT(),"workspace_id":"wA"}),
        )
        .await;
    assert_eq!(r["result"]["isError"], false, "{r}");
    assert_eq!(calls(&f).len(), 3);
}

#[tokio::test]
async fn allow_self_lets_it_through_without_extra_params() {
    let f = fake("l6allow");
    let mut mcp = start(&f, &[]).await;
    let r = mcp
        .call(
            "herdr_worktree_remove",
            json!({"workspace_id":"wA","allow_self":true}),
        )
        .await;
    assert_eq!(r["result"]["isError"], false, "{r}");
    assert_eq!(calls(&f)[0]["params"], json!({"workspace_id":"wA"}));
}

#[tokio::test]
async fn a_bad_enum_is_refused_before_herdr_sees_it() {
    let f = fake("l6enum");
    let mut mcp = start(&f, &[]).await;
    let r = mcp
        .call("herdr_notify", json!({"title":"t","position":"middle"}))
        .await;
    assert!(
        r["error"].is_object() || r["result"]["isError"] == true,
        "{r}"
    );
    assert!(calls(&f).is_empty());
}
