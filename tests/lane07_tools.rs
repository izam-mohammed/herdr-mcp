//! Plan 07 on a fake herdr: resources and prompts.
#[path = "support/fake_events.rs"]
mod fake_events;
mod support;
use fake_events::FakeEvents;
use serde_json::{Value, json};
use support::{FakeHerdr, Mcp, Opts};

const FIXTURES: &str = include_str!("fixtures/lane07.jsonl");
/// Groups named, so no hub sends requests of its own.
const READS: &[(&str, &str)] = &[("HERDR_ENV", "1"), ("HERDR_MCP_TOOLS", "session")];

async fn start(fake: &FakeHerdr) -> Mcp {
    Mcp::start(
        &fake.socket,
        &Opts {
            args: &[],
            env: READS,
        },
    )
    .await
}

async fn read(mcp: &mut Mcp, uri: &str) -> Value {
    mcp.request("resources/read", json!({"uri": uri})).await
}

fn text(r: &Value) -> Value {
    let c = &r["result"]["contents"][0];
    assert_eq!(c["mimeType"], "application/json", "{r}");
    serde_json::from_str(c["text"].as_str().expect("text")).expect("json text")
}

fn uris(r: &Value, key: &str, field: &str) -> Vec<String> {
    let mut v: Vec<String> = r["result"][key]
        .as_array()
        .expect(key)
        .iter()
        .map(|x| x[field].as_str().expect(field).to_owned())
        .collect();
    v.sort();
    v
}

#[tokio::test]
async fn lists_resources_and_templates_without_events() {
    let fake = FakeHerdr::start_with("r1", false, FIXTURES);
    let mut mcp = start(&fake).await;
    let r = mcp.request("resources/list", json!({})).await;
    assert_eq!(
        uris(&r, "resources", "uri"),
        ["herdr://agents", "herdr://session", "herdr://workspaces"]
    );
    let t = mcp.request("resources/templates/list", json!({})).await;
    assert_eq!(
        uris(&t, "resourceTemplates", "uriTemplate"),
        [
            "herdr://pane/{id}/output",
            "herdr://tab/{id}",
            "herdr://workspace/{id}"
        ]
    );
    let desc = t["result"]["resourceTemplates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["name"] == "pane-output")
        .unwrap()["description"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(desc.contains("untrusted"), "{desc}");
    assert!(fake.requests().is_empty(), "listing must not call herdr");
}

#[tokio::test]
async fn each_uri_reads_through_to_its_herdr_call() {
    let fake = FakeHerdr::start_with("r2", false, FIXTURES);
    let mut mcp = start(&fake).await;
    let cases = [
        ("herdr://session", "session.snapshot"),
        ("herdr://workspaces", "workspace.list"),
        ("herdr://workspace/wA", "workspace.get"),
        ("herdr://tab/wA:t1", "tab.get"),
        ("herdr://pane/wA:p1/output", "pane.read"),
        ("herdr://agents", "agent.list"),
    ];
    for (uri, method) in cases {
        let r = read(&mut mcp, uri).await;
        assert!(r["error"].is_null(), "{uri}: {r}");
        assert_eq!(r["result"]["contents"][0]["uri"], uri);
        let seen = fake.requests();
        assert_eq!(seen.last().unwrap()["method"], method, "{uri}");
    }
    assert_eq!(fake.requests().len(), cases.len());
    let r = read(&mut mcp, "herdr://agents").await;
    assert_eq!(text(&r)["agents"][0]["agent_status"], "blocked");
    let r = read(&mut mcp, "herdr://workspace/wA").await;
    assert_eq!(text(&r)["workspace"]["label"], "main");
    let r = read(&mut mcp, "herdr://pane/wA:p1/output").await;
    assert_eq!(text(&r)["read"]["text"], "hello from the pane");
}

#[tokio::test]
async fn unknown_uris_and_ids_are_not_found_without_calling_herdr() {
    let fake = FakeHerdr::start_with("r3", false, FIXTURES);
    let mut mcp = start(&fake).await;
    for uri in [
        "herdr://nope",
        "herdr://workspace/",
        "herdr://workspace/a/b",
        "herdr://pane/wA:p1",
        "file:///etc/passwd",
        "herdr://events/recent",
    ] {
        let r = read(&mut mcp, uri).await;
        assert_eq!(r["error"]["code"], -32002, "{uri}: {r}");
    }
    assert!(fake.requests().is_empty());
    // A well-formed id herdr does not know: herdr's not_found becomes the same error.
    let r = read(&mut mcp, "herdr://workspace/wZ").await;
    assert_eq!(r["error"]["code"], -32002, "{r}");
    let r = read(&mut mcp, "herdr://tab/wZ:t9").await;
    assert_eq!(r["error"]["code"], -32002, "{r}");
}

#[tokio::test]
async fn pane_output_is_capped_keeping_the_end() {
    let fake = FakeHerdr::start_with("r4", false, FIXTURES);
    let mut mcp = start(&fake).await;
    let r = read(&mut mcp, "herdr://pane/wA:p9/output").await;
    let v = text(&r);
    assert_eq!(v["truncated"], true);
    let t = v["read"]["text"].as_str().unwrap();
    assert_eq!(t.chars().count(), 40_000);
    assert!(t.ends_with("END") && !t.contains("START"));
}

#[tokio::test]
async fn events_resource_is_listed_only_with_the_events_group() {
    let fake = FakeEvents::start("r5", &[]);
    let mut mcp = Mcp::start(
        &fake.socket,
        &Opts {
            args: &[],
            env: &[("HERDR_ENV", "1"), ("HERDR_MCP_TOOLS", "events")],
        },
    )
    .await;
    let r = mcp.request("resources/list", json!({})).await;
    assert!(
        uris(&r, "resources", "uri").contains(&"herdr://events/recent".to_owned()),
        "{r}"
    );
    fake.wait_subscribed(1).await;
    let tab =
        json!({"type":"tab_created","tab":{"tab_id":"wA:t2","workspace_id":"wA","label":"x"}});
    fake.emit("tab.created", "tab_created", &tab);
    let mut got = Value::Null;
    for _ in 0..50 {
        got = text(&read(&mut mcp, "herdr://events/recent").await);
        if !got["events"].as_array().unwrap().is_empty() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    assert!(got["next"].as_u64().unwrap() >= 1, "{got}");
    assert!(got["gaps"].is_array());
}

#[tokio::test]
async fn prompts_list_and_fill_their_argument() {
    let fake = FakeHerdr::start_with("r6", false, FIXTURES);
    let mut mcp = start(&fake).await;
    let r = mcp.request("prompts/list", json!({})).await;
    assert_eq!(
        uris(&r, "prompts", "name"),
        ["check-agents", "dashboard", "run-and-watch"]
    );
    let run = r["result"]["prompts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["name"] == "run-and-watch")
        .unwrap();
    assert_eq!(run["arguments"][0]["name"], "command");
    assert_eq!(run["arguments"][0]["required"], true);

    for (name, tool) in [
        ("dashboard", "herdr_snapshot"),
        ("check-agents", "herdr_agent_list"),
    ] {
        let r = mcp.request("prompts/get", json!({"name": name})).await;
        let t = r["result"]["messages"][0]["content"]["text"]
            .as_str()
            .unwrap();
        assert!(t.contains(tool), "{name}: {t}");
    }
    let r = mcp
        .request(
            "prompts/get",
            json!({"name":"run-and-watch","arguments":{"command":"cargo test"}}),
        )
        .await;
    let t = r["result"]["messages"][0]["content"]["text"]
        .as_str()
        .unwrap();
    assert!(
        t.contains("cargo test") && t.contains("herdr_pane_run"),
        "{t}"
    );

    let r = mcp
        .request("prompts/get", json!({"name":"run-and-watch"}))
        .await;
    assert_eq!(r["error"]["code"], -32602, "{r}");
    let r = mcp.request("prompts/get", json!({"name":"nope"})).await;
    assert_eq!(r["error"]["code"], -32602, "{r}");
    assert!(fake.requests().is_empty());
}
