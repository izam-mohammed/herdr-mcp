//! Plan 05 on a fake herdr with an event stream: the ring, the three tools,
//! and (later phases) rebuilds, lost events and reconnects.
#[path = "support/fake_events.rs"]
mod fake_events;
mod support;
use fake_events::FakeEvents;
use serde_json::{Value, json};
use std::time::Duration;
use support::{Mcp, Opts};

const INSIDE: &[(&str, &str)] = &[("HERDR_ENV", "1"), ("HERDR_MCP_TOOLS", "events")];

async fn start(fake: &FakeEvents, args: &[&str]) -> Mcp {
    Mcp::start(&fake.socket, &Opts { args, env: INSIDE }).await
}

fn ok(r: &Value) -> &Value {
    assert_eq!(r["result"]["isError"], false, "{r}");
    &r["result"]["structuredContent"]
}

fn tab_created(ws: &str, tab: &str) -> Value {
    json!({"type":"tab_created","tab":{"tab_id":tab,"workspace_id":ws,"label":"x"}})
}

fn seqs(page: &Value) -> Vec<u64> {
    page["events"]
        .as_array()
        .expect("events")
        .iter()
        .map(|e| e["seq"].as_u64().expect("seq"))
        .collect()
}

#[tokio::test]
async fn event_tools_are_listed_and_work_in_read_only_mode() {
    let fake = FakeEvents::start("l1", &[]);
    // Outside herdr, so read-only: the three tools are reads and stay.
    let mut mcp = Mcp::start(
        &fake.socket,
        &Opts {
            args: &[],
            env: &[("HERDR_MCP_TOOLS", "events")],
        },
    )
    .await;
    let names = mcp.tool_names().await;
    for n in [
        "herdr_events_recent",
        "herdr_events_status",
        "herdr_events_wait",
    ] {
        assert!(names.iter().any(|x| x == n), "{n} missing in {names:?}");
    }
    let r = mcp.request("tools/list", json!({})).await;
    for t in r["result"]["tools"].as_array().unwrap() {
        if t["name"].as_str().unwrap().starts_with("herdr_events_") {
            assert_eq!(t["annotations"]["readOnlyHint"], true);
        }
    }
    // The group can be left out.
    let mut off = Mcp::start(
        &fake.socket,
        &Opts {
            args: &["--tools", "session"],
            env: INSIDE,
        },
    )
    .await;
    assert!(
        !off.tool_names()
            .await
            .iter()
            .any(|n| n.starts_with("herdr_events_"))
    );
}

#[tokio::test]
async fn recent_returns_events_by_cursor_with_herdrs_names_unchanged() {
    let fake = FakeEvents::start("r1", &[]);
    let mut mcp = start(&fake, &[]).await;
    fake.wait_subscribed(1).await;
    let subs = fake.subscribes();
    let types: Vec<&str> = subs[0]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["type"].as_str().unwrap())
        .collect();
    assert!(types.contains(&"tab.created") && types.contains(&"workspace.focused"));

    fake.emit("tab.created", "tab_created", &tab_created("w1", "w1:t2"));
    fake.emit(
        "tab.closed",
        "tab_closed",
        &json!({"type":"tab_closed","tab_id":"w1:t2","workspace_id":"w1"}),
    );
    let st = poll_status(&mut mcp, 2).await;
    assert_eq!(st["last_seq"], 2);
    assert_eq!(st["connected"], true);

    let all = mcp.call("herdr_events_recent", json!({"since": 0})).await;
    let page = ok(&all);
    assert_eq!(seqs(page), [1, 2]);
    assert_eq!(page["events"][0]["event"], "tab_created");
    assert_eq!(page["events"][0]["data"]["tab"]["tab_id"], "w1:t2");
    assert_eq!(page["next"], 2);
    assert_eq!(page["gaps"], json!([]));

    let after = mcp.call("herdr_events_recent", json!({"since": 1})).await;
    assert_eq!(seqs(ok(&after)), [2]);
    let latest = mcp.call("herdr_events_recent", json!({"limit": 1})).await;
    assert_eq!(seqs(ok(&latest)), [2]);
    let paged = mcp
        .call("herdr_events_recent", json!({"since": 0, "limit": 1}))
        .await;
    assert_eq!(
        (seqs(ok(&paged)), ok(&paged)["next"].clone()),
        (vec![1], json!(1))
    );

    let by_type = mcp
        .call(
            "herdr_events_recent",
            json!({"since": 0, "types": ["tab.closed"]}),
        )
        .await;
    assert_eq!(seqs(ok(&by_type)), [2]);
    let by_tab = mcp
        .call(
            "herdr_events_recent",
            json!({"since": 0, "tab_id": "w1:t2", "workspace_id": "w1"}),
        )
        .await;
    assert_eq!(seqs(ok(&by_tab)), [1, 2]);
    let none = mcp
        .call(
            "herdr_events_recent",
            json!({"since": 0, "workspace_id": "w9"}),
        )
        .await;
    assert!(seqs(ok(&none)).is_empty());
}

async fn poll_status(mcp: &mut Mcp, last_seq: u64) -> Value {
    for _ in 0..100 {
        let s = mcp.call("herdr_events_status", json!({})).await;
        if ok(&s)["last_seq"].as_u64() >= Some(last_seq) {
            return ok(&s).clone();
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("last_seq never reached {last_seq}");
}

#[tokio::test]
async fn the_buffer_evicts_oldest_and_says_so() {
    let fake = FakeEvents::start("e1", &[]);
    let mut mcp = start(&fake, &["--event-buffer", "3"]).await;
    fake.wait_subscribed(1).await;
    for i in 0..5 {
        fake.emit(
            "tab.created",
            "tab_created",
            &tab_created("w1", &format!("w1:t{i}")),
        );
    }
    let st = poll_status(&mut mcp, 5).await;
    assert_eq!(
        (st["buffered"].clone(), st["capacity"].clone()),
        (json!(3), json!(3))
    );
    let r = mcp.call("herdr_events_recent", json!({"since": 1})).await;
    let page = ok(&r);
    assert_eq!(seqs(page), [3, 4, 5]);
    assert_eq!(page["gaps"][0]["from_seq"], 2);
    assert_eq!(page["gaps"][0]["to_seq"], 2);
}

#[tokio::test]
async fn bad_event_buffer_is_refused_at_start() {
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_herdr-mcp"))
        .args(["--event-buffer", "0"])
        .env_clear()
        .output()
        .expect("run");
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("--event-buffer"));
}

#[tokio::test]
async fn wait_returns_the_next_matching_event_and_times_out_otherwise() {
    let fake = FakeEvents::start("w1", &[]);
    let mut mcp = start(&fake, &[]).await;
    fake.wait_subscribed(1).await;

    // An event before the call does not count; one after it does.
    fake.emit("tab.created", "tab_created", &tab_created("w1", "w1:t1"));
    poll_status(&mut mcp, 1).await;
    let emitter = fake.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(300)).await;
        emitter.emit(
            "tab.focused",
            "tab_focused",
            &json!({"type":"tab_focused","tab_id":"w1:t5","workspace_id":"w1"}),
        );
        emitter.emit("tab.created", "tab_created", &tab_created("w1", "w1:t6"));
    });
    let r = mcp
        .call(
            "herdr_events_wait",
            json!({"types": ["tab.created"], "timeout_ms": 5000}),
        )
        .await;
    let got = ok(&r);
    assert_eq!(got["timed_out"], false);
    assert_eq!(got["event"]["data"]["tab"]["tab_id"], "w1:t6");
    assert_eq!(got["event"]["seq"], 3);
    assert_eq!(got["next"], 3);

    // A cursor returns a buffered event at once.
    let r = mcp
        .call("herdr_events_wait", json!({"since": 0, "timeout_ms": 100}))
        .await;
    assert_eq!(ok(&r)["event"]["seq"], 1);

    // Nothing matches: it ends at its timeout, even for a pane that does not exist.
    let started = std::time::Instant::now();
    let r = mcp
        .call(
            "herdr_events_wait",
            json!({"pane_id": "w9:p9", "timeout_ms": 300}),
        )
        .await;
    assert_eq!(ok(&r)["timed_out"], true);
    assert!(started.elapsed() >= Duration::from_millis(250));
}

fn status_changed(pane: &str, state: &str) -> Value {
    json!({"pane_id": pane, "workspace_id": "w1", "agent": "claude", "agent_status": state})
}

fn pane_event(pane: &str) -> Value {
    json!({"type":"pane_created","pane":{"pane_id":pane,"workspace_id":"w1","tab_id":"w1:t1"}})
}

/// The `pane_id`s of the `pane.agent_status_changed` entries of a subscribe.
fn watched(sub: &Value) -> Vec<String> {
    let mut panes: Vec<String> = sub
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e["type"] == "pane.agent_status_changed")
        .map(|e| {
            e["pane_id"]
                .as_str()
                .expect("pane_id on every entry")
                .to_owned()
        })
        .collect();
    panes.sort();
    panes
}

async fn until(what: &str, mut ok: impl FnMut() -> bool) {
    for _ in 0..300 {
        if ok() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("never: {what}");
}

#[tokio::test]
async fn every_pane_gets_a_status_entry_and_a_wait_can_name_the_state() {
    let fake = FakeEvents::start("a1", &["w1:p1", "w1:p2"]);
    let mut mcp = start(&fake, &[]).await;
    fake.wait_subscribed(1).await;
    assert_eq!(watched(&fake.subscribes()[0]), ["w1:p1", "w1:p2"]);
    assert_eq!(
        ok(&mcp.call("herdr_events_status", json!({})).await)["panes_watched"],
        2
    );

    let emitter = fake.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(200)).await;
        emitter.emit(
            "pane.agent_status_changed",
            "pane.agent_status_changed",
            &status_changed("w1:p2", "working"),
        );
        emitter.emit(
            "pane.agent_status_changed",
            "pane.agent_status_changed",
            &status_changed("w1:p1", "blocked"),
        );
    });
    let r = mcp
        .call(
            "herdr_events_wait",
            json!({"agent_status": "blocked", "timeout_ms": 5000}),
        )
        .await;
    let got = ok(&r);
    assert_eq!(got["timed_out"], false);
    assert_eq!(got["event"]["data"]["pane_id"], "w1:p1");
    // Dotted or snake, the type filter finds it.
    let r = mcp
        .call(
            "herdr_events_recent",
            json!({"since": 0, "types": ["pane_agent_status_changed"], "pane_id": "w1:p2"}),
        )
        .await;
    assert_eq!(ok(&r)["events"][0]["data"]["agent_status"], "working");
}

#[tokio::test]
async fn a_new_pane_is_watched_without_losing_or_doubling_events() {
    let fake = FakeEvents::start("a2", &["w1:p1"]);
    let mut mcp = start(&fake, &[]).await;
    fake.wait_subscribed(1).await;

    fake.set_panes(&["w1:p1", "w1:p2"]);
    fake.emit("pane.created", "pane_created", &pane_event("w1:p2"));
    until("the rebuilt subscription", || fake.subscribes().len() == 2).await;
    assert_eq!(watched(&fake.subscribes()[1]), ["w1:p1", "w1:p2"]);
    // The overlap: both subscriptions are live; an event now reaches both.
    until("two live subscriptions", || fake.live_subscriptions() == 2).await;
    fake.emit(
        "tab.focused",
        "tab_focused",
        &json!({"type":"tab_focused","tab_id":"w1:t1","workspace_id":"w1"}),
    );
    // ...and then the old one is closed.
    until("the old one closed", || fake.live_subscriptions() == 1).await;
    assert_eq!(
        ok(&mcp.call("herdr_events_status", json!({})).await)["panes_watched"],
        2
    );

    // The new pane's status changes arrive; nothing doubled, nothing lost.
    fake.emit(
        "pane.agent_status_changed",
        "pane.agent_status_changed",
        &status_changed("w1:p2", "blocked"),
    );
    poll_status(&mut mcp, 3).await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    let r = mcp.call("herdr_events_recent", json!({"since": 0})).await;
    let names: Vec<&str> = ok(&r)["events"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["event"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        ["pane_created", "tab_focused", "pane.agent_status_changed"]
    );
    assert_eq!(ok(&r)["gaps"], json!([]));

    // A pane that goes is dropped from the set.
    fake.set_panes(&["w1:p1"]);
    fake.emit(
        "pane.closed",
        "pane_closed",
        &json!({"type":"pane_closed","pane_id":"w1:p2","workspace_id":"w1"}),
    );
    until("the third subscription", || fake.subscribes().len() == 3).await;
    assert_eq!(watched(&fake.subscribes()[2]), ["w1:p1"]);
}
