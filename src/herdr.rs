//! One connection per call, as herdr requires (docs/external/herdr-socket-api.md).
use crate::error::{self, Failure};
use serde_json::{Value, json};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

pub struct HerdrClient {
    socket: PathBuf,
    timeout: Duration,
    debug: bool,
    next: AtomicU64,
}

impl HerdrClient {
    #[must_use]
    pub fn new(socket: PathBuf, timeout: Duration, debug: bool) -> Self {
        Self {
            socket,
            timeout,
            debug,
            next: AtomicU64::new(1),
        }
    }

    #[must_use]
    pub fn socket(&self) -> &std::path::Path {
        &self.socket
    }

    /// Send `method` and return herdr's `result`, or the contract's failure.
    pub async fn call(&self, method: &str, params: Value) -> Result<Value, Failure> {
        let id = format!("m{}", self.next.fetch_add(1, Ordering::Relaxed));
        // Method and id only; payloads are pane text and cwds, kept out of logs.
        eprintln!("herdr-mcp: call {id} {method}");
        let request = json!({"id": id, "method": method, "params": params});
        if self.debug {
            eprintln!("herdr-mcp: request {request}");
        }
        let reply = tokio::time::timeout(self.timeout, self.exchange(&request))
            .await
            .map_err(|_| {
                Failure::new(
                    error::TIMEOUT,
                    format!(
                        "herdr did not answer {method} within {} ms",
                        self.timeout.as_millis()
                    ),
                )
            })??;
        if self.debug {
            eprintln!("herdr-mcp: reply {reply}");
        }
        if let Some(e) = reply.get("error") {
            let text = |k| e.get(k).and_then(Value::as_str).unwrap_or_default();
            return Err(Failure::new(text("code"), text("message")));
        }
        reply
            .get("result")
            .cloned()
            .ok_or_else(|| Failure::new("invalid_reply", "herdr reply has no result or error"))
    }

    async fn exchange(&self, request: &Value) -> Result<Value, Failure> {
        let unreachable = |e: std::io::Error| {
            Failure::new(
                error::HERDR_UNREACHABLE,
                format!("cannot talk to herdr at {}: {e}", self.socket.display()),
            )
        };
        let mut stream = UnixStream::connect(&self.socket)
            .await
            .map_err(unreachable)?;
        let mut line = request.to_string();
        line.push('\n');
        stream
            .write_all(line.as_bytes())
            .await
            .map_err(unreachable)?;
        let mut reply = String::new();
        let n = BufReader::new(stream)
            .read_line(&mut reply)
            .await
            .map_err(unreachable)?;
        if n == 0 {
            return Err(Failure::new(
                error::HERDR_UNREACHABLE,
                "herdr closed the connection",
            ));
        }
        serde_json::from_str(&reply)
            .map_err(|e| Failure::new("invalid_reply", format!("herdr reply is not JSON: {e}")))
    }
}
