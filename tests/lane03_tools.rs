//! Plan 03 (panes) against a fake herdr: methods and params sent, caps, waits,
//! own-pane refusals, read-only mode, and the `## Security` rows of the plan.
mod support;
use serde_json::{Value, json};
use support::{FakeHerdr, Mcp, Opts};

const FIXTURES: &str = include_str!("fixtures/lane03.jsonl");
const INSIDE: &[(&str, &str)] = &[
    ("HERDR_ENV", "1"),
    ("HERDR_PANE_ID", "wA:p1"),
    ("HERDR_MCP_TOOLS", "panes"),
];

fn fake(tag: &str, extra: &str) -> FakeHerdr {
    let all = if extra.is_empty() {
        FIXTURES.to_owned()
    } else {
        format!("{extra}\n{FIXTURES}")
    };
    FakeHerdr::start_with(tag, false, all.trim_end())
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

/// Requests minus the own-pane check's lookups (`pane.current`, and the
/// `pane.list` that follows the own terminal), which `src/safety.rs` makes
/// before input to a pane that is not the own pane by id.
fn calls(f: &FakeHerdr) -> Vec<Value> {
    f.requests()
        .into_iter()
        .filter(|r| r["method"] != "pane.current" && r["method"] != "pane.list")
        .collect()
}

fn sc(r: &Value) -> &Value {
    &r["result"]["structuredContent"]
}

fn assert_ok(r: &Value) {
    assert_eq!(r["result"]["isError"], false, "{r}");
}

#[tokio::test]
#[allow(clippy::too_many_lines, clippy::type_complexity)]
async fn each_pane_tool_sends_its_method_and_params() {
    let cases: &[(&str, Value, &[(&str, Value)])] = &[
        ("herdr_pane_list", json!({}), &[("pane.list", json!({}))]),
        (
            "herdr_pane_list",
            json!({"workspace_id":"wA"}),
            &[("pane.list", json!({"workspace_id":"wA"}))],
        ),
        (
            "herdr_pane_get",
            json!({"pane_id":"wA:p2"}),
            &[("pane.get", json!({"pane_id":"wA:p2"}))],
        ),
        (
            "herdr_pane_read",
            json!({"pane_id":"wA:p2"}),
            &[(
                "pane.read",
                json!({"pane_id":"wA:p2","source":"recent_unwrapped","lines":200,"format":"text","strip_ansi":true}),
            )],
        ),
        (
            "herdr_pane_read",
            json!({"pane_id":"wA:p2","source":"visible","lines":5}),
            &[(
                "pane.read",
                json!({"pane_id":"wA:p2","source":"visible","lines":5,"format":"text","strip_ansi":true}),
            )],
        ),
        (
            "herdr_pane_split",
            json!({"direction":"down","target_pane_id":"wA:p2","cwd":"/tmp","ratio":0.3}),
            &[(
                "pane.split",
                json!({"direction":"down","target_pane_id":"wA:p2","cwd":"/tmp","ratio":0.3,"focus":false}),
            )],
        ),
        (
            "herdr_pane_send_text",
            json!({"pane_id":"wA:p2","text":"ls"}),
            &[("pane.send_text", json!({"pane_id":"wA:p2","text":"ls"}))],
        ),
        (
            "herdr_pane_send_keys",
            json!({"pane_id":"wA:p2","keys":["ctrl-c","enter"]}),
            &[(
                "pane.send_keys",
                json!({"pane_id":"wA:p2","keys":["ctrl-c","enter"]}),
            )],
        ),
        (
            "herdr_pane_close",
            json!({"pane_id":"wA:p2"}),
            &[("pane.close", json!({"pane_id":"wA:p2"}))],
        ),
        (
            "herdr_pane_clear",
            json!({"pane_id":"wA:p2"}),
            &[("pane.clear", json!({"pane_id":"wA:p2"}))],
        ),
        (
            "herdr_pane_layout",
            json!({"pane_id":"wA:p2","direction":"left"}),
            &[
                ("pane.layout", json!({"pane_id":"wA:p2"})),
                ("pane.edges", json!({"pane_id":"wA:p2"})),
                (
                    "pane.neighbor",
                    json!({"pane_id":"wA:p2","direction":"left"}),
                ),
            ],
        ),
        (
            "herdr_pane_process_info",
            json!({"pane_id":"wA:p2"}),
            &[("pane.process_info", json!({"pane_id":"wA:p2"}))],
        ),
        (
            "herdr_pane_focus",
            json!({"pane_id":"wA:p2"}),
            &[("pane.focus", json!({"pane_id":"wA:p2"}))],
        ),
        (
            "herdr_pane_focus",
            json!({"direction":"left"}),
            &[("pane.focus_direction", json!({"direction":"left"}))],
        ),
        (
            "herdr_pane_zoom",
            json!({"pane_id":"wA:p2","mode":"toggle"}),
            &[("pane.zoom", json!({"pane_id":"wA:p2","mode":"toggle"}))],
        ),
        (
            "herdr_pane_resize",
            json!({"direction":"right","amount":0.1}),
            &[("pane.resize", json!({"direction":"right","amount":0.1}))],
        ),
        (
            "herdr_pane_swap",
            json!({"pane_id":"wA:p2","target_pane_id":"wA:p3"}),
            &[(
                "pane.swap",
                json!({"pane_id":"wA:p2","target_pane_id":"wA:p3"}),
            )],
        ),
        (
            "herdr_pane_move",
            json!({"pane_id":"wA:p2","destination":{"type":"new_tab","workspace_id":"wB","label":"x"}}),
            &[(
                "pane.move",
                json!({"pane_id":"wA:p2","destination":{"type":"new_tab","workspace_id":"wB","label":"x"},"focus":false}),
            )],
        ),
        (
            "herdr_pane_rename",
            json!({"pane_id":"wA:p2","label":"build"}),
            &[("pane.rename", json!({"pane_id":"wA:p2","label":"build"}))],
        ),
    ];
    for (i, (tool, args, expect)) in cases.iter().enumerate() {
        let fake = fake(&format!("each{i}"), "");
        let mut mcp = inside(&fake).await;
        let r = mcp.call(tool, args.clone()).await;
        assert_ok(&r);
        let seen = if *tool == "herdr_pane_list" {
            fake.requests()
        } else {
            calls(&fake)
        };
        assert_eq!(seen.len(), expect.len(), "{tool}: {seen:?}");
        for (req, (method, params)) in seen.iter().zip(*expect) {
            assert_eq!(req["method"], *method, "{tool}");
            assert_eq!(req["params"], *params, "{tool}");
        }
    }
}

#[tokio::test]
async fn pane_list_filters_by_tab_here() {
    let fake = fake("listtab", "");
    let mut mcp = inside(&fake).await;
    let r = mcp.call("herdr_pane_list", json!({"tab_id":"wA:t2"})).await;
    assert_eq!(sc(&r)["panes"].as_array().unwrap().len(), 1);
    assert_eq!(sc(&r)["panes"][0]["pane_id"], "wA:p2");
    assert_eq!(fake.requests()[0]["params"], json!({}));
}

#[tokio::test]
async fn layout_merges_edges_and_neighbour() {
    let fake = fake("layout", "");
    let mut mcp = inside(&fake).await;
    let r = mcp
        .call(
            "herdr_pane_layout",
            json!({"pane_id":"wA:p2","direction":"left"}),
        )
        .await;
    let s = sc(&r);
    assert_eq!(s["layout"]["tab_id"], "wA:t1");
    assert_eq!(s["edges"]["right"], true);
    assert!(s["edges"].get("layout").is_none());
    assert_eq!(s["neighbor"]["pane_id"], "wA:p1");
}

#[tokio::test]
async fn focus_needs_a_pane_or_a_direction() {
    let fake = fake("focus0", "");
    let mut mcp = inside(&fake).await;
    let r = mcp.call("herdr_pane_focus", json!({})).await;
    assert_eq!(r["result"]["isError"], true, "{r}");
    assert!(fake.requests().is_empty());
}

#[tokio::test]
async fn run_splits_types_the_exact_command_and_presses_enter_in_one_call() {
    let fake = fake("run", "");
    let mut mcp = inside(&fake).await;
    let cmd = "echo \"a b\"; rm -rf '$HOME' `x` && \\n";
    let r = mcp
        .call("herdr_pane_run", json!({"command": cmd, "cwd": "/tmp"}))
        .await;
    assert_ok(&r);
    assert_eq!(sc(&r)["pane_id"], "wA:p3");
    let seen = fake.requests();
    assert_eq!(seen.len(), 2, "{seen:?}");
    assert_eq!(seen[0]["method"], "pane.split");
    assert_eq!(
        seen[0]["params"],
        json!({"direction":"right","target_pane_id":"wA:p1","cwd":"/tmp","focus":false})
    );
    assert_eq!(seen[1]["method"], "pane.send_input");
    assert_eq!(
        seen[1]["params"],
        json!({"pane_id":"wA:p3","text":cmd,"keys":["enter"]})
    );
}

#[tokio::test]
async fn run_in_a_given_pane_does_not_split() {
    let fake = fake("runin", "");
    let mut mcp = inside(&fake).await;
    let r = mcp
        .call("herdr_pane_run", json!({"command":"ls","pane_id":"wA:p2"}))
        .await;
    assert_ok(&r);
    let seen = calls(&fake);
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0]["method"], "pane.send_input");
}

#[tokio::test]
async fn run_with_wait_returns_matched() {
    let fake = fake("runwait", "");
    let mut mcp = inside(&fake).await;
    let r = mcp
        .call(
            "herdr_pane_run",
            json!({"command":"echo herdr-$((1+1))","wait":{"match":{"type":"substring","value":"herdr-2"},"timeout_ms":5000}}),
        )
        .await;
    assert_ok(&r);
    assert_eq!(sc(&r)["matched"]["matched_line"], "herdr-2");
    assert!(sc(&r).get("timed_out").is_none());
    let seen = fake.requests();
    assert_eq!(seen[2]["method"], "pane.wait_for_output");
    assert_eq!(
        seen[2]["params"],
        json!({"pane_id":"wA:p3","source":"recent_unwrapped","match":{"type":"substring","value":"herdr-2"},"lines":200,"timeout_ms":5000,"strip_ansi":true})
    );
}

#[tokio::test]
async fn herdr_timeout_is_a_normal_result_with_timed_out() {
    let extra = r#"{"method":"pane.wait_for_output","error":{"code":"timeout","message":"timed out waiting for output match"}}"#;
    let fake = fake("timeout", extra);
    let mut mcp = inside(&fake).await;
    let r = mcp
        .call(
            "herdr_pane_wait_output",
            json!({"pane_id":"wA:p2","match":{"type":"substring","value":"zzz"},"timeout_ms":100}),
        )
        .await;
    assert_ok(&r);
    assert_eq!(sc(&r)["timed_out"], true);
    let r = mcp
        .call(
            "herdr_pane_run",
            json!({"command":"sleep 9","pane_id":"wA:p2","wait":{"match":{"type":"regex","value":"done"}}}),
        )
        .await;
    assert_ok(&r);
    assert_eq!(sc(&r)["timed_out"], true);
    assert!(sc(&r).get("matched").is_none());
}

#[tokio::test]
async fn a_hung_herdr_is_cut_off_two_seconds_past_the_wait() {
    let fake = FakeHerdr::start("hang", true);
    let mut mcp = inside(&fake).await;
    let t = std::time::Instant::now();
    let r = mcp
        .call(
            "herdr_pane_wait_output",
            json!({"pane_id":"wA:p2","match":{"type":"substring","value":"x"},"timeout_ms":100}),
        )
        .await;
    assert_ok(&r);
    assert_eq!(sc(&r)["timed_out"], true);
    let took = t.elapsed();
    assert!(took.as_millis() >= 2000 && took.as_secs() < 8, "{took:?}");
}

#[tokio::test]
async fn wait_timeout_is_capped_at_ten_minutes_and_lines_at_2000() {
    let fake = fake("waitcap", "");
    let mut mcp = inside(&fake).await;
    let r = mcp
        .call(
            "herdr_pane_wait_output",
            json!({"pane_id":"wA:p2","match":{"type":"substring","value":"x"},"timeout_ms":99_999_999,"lines":99999}),
        )
        .await;
    assert_ok(&r);
    let p = &fake.requests()[0]["params"];
    assert_eq!(p["timeout_ms"], 600_000);
    assert_eq!(p["lines"], 2000);
}

#[tokio::test]
async fn a_herdr_error_on_a_bad_regex_passes_through_before_any_wait() {
    let extra = r#"{"method":"pane.wait_for_output","error":{"code":"invalid_regex","message":"unclosed group"}}"#;
    let fake = fake("regex", extra);
    let mut mcp = inside(&fake).await;
    let r = mcp
        .call(
            "herdr_pane_wait_output",
            json!({"pane_id":"wA:p2","match":{"type":"regex","value":"("}}),
        )
        .await;
    assert_eq!(r["result"]["isError"], true);
    assert_eq!(sc(&r)["code"], "invalid_regex");
}

#[tokio::test]
async fn read_is_capped_in_lines_and_characters_keeping_the_tail() {
    let text = format!("{}TAIL", "a".repeat(50_000));
    let extra = json!({"method":"pane.read","result":{"type":"pane_read","read":{"pane_id":"wA:p2","text":text}}}).to_string();
    let fake = fake("readcap", &extra);
    let mut mcp = inside(&fake).await;
    let r = mcp
        .call("herdr_pane_read", json!({"pane_id":"wA:p2","lines":99999}))
        .await;
    assert_ok(&r);
    assert_eq!(fake.requests()[0]["params"]["lines"], 2000);
    assert_eq!(sc(&r)["truncated"], true);
    let kept = sc(&r)["read"]["text"].as_str().unwrap();
    assert_eq!(kept.chars().count(), 40_000);
    assert!(kept.ends_with("TAIL"));
}

#[tokio::test]
async fn a_short_read_is_not_marked_truncated() {
    let fake = fake("readsmall", "");
    let mut mcp = inside(&fake).await;
    let r = mcp
        .call("herdr_pane_read", json!({"pane_id":"wA:p2"}))
        .await;
    assert!(sc(&r).get("truncated").is_none());
    assert_eq!(sc(&r)["read"]["text"], "hello\nworld");
}

#[tokio::test]
async fn an_oversized_herdr_reply_is_refused_not_buffered() {
    let big = "a".repeat(17 * 1024 * 1024);
    let extra =
        json!({"method":"pane.read","result":{"type":"pane_read","read":{"text":big}}}).to_string();
    let fake = fake("huge", &extra);
    let mut mcp = inside(&fake).await;
    let r = mcp
        .call("herdr_pane_read", json!({"pane_id":"wA:p2"}))
        .await;
    assert_eq!(r["result"]["isError"], true, "{}", &r.to_string()[..200]);
    assert_eq!(sc(&r)["code"], "reply_too_large");
}

#[tokio::test]
async fn own_pane_input_clear_and_close_are_refused_without_calling_herdr() {
    let fake = fake("self", "");
    let mut mcp = inside(&fake).await;
    let own = "wA:p1";
    let cases = [
        ("herdr_pane_send_text", json!({"pane_id":own,"text":"x"})),
        (
            "herdr_pane_send_keys",
            json!({"pane_id":own,"keys":["enter"]}),
        ),
        ("herdr_pane_run", json!({"pane_id":own,"command":"ls"})),
        ("herdr_pane_clear", json!({"pane_id":own})),
        ("herdr_pane_close", json!({"pane_id":own})),
    ];
    for (tool, args) in cases {
        let r = mcp.call(tool, args).await;
        assert_eq!(r["result"]["isError"], true, "{tool}: {r}");
        assert_eq!(sc(&r)["code"], "self_target", "{tool}");
    }
    assert!(fake.requests().is_empty(), "{:?}", fake.requests());
    // Another pane is fine.
    let r = mcp
        .call(
            "herdr_pane_send_text",
            json!({"pane_id":"wA:p2","text":"x"}),
        )
        .await;
    assert_ok(&r);
}

#[tokio::test]
async fn read_only_mode_registers_only_pane_reads() {
    let fake = fake("ro", "");
    let mut mcp = Mcp::start(
        &fake.socket,
        &Opts {
            args: &["--read-only", "--tools", "panes"],
            env: INSIDE,
        },
    )
    .await;
    assert_eq!(
        mcp.tool_names().await,
        [
            "herdr_pane_get",
            "herdr_pane_layout",
            "herdr_pane_list",
            "herdr_pane_process_info",
            "herdr_pane_read",
            "herdr_pane_selection_read",
            "herdr_pane_wait_output"
        ]
    );
}

#[tokio::test]
async fn input_tools_are_not_read_only_and_destructive_ones_say_so() {
    let fake = fake("ann", "");
    let mut mcp = Mcp::start(
        &fake.socket,
        &Opts {
            args: &["--tools", "panes"],
            env: INSIDE,
        },
    )
    .await;
    let r = mcp.request("tools/list", json!({})).await;
    for t in r["result"]["tools"].as_array().unwrap() {
        let name = t["name"].as_str().unwrap();
        let a = &t["annotations"];
        let input = [
            "herdr_pane_run",
            "herdr_pane_send_text",
            "herdr_pane_send_keys",
        ];
        if input.contains(&name) {
            assert_eq!(a["readOnlyHint"], false, "{name}");
            assert_eq!(a["openWorldHint"], true, "{name}");
        }
        if ["herdr_pane_close", "herdr_pane_clear"].contains(&name) {
            assert_eq!(a["destructiveHint"], true, "{name}");
            assert_eq!(a["readOnlyHint"], false, "{name}");
        }
        let d = t["description"].as_str().unwrap_or_default();
        if name == "herdr_pane_read" {
            assert!(d.contains("untrusted"), "{d}");
        }
    }
}

#[tokio::test]
async fn stderr_never_carries_pane_text() {
    let fake = fake("log", "");
    let mut mcp = inside(&fake).await;
    mcp.call("herdr_pane_read", json!({"pane_id":"wA:p2"}))
        .await;
    mcp.call(
        "herdr_pane_run",
        json!({"command":"echo SECRET-TOKEN-123","pane_id":"wA:p2"}),
    )
    .await;
    let err = mcp.finish().await;
    assert!(err.contains("pane.read"), "{err}");
    assert!(
        !err.contains("SECRET-TOKEN-123") && !err.contains("hello"),
        "{err}"
    );
}

#[tokio::test]
async fn input_to_the_own_pane_after_it_moved_is_refused_by_terminal() {
    // The env still says wA:p1, but herdr now lists that terminal as wB:p9.
    let moved = r#"{"method":"pane.list","result":{"type":"pane_list","panes":[{"pane_id":"wB:p9","terminal_id":"term_me","workspace_id":"wB","tab_id":"wB:t1"}]}}"#;
    let fake = fake("moved", moved);
    let mut mcp = inside(&fake).await;
    let r = mcp
        .call(
            "herdr_pane_send_text",
            json!({"pane_id":"wB:p9","text":"x"}),
        )
        .await;
    assert_eq!(r["result"]["isError"], true, "{r}");
    assert_eq!(sc(&r)["code"], "self_target");
    assert!(calls(&fake).is_empty(), "{:?}", fake.requests());
}
