//! Plan 04: agent tools against a fake herdr, and every `## Security` row.
mod support;
use serde_json::{Value, json};
use support::{FakeHerdr, Mcp, Opts};

const INSIDE: &[(&str, &str)] = &[
    ("HERDR_ENV", "1"),
    ("HERDR_PANE_ID", "wA:p1"),
    ("HERDR_MCP_TOOLS", "agents"),
];
const FIXTURES: &str = include_str!("fixtures/lane04.jsonl");
const WRITES: &[&str] = &[
    "herdr_agent_prompt",
    "herdr_agent_send_keys",
    "herdr_agent_start",
    "herdr_agent_focus",
    "herdr_agent_rename",
];
const READS: &[&str] = &[
    "herdr_agent_list",
    "herdr_agent_get",
    "herdr_agent_read",
    "herdr_agent_explain",
    "herdr_agent_wait",
];

async fn boot(tag: &str, args: &'static [&'static str]) -> (FakeHerdr, Mcp) {
    let fake = FakeHerdr::start_with(tag, false, FIXTURES);
    let mcp = Mcp::start(&fake.socket, &Opts { args, env: INSIDE }).await;
    (fake, mcp)
}

fn methods(fake: &FakeHerdr) -> Vec<String> {
    fake.requests()
        .iter()
        .map(|r| r["method"].as_str().unwrap().to_owned())
        .collect()
}

fn last(fake: &FakeHerdr, method: &str) -> Value {
    fake.requests()
        .into_iter()
        .rev()
        .find(|r| r["method"] == method)
        .unwrap_or_else(|| panic!("no {method} request"))["params"]
        .clone()
}

fn err_code(r: &Value) -> &str {
    assert_eq!(r["result"]["isError"], true, "{r}");
    r["result"]["structuredContent"]["code"].as_str().unwrap()
}

#[tokio::test]
async fn annotations_split_reads_from_input_tools() {
    let (_f, mut mcp) = boot("l4ann", &[]).await;
    let r = mcp.request("tools/list", json!({})).await;
    let tools = r["result"]["tools"].as_array().unwrap();
    let hint = |n: &str| {
        tools
            .iter()
            .find(|t| t["name"] == n)
            .unwrap_or_else(|| panic!("{n} missing"))["annotations"]["readOnlyHint"]
            .as_bool()
            .unwrap()
    };
    for n in READS {
        assert!(hint(n), "{n} should be read-only");
    }
    for n in WRITES {
        assert!(!hint(n), "{n} must not be read-only");
    }
}

#[tokio::test]
async fn read_only_mode_drops_the_input_tools() {
    let (_f, mut mcp) = boot("l4ro", &["--read-only"]).await;
    let names = mcp.tool_names().await;
    for n in READS {
        assert!(names.iter().any(|x| x == n), "{n} should stay");
    }
    for n in WRITES {
        assert!(!names.iter().any(|x| x == n), "{n} should be dropped");
    }
}

#[tokio::test]
async fn read_tools_send_their_methods() {
    let (fake, mut mcp) = boot("l4read", &[]).await;
    mcp.call("herdr_agent_get", json!({"target":"reviewer"}))
        .await;
    assert_eq!(last(&fake, "agent.get"), json!({"target":"reviewer"}));
    let r = mcp
        .call(
            "herdr_agent_read",
            json!({"target":"reviewer","lines":5000}),
        )
        .await;
    assert_eq!(r["result"]["structuredContent"]["read"]["text"], "hello");
    assert_eq!(
        last(&fake, "agent.read"),
        json!({"target":"reviewer","source":"recent","lines":1000,"format":"text","strip_ansi":true})
    );
    mcp.call("herdr_agent_explain", json!({"target":"reviewer"}))
        .await;
    assert_eq!(last(&fake, "agent.explain"), json!({"target":"reviewer"}));
}

#[tokio::test]
async fn other_input_tools_send_their_methods() {
    let (fake, mut mcp) = boot("l4in", &[]).await;
    mcp.call("herdr_agent_focus", json!({"target":"reviewer"}))
        .await;
    assert_eq!(last(&fake, "agent.focus"), json!({"target":"reviewer"}));
    mcp.call(
        "herdr_agent_rename",
        json!({"target":"reviewer","name":"x"}),
    )
    .await;
    assert_eq!(
        last(&fake, "agent.rename"),
        json!({"target":"reviewer","name":"x"})
    );
    mcp.call(
        "herdr_agent_send_keys",
        json!({"target":"reviewer","keys":["y"]}),
    )
    .await;
    assert_eq!(
        last(&fake, "agent.send_keys"),
        json!({"target":"wA:p2","keys":["y"]})
    );
    mcp.call(
        "herdr_agent_prompt",
        json!({"target":"reviewer","text":"hi"}),
    )
    .await;
    assert_eq!(
        last(&fake, "agent.prompt"),
        json!({"target":"wA:p2","text":"hi"})
    );
}

#[tokio::test]
async fn own_agent_is_refused_by_pane_id_by_name_and_after_a_move() {
    let (fake, mut mcp) = boot("l4own", &[]).await;
    for target in ["wA:p1", "boss", "moved"] {
        let p = mcp
            .call("herdr_agent_prompt", json!({"target":target,"text":"x"}))
            .await;
        assert_eq!(err_code(&p), "own_agent", "{target}");
        let k = mcp
            .call(
                "herdr_agent_send_keys",
                json!({"target":target,"keys":["Enter"]}),
            )
            .await;
        assert_eq!(err_code(&k), "own_agent", "{target}");
    }
    let m = methods(&fake);
    assert!(
        !m.iter()
            .any(|x| x == "agent.prompt" || x == "agent.send_keys"),
        "{m:?}"
    );
}

#[tokio::test]
async fn an_unresolvable_target_fails_closed() {
    let (fake, mut mcp) = boot("l4ghost", &[]).await;
    let r = mcp
        .call("herdr_agent_prompt", json!({"target":"ghost","text":"x"}))
        .await;
    assert_eq!(err_code(&r), "agent_not_found");
    assert!(!methods(&fake).iter().any(|x| x == "agent.prompt"));
}

#[tokio::test]
async fn prompt_with_wait_is_one_request_with_bounds() {
    let (fake, mut mcp) = boot("l4wait", &[]).await;
    mcp.call(
        "herdr_agent_prompt",
        json!({"target":"reviewer","text":"go","wait":{}}),
    )
    .await;
    assert_eq!(
        last(&fake, "agent.prompt"),
        json!({"target":"wA:p2","text":"go","wait":{"until":["idle","done","blocked"],"timeout_ms":60000}})
    );
    assert_eq!(
        methods(&fake)
            .iter()
            .filter(|m| *m == "agent.prompt")
            .count(),
        1
    );
    assert!(!methods(&fake).iter().any(|m| m == "agent.wait"));
}

#[tokio::test]
async fn blocked_and_stalled_pass_through() {
    let (_f, mut mcp) = boot("l4pass", &[]).await;
    for (t, code) in [
        ("blocker", "agent_blocked"),
        ("stall", "agent_prompt_stalled"),
    ] {
        let r = mcp
            .call(
                "herdr_agent_prompt",
                json!({"target":t,"text":"go","wait":{}}),
            )
            .await;
        assert_eq!(err_code(&r), code);
    }
}

#[tokio::test]
async fn waits_end_at_the_cap_with_timed_out() {
    let (fake, mut mcp) = boot("l4cap", &[]).await;
    let r = mcp.call("herdr_agent_wait", json!({"target":"slow"})).await;
    assert_eq!(r["result"]["isError"], false, "{r}");
    assert_eq!(r["result"]["structuredContent"]["timed_out"], true);
    let p = mcp
        .call(
            "herdr_agent_prompt",
            json!({"target":"slow","text":"go","wait":{"until":["idle"],"timeout_ms":1500}}),
        )
        .await;
    assert_eq!(p["result"]["structuredContent"]["timed_out"], true);
    // Caller's timeout above the cap is clamped.
    mcp.call(
        "herdr_agent_wait",
        json!({"target":"reviewer","timeout_ms":9_999_999}),
    )
    .await;
    assert_eq!(last(&fake, "agent.wait")["timeout_ms"], 600_000);
}

#[tokio::test]
async fn start_checks_the_timeout_range_before_calling_herdr() {
    let (fake, mut mcp) = boot("l4start", &[]).await;
    for bad in [100u64, 3000, 300_001] {
        let r = mcp
            .call(
                "herdr_agent_start",
                json!({"name":"a","kind":"claude","pane_id":"wA:p2","timeout_ms":bad}),
            )
            .await;
        assert_eq!(err_code(&r), "invalid_agent_timeout", "{bad}");
    }
    assert!(!methods(&fake).iter().any(|m| m == "agent.start"));
    mcp.call(
        "herdr_agent_start",
        json!({"name":"a","kind":"claude","pane_id":"wA:p2","args":["--x"]}),
    )
    .await;
    assert_eq!(
        last(&fake, "agent.start"),
        json!({"name":"a","kind":"claude","pane_id":"wA:p2","timeout_ms":30000,"args":["--x"]})
    );
}
