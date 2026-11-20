//! The contract's error shape and result builders.
use rmcp::model::{CallToolResult, ContentBlock};
use serde_json::{Value, json};

/// Codes the server itself raises; herdr's own codes pass through unchanged.
pub const READ_ONLY: &str = "read_only";
pub const HERDR_UNREACHABLE: &str = "herdr_unreachable";
pub const TIMEOUT: &str = "timeout";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    pub code: String,
    pub message: String,
}

impl Failure {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }

    #[must_use]
    pub fn into_result(self) -> CallToolResult {
        let mut out =
            CallToolResult::structured_error(json!({"code": self.code, "message": self.message}));
        out.content = vec![ContentBlock::text(format!(
            "herdr {}: {}",
            self.code, self.message
        ))];
        out
    }
}

/// A success: herdr's `result` as `structuredContent`, with a one-line summary.
#[must_use]
pub fn success(result: Value, summary: &str) -> CallToolResult {
    let mut out = CallToolResult::structured(result);
    out.content = vec![ContentBlock::text(summary)];
    out
}

/// `tab_list (3 tabs)` from a result like `{"type":"tab_list","tabs":[..]}`.
#[must_use]
pub fn summarize(result: &Value) -> String {
    let kind = result.get("type").and_then(Value::as_str).unwrap_or("ok");
    let counted = result
        .as_object()
        .into_iter()
        .flatten()
        .find_map(|(k, v)| v.as_array().map(|a| (k, a.len())));
    match counted {
        Some((key, n)) => format!("{kind} ({n} {key})"),
        None => kind.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_counts_the_first_array() {
        assert_eq!(
            summarize(&json!({"type":"tab_list","tabs":[1,2]})),
            "tab_list (2 tabs)"
        );
        assert_eq!(summarize(&json!({"type":"pong","version":"1"})), "pong");
        assert_eq!(summarize(&json!({})), "ok");
    }

    #[test]
    fn failure_has_the_contract_shape() {
        let r = Failure::new("not_found", "no tab").into_result();
        assert_eq!(r.is_error, Some(true));
        assert_eq!(
            r.structured_content,
            Some(json!({"code":"not_found","message":"no tab"}))
        );
    }
}
