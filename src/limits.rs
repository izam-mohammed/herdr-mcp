//! Caps on what a read returns: every character is context the model pays for
//! on every later turn, and herdr states no limit of its own.
use serde_json::Value;

pub const DEFAULT_LINES: u32 = 200;
pub const MAX_LINES: u32 = 2000;
pub const MAX_CHARS: usize = 40_000;

/// Waits end at the default unless the caller asks for less, and never pass the
/// maximum (milliseconds). Shared by pane and agent waits.
pub const WAIT_DEFAULT_MS: u64 = 60_000;
pub const WAIT_MAX_MS: u64 = 600_000;

/// The `lines` to ask herdr for: the default when unset, never over the maximum.
#[must_use]
pub fn lines(requested: Option<u32>) -> u32 {
    lines_within(requested, DEFAULT_LINES, MAX_LINES)
}

/// `lines` with another default and maximum (agent reads: 100 and 1000).
#[must_use]
pub fn lines_within(requested: Option<u32>, default: u32, max: u32) -> u32 {
    requested.unwrap_or(default).clamp(1, max)
}

/// The wait to ask herdr for, in milliseconds.
#[must_use]
pub fn wait_ms(requested: Option<u64>) -> u64 {
    requested.unwrap_or(WAIT_DEFAULT_MS).clamp(1, WAIT_MAX_MS)
}

/// The last `MAX_CHARS` characters of `text`, or `None` when it already fits.
/// The tail is kept because that is where a command's result is.
#[must_use]
pub fn tail(text: &str) -> Option<&str> {
    let extra = text.chars().count().checked_sub(MAX_CHARS)?;
    if extra == 0 {
        return None;
    }
    let start = text.char_indices().nth(extra).map_or(0, |(i, _)| i);
    Some(&text[start..])
}

/// Cap `result.read.text` (the shape of `pane.read` and `pane.wait_for_output`
/// results) and set a top-level `truncated: true` when the cap applied.
pub fn cap_read(result: &mut Value) {
    let Some(text) = result.pointer_mut("/read/text") else {
        return;
    };
    let Some(kept) = text.as_str().and_then(tail).map(str::to_owned) else {
        return;
    };
    *text = Value::String(kept);
    result["truncated"] = Value::Bool(true);
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn lines_default_and_clamp() {
        assert_eq!(lines(None), 200);
        assert_eq!(lines(Some(50)), 50);
        assert_eq!(lines(Some(99_999)), 2000);
        assert_eq!(lines(Some(0)), 1);
    }

    #[test]
    fn tail_keeps_the_end_on_a_char_boundary() {
        let s = "é".repeat(MAX_CHARS + 5);
        assert_eq!(tail(&s).map(|t| t.chars().count()), Some(MAX_CHARS));
        assert_eq!(tail("short"), None);
        assert_eq!(tail(&"x".repeat(MAX_CHARS)), None);
    }

    #[test]
    fn cap_read_marks_truncation() {
        let mut r = json!({"read":{"text": format!("{}END", "a".repeat(MAX_CHARS))}});
        cap_read(&mut r);
        assert_eq!(r["truncated"], true);
        assert!(r["read"]["text"].as_str().unwrap().ends_with("END"));
        let mut small = json!({"read":{"text":"hi"}});
        cap_read(&mut small);
        assert!(small.get("truncated").is_none());
    }
}
