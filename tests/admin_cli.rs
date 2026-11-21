//! Plan 13 Phase 4: the subprocess-backed admin tools, against a stub `herdr` script on
//! PATH (never the real binary). The stub records its argv, so the tests can say exactly what
//! would have run: argv items only, flag-like values refused, `confirm` and own-session
//! rules, the timeout, the cap and a missing binary.
mod support;
use serde_json::{Value, json};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use support::{FakeHerdr, Mcp, Opts};

const STUB: &str = r#"#!/bin/sh
{ printf '%s\n' "$@"; echo '--'; } >> "$STUB_LOG"
case "$STUB_MODE" in
  sleep) exec sleep 5 ;;
  big) head -c 200000 /dev/zero | tr '\0' x; exit 0 ;;
  exit3) echo out; echo err >&2; exit 3 ;;
  *) echo "stub out"; echo "stub err" >&2 ;;
esac
"#;

struct Stub {
    dir: PathBuf,
    log: PathBuf,
}

impl Stub {
    fn new(tag: &str) -> Stub {
        let dir = std::env::temp_dir().join(format!("herdr-mcp-stub-{}-{tag}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("stub dir");
        let bin = dir.join("herdr");
        std::fs::write(&bin, STUB).expect("write stub");
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).expect("chmod");
        let log = dir.join("argv.log");
        let _ = std::fs::remove_file(&log);
        Stub { dir, log }
    }

    /// Every call the stub got, as its argv.
    fn calls(&self) -> Vec<Vec<String>> {
        let text = std::fs::read_to_string(&self.log).unwrap_or_default();
        text.split("--\n")
            .filter(|c| !c.is_empty())
            .map(|c| c.lines().map(str::to_owned).collect())
            .collect()
    }
}

fn env_with(stub: &Stub, extra: &[(&str, &str)]) -> Vec<(String, String)> {
    let mut env = vec![
        ("HERDR_ENV".to_owned(), "1".to_owned()),
        ("HERDR_MCP_TOOLS".to_owned(), "admin".to_owned()),
        (
            "PATH".to_owned(),
            format!("{}:/usr/bin:/bin", stub.dir.display()),
        ),
        ("STUB_LOG".to_owned(), stub.log.display().to_string()),
    ];
    env.extend(
        extra
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned())),
    );
    env
}

async fn start_at(socket: &Path, stub: &Stub, args: &[&str], extra: &[(&str, &str)]) -> Mcp {
    let env = env_with(stub, extra);
    let env: Vec<(&str, &str)> = env.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    Mcp::start(socket, &Opts { args, env: &env }).await
}

async fn start(tag: &str, extra: &[(&str, &str)]) -> (Stub, FakeHerdr, Mcp) {
    let stub = Stub::new(tag);
    let fake = FakeHerdr::start(&format!("cli-{tag}"), false);
    let mcp = start_at(&fake.socket, &stub, &[], extra).await;
    (stub, fake, mcp)
}

fn sc(r: &Value) -> &Value {
    &r["result"]["structuredContent"]
}

fn strs(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).to_owned()).collect()
}

#[tokio::test]
async fn each_tool_runs_its_command_as_argv() {
    let (stub, _f, mut mcp) = start("argv", &[]).await;
    let cases: &[(&str, Value, &[&str])] = &[
        ("herdr_config_check", json!({}), &["config", "check"]),
        (
            "herdr_config_reset_keys",
            json!({"confirm": true}),
            &["config", "reset-keys"],
        ),
        ("herdr_channel_show", json!({}), &["channel", "show"]),
        (
            "herdr_channel_set",
            json!({"channel":"preview"}),
            &["channel", "set", "preview"],
        ),
        (
            "herdr_session_list",
            json!({}),
            &["session", "list", "--json"],
        ),
        (
            "herdr_session_stop",
            json!({"name":"work"}),
            &["session", "stop", "work"],
        ),
        (
            "herdr_session_delete",
            json!({"name":"work"}),
            &["session", "delete", "work"],
        ),
        (
            "herdr_machine_list",
            json!({}),
            &["machine", "list", "--json"],
        ),
        (
            "herdr_machine_status",
            json!({}),
            &["machine", "status", "--json"],
        ),
        (
            "herdr_machine_status",
            json!({"machine":"box"}),
            &["machine", "status", "box", "--json"],
        ),
        (
            "herdr_machine_rename",
            json!({"machine":"m1","new_label":"new name"}),
            &["machine", "rename", "m1", "--label", "new name"],
        ),
        (
            "herdr_machine_remove",
            json!({"machine":"m1"}),
            &["machine", "remove", "m1"],
        ),
        (
            "herdr_machine_enable",
            json!({"machine":"m1"}),
            &["machine", "enable", "m1"],
        ),
        (
            "herdr_machine_disable",
            json!({"machine":"m1"}),
            &["machine", "disable", "m1"],
        ),
        ("herdr_update", json!({"confirm": true}), &["update"]),
        (
            "herdr_update",
            json!({"confirm": true, "handoff": true}),
            &["update", "--handoff"],
        ),
    ];
    for (n, (tool, args, argv)) in cases.iter().enumerate() {
        let r = mcp.call(tool, args.clone()).await;
        assert_eq!(r["result"]["isError"], false, "{tool}: {r}");
        assert_eq!(sc(&r)["exit"], 0, "{tool}");
        assert_eq!(sc(&r)["stdout"], "stub out\n", "{tool}");
        assert_eq!(sc(&r)["stderr"], "stub err\n", "{tool}");
        assert_eq!(sc(&r)["truncated"], false, "{tool}");
        assert_eq!(stub.calls()[n], strs(argv), "{tool}");
    }
}

#[tokio::test]
async fn a_nonzero_exit_is_reported_not_hidden() {
    let (_s, _f, mut mcp) = start("exit", &[("STUB_MODE", "exit3")]).await;
    let r = mcp.call("herdr_config_check", json!({})).await;
    assert_eq!(sc(&r)["exit"], 3);
    assert_eq!(sc(&r)["stdout"], "out\n");
    assert_eq!(sc(&r)["stderr"], "err\n");
}

#[tokio::test]
async fn values_that_look_like_flags_never_reach_herdr() {
    let (stub, _f, mut mcp) = start("flags", &[]).await;
    let tools = [
        ("herdr_session_stop", "name"),
        ("herdr_session_delete", "name"),
        ("herdr_machine_status", "machine"),
        ("herdr_machine_remove", "machine"),
        ("herdr_machine_enable", "machine"),
        ("herdr_machine_disable", "machine"),
        ("herdr_machine_rename", "machine"),
    ];
    for (tool, field) in tools {
        for bad in ["--help", "-x", "", "--json", "a\nb"] {
            let mut args = json!({"new_label": "ok"});
            args[field] = json!(bad);
            let r = mcp.call(tool, args).await;
            assert_eq!(r["result"]["isError"], true, "{tool} {bad:?}: {r}");
            assert_eq!(sc(&r)["code"], "invalid_params", "{tool} {bad:?}");
        }
    }
    let r = mcp
        .call(
            "herdr_machine_rename",
            json!({"machine":"m1","new_label":"--label"}),
        )
        .await;
    assert_eq!(sc(&r)["code"], "invalid_params");
    assert!(stub.calls().is_empty(), "{:?}", stub.calls());
}

#[tokio::test]
async fn shell_syntax_is_one_argument_never_a_command() {
    let (stub, _f, mut mcp) = start("shell", &[]).await;
    for value in ["x; rm -rf /", "$(id)", "a\"b'c", "a && b", "`id`"] {
        let r = mcp
            .call("herdr_machine_remove", json!({"machine": value}))
            .await;
        assert_eq!(r["result"]["isError"], false, "{value}: {r}");
    }
    let got: Vec<_> = stub.calls().into_iter().map(|c| c[2].clone()).collect();
    assert_eq!(got, ["x; rm -rf /", "$(id)", "a\"b'c", "a && b", "`id`"]);
    assert!(stub.calls().iter().all(|c| c.len() == 3));
}

#[tokio::test]
async fn update_and_reset_keys_need_confirm() {
    let (stub, _f, mut mcp) = start("confirm", &[]).await;
    for tool in ["herdr_update", "herdr_config_reset_keys"] {
        for args in [json!({}), json!({"confirm": false})] {
            let r = mcp.call(tool, args).await;
            assert_eq!(r["result"]["isError"], true, "{tool}: {r}");
            assert_eq!(sc(&r)["code"], "invalid_params");
        }
    }
    let r = mcp.call("herdr_update", json!({})).await;
    let text = r["result"]["content"][0]["text"].as_str().expect("text");
    assert!(text.contains("restarts"), "{text}");
    assert!(stub.calls().is_empty());
}

#[tokio::test]
async fn the_own_session_is_not_stopped_or_deleted() {
    let stub = Stub::new("own");
    // A socket in herdr's own `sessions/<name>/` layout, so the session is `own`.
    let fake = FakeHerdr::start("cli-own/sessions/own", false);
    let mut mcp = start_at(&fake.socket, &stub, &[], &[("HERDR_PANE_ID", "wA:p1")]).await;
    for tool in ["herdr_session_stop", "herdr_session_delete"] {
        let r = mcp.call(tool, json!({"name":"own"})).await;
        assert_eq!(r["result"]["isError"], true, "{tool}: {r}");
        assert_eq!(sc(&r)["code"], "self_target");
        let r = mcp
            .call(tool, json!({"name":"own","allow_self":false}))
            .await;
        assert_eq!(sc(&r)["code"], "self_target");
    }
    assert!(stub.calls().is_empty());
    let r = mcp
        .call("herdr_session_stop", json!({"name":"other"}))
        .await;
    assert_eq!(r["result"]["isError"], false, "{r}");
    let r = mcp
        .call(
            "herdr_session_delete",
            json!({"name":"own","allow_self":true}),
        )
        .await;
    assert_eq!(r["result"]["isError"], false, "{r}");
    assert_eq!(
        stub.calls(),
        vec![
            strs(&["session", "stop", "other"]),
            strs(&["session", "delete", "own"])
        ]
    );
}

#[tokio::test]
async fn an_unknown_own_session_refuses_a_stop() {
    // The harness socket is outside herdr's layout; with a pane id the server cannot
    // tell which session it is in, so it refuses (fail closed) until `allow_self`.
    let (stub, _f, mut mcp) = start("unknown", &[("HERDR_PANE_ID", "wA:p1")]).await;
    let r = mcp.call("herdr_session_stop", json!({"name":"x"})).await;
    assert_eq!(sc(&r)["code"], "self_target");
    assert!(stub.calls().is_empty());
}

#[tokio::test]
async fn a_command_that_never_ends_times_out() {
    let (_s, _f, mut mcp) = start(
        "slow",
        &[("STUB_MODE", "sleep"), ("HERDR_MCP_TIMEOUT_MS", "300")],
    )
    .await;
    let began = std::time::Instant::now();
    let r = mcp.call("herdr_config_check", json!({})).await;
    assert!(began.elapsed() < std::time::Duration::from_secs(4));
    assert_eq!(r["result"]["isError"], false, "{r}");
    assert_eq!(sc(&r)["timed_out"], true);
    assert_eq!(sc(&r)["exit"], Value::Null);
}

#[tokio::test]
async fn a_huge_reply_is_capped_and_marked() {
    let (_s, _f, mut mcp) = start("big", &[("STUB_MODE", "big")]).await;
    let r = mcp.call("herdr_channel_show", json!({})).await;
    assert_eq!(sc(&r)["truncated"], true);
    let out = sc(&r)["stdout"].as_str().expect("stdout");
    assert!(out.len() <= 40_000, "{}", out.len());
}

#[tokio::test]
async fn a_missing_herdr_binary_is_unreachable_not_a_hang() {
    let stub = Stub::new("nobin");
    std::fs::remove_file(stub.dir.join("herdr")).expect("remove stub");
    let fake = FakeHerdr::start("cli-nobin", false);
    let env = [
        ("HERDR_ENV", "1"),
        ("HERDR_MCP_TOOLS", "admin"),
        ("PATH", "/nonexistent-herdr-path"),
    ];
    let mut mcp = Mcp::start(
        &fake.socket,
        &Opts {
            args: &[],
            env: &env,
        },
    )
    .await;
    let r = mcp.call("herdr_config_check", json!({})).await;
    assert_eq!(r["result"]["isError"], true, "{r}");
    assert_eq!(sc(&r)["code"], "herdr_unreachable");
}

#[tokio::test]
async fn read_only_keeps_only_the_cli_reads_and_the_rest_are_destructive() {
    let (_s, _f, mut mcp) = start("ro", &[]).await;
    let r = mcp.request("tools/list", json!({})).await;
    let reads = [
        "herdr_channel_show",
        "herdr_config_check",
        "herdr_machine_list",
        "herdr_machine_status",
        "herdr_session_list",
    ];
    for t in r["result"]["tools"].as_array().expect("tools") {
        let name = t["name"].as_str().expect("name");
        if ["herdr_integration_list", "herdr_server_agent_manifests"].contains(&name) {
            continue;
        }
        let a = &t["annotations"];
        if reads.contains(&name) {
            assert_eq!(a["readOnlyHint"], true, "{name}");
        } else if [
            "herdr_channel_set",
            "herdr_config_reset_keys",
            "herdr_machine_disable",
            "herdr_machine_enable",
            "herdr_machine_remove",
            "herdr_machine_rename",
            "herdr_session_delete",
            "herdr_session_stop",
            "herdr_update",
        ]
        .contains(&name)
        {
            assert_eq!(a["destructiveHint"], true, "{name}");
            assert_eq!(a["readOnlyHint"], false, "{name}");
        }
    }
    let stub = Stub::new("ro2");
    let fake = FakeHerdr::start("cli-ro2", false);
    let mut ro = start_at(&fake.socket, &stub, &["--read-only"], &[]).await;
    let names = ro.tool_names().await;
    for n in reads {
        assert!(names.contains(&n.to_owned()), "{n} missing read-only");
    }
    for gone in [
        "herdr_update",
        "herdr_session_stop",
        "herdr_config_reset_keys",
    ] {
        assert!(!names.contains(&gone.to_owned()), "{gone} in read-only");
    }
}
