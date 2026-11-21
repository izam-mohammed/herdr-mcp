//! Plan 13: herdr's surface is decided, not drifting. `docs/external/herdr-methods.txt`
//! and `herdr-commands.txt` hold every socket method and CLI command of the herdr
//! version they name; `docs/herdr-coverage.toml` maps each to a tool or to a skip
//! with a reason. This fails on an entry with no row, a row for something herdr
//! no longer has, a malformed row, and a row naming a tool the server does not
//! register. It reads the checked-in lists, so it runs offline in CI; after a
//! herdr upgrade `sh scripts/sync-herdr.sh` regenerates the lists and prints what
//! is new, and the fix is a row.
mod support;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use support::{FakeHerdr, Mcp, Opts};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn entries(file: &str) -> BTreeSet<String> {
    let text = std::fs::read_to_string(root().join(file)).expect("read list");
    assert!(
        text.starts_with("# herdr "),
        "{file} must start with its herdr version line"
    );
    text.lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(str::to_owned)
        .collect()
}

#[derive(Debug, PartialEq, Eq)]
enum Row {
    Tool(String),
    Skip(String),
}

/// `[methods]` and `[commands]` tables, one row per line: `"key" = "tool"` or
/// `"key" = { skip = "reason" }`. Anything else is a failure, so the sh script
/// that reads the same file and this test agree on the shape.
fn manifest() -> (BTreeMap<String, Row>, BTreeMap<String, Row>) {
    let text = std::fs::read_to_string(root().join("docs/herdr-coverage.toml")).expect("manifest");
    let (mut methods, mut commands) = (BTreeMap::new(), BTreeMap::new());
    let mut table = "";
    for (n, line) in text.lines().enumerate() {
        let at = format!("docs/herdr-coverage.toml:{}", n + 1);
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        match line {
            "[methods]" => table = "methods",
            "[commands]" => table = "commands",
            _ => {
                let rest = line
                    .strip_prefix('"')
                    .unwrap_or_else(|| panic!("{at}: a row starts with a quote"));
                let (key, value) = rest
                    .split_once("\" = ")
                    .unwrap_or_else(|| panic!("{at}: a row is \"key\" = value"));
                let row = if let Some(reason) = value
                    .strip_prefix("{ skip = \"")
                    .and_then(|v| v.strip_suffix("\" }"))
                {
                    assert!(
                        !reason.is_empty() && !reason.contains('"'),
                        "{at}: a skip needs a reason without quotes"
                    );
                    Row::Skip(reason.to_owned())
                } else {
                    let tool = value
                        .strip_prefix('"')
                        .and_then(|v| v.strip_suffix('"'))
                        .unwrap_or_else(|| panic!("{at}: a tool name or a skip"));
                    assert!(
                        tool.starts_with("herdr_")
                            && tool.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
                        "{at}: {tool} is not a tool name"
                    );
                    Row::Tool(tool.to_owned())
                };
                let target = match table {
                    "methods" => &mut methods,
                    "commands" => &mut commands,
                    _ => panic!("{at}: a row before [methods] or [commands]"),
                };
                assert!(
                    target.insert(key.to_owned(), row).is_none(),
                    "{at}: {key} twice"
                );
            }
        }
    }
    (methods, commands)
}

fn compare(what: &str, listed: &BTreeSet<String>, rows: &BTreeMap<String, Row>) {
    let unmapped: Vec<_> = listed.iter().filter(|e| !rows.contains_key(*e)).collect();
    assert!(
        unmapped.is_empty(),
        "{what} with no row in docs/herdr-coverage.toml (give each a tool or a skip with a reason): {unmapped:?}"
    );
    let stale: Vec<_> = rows.keys().filter(|k| !listed.contains(*k)).collect();
    assert!(
        stale.is_empty(),
        "{what} rows for something herdr no longer lists (delete them): {stale:?}"
    );
}

#[test]
fn every_method_and_command_has_a_row_and_none_is_stale() {
    let (methods, commands) = manifest();
    compare(
        "methods",
        &entries("docs/external/herdr-methods.txt"),
        &methods,
    );
    compare(
        "commands",
        &entries("docs/external/herdr-commands.txt"),
        &commands,
    );
}

#[tokio::test]
async fn every_mapped_tool_is_registered() {
    let fake = FakeHerdr::start("coverage", false);
    let groups = herdr_mcp::config::GROUPS.join(",");
    let mut mcp = Mcp::start(
        &fake.socket,
        &Opts {
            args: &["--tools", &groups],
            env: &[("HERDR_ENV", "1")],
        },
    )
    .await;
    let have: BTreeSet<String> = mcp.tool_names().await.into_iter().collect();
    let (methods, commands) = manifest();
    let missing: BTreeSet<&str> = methods
        .values()
        .chain(commands.values())
        .filter_map(|r| match r {
            Row::Tool(t) => Some(t.as_str()),
            Row::Skip(_) => None,
        })
        .filter(|t| !have.contains(*t))
        .collect();
    assert!(
        missing.is_empty(),
        "rows name tools the server does not register: {missing:?}"
    );
}

fn set(items: &[&str]) -> BTreeSet<String> {
    items.iter().map(|s| (*s).to_owned()).collect()
}

#[test]
#[should_panic(expected = "with no row")]
fn an_entry_without_a_row_fails() {
    let rows = BTreeMap::from([("a".to_owned(), Row::Skip("x".to_owned()))]);
    compare("methods", &set(&["a", "b"]), &rows);
}

#[test]
#[should_panic(expected = "no longer lists")]
fn a_row_for_something_gone_fails() {
    let rows = BTreeMap::from([
        ("a".to_owned(), Row::Skip("x".to_owned())),
        ("gone".to_owned(), Row::Tool("herdr_x".to_owned())),
    ]);
    compare("methods", &set(&["a"]), &rows);
}
