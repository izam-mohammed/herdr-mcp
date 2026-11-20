//! Keeping `session.snapshot` small: filter by workspace or tab, and fall
//! back to counts and ids once the reply is large.
use serde_json::{Map, Value, json};

/// A reply longer than this (in characters) is summarised unless the caller
/// asked for the full snapshot.
pub const SUMMARY_OVER: usize = 20_000;

fn field<'a>(entry: &'a Value, key: &str) -> Option<&'a str> {
    entry.get(key).and_then(Value::as_str)
}

fn retain(snap: &mut Map<String, Value>, name: &str, keep: impl Fn(&Value) -> bool) {
    if let Some(list) = snap.get_mut(name).and_then(Value::as_array_mut) {
        list.retain(keep);
    }
}

/// Keep only what belongs to `workspace_id` and `tab_id`, in place.
pub fn filter(snapshot: &mut Value, workspace_id: Option<&str>, tab_id: Option<&str>) {
    let Some(snap) = snapshot.as_object_mut() else {
        return;
    };
    let in_scope = |e: &Value| {
        workspace_id.is_none_or(|w| field(e, "workspace_id") == Some(w))
            && tab_id.is_none_or(|t| field(e, "tab_id") == Some(t))
    };
    for name in ["tabs", "panes", "layouts"] {
        retain(snap, name, in_scope);
    }
    let panes = ids(snap.get("panes"), "pane_id");
    let tabs_ws = ids(snap.get("tabs"), "workspace_id");
    retain(snap, "agents", |a| {
        field(a, "pane_id").is_none_or(|p| panes.iter().any(|x| x == p))
    });
    retain(snap, "workspaces", |w| {
        let id = field(w, "workspace_id").unwrap_or_default();
        if tab_id.is_some() {
            tabs_ws.iter().any(|x| x == id)
        } else {
            workspace_id.is_none_or(|x| x == id)
        }
    });
}

fn ids(list: Option<&Value>, key: &str) -> Vec<String> {
    list.and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|e| field(e, key).map(str::to_owned))
        .collect()
}

/// Counts and ids only.
#[must_use]
pub fn summarize(snapshot: &Value) -> Value {
    let mut out = Map::new();
    for key in [
        "version",
        "protocol",
        "focused_workspace_id",
        "focused_tab_id",
        "focused_pane_id",
    ] {
        if let Some(v) = snapshot.get(key) {
            out.insert(key.into(), v.clone());
        }
    }
    let mut counts = Map::new();
    for (name, id_key) in [
        ("workspaces", "workspace_id"),
        ("tabs", "tab_id"),
        ("panes", "pane_id"),
        ("agents", "pane_id"),
    ] {
        let list = ids(snapshot.get(name), id_key);
        counts.insert(name.into(), json!(list.len()));
        out.insert(name.into(), json!(list));
    }
    out.insert("counts".into(), Value::Object(counts));
    Value::Object(out)
}

/// What the tool returns: the (filtered) result, or its summary when asked
/// for or when the full reply passes [`SUMMARY_OVER`] and the caller did not
/// ask for it. The second value is the one-line text for the client.
#[must_use]
pub fn shape(mut result: Value, args: &Args) -> (Value, String) {
    if let Some(snap) = result.get_mut("snapshot") {
        filter(snap, args.workspace_id.as_deref(), args.tab_id.as_deref());
    }
    let size = result.to_string().chars().count();
    let summarise = args.summary.unwrap_or(size > SUMMARY_OVER);
    if !summarise {
        let text = crate::error::summarize(&result);
        return (result, text);
    }
    let snapshot = result.get("snapshot").unwrap_or(&Value::Null);
    let note = if args.summary.is_none() {
        format!(
            "the snapshot is {size} characters, over {SUMMARY_OVER}, so this is the summary \
(counts and ids); pass summary=false for all of it, or workspace_id or tab_id to narrow it"
        )
    } else {
        "summary: counts and ids only; pass summary=false for the full snapshot".to_owned()
    };
    let out = json!({"type": "session_snapshot_summary", "summary": true, "note": note,
        "snapshot": summarize(snapshot)});
    (out, format!("session_snapshot_summary: {note}"))
}

#[derive(Default, serde::Deserialize, schemars::JsonSchema)]
pub struct Args {
    /// Only this workspace, e.g. `wA`.
    pub workspace_id: Option<String>,
    /// Only this tab, e.g. `wA:t1`.
    pub tab_id: Option<String>,
    /// True: counts and ids only. False: always the full snapshot. Omit:
    /// full, unless it passes 20 000 characters.
    pub summary: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap() -> Value {
        json!({"type":"session_snapshot","snapshot":{
            "protocol":22,"focused_tab_id":"a:t1",
            "workspaces":[{"workspace_id":"a"},{"workspace_id":"b"}],
            "tabs":[{"tab_id":"a:t1","workspace_id":"a"},{"tab_id":"a:t2","workspace_id":"a"},{"tab_id":"b:t1","workspace_id":"b"}],
            "panes":[{"pane_id":"a:p1","workspace_id":"a","tab_id":"a:t1"},{"pane_id":"b:p1","workspace_id":"b","tab_id":"b:t1"}],
            "layouts":[{"workspace_id":"a","tab_id":"a:t1"},{"workspace_id":"b","tab_id":"b:t1"}],
            "agents":[{"pane_id":"a:p1"},{"pane_id":"b:p1"}]}})
    }

    fn args(w: Option<&str>, t: Option<&str>, s: Option<bool>) -> Args {
        Args {
            workspace_id: w.map(Into::into),
            tab_id: t.map(Into::into),
            summary: s,
        }
    }

    #[test]
    fn filters_by_workspace_and_tab() {
        let (out, _) = shape(snap(), &args(Some("a"), None, None));
        let s = &out["snapshot"];
        assert_eq!(s["workspaces"].as_array().unwrap().len(), 1);
        assert_eq!(s["tabs"].as_array().unwrap().len(), 2);
        assert_eq!(s["agents"], json!([{"pane_id":"a:p1"}]));

        let (out, _) = shape(snap(), &args(None, Some("b:t1"), None));
        let s = &out["snapshot"];
        assert_eq!(s["workspaces"], json!([{"workspace_id":"b"}]));
        assert_eq!(s["panes"].as_array().unwrap().len(), 1);
        assert_eq!(s["layouts"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn small_replies_stay_full_and_summary_true_gives_counts_and_ids() {
        let (out, _) = shape(snap(), &args(None, None, None));
        assert_eq!(out["type"], "session_snapshot");
        let (out, _) = shape(snap(), &args(None, None, Some(true)));
        assert_eq!(out["type"], "session_snapshot_summary");
        assert_eq!(out["snapshot"]["counts"]["tabs"], 3);
        assert_eq!(out["snapshot"]["tabs"], json!(["a:t1", "a:t2", "b:t1"]));
    }

    #[test]
    fn a_large_reply_is_summarised_by_default_and_says_so() {
        let mut big = snap();
        let tabs: Vec<Value> = (0..600)
            .map(|i| json!({"tab_id":format!("a:t{i}"),"workspace_id":"a","label":"x".repeat(30)}))
            .collect();
        big["snapshot"]["tabs"] = json!(tabs);
        let (out, text) = shape(big.clone(), &args(None, None, None));
        assert_eq!(out["type"], "session_snapshot_summary");
        assert!(out["note"].as_str().unwrap().contains("20000"));
        assert!(text.contains("summary"));
        assert_eq!(out["snapshot"]["counts"]["tabs"], 600);
        let (full, _) = shape(big, &args(None, None, Some(false)));
        assert_eq!(full["type"], "session_snapshot");
    }
}
