//! Plan 11 Phase 2: the reference pages in `docs/book/src/reference/` are
//! written from the running server (its `tools/list`, `resources/list`,
//! `resources/templates/list`, `prompts/list` and `--help`), never by hand.
//! `HERDR_MCP_UPDATE_DOCS=1 cargo test --locked --test docs` rewrites them;
//! without it the test fails on any difference. Output is sorted and has no
//! dates or paths, so it is the same on every run.
#[path = "support/fake_events.rs"]
mod fake_events;
mod support;
use serde_json::Value;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::PathBuf;
use support::{FakeHerdr, Mcp, Opts};

const GROUPS: &[&str] = herdr_mcp::config::GROUPS;
const ENV: &[(&str, &str)] = &[("HERDR_ENV", "1")];

struct Tool {
    group: &'static str,
    json: Value,
}

async fn start(fake: &FakeHerdr, groups: &str) -> Mcp {
    Mcp::start(
        &fake.socket,
        &Opts {
            args: &["--tools", groups],
            env: ENV,
        },
    )
    .await
}

async fn list(mcp: &mut Mcp, method: &str, key: &str) -> Vec<Value> {
    let r = mcp.request(method, serde_json::json!({})).await;
    r["result"][key]
        .as_array()
        .unwrap_or_else(|| panic!("{method}: {r}"))
        .clone()
}

fn sorted_by(mut v: Vec<Value>, key: &str) -> Vec<Value> {
    v.sort_by_key(|x| x[key].as_str().unwrap_or_default().to_owned());
    v
}

/// One line for a table cell or a paragraph: no newlines, escaped bars.
fn cell(s: &str) -> String {
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace('|', "\\|")
}

fn mode(tool: &Value) -> &'static str {
    let a = &tool["annotations"];
    if a["readOnlyHint"] == true {
        "read"
    } else if a["destructiveHint"] == true {
        "destructive"
    } else {
        "write"
    }
}

fn yes(v: &Value) -> &'static str {
    if *v == true { "yes" } else { "no" }
}

fn resolve<'a>(root: &'a Value, schema: &'a Value) -> &'a Value {
    match schema["$ref"].as_str().and_then(|r| r.strip_prefix("#/")) {
        Some(path) => path.split('/').fold(root, |node, part| &node[part]),
        None => schema,
    }
}

/// A schema's type for a human: refs resolved, `null` dropped.
fn type_of(root: &Value, schema: &Value) -> String {
    let s = resolve(root, schema);
    if let Some(options) = s["anyOf"].as_array().or_else(|| s["oneOf"].as_array()) {
        let kinds: Vec<String> = options
            .iter()
            .filter(|o| o["type"] != "null")
            .map(|o| type_of(root, o))
            .collect();
        return kinds.join(" or ");
    }
    if let Some(c) = s.get("const") {
        return format!(
            "`{}`",
            c.as_str().map_or_else(|| c.to_string(), str::to_owned)
        );
    }
    if let Some(values) = s["enum"].as_array() {
        let all: Vec<String> = values
            .iter()
            .map(|v| {
                format!(
                    "`{}`",
                    v.as_str().map_or_else(|| v.to_string(), str::to_owned)
                )
            })
            .collect();
        return all.join(", ");
    }
    let kind = match &s["type"] {
        Value::String(t) => t.clone(),
        Value::Array(t) => t
            .iter()
            .filter_map(Value::as_str)
            .filter(|t| *t != "null")
            .collect::<Vec<_>>()
            .join(" or "),
        _ => "any".to_owned(),
    };
    if kind == "array" {
        return format!("array of {}", type_of(root, &s["items"]));
    }
    kind
}

fn is_object(root: &Value, schema: &Value) -> Option<Value> {
    let s = resolve(root, schema);
    if s["properties"].is_object() {
        return Some(s.clone());
    }
    for option in s["anyOf"].as_array().into_iter().flatten() {
        let o = resolve(root, option);
        if o["properties"].is_object() {
            return Some(o.clone());
        }
    }
    None
}

fn rows(root: &Value, object: &Value, prefix: &str, out: &mut Vec<String>) {
    let required: Vec<&str> = object["required"]
        .as_array()
        .map_or_default(|r| r.iter().filter_map(Value::as_str).collect());
    let props: BTreeMap<&String, &Value> = object["properties"]
        .as_object()
        .map_or_default(|p| p.iter().collect());
    for (name, schema) in props {
        let described = schema["description"]
            .as_str()
            .or_else(|| resolve(root, schema)["description"].as_str())
            .unwrap_or("");
        out.push(format!(
            "| `{prefix}{name}` | {} | {} | {} |",
            type_of(root, schema),
            if required.contains(&name.as_str()) {
                "yes"
            } else {
                "no"
            },
            cell(described),
        ));
        if let Some(inner) = is_object(root, schema) {
            rows(root, &inner, &format!("{prefix}{name}."), out);
        }
    }
}

fn tool_section(t: &Tool) -> String {
    let j = &t.json;
    let a = &j["annotations"];
    let name = j["name"].as_str().unwrap_or_default();
    let mut s = format!(
        "## `{name}`\n\n{}\n\n",
        cell(j["description"].as_str().unwrap_or(""))
    );
    let _ = writeln!(
        s,
        "Group `{}` ({}). Read-only: {}. Destructive: {}. Idempotent: {}. Reaches beyond herdr: {}.\n",
        t.group,
        mode(j),
        yes(&a["readOnlyHint"]),
        yes(&a["destructiveHint"]),
        yes(&a["idempotentHint"]),
        yes(&a["openWorldHint"]),
    );
    let schema = &j["inputSchema"];
    let mut table = Vec::new();
    rows(schema, schema, "", &mut table);
    if table.is_empty() {
        s.push_str("No parameters.\n");
    } else {
        s.push_str("| Parameter | Type | Required | What it is |\n| --- | --- | --- | --- |\n");
        for r in table {
            s.push_str(&r);
            s.push('\n');
        }
    }
    s
}

const NOTE: &str = "<!-- Generated from the server by `tests/docs.rs`. Do not edit: run \
`HERDR_MCP_UPDATE_DOCS=1 cargo test --locked --test docs`. -->\n\n";

fn tool_pages(tools: &[Tool], out: &mut BTreeMap<String, String>) {
    let mut index = format!(
        "{NOTE}# Tools\n\n{} tools in {} groups. `--tools` picks groups; `--read-only` (or running outside herdr) keeps only the `read` tools. \"destructive\" tools close or remove things.\n\n",
        tools.len(),
        GROUPS.len()
    );
    index.push_str(
        "| Group | Tools | Read | Write | Destructive |\n| --- | ---: | ---: | ---: | ---: |\n",
    );
    for g in GROUPS {
        let own: Vec<&Tool> = tools.iter().filter(|t| t.group == *g).collect();
        let count = |m: &str| own.iter().filter(|t| mode(&t.json) == m).count();
        let _ = writeln!(
            index,
            "| [`{g}`](tools-{g}.md) | {} | {} | {} | {} |",
            own.len(),
            count("read"),
            count("write"),
            count("destructive")
        );
    }
    index.push_str(
        "\n## Every tool\n\n| Tool | Group | Mode | What it does |\n| --- | --- | --- | --- |\n",
    );
    let mut all: Vec<&Tool> = tools.iter().collect();
    all.sort_by_key(|t| t.json["name"].as_str().unwrap_or_default().to_owned());
    for t in all {
        let name = t.json["name"].as_str().unwrap_or_default();
        let _ = writeln!(
            index,
            "| [`{name}`](tools-{}.md#{name}) | `{}` | {} | {} |",
            t.group,
            t.group,
            mode(&t.json),
            cell(t.json["description"].as_str().unwrap_or(""))
        );
    }
    out.insert("tools.md".to_owned(), index);

    for g in GROUPS {
        let mut page = format!("{NOTE}# Tools: {g}\n\n");
        let mut own: Vec<&Tool> = tools.iter().filter(|t| t.group == *g).collect();
        own.sort_by_key(|t| t.json["name"].as_str().unwrap_or_default().to_owned());
        for t in own {
            page.push_str(&tool_section(t));
            page.push('\n');
        }
        out.insert(format!("tools-{g}.md"), page.trim_end().to_owned() + "\n");
    }
}

fn pages(
    tools: &[Tool],
    resources: &[Value],
    templates: &[Value],
    prompts: &[Value],
    help: &str,
) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    tool_pages(tools, &mut out);

    let mut page = format!(
        "{NOTE}# Resources\n\nRead-only JSON you can attach with `@herdr:` in Claude Code. Each read goes through to herdr. Text in them is untrusted data.\n\n## Fixed resources\n\n| URI | Name | What it is |\n| --- | --- | --- |\n"
    );
    for r in resources {
        let _ = writeln!(
            page,
            "| `{}` | `{}` | {} |",
            r["uri"].as_str().unwrap_or(""),
            r["name"].as_str().unwrap_or(""),
            cell(r["description"].as_str().unwrap_or(""))
        );
    }
    page.push_str("\n## Templates\n\n| URI template | Name | What it is |\n| --- | --- | --- |\n");
    for r in templates {
        let _ = writeln!(
            page,
            "| `{}` | `{}` | {} |",
            r["uriTemplate"].as_str().unwrap_or(""),
            r["name"].as_str().unwrap_or(""),
            cell(r["description"].as_str().unwrap_or(""))
        );
    }
    page.push_str("\n`herdr://events/recent` is listed only when the `events` group is on.\n");
    out.insert("resources.md".to_owned(), page);

    let mut page = format!(
        "{NOTE}# Prompts\n\nShown as slash commands in Claude Code, for example `/mcp__herdr__dashboard`. Each is a short instruction that names the tools to use.\n\n"
    );
    for p in prompts {
        let _ = write!(
            page,
            "## `{}`\n\n{}\n\n",
            p["name"].as_str().unwrap_or(""),
            cell(p["description"].as_str().unwrap_or(""))
        );
        match p["arguments"].as_array().filter(|a| !a.is_empty()) {
            None => page.push_str("No arguments.\n\n"),
            Some(args) => {
                page.push_str("| Argument | Required | What it is |\n| --- | --- | --- |\n");
                for a in args {
                    let _ = writeln!(
                        page,
                        "| `{}` | {} | {} |",
                        a["name"].as_str().unwrap_or(""),
                        yes(&a["required"]),
                        cell(a["description"].as_str().unwrap_or(""))
                    );
                }
                page.push('\n');
            }
        }
    }
    out.insert("prompts.md".to_owned(), page.trim_end().to_owned() + "\n");

    let page = format!(
        "{NOTE}# Command line\n\n`herdr-mcp --help`:\n\n```text\n{}\n```\n\nFlags win over the environment variable named beside them.\n",
        help.trim_end()
    );
    out.insert("cli.md".to_owned(), page);
    out
}

fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("docs/book/src/reference")
}

#[tokio::test]
async fn docs_match_server() {
    let mut tools = Vec::new();
    for g in GROUPS {
        let fake = FakeHerdr::start(&format!("docs-{g}"), false);
        let mut mcp = start(&fake, g).await;
        for json in list(&mut mcp, "tools/list", "tools").await {
            tools.push(Tool { group: g, json });
        }
    }
    let fake = FakeHerdr::start("docs-all", false);
    let mut mcp = start(&fake, &GROUPS.join(",")).await;
    let all = list(&mut mcp, "tools/list", "tools").await;
    assert_eq!(all.len(), tools.len(), "a tool is in two groups or none");
    let resources = sorted_by(list(&mut mcp, "resources/list", "resources").await, "uri");
    let templates = sorted_by(
        list(&mut mcp, "resources/templates/list", "resourceTemplates").await,
        "uriTemplate",
    );
    let prompts = sorted_by(list(&mut mcp, "prompts/list", "prompts").await, "name");
    drop(mcp);

    let help = std::process::Command::new(env!("CARGO_BIN_EXE_herdr-mcp"))
        .arg("--help")
        .env_clear()
        .output()
        .expect("run --help");
    assert!(help.status.success());
    let help = String::from_utf8(help.stdout).expect("utf8 help");

    let want = pages(&tools, &resources, &templates, &prompts, &help);
    let dir = dir();
    if std::env::var_os("HERDR_MCP_UPDATE_DOCS").is_some() {
        std::fs::create_dir_all(&dir).expect("create reference dir");
        for entry in std::fs::read_dir(&dir)
            .expect("read reference dir")
            .flatten()
        {
            if !want.contains_key(entry.file_name().to_string_lossy().as_ref()) {
                std::fs::remove_file(entry.path()).expect("remove stale page");
            }
        }
        for (name, text) in &want {
            std::fs::write(dir.join(name), text).expect("write page");
        }
        return;
    }
    let mut stale = Vec::new();
    for (name, text) in &want {
        if std::fs::read_to_string(dir.join(name)).ok().as_deref() != Some(text) {
            stale.push(name.clone());
        }
    }
    for entry in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if !want.contains_key(&name) {
            stale.push(format!("{name} (no longer generated)"));
        }
    }
    assert!(
        stale.is_empty(),
        "reference pages differ from the server: {stale:?}. Run `HERDR_MCP_UPDATE_DOCS=1 cargo test --locked --test docs`"
    );
}
