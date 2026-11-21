//! Plan 13 Phase 3: the `admin` group over the socket. Absent by default, present with
//! `--tools all,admin`, hidden by `--read-only` where it changes things, and `herdr_server_stop`
//! refuses the session the server runs in. Fake herdr, so nothing real is stopped.
mod support;
use serde_json::{Value, json};
use support::{FakeHerdr, Mcp, Opts};

const FIXTURES: &str = r#"{"method":"integration.list","result":{"type":"integration_list","integrations":[]}}
{"method":"integration.install","result":{"type":"integration_install","messages":["ok"]}}
{"method":"integration.uninstall","result":{"type":"integration_uninstall","messages":["ok"]}}
{"method":"server.reload_config","result":{"type":"ok"}}
{"method":"server.agent_manifests","result":{"type":"agent_manifests","manifests":[]}}
{"method":"server.reload_agent_manifests","result":{"type":"ok"}}
{"method":"server.stop","result":{"type":"ok"}}
{"method":"pane.input.set","result":{"type":"ok"}}
{"method":"pane.report_agent","result":{"type":"ok"}}
{"method":"pane.report_agent_session","result":{"type":"ok"}}
{"method":"pane.release_agent","result":{"type":"ok"}}
{"method":"pane.clear_agent_authority","result":{"type":"ok"}}"#;

const ADMIN: &[&str] = &[
    "herdr_integration_install",
    "herdr_integration_list",
    "herdr_integration_uninstall",
    "herdr_pane_clear_agent_authority",
    "herdr_pane_release_agent",
    "herdr_pane_report_agent",
    "herdr_pane_report_agent_session",
    "herdr_pane_set_input",
    "herdr_server_agent_manifests",
    "herdr_server_reload_agent_manifests",
    "herdr_server_reload_config",
    "herdr_server_stop",
];
const ADMIN_READS: &[&str] = &[
    "herdr_channel_show",
    "herdr_config_check",
    "herdr_integration_list",
    "herdr_machine_list",
    "herdr_machine_status",
    "herdr_server_agent_manifests",
    "herdr_session_list",
];

const INSIDE: &[(&str, &str)] = &[("HERDR_ENV", "1"), ("HERDR_PANE_ID", "wA:p1")];

async fn start(tag: &str, args: &[&str], env: &[(&str, &str)]) -> (FakeHerdr, Mcp) {
    let fake = FakeHerdr::start_with(tag, false, FIXTURES);
    let mcp = Mcp::start(&fake.socket, &Opts { args, env }).await;
    (fake, mcp)
}

fn admin_of(names: &[String]) -> Vec<&str> {
    names
        .iter()
        .map(String::as_str)
        .filter(|n| ADMIN.contains(n))
        .collect()
}

fn sc(r: &Value) -> &Value {
    &r["result"]["structuredContent"]
}

#[tokio::test]
async fn admin_is_absent_by_default_and_present_when_asked() {
    for (tag, args, env, want) in [
        ("adm-def", &[][..], INSIDE, 0),
        ("adm-env", &[][..], INSIDE, 0),
        ("adm-flag", &["--tools", "all"][..], INSIDE, 0),
        ("adm-on", &["--tools", "all,admin"][..], INSIDE, ADMIN.len()),
        ("adm-only", &["--tools", "admin"][..], INSIDE, ADMIN.len()),
    ] {
        let (_f, mut mcp) = start(tag, args, env).await;
        let names = mcp.tool_names().await;
        assert_eq!(admin_of(&names).len(), want, "{tag}: {names:?}");
        assert_eq!(names.iter().any(|n| n == "herdr_update"), want > 0, "{tag}");
    }
    let (_f, mut mcp) = start(
        "adm-envvar",
        &[],
        &[("HERDR_ENV", "1"), ("HERDR_MCP_TOOLS", "all,admin")],
    )
    .await;
    assert_eq!(admin_of(&mcp.tool_names().await).len(), ADMIN.len());
}

#[tokio::test]
async fn read_only_keeps_only_the_admin_reads() {
    let (_f, mut mcp) = start("adm-ro", &["--read-only", "--tools", "admin"], INSIDE).await;
    assert_eq!(mcp.tool_names().await, ADMIN_READS);
}

#[tokio::test]
async fn every_mutating_admin_tool_is_marked_destructive() {
    let (_f, mut mcp) = start("adm-hints", &["--tools", "admin"], INSIDE).await;
    let r = mcp.request("tools/list", json!({})).await;
    for t in r["result"]["tools"].as_array().expect("tools") {
        let name = t["name"].as_str().expect("name");
        let a = &t["annotations"];
        if ADMIN_READS.contains(&name) {
            assert_eq!(a["readOnlyHint"], true, "{name}");
        } else {
            assert_eq!(a["readOnlyHint"], false, "{name}");
            assert_eq!(a["destructiveHint"], true, "{name}");
        }
    }
}

#[tokio::test]
async fn each_tool_sends_its_method_and_params() {
    let (fake, mut mcp) = start("adm-send", &["--tools", "admin"], INSIDE).await;
    let cases = [
        (
            "herdr_integration_list",
            json!({}),
            "integration.list",
            json!({}),
        ),
        (
            "herdr_integration_install",
            json!({"target":"claude"}),
            "integration.install",
            json!({"target":"claude"}),
        ),
        (
            "herdr_integration_uninstall",
            json!({"target":"codex"}),
            "integration.uninstall",
            json!({"target":"codex"}),
        ),
        (
            "herdr_server_reload_config",
            json!({}),
            "server.reload_config",
            json!({}),
        ),
        (
            "herdr_server_agent_manifests",
            json!({}),
            "server.agent_manifests",
            json!({}),
        ),
        (
            "herdr_server_reload_agent_manifests",
            json!({}),
            "server.reload_agent_manifests",
            json!({}),
        ),
        (
            "herdr_pane_set_input",
            json!({"pane_id":"wA:p2","right_click":"pane"}),
            "pane.input.set",
            json!({"pane_id":"wA:p2","right_click":"pane"}),
        ),
        (
            "herdr_pane_report_agent",
            json!({"pane_id":"wA:p2","source":"s","agent":"claude","state":"blocked","seq":3}),
            "pane.report_agent",
            json!({"pane_id":"wA:p2","source":"s","agent":"claude","state":"blocked","seq":3}),
        ),
        (
            "herdr_pane_report_agent_session",
            json!({"pane_id":"wA:p2","source":"s","agent":"claude","agent_session_id":"x"}),
            "pane.report_agent_session",
            json!({"pane_id":"wA:p2","source":"s","agent":"claude","agent_session_id":"x"}),
        ),
        (
            "herdr_pane_release_agent",
            json!({"pane_id":"wA:p2","source":"s","agent":"claude"}),
            "pane.release_agent",
            json!({"pane_id":"wA:p2","source":"s","agent":"claude"}),
        ),
        (
            "herdr_pane_clear_agent_authority",
            json!({"pane_id":"wA:p2"}),
            "pane.clear_agent_authority",
            json!({"pane_id":"wA:p2"}),
        ),
    ];
    for (tool, args, method, params) in cases {
        let r = mcp.call(tool, args).await;
        assert_eq!(r["result"]["isError"], false, "{tool}: {r}");
        let seen = fake.requests();
        let last = seen.last().expect("a request");
        assert_eq!(last["method"], method, "{tool}");
        assert_eq!(last["params"], params, "{tool}");
    }
}

#[tokio::test]
async fn an_unknown_state_is_refused_by_the_schema() {
    let (fake, mut mcp) = start("adm-state", &["--tools", "admin"], INSIDE).await;
    let r = mcp
        .call(
            "herdr_pane_report_agent",
            json!({"pane_id":"wA:p2","source":"s","agent":"x","state":"on-fire"}),
        )
        .await;
    assert!(
        r["error"].is_object() || r["result"]["isError"] == true,
        "{r}"
    );
    assert!(fake.requests().is_empty());
}

#[tokio::test]
async fn server_stop_refuses_the_own_session() {
    // The harness socket path is outside herdr's `sessions/<name>/` layout, so the own
    // session cannot be told and the stop is refused (fail closed).
    let (fake, mut mcp) = start("adm-stop", &["--tools", "admin"], INSIDE).await;
    let r = mcp.call("herdr_server_stop", json!({})).await;
    assert_eq!(r["result"]["isError"], true, "{r}");
    assert_eq!(sc(&r)["code"], "self_target");
    assert!(
        fake.requests().iter().all(|q| q["method"] != "server.stop"),
        "nothing reached herdr"
    );
    let r = mcp
        .call("herdr_server_stop", json!({"allow_self": false}))
        .await;
    assert_eq!(sc(&r)["code"], "self_target");
    let r = mcp
        .call("herdr_server_stop", json!({"allow_self": true}))
        .await;
    assert_eq!(r["result"]["isError"], false, "{r}");
    assert!(fake.requests().iter().any(|q| q["method"] == "server.stop"));
}

#[tokio::test]
async fn server_stop_outside_herdr_has_no_self_to_protect() {
    let (fake, mut mcp) = start(
        "adm-stop-out",
        &["--tools", "admin", "--allow-outside-herdr"],
        &[],
    )
    .await;
    let r = mcp.call("herdr_server_stop", json!({})).await;
    assert_eq!(r["result"]["isError"], false, "{r}");
    assert!(fake.requests().iter().any(|q| q["method"] == "server.stop"));
}

#[tokio::test]
async fn the_existing_groups_are_unchanged_by_the_admin_group() {
    let (_f, mut mcp) = start("adm-none", &[], INSIDE).await;
    let names = mcp.tool_names().await;
    assert_eq!(names.len(), 63, "{names:?}");
}
