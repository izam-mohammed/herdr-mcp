//! The one place a `herdr` subprocess runs (plan 13, decision 1): for the commands that
//! are not socket methods. Argv items only, no shell; a value that looks like a flag is
//! refused; a timeout; and a capped reply, which is text written by another program and
//! is passed through, never parsed.
use crate::error::{self, Failure};
use crate::limits;
use serde_json::{Value, json};
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::process::Command;

/// What each of stdout and stderr keeps, in bytes; the rest is read and dropped.
const MAX_BYTES: u64 = 64 * 1024;
/// `herdr update` downloads and installs a binary, so it gets longer than the default.
pub const UPDATE_TIMEOUT: Duration = Duration::from_secs(120);

/// Refuse a value that a CLI could read as a flag or that cannot be one argument.
///
/// # Errors
/// `invalid_params` for an empty value, one starting with `-`, or one with a control character.
pub fn check_value(name: &str, value: &str) -> Result<(), Failure> {
    let bad = |why: &str| Err(Failure::new("invalid_params", format!("{name} {why}")));
    if value.is_empty() {
        bad("must not be empty")
    } else if value.starts_with('-') {
        bad("must not start with '-'")
    } else if value.chars().any(char::is_control) {
        bad("must not contain control characters")
    } else {
        Ok(())
    }
}

/// `confirm: true` is required for the commands that replace the binary or reset a file.
///
/// # Errors
/// `invalid_params` naming what would happen.
pub fn need_confirm(confirm: Option<bool>, what: &str) -> Result<(), Failure> {
    if confirm == Some(true) {
        Ok(())
    } else {
        Err(Failure::new(
            "invalid_params",
            format!("{what}; pass confirm: true only if the user asked for exactly that"),
        ))
    }
}

/// Read up to `MAX_BYTES` and drop the rest, so a chatty command never blocks on a full pipe.
async fn capture<R: AsyncRead + Unpin>(mut reader: R) -> (String, bool) {
    let mut kept = Vec::new();
    let _ = (&mut reader).take(MAX_BYTES).read_to_end(&mut kept).await;
    let mut more = [0_u8; 4096];
    let mut cut = false;
    while let Ok(n) = reader.read(&mut more).await {
        if n == 0 {
            break;
        }
        cut = true;
    }
    let text = String::from_utf8_lossy(&kept).into_owned();
    let tailed = limits::tail(&text).map(str::to_owned);
    (tailed.clone().unwrap_or(text), cut || tailed.is_some())
}

/// Run `herdr <args>` and return `{stdout, stderr, exit, truncated}`. A command that
/// outlives `timeout` is killed and returns `timed_out: true` with `exit: null`.
///
/// # Errors
/// `herdr_unreachable` when the `herdr` binary cannot be started.
pub async fn run(args: &[String], timeout: Duration) -> Result<Value, Failure> {
    // Argv only; no payloads in the log.
    eprintln!(
        "herdr-mcp: run herdr {}",
        args.first().map_or("", String::as_str)
    );
    let mut child = Command::new("herdr")
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| {
            Failure::new(
                error::HERDR_UNREACHABLE,
                format!("cannot run the herdr binary: {e}"),
            )
        })?;
    let (out, err) = (
        child.stdout.take().expect("piped stdout"),
        child.stderr.take().expect("piped stderr"),
    );
    let finished = tokio::time::timeout(timeout, async {
        let ((stdout, cut_out), (stderr, cut_err), status) =
            tokio::join!(capture(out), capture(err), child.wait());
        (stdout, stderr, cut_out || cut_err, status)
    })
    .await;
    match finished {
        Ok((stdout, stderr, truncated, status)) => {
            let status = status.map_err(|e| {
                Failure::new(
                    error::HERDR_UNREACHABLE,
                    format!("herdr did not finish: {e}"),
                )
            })?;
            Ok(json!({
                "stdout": stdout, "stderr": stderr, "exit": status.code(), "truncated": truncated,
            }))
        }
        // Dropping the future drops the child, which `kill_on_drop` kills.
        Err(_) => Ok(json!({
            "stdout": "", "stderr": "", "exit": null, "truncated": false,
            "timed_out": true, "timeout_ms": u64::try_from(timeout.as_millis()).unwrap_or(u64::MAX),
        })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_that_look_like_flags_are_refused() {
        for bad in ["", "-x", "--help", "--json", "a\nb", "a\0b"] {
            assert_eq!(check_value("name", bad).unwrap_err().code, "invalid_params");
        }
        for fine in ["work", "a b", "x; rm -rf /", "it's", "$(id)"] {
            assert!(check_value("name", fine).is_ok(), "{fine}");
        }
    }

    #[test]
    fn confirm_must_be_true() {
        assert!(need_confirm(None, "x").is_err());
        assert!(need_confirm(Some(false), "x").is_err());
        assert!(need_confirm(Some(true), "x").is_ok());
    }
}
