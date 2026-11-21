# Research for plans/01-first-version.md

Dated evidence behind the language decision and the herdr connection model.
Measured 22/10/2026 on this machine: AMD Ryzen 7 7435HS (16 threads), 22 GB
RAM, Linux 7.0, herdr 0.9.5 (live default session, 3 workspaces).

## The four minimal servers

Each one is a stdio MCP server built with its official SDK, with one tool,
`ping`, that opens the herdr socket, sends `ping` and returns the line. The
sources are in `docs/research/01-first-version-bench/`:

| Language | SDK | Runtime | Build | Install size |
| --- | --- | --- | --- | --- |
| Rust | `rmcp` 3.6.0 | rustc 1.99.1, tokio `current_thread` | `cargo build --release`, LTO, `codegen-units = 1`, stripped, `panic = "abort"` | 1.75 MB binary, no runtime |
| Go | `github.com/modelcontextprotocol/go-sdk` v1.8.0 | go 1.26.8 | `go build -trimpath -ldflags="-s -w"` | 7.9 MB binary, no runtime |
| TypeScript/JS | `@modelcontextprotocol/sdk` 1.32.1 | node 22.23.4, plain ESM | none | 27 MB `node_modules` plus node |
| Python | `mcp` 2.3.0 (`MCPServer`) | CPython 3.13.16 | `uv venv` | 37 MB venv plus Python |

## The driver

`drive.py` (beside the sources) spawns each server the way Claude Code
does, over pipes:

- **start**: time from spawn to the `initialize` response, median of 15
  cold starts;
- **idle RSS**: `VmRSS` after the first tool call;
- **after 1000 calls**: `VmRSS` and `VmHWM` (peak) after 1000 sequential
  `tools/call ping`, each a real herdr round trip;
- **call**: p50 and p99 of those 1000 calls, measured at the client;
- **CPU**: user + system CPU time the server process used over its life.

Run: `HERDR_SOCKET_PATH=... python3 -I drive.py`.

## Results

Run 1:

| Server | Start p50 | Idle RSS | RSS after 1000 | Call p50 | Call p99 | CPU total |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Rust | 1.5 ms | 3.5 MB | 3.6 MB | 0.556 ms | 0.692 ms | 110 ms |
| Go | 4.3 ms | 10.3 MB | 15.4 MB | 0.692 ms | 1.316 ms | 330 ms |
| Node | 127.4 ms | 72.2 MB | 78.6 MB | 0.502 ms | 1.216 ms | 450 ms |
| Python | 397.9 ms | 67.2 MB | 67.3 MB | 1.103 ms | 103.022 ms | 1110 ms |

Run 2 (same day, same build):

| Server | Start p50 | Idle RSS | RSS after 1000 | Call p50 | Call p99 | CPU total |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Rust | 1.4 ms | 3.5 MB | 3.6 MB | 0.553 ms | 0.847 ms | 110 ms |
| Go | 4.3 ms | 10.3 MB | 15.4 MB | 0.684 ms | 1.125 ms | 330 ms |
| Node | 127.9 ms | 71.3 MB | 78.6 MB | 0.493 ms | 101.175 ms | 440 ms |
| Python | 399.1 ms | 67.3 MB | 67.4 MB | 1.119 ms | 103.346 ms | 1120 ms |

Reading it:

- **Per call, the herdr round trip dominates** (0.14 to 0.41 ms for herdr
  itself, see below); the SDKs add 0.35 to 0.7 ms on top. Every language is
  fast enough per call at p50. The differences are memory, start, CPU and
  the tail.
- **Memory is the one that multiplies.** Every Claude Code session starts
  its own copy of a stdio server, and herdr exists to run many agents at
  once. Ten agents cost 36 MB of herdr-mcp in Rust, 154 MB in Go, 786 MB in
  Node, 674 MB in Python.
- **CPU per call**: Rust 0.11 ms, Go 0.33 ms, Node 0.44 ms, Python 1.11 ms.
- **The tail**: Node (run 2) and Python both showed a p99 near 100 ms; Rust
  and Go stayed under 1.4 ms in both runs.
- **Start** matters for every new Claude Code session and every restart:
  Rust 1.4 ms against Node 128 ms and Python 399 ms.

## Published benchmarks, for comparison

- TM Dev Lab's MCP server benchmarks (2025-2026, HTTP transport, 50
  virtual users): Java 0.835 ms and Go 0.855 ms average latency, Node 10.66
  ms, Python 26.45 ms in round 1; in the 15-implementation follow-up, Rust
  (rmcp 0.17.0) 4 845 RPS at 11 MB RAM once rmcp's Streamable HTTP
  `json_response` fix was applied. Reposted on the Java SDK blog,
  15/02/2026:
  https://java.sdk.modelcontextprotocol.io/latest/blog/2026/02/15/java-leads-mcp-server-performance-benchmarks-with-sub-millisecond-latency/
- Independent rerun noting rmcp's 40 ms floor before that fix (HTTP only):
  https://armantik.medium.com/where-does-rust-land-in-an-mcp-benchmark-i-tried-to-find-out-4695909717b8

These measure HTTP servers under concurrent load, which is not this
project's shape (one stdio client per process, sequential calls). They
agree with the local numbers on ordering for memory (Rust and Go far below
Java, Node and Python).

## The herdr side

Measured with a Python client opening a new connection per call, 100 calls
each: `ping` p50 0.14 ms (p90 0.33), `tab.list` 0.18 ms, `agent.list` 0.26
ms, `session.snapshot` 0.41 ms. A second request on the same connection is
never answered, so connection-per-call is the protocol, not a choice
(`docs/external/herdr-socket-api.md` § Wire format).

## Reproducing

The servers need their toolchains (rustup, go, node, uv). The scratch
builds used for the numbers above are not kept in the repository; the
sources and driver are.

## Post-implementation review (20/11/2026)

Scope: phases 1 to 3 of `plans/01-first-version.md`: `src/` (config,
herdr client, safety gate, snapshot shaping, the four tool groups),
`tests/`, `scripts/`, the CI workflow, and the dependency tree. Each row of
the plan's `## Security` table was read against the code and run.

| Threat | Result | Proof |
| --- | --- | --- |
| Elevation: started outside herdr | Held. Without `HERDR_ENV=1` the router keeps only tools whose `readOnlyHint` is true, so write tools are never listed; a call to one is a protocol error and herdr sees nothing | `outside_herdr_registers_no_write_tool`, `allow_outside_herdr_registers_writes` |
| Elevation: `--read-only` | Held, same gate, also with `HERDR_ENV=1` | `read_only_flag_lists_reads_only` |
| Tampering: malformed arguments | Held. rmcp deserialises into typed structs and the request to herdr is built from the typed struct, so unknown extra fields never reach herdr | `a_wrong_typed_argument_is_refused_before_herdr_sees_it` |
| Information disclosure: logs | Held. stderr has method and id only; payloads need `--debug` | `stderr_has_no_reply_body_without_debug` |
| Denial of service: hung herdr | Held. One timeout covers connect, write and read, default 10 s | `a_hung_socket_gives_timeout`, `an_absent_socket_gives_herdr_unreachable` |
| Prompt injection | Held as far as text can. Server instructions and every tool that returns labels, titles or agent names say the output is untrusted data | `instructions_say_output_is_untrusted` |

Findings, none needing a fix in this plan:

- The reply line from herdr is read without a size cap. herdr is the
  user's own process behind a 0600 socket, and the large-result tools
  (`pane.read`) arrive in plan 03, which sets the `lines` cap there. Left as
  is; plan 03 must carry the cap.
- `HERDR_MCP_TIMEOUT_MS` is an undocumented-in-contract env var (used by the
  timeout test); a value of 0 makes every call time out. Harmless, but it is
  not part of `## Contract`.
- The advisory database flags `chrono` as unmaintained (RUSTSEC-2026-0543),
  pulled in by rmcp and schemars. It is ignored in `deny.toml` with the
  reason; recheck when rmcp or schemars drops it.
- No server-side `self_target` refusal exists yet because no tool targets a
  pane; plans 02 to 04 add it with their tools.

Re-run after the review: `HERDR_MCP_LIVE=1 sh scripts/check.sh` passes, and
`sh scripts/bench.sh` prints PASS (numbers in the PR).
