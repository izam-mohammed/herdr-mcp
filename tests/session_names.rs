//! Every live test file and the bench script own one isolated herdr session
//! name. Two sharing a name would stop each other's server, so this fails on a
//! duplicate or on a name outside `herdr-mcp-test*` (never the default session).
use std::collections::BTreeMap;

fn names() -> BTreeMap<String, String> {
    let mut found = BTreeMap::new();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files: Vec<_> = std::fs::read_dir(root.join("tests"))
        .expect("tests dir")
        .map(|e| e.expect("entry").path())
        .filter(|p| p.extension().is_some_and(|x| x == "rs"))
        .collect();
    files.push(root.join("scripts/bench.sh"));
    files.push(root.join("scripts/verify/skill-doctor-ok.sh"));
    for path in files {
        let text = std::fs::read_to_string(&path).expect("read");
        for line in text.lines() {
            let name = line
                .strip_prefix("const SESSION: &str = \"")
                .and_then(|r| r.strip_suffix("\";"))
                .or_else(|| line.strip_prefix("SESSION="));
            if let Some(name) = name {
                let file = path.file_name().unwrap().to_string_lossy().into_owned();
                if let Some(other) = found.insert(name.to_owned(), file.clone()) {
                    panic!("session {name} is used by {other} and {file}");
                }
            }
        }
    }
    found
}

#[test]
fn live_sessions_are_isolated_and_unique() {
    let found = names();
    assert_eq!(
        found.len(),
        10,
        "expected 8 live files (live, lane02 to lane07, lane13), the bench and the skill verify: {found:?}"
    );
    for name in found.keys() {
        assert!(name.starts_with("herdr-mcp-test"), "{name}");
    }
}
