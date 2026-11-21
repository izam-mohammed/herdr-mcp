//! The budget: start p50, RSS after 1000 calls, call p99, and the size of
//! `tools/list`. Run by `scripts/bench.sh` against an isolated herdr session
//! (`HERDR_SOCKET_PATH`); exits 1 past the budget. Needs a release build.
use serde_json::{Value, json};
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

const CALLS: usize = 1000;
const STARTS: usize = 30;

struct Server {
    child: Child,
    stdin: ChildStdin,
    out: BufReader<ChildStdout>,
}

fn env_f64(key: &str, default: f64) -> f64 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn spawn() -> Server {
    let mut child = Command::new("target/release/herdr-mcp")
        .env("HERDR_ENV", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .expect("target/release/herdr-mcp (cargo build --release)");
    let stdin = child.stdin.take().expect("stdin");
    let out = BufReader::new(child.stdout.take().expect("stdout"));
    Server { child, stdin, out }
}

impl Server {
    async fn rpc(&mut self, v: Value) -> String {
        self.stdin
            .write_all(format!("{v}\n").as_bytes())
            .await
            .expect("write");
        let mut line = String::new();
        self.out.read_line(&mut line).await.expect("read");
        line
    }

    async fn init(&mut self) {
        let params = json!({"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"bench","version":"0"}});
        self.rpc(json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":params}))
            .await;
        let note = json!({"jsonrpc":"2.0","method":"notifications/initialized"});
        self.stdin
            .write_all(format!("{note}\n").as_bytes())
            .await
            .expect("write");
    }
}

/// Nearest-rank percentile; `percent` is 50 or 99.
fn pct(sorted: &[f64], percent: usize) -> f64 {
    let rank = (sorted.len() * percent)
        .div_ceil(100)
        .clamp(1, sorted.len());
    sorted[rank - 1]
}

fn rss_mb(pid: u32) -> f64 {
    let status = std::fs::read_to_string(format!("/proc/{pid}/status")).expect("proc status");
    let kb: f64 = status
        .lines()
        .find_map(|l| l.strip_prefix("VmRSS:"))
        .and_then(|l| l.split_whitespace().next())
        .and_then(|n| n.parse().ok())
        .expect("VmRSS");
    kb / 1024.0
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let (start_budget, rss_budget, p99_budget) = (
        env_f64("BENCH_START_P50_MS", 10.0),
        env_f64("BENCH_RSS_MB", 8.0),
        env_f64("BENCH_CALL_P99_MS", 2.0),
    );

    let mut starts = Vec::new();
    for _ in 0..STARTS {
        let t = Instant::now();
        let mut s = spawn();
        s.init().await;
        starts.push(t.elapsed().as_secs_f64() * 1e3);
        drop(s);
    }
    starts.sort_by(f64::total_cmp);

    let mut s = spawn();
    s.init().await;
    let list = s
        .rpc(json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}))
        .await;
    let list_bytes = list.trim_end().len();
    for i in 0..100 {
        s.rpc(json!({"jsonrpc":"2.0","id":i + 5000,"method":"tools/call",
            "params":{"name":"herdr_tab_list","arguments":{}}}))
            .await;
    }
    let mut calls = Vec::with_capacity(CALLS);
    for i in 0..CALLS {
        let t = Instant::now();
        let r = s
            .rpc(json!({"jsonrpc":"2.0","id":i + 10,"method":"tools/call",
                "params":{"name":"herdr_tab_list","arguments":{}}}))
            .await;
        calls.push(t.elapsed().as_secs_f64() * 1e3);
        assert!(r.contains("\"isError\":false"), "call failed: {r}");
    }
    calls.sort_by(f64::total_cmp);
    let rss = rss_mb(s.child.id().expect("pid"));
    tokio::time::sleep(Duration::from_millis(10)).await;

    let (start_p50, call_p50, call_p99) = (pct(&starts, 50), pct(&calls, 50), pct(&calls, 99));
    println!("start p50      {start_p50:.2} ms   (budget {start_budget} ms)");
    println!("rss after {CALLS} {rss:.2} MB   (budget {rss_budget} MB)");
    println!("call p50       {call_p50:.2} ms");
    println!("call p99       {call_p99:.2} ms   (budget {p99_budget} ms)");
    println!(
        "tools/list     {list_bytes} bytes (about {} tokens)",
        list_bytes / 4
    );

    if start_p50 > start_budget || rss > rss_budget || call_p99 > p99_budget {
        println!("FAIL: past the budget");
        std::process::exit(1);
    }
    println!("PASS: inside the budget");
}
