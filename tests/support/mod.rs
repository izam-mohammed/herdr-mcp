//! Test harness: a fake herdr on a Unix socket, and the real binary driven
//! over stdio. Fixtures are `tests/fixtures/herdr.jsonl`, one line per method.
#![allow(dead_code)]
use serde_json::{Value, json};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

pub mod fake_herdr;
#[allow(unused_imports)]
pub use fake_herdr::FakeHerdr;

pub struct Mcp {
    child: Child,
    stdin: Option<ChildStdin>,
    out: BufReader<ChildStdout>,
    next: u64,
}

pub struct Opts<'a> {
    pub args: &'a [&'a str],
    pub env: &'a [(&'a str, &'a str)],
}

impl Mcp {
    /// Start the binary against `socket` and complete the MCP handshake.
    pub async fn start(socket: &std::path::Path, opts: &Opts<'_>) -> Mcp {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_herdr-mcp"));
        cmd.env_clear()
            .env("HOME", "/nonexistent")
            .env("HERDR_SOCKET_PATH", socket)
            // Plan 01 tests list tools and requests; groups added later opt in via `opts.env`.
            .env("HERDR_MCP_TOOLS", "session,workspaces,tabs,agents")
            .args(opts.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        for (k, v) in opts.env {
            cmd.env(k, v);
        }
        let mut child = cmd.spawn().expect("spawn herdr-mcp");
        let stdin = child.stdin.take();
        let out = BufReader::new(child.stdout.take().expect("stdout"));
        let mut mcp = Mcp {
            child,
            stdin,
            out,
            next: 1,
        };
        let init = mcp
            .request(
                "initialize",
                json!({"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"t","version":"0"}}),
            )
            .await;
        assert!(init["result"]["serverInfo"].is_object(), "{init}");
        mcp.notify("notifications/initialized").await;
        mcp
    }

    async fn send(&mut self, v: &Value) {
        let stdin = self.stdin.as_mut().expect("stdin open");
        stdin
            .write_all(format!("{v}\n").as_bytes())
            .await
            .expect("write");
    }

    async fn notify(&mut self, method: &str) {
        self.send(&json!({"jsonrpc":"2.0","method":method})).await;
    }

    pub async fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next;
        self.next += 1;
        self.send(&json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}))
            .await;
        loop {
            let mut line = String::new();
            let n = tokio::time::timeout(
                std::time::Duration::from_secs(20),
                self.out.read_line(&mut line),
            )
            .await
            .expect("mcp reply in time")
            .expect("read");
            assert!(n > 0, "server closed stdout");
            let v: Value = serde_json::from_str(&line).expect("json line");
            if v["id"] == json!(id) {
                return v;
            }
        }
    }

    pub async fn tool_names(&mut self) -> Vec<String> {
        let r = self.request("tools/list", json!({})).await;
        let mut names: Vec<String> = r["result"]["tools"]
            .as_array()
            .expect("tools")
            .iter()
            .map(|t| t["name"].as_str().expect("name").to_owned())
            .collect();
        names.sort();
        names
    }

    pub async fn call(&mut self, name: &str, args: Value) -> Value {
        self.request("tools/call", json!({"name": name, "arguments": args}))
            .await
    }

    /// Close stdin, wait for exit and return what the server wrote to stderr.
    pub async fn finish(mut self) -> String {
        drop(self.stdin.take());
        let mut err = String::new();
        if let Some(mut e) = self.child.stderr.take() {
            let _ = tokio::time::timeout(
                std::time::Duration::from_secs(5),
                e.read_to_string(&mut err),
            )
            .await;
        }
        err
    }
}

pub fn socket_path(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("herdr-mcp-test-{}-{tag}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("tmp dir");
    dir.join("herdr.sock")
}

pub type Seen = Arc<Mutex<Vec<Value>>>;

pub(crate) fn bind(path: &std::path::Path) -> UnixListener {
    let _ = std::fs::remove_file(path);
    UnixListener::bind(path).expect("bind fake herdr")
}
