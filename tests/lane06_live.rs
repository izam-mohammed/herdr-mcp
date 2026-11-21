//! Plan 06 live tests against an isolated `herdr-mcp-test-06` session. They
//! run only with `HERDR_MCP_LIVE=1` and never touch the default session:
//! every herdr command names the test session and starts with `HERDR_*` unset.
mod support;
use serde_json::{Value, json};
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use support::{Mcp, Opts};

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

const SESSION: &str = "herdr-mcp-test-06";

/// One session, so one live test at a time, even under `cargo test` threads.
static ONE_AT_A_TIME: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

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
    server: Option<Child>,
    socket: std::path::PathBuf,
}

impl TestSession {
    fn socket_path() -> std::path::PathBuf {
        let home = std::env::var("HOME").expect("HOME");
        format!("{home}/.config/herdr/sessions/{SESSION}/herdr.sock").into()
    }

    async fn start() -> Self {
        let mut session = Self {
            server: None,
            socket: Self::socket_path(),
        };
        session.boot().await;
        session
    }

    async fn boot(&mut self) {
        self.server = Some(
            herdr()
                .arg("server")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .expect("start herdr server"),
        );
        for _ in 0..100 {
            if raw(&self.socket, "ping", json!({})).await.is_ok() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        panic!("the isolated herdr server did not come up");
    }

    /// Stop the server (not the session's files), as an update or crash would.
    fn stop_server(&mut self) {
        let _ = herdr().args(["server", "stop"]).output();
        if let Some(mut s) = self.server.take() {
            let _ = s.wait();
        }
    }
}

impl Drop for TestSession {
    fn drop(&mut self) {
        self.stop_server();
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

async fn mcp(session: &TestSession) -> Mcp {
    let home = std::env::var("HOME").expect("HOME");
    Mcp::start(
        &session.socket,
        &Opts {
            args: &["--session", SESSION],
            env: &[
                ("HERDR_ENV", "1"),
                ("HERDR_MCP_TOOLS", "worktrees,layout,notify,tabs,panes"),
                ("HOME", &home),
            ],
        },
    )
    .await
}

/// A throwaway git repository under `HERDR_MCP_SCRATCH` or the temp dir, never this repository. Removed on drop.
struct Scratch(std::path::PathBuf);

impl Scratch {
    fn new() -> Self {
        let base =
            std::env::var_os("HERDR_MCP_SCRATCH").map_or_else(std::env::temp_dir, Into::into);
        let dir = base.join(format!("herdr-mcp-06-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let repo = dir.join("repo");
        std::fs::create_dir_all(&repo).expect("mkdir");
        let git = |args: &[&str]| {
            let out = Command::new("git")
                .current_dir(&repo)
                .args(["-c", "user.name=t", "-c", "user.email=t@t"])
                .args(args)
                .output()
                .expect("git");
            assert!(out.status.success(), "git {args:?}");
        };
        git(&["init", "-q", "-b", "main"]);
        git(&["commit", "-q", "--allow-empty", "-m", "init"]);
        Self(dir)
    }

    fn repo(&self) -> String {
        self.0.join("repo").to_string_lossy().into_owned()
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[tokio::test]
async fn a_worktree_is_created_listed_and_removed_and_a_toast_is_accepted() {
    if !live() {
        eprintln!("skipped: set HERDR_MCP_LIVE=1");
        return;
    }
    let _one = ONE_AT_A_TIME.lock().await;
    let scratch = Scratch::new();
    let session = TestSession::start().await;
    let repo = scratch.repo();
    raw(
        &session.socket,
        "workspace.create",
        json!({"label":"live","cwd":repo,"focus":true}),
    )
    .await
    .expect("workspace.create");
    let mut mcp = mcp(&session).await;

    // No trust_repository sent: herdr creates the worktree anyway (docs/external).
    let path = scratch.0.join("wt").to_string_lossy().into_owned();
    let r = mcp
        .call(
            "herdr_worktree_create",
            json!({"cwd":repo,"branch":"feat-x","path":path}),
        )
        .await;
    let ws = ok(&r)["workspace"]["workspace_id"]
        .as_str()
        .expect("workspace id")
        .to_owned();
    assert!(std::path::Path::new(&path).join(".git").exists());

    let r = mcp.call("herdr_worktree_list", json!({"cwd":repo})).await;
    let listed = ok(&r)["worktrees"].as_array().expect("worktrees").clone();
    assert!(listed.iter().any(|w| w["branch"] == "feat-x"), "{listed:?}");

    let r = mcp
        .call("herdr_notify", json!({"title":"ready","body":"wt"}))
        .await;
    assert_eq!(ok(&r)["type"], "notification_show");

    // A clean worktree goes with force off (the default).
    let r = mcp
        .call("herdr_worktree_remove", json!({"workspace_id":ws}))
        .await;
    assert_eq!(ok(&r)["forced"], false);
    assert!(!std::path::Path::new(&path).exists());
    let r = mcp.call("herdr_worktree_list", json!({"cwd":repo})).await;
    assert!(
        !ok(&r)["worktrees"]
            .as_array()
            .expect("worktrees")
            .iter()
            .any(|w| w["branch"] == "feat-x")
    );
}

fn count_panes(layout: &Value) -> usize {
    match layout["type"].as_str() {
        Some("split") => count_panes(&layout["first"]) + count_panes(&layout["second"]),
        _ => 1,
    }
}

#[tokio::test]
async fn a_two_pane_tab_round_trips_through_export_and_apply() {
    if !live() {
        eprintln!("skipped: set HERDR_MCP_LIVE=1");
        return;
    }
    let _one = ONE_AT_A_TIME.lock().await;
    let session = TestSession::start().await;
    let ws = raw(
        &session.socket,
        "workspace.create",
        json!({"label":"live","focus":true}),
    )
    .await
    .expect("workspace.create");
    let pane = ws["root_pane"]["pane_id"]
        .as_str()
        .expect("pane")
        .to_owned();
    let tab = ws["tab"]["tab_id"].as_str().expect("tab").to_owned();
    raw(
        &session.socket,
        "pane.split",
        json!({"target_pane_id":pane,"direction":"right"}),
    )
    .await
    .expect("pane.split");
    let mut mcp = mcp(&session).await;

    let r = mcp.call("herdr_layout_export", json!({"tab_id":tab})).await;
    let exported = ok(&r)["layout"]["root"].clone();
    assert_eq!(count_panes(&exported), 2, "{exported}");

    // Applied with no tab_id: a new tab, the old one untouched.
    let r = mcp
        .call("herdr_layout_apply", json!({"root":exported}))
        .await;
    let new_tab = ok(&r)["layout"]["tab_id"].as_str().expect("tab").to_owned();
    assert_ne!(new_tab, tab);
    assert_eq!(count_panes(&ok(&r)["layout"]["root"]), 2);

    let r = mcp
        .call(
            "herdr_layout_set_split_ratio",
            json!({"tab_id":new_tab,"path":[],"ratio":0.7}),
        )
        .await;
    let ratio = ok(&r)["layout"]["root"]["ratio"].as_f64().expect("ratio");
    assert!((ratio - 0.7).abs() < 1e-6, "{ratio}");

    let r = mcp.call("herdr_tab_list", json!({})).await;
    assert_eq!(ok(&r)["tabs"].as_array().expect("tabs").len(), 2);
}
