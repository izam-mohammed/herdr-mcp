# Claude Code lists, focuses, creates and renames herdr tabs through typed MCP tools

## The ask

"Is it possible to have an MCP for herdr, which is basically manage the
active tabs and other events", then: plan it in separate plans, with "a
proper planning on the language", "efficient and performance oriented".
`features.md` is the spec. This plan is the first version someone can try:
a Rust stdio server that Claude Code starts, which can see the whole
session and manage tabs, with the safety rules in from the start. Later
plans add the other areas (`PLAN.md`).

Two things the research found that `features.md` had wrong, now fixed
there: herdr answers **one request per connection**, and the pane-scoped
events (`pane.agent_status_changed`, `pane.output_matched`,
`pane.scroll_changed`) need a `pane_id` per subscription entry
(`docs/external/herdr-socket-api.md`).

## Alternatives

### Decision 1: which language and runtime is the server written in? (one-way door)

Everything else is built on it, and Claude Code starts one copy per session:
with herdr running many agents, memory and start time multiply. The
features.md candidates were TypeScript and Python; Go and Rust have official
SDKs too.

Searched 2026-10-22: "MCP server benchmark Rust Go Java Python Node.js
latency memory 2025 2026", "rmcp official Rust MCP SDK", "modelcontextprotocol
go-sdk v1"; TM Dev Lab's benchmark rounds (via the Java SDK blog,
15/02/2026), an independent rmcp rerun, the go-sdk and rust-sdk READMEs,
crates.io, proxy.golang.org, npm and PyPI for current versions; then built
and measured all four on this machine (`docs/research/01-first-version.md`).

| Must | Rust + `rmcp` | Go + go-sdk | TypeScript + `@modelcontextprotocol/sdk` | Python + `mcp` |
| --- | --- | --- | --- | --- |
| Official MCP SDK with stdio, tools, annotations, resources, prompts | yes (3.6.0) | yes (v1.8.0) | yes (1.32.1) | yes (2.3.0) |
| Unix socket client in the standard runtime | yes (tokio) | yes | yes | yes |
| No unpatched high or critical advisory | yes (all fixed by 2.1.0) | yes (none found) | yes (`npm audit`: 0) | yes (none found) |
| **Survives** | **yes** | **yes** | **yes** | **yes** |

| Criterion | Weight | Rust + `rmcp` | Go + go-sdk | TypeScript | Python |
| --- | ---: | ---: | ---: | ---: | ---: |
| Performance efficiency | 40 | 5 | 4 | 2 | 1 |
| Fit to the ask | 15 | 4 | 5 | 5 | 4 |
| Reliability | 15 | 5 | 4 | 3 | 3 |
| Running cost and operability | 15 | 5 | 5 | 2 | 2 |
| Security | 10 | 5 | 4 | 3 | 3 |
| Compatibility | 5 | 5 | 3 | 3 | 3 |
| **Weighted total** | **100** | **4.85** | **4.25** | **2.75** | **2.20** |

- Efficiency, measured 22/10/2026 (two runs, 1000 real herdr calls each):
  Rust starts in 1.4 ms and holds 3.6 MB, using 0.11 ms of CPU per call;
  Go 4.3 ms, 15.4 MB, 0.33 ms; Node 128 ms, 78.6 MB, 0.44 ms; Python 399
  ms, 67.4 MB, 1.11 ms. Ten agents cost 36 MB in Rust, 154 MB in Go, 786
  MB in Node. Call p50 is 0.5 to 1.1 ms in all four; p99 near 100 ms for
  Node (run 2) and Python, under 1.4 ms for Rust and Go.
- Fit: go-sdk v1.7.0+ states support for spec 2026-07-28 in its README
  table; rmcp 3.x has every feature this project uses
  (`docs/external/rmcp.md`) but the spec version was not confirmed: 4.
- Reliability: Rust and Go have the flattest tail; Rust has no GC pauses
  and no null or data-race class of crash in a long-lived event task.
- Operability: Rust and Go ship one static binary (1.75 MB, 7.9 MB); Node
  and Python need a runtime plus 27 MB and 37 MB of packages per install.
- Security: Rust's dependency tree for stdio has no network code enabled;
  Go's SDK pulls `golang.org/x/oauth2` even for stdio; npm and PyPI trees
  are larger.
- Compatibility: herdr itself is Rust (Apache-2.0), so its serde shapes
  map one to one.

Sensitivity: moving 10 points from efficiency to fit gives Rust 4.75, Go
4.35; Rust holds. Go is the runner-up on every shift tried; TypeScript and
Python lose on efficiency alone (it would take efficiency under 10 points
for TypeScript to pass Go).
Risk: rmcp's API has moved fast (0.x to 3.x in about a year); it sits
behind `src/server.rs` and the tool macros, and the version is pinned in
`Cargo.lock`.

**Chosen: Rust with `rmcp`.** Lowest memory, start and CPU by a wide
margin, which is what multiplies across many agents, and a single small
binary. This departs from features.md's two candidates; the measurements
are why. Revisit if rmcp falls behind the MCP spec by more than one
revision.

### Decision 2: which MCP layer, within Rust? (two-way door)

Searched 2026-10-22: "rmcp vs rust-mcp-sdk", "MCP stdio JSON-RPC
implement by hand"; the rust-sdk README, docs.rs for `rmcp`,
`rust-mcp-sdk`, `mcpkit`.

| Criterion | `rmcp` (official) | `rust-mcp-sdk` | Hand-rolled JSON-RPC over stdio |
| --- | --- | --- | --- |
| Performance efficiency | measured 1.4 ms start, 3.6 MB | not measured | at best equal; the protocol work is the same |
| Fit to the ask | annotations, `_meta`, resources, prompts, notifications, version negotiation | similar, third party | all of it written and kept current by this project |
| Reliability | tracks the spec under the MCP org | one maintainer group | every spec revision is ours to follow |
| Security | advisories fixed and published (RustSec) | not tracked | none known, none reviewed |

**Chosen: `rmcp`.** Already at the efficiency floor in the measurement,
and the official one tracks the spec. Revisit if it blocks a feature the
spec allows.

### Decision 3: how are herdr's replies passed to the client? (two-way door)

Searched 2026-10-22: "typify json schema to rust", "serde_json RawValue
pass through"; the typify README, serde_json docs.

| Criterion | Typed params, results passed through as JSON | Hand-written structs for every result | Types generated from `herdr api schema` |
| --- | --- | --- | --- |
| Performance efficiency | one parse of the reply, no re-shaping | parse into structs, serialise again | the same as structs |
| Fit to the ask | new herdr fields reach the client untouched ("ignore unknown fields") | new fields dropped until coded | new fields need regeneration |
| Compatibility | herdr's own field names, as the skill and docs use them | ours | herdr's |
| Maintainability of the result | one place per tool | about 60 result types | a generated 272 KB schema in the build |

**Chosen: typed params, results passed through.** Tool inputs are Rust
structs (`schemars` gives the input schemas and validation); a herdr
`result` becomes the tool's `structuredContent` unchanged, with a one-line
text summary. Revisit if a client needs `outputSchema` per tool.

### Decision 4: how many tools, and how are they named? (one-way door)

Tool names are what people write permission rules against
(`mcp__herdr__herdr_tab_close`), so renaming later breaks their settings.

Searched 2026-10-22: "MCP many tools context cost", "Claude Code MCP tool
search"; Claude Code's MCP docs on tool search, `alwaysLoad` and
`_meta` (`docs/external/claude-code-mcp.md`).

| Must | One tool per herdr method | One tool per area with an `action` argument | One generic `herdr_call(method, params)` |
| --- | --- | --- | --- |
| A permission rule can allow reads and ask on writes | yes | no (one name covers both) | no |
| Never reaches server lifecycle or plugin methods | yes | yes | no |
| **Survives** | **yes** | no | no |

| Criterion | Weight | Narrow tools, all registered | Narrow tools, `--tools` groups, all on by default |
| --- | ---: | ---: | ---: |
| Performance efficiency | 50 | 4 | 5 |
| Fit to the ask | 30 | 4 | 5 |
| Security | 20 | 4 | 5 |
| **Weighted total** | **100** | **4.00** | **5.00** |

- Efficiency: with tool search on (Claude Code's default) only names load
  upfront; a client without it loads every definition, and `--tools
  tabs,agents` cuts that to the groups used. The size of `tools/list` is
  measured in Phase 3.
- Security: a group left out is not registered at all, stronger than a
  deny rule.

**Chosen: narrow `herdr_<area>_<verb>` tools, in groups, all on by
default.** Revisit if `tools/list` passes 10 000 tokens.

### Decision 5: which async runtime shape? (two-way door)

`rmcp` needs tokio.

Searched 2026-10-22: "tokio current_thread vs multi_thread memory"; the
tokio runtime docs (the multi-thread scheduler starts one worker per core,
16 here).

| Criterion | `current_thread` | `multi_thread` |
| --- | --- | --- |
| Performance efficiency | measured 3.6 MB, 0.11 ms CPU per call | ? (16 worker threads; not measured) |
| Fit to the ask | all work is socket waits; nothing CPU-bound | the same |

**Chosen: `current_thread`.** Measured at the floor; the server only waits
on sockets. Revisit if a tool does CPU work over 1 ms.

### Decision 6: which standard checks? (two-way door)

A new project chooses them now (`docs/standards.md`).

Searched 2026-10-22: "cargo-deny vs cargo-audit 2026", "cargo-nextest vs
cargo test"; RustSec, a 2026 supply-chain tool comparison, the gitleaks
README.

| Check | Chosen | Over | Why |
| --- | --- | --- | --- |
| Format, lint | rustfmt, Clippy `-D warnings` | none needed | the toolchain's own, fast |
| Tests | `cargo test` | cargo-nextest | no extra binary at this size; `?` on speed until the suite is big |
| Audit | cargo-deny | cargo-audit | advisories plus sources, bans and licences in one |
| Secrets | gitleaks | TruffleHog | one offline binary |
| Runner | `scripts/check.sh` | Makefile, justfile | POSIX sh, nothing to install |

**Chosen: the set above.** Revisit if the tests pass 30 seconds.

## The decision

Feature context. A Rust 2024 binary, `herdr-mcp` (decision 1), built on
`rmcp` 3.6 (decision 2) with a `current_thread` tokio runtime (decision 5).
`HerdrClient::call` opens a connection per request, as herdr requires, with
a timeout; replies pass through as `structuredContent` (decision 3). Tools
are narrow and grouped (decision 4); this plan lands the `session` group
and the read and non-destructive half of `workspaces`, `tabs` and `agents`.
Usable first: Phase 2 is the rough cut you try in Claude Code; Phase 1 only
proves the two integrations and lands the check, since nothing else can be
trusted before they pass.

Dependencies: `rmcp` (features `server`, `transport-io`), `tokio`
(`rt`, `macros`, `net`, `io-util`, `io-std`, `time`), `serde`,
`serde_json`, `schemars`, `anyhow`. Tools for the check: rustup with the
pinned toolchain, cargo-deny and gitleaks. No network host at run time, no
secret, no background service; it talks only to the user's own herdr
socket.

Standards: `docs/standards.md` (format, lint, tests, audit, secret scan,
shell check and build through `sh scripts/check.sh`).

## Integrations

| Integration | Local docs | Proof script | Env vars |
| --- | --- | --- | --- |
| herdr socket API | `docs/external/herdr-socket-api.md` | `scripts/prove/herdr-socket.sh` | `HERDR_SOCKET_PATH` |
| rmcp stdio server | `docs/external/rmcp.md` | `scripts/prove/rmcp-stdio.sh` | none |
| Claude Code MCP client | `docs/external/claude-code-mcp.md` | none | none |

## Contract

Binding for every later plan:

- Binary `herdr-mcp`; registered as `claude mcp add herdr -- herdr-mcp`.
- Flags and env, flag first: `--session <name>` / `HERDR_SESSION`,
  `--socket <path>` / `HERDR_SOCKET_PATH`, `--read-only` /
  `HERDR_MCP_READ_ONLY=1`, `--allow-outside-herdr` /
  `HERDR_MCP_ALLOW_OUTSIDE=1`, `--tools <group,...>` / `HERDR_MCP_TOOLS`
  (groups: `session`, `workspaces`, `tabs`, `panes`, `agents`, `events`,
  `worktrees`, `layout`, `notify`; default all), `--debug`.
- Tool names `herdr_<area>_<verb>`, snake case, as listed in `features.md`.
- Success: `structuredContent` is herdr's `result` object unchanged; text
  content is a one-line summary.
- Failure: `isError: true`, text `herdr <code>: <message>`,
  `structuredContent` `{"code": "...", "message": "..."}`. The server's own
  refusals use codes `read_only`, `self_target`, `herdr_unreachable`,
  `timeout`.
- Annotations on every tool: `readOnlyHint`, `destructiveHint`,
  `idempotentHint`, `openWorldHint: false`.

## Where it lands

| Path | New or edit | Phase | What changes | Firm? |
| --- | --- | --- | --- | --- |
| `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml` | new | 1 | crate, pinned deps and toolchain, release profile | expected |
| `examples/prove_rmcp.rs` | new | 1 | the rmcp proof server | expected |
| `scripts/prove/herdr-socket.sh`, `scripts/prove/rmcp-stdio.sh` | new | 1 | the proofs | fixed |
| `src/main.rs`, `src/config.rs` | new | 2 | flags, env, socket resolution, mode | expected |
| `src/herdr.rs` | new | 2 | `HerdrClient::call` | expected |
| `src/server.rs`, `src/tools/*.rs` | new | 2 | handler, instructions, tool groups | expected |
| `src/safety.rs`, `src/error.rs` | new | 2 | read-only gate, error mapping | expected |
| `tests/support/fake_herdr.rs`, `tests/fixtures/`, `tests/tools.rs` | new | 2 | fake socket and tool tests | expected |
| `tests/live.rs` | new | 2 | isolated-session tests | expected |
| `scripts/check.sh`, `rustfmt.toml`, `deny.toml`, `.gitignore` | new | 1 | the standard check | fixed |
| `.github/workflows/check.yml` | new | 1 | CI | expected |
| `README.md` | edit | 2 | register, flags, safety, added to the existing banner page | reuse |
| `examples/bench.rs`, `scripts/bench.sh` | new | 3 | the budget | expected |

*Reused, not rewritten:* the repository holds only `features.md`, docs and
the README banner page (extended, not rebuilt).
Searched crates.io for a herdr client library: `herdr` 0.1.0 on docs.rs is
an old release of herdr itself, not a client. The bench servers in
`docs/research/01-first-version-bench/` are the starting point for
`examples/prove_rmcp.rs` and `examples/bench.rs`.

## Security

What is protected: the user's terminals. Tools can focus, create and
rename tabs now, and in later plans type into other panes, which can run
any command. Who can reach it: only the MCP client that spawned the
process, over stdio; the herdr socket is mode 0600, owner only.

| Threat | Here | Handled by | Proved by |
| --- | --- | --- | --- |
| Elevation of privilege | a server started outside herdr steers the focused session | read-only unless `HERDR_ENV=1` or `--allow-outside-herdr`; write tools are not registered at all | `tests/tools.rs`: without `HERDR_ENV`, `tools/list` has no write tool |
| Elevation of privilege | write tools run in `--read-only` | the same gate | `tests/tools.rs`: `--read-only` lists reads only |
| Tampering | tool arguments reach herdr malformed | `schemars` types validate, then serde builds the request; no string-built JSON | `tests/tools.rs`: a wrong-typed argument is refused before herdr sees it |
| Information disclosure | pane text, titles and cwds land in logs | stderr logs carry method and id only; payloads only with `--debug` | `tests/tools.rs`: captured stderr has no reply body without `--debug` |
| Denial of service | herdr hangs and the tool never returns | per-call timeout (10 s for non-wait calls) | `tests/tools.rs`: a fake socket that never answers gives `timeout` |
| Prompt injection | tab labels and agent titles hold text that reads like instructions | descriptions and server instructions say output is untrusted data | review item in Phase 3 |

Data: no personal data stored; nothing persisted. Secrets: none. Logged:
method names and ids. Checklist: OWASP Top 10 for LLM Applications (2025
edition, current on 22/10/2026), LLM06 Excessive Agency and LLM01 Prompt
Injection.

## Declined

- Go (decision 1): runner-up at 4.25; 4 times Rust's memory and 3 times
  its CPU per call.
- TypeScript and Python (decision 1): 20 to 22 times Rust's memory and
  90 to 280 times its start time, with a ~100 ms p99 tail.
- `rust-mcp-sdk` and hand-rolled JSON-RPC (decision 2): no efficiency gain
  over rmcp's measured floor, and the spec to track alone.
- Hand-written or generated result types (decision 3): drop new herdr
  fields and re-serialise every reply.
- Area tools with `action`, and a generic `herdr_call` (decision 4): fail
  the per-tool permission must; the generic one also reaches
  `server.stop`.
- `multi_thread` runtime (decision 5): 16 idle workers for socket waits.
- cargo-audit, cargo-nextest, TruffleHog, make, just (decision 6).

## What the tests hold

Fake-socket tests: each rough-cut tool sends the method and params the API
doc names and returns herdr's result unchanged; a herdr error becomes the
contract's error shape; read-only mode and outside-herdr mode register no
write tool; a hung socket times out. Live tests against an isolated
session: `herdr_tab_create`, `_rename`, `_focus` and `_list` agree with
`session.snapshot`. The proofs pin the two integrations; the bench pins
the budget.

## Repository knowledge

Read root `KNOWLEDGE.md` when present before each phase. Follow the global and
repository instructions for recording useful development findings.

## Phases

### Phase 1: prove herdr and rmcp, and the check (needs you: install Rust with rustup)

- [x] The research is re-checked, since it is dated 22/10/2026: `herdr
  --version` and `ping` protocol, `rustc --version`, and the newest `rmcp`
  on crates.io, each compared with `docs/standards.md` and
  `docs/external/`; any that moved is updated there with the new date
  before anything is built.
  Landed 2026-11-20: herdr 0.9.5 and protocol 22 unchanged; rustc is 1.100.0 (standards.md said 1.99, updated); newest rmcp is 3.11.0 (was 3.6.0), so `Cargo.toml` uses 3.11 and `docs/external/rmcp.md` records it.
- [x] `scripts/prove/herdr-socket.sh` sends `ping` to the resolved socket
  with `nc -U` per `docs/external/herdr-socket-api.md` § Wire format and
  prints `PASS` when the reply is `pong` at protocol 22.
  Landed 2026-11-20: `nc -U -w 2` works (OpenBSD netcat); proof run against the isolated `herdr-mcp-test` session because no default herdr server was running here.
- [x] `examples/prove_rmcp.rs` (one `ping` tool, per `docs/external/rmcp.md`)
  with `Cargo.toml` and `rust-toolchain.toml`, and
  `scripts/prove/rmcp-stdio.sh` that builds it, pipes `initialize`,
  `notifications/initialized` and `tools/call`, and prints `PASS` when the
  call returns herdr's `pong`; the MCP `protocolVersion` rmcp negotiates is
  recorded in `docs/external/rmcp.md`, settling decision 1's open spec
  question.
  Landed 2026-11-20: negotiated protocolVersion is 2025-11-25 at most (2025-06-18 if offered); recorded in `docs/external/rmcp.md`.
- [x] The standard check lands: `scripts/check.sh`, `rustfmt.toml`,
  `deny.toml`, Clippy lints in `Cargo.toml`, `.gitignore`, and
  `.github/workflows/check.yml`, as `docs/standards.md` says.
  Landed 2026-11-20: `deny.toml` ignores RUSTSEC-2026-0543 (chrono unmaintained, transitive via rmcp and schemars).
- [x] Check: both scripts exit 0 and print `PASS`, and `sh scripts/check.sh`
  passes on the proof crate.

### Phase 2: the rough cut: tabs from Claude Code (needs you: try it)

- [x] `HerdrClient::call` (expected in `src/herdr.rs`) resolves the socket
  in herdr's order, opens one connection per call, applies the timeout and
  maps errors per `## Contract` (`src/error.rs`); `src/config.rs` reads the
  flags and env, with read-only the default outside herdr
  (`src/safety.rs`).
  Landed 2026-11-20: `src/lib.rs` added so tests and the binary share the crate; socket order is `--socket`, `--session`, `HERDR_SOCKET_PATH`, `HERDR_SESSION`, default. `HERDR_MCP_TIMEOUT_MS` (default 10000) is an extra env var used by the timeout test.
- [x] Tools `herdr_whoami`, `herdr_status`, `herdr_snapshot`,
  `herdr_workspace_list`, `herdr_tab_list`, `herdr_tab_get`,
  `herdr_tab_focus`, `herdr_tab_create`, `herdr_tab_rename` and
  `herdr_agent_list` (expected in `src/tools/`), with annotations and
  server instructions saying output is untrusted; pinned by
  `tests/tools.rs` on `tests/support/fake_herdr.rs`, including every
  `## Security` row's test.
  Landed 2026-11-20: registry is `src/tools/mod.rs`: one `mod` line and one `GROUPS` line per group, each group in `src/tools/<group>.rs` exposing `router()`; the read-only gate removes every tool whose `readOnlyHint` is not true, so a new group needs no gate edit. `herdr_whoami` is the one composite result (env plus herdr's `pane.current` under `pane`). Server `self_target` refusals have no tool to guard yet (no pane tool in this plan).
- [x] `tests/live.rs` runs the binary against an isolated
  `herdr-mcp-test` session: create, rename, focus and list a tab, then
  stop and delete the session.
  Landed 2026-11-20: needs `HERDR_MCP_LIVE=1`, else it skips; a workspace is created over the raw socket first because a fresh session has none; the session is stopped and deleted in a drop guard.
- [x] `README.md` gains how to build, register with `claude mcp add herdr
  -- <path>/herdr-mcp`, the flags, and the safety defaults.
- [x] Check: `HERDR_MCP_LIVE=1 sh scripts/check.sh` passes.
- [ ] You try it: run `cargo build --release`, then `claude mcp add herdr
  -- $PWD/target/release/herdr-mcp`, start Claude Code in a herdr pane, ask
  it to list your tabs, open a new tab called `scratch` and focus it; say
  whether the answers and the permission prompts feel right. What you want
  changed goes into Phase 3 or the later plans.

### Phase 3: within budget, and big sessions stay small

- [ ] `examples/bench.rs` and `scripts/bench.sh` measure start, RSS after
  1000 calls and call p99 against the live socket, failing past the budget
  (start p50 under 10 ms, RSS under 8 MB, call p99 under 2 ms), and record
  the byte size of `tools/list`; run by `scripts/check.sh` when
  `HERDR_MCP_LIVE=1`.
- [ ] `herdr_snapshot` takes `workspace_id`, `tab_id` and `summary`
  (counts and ids only) and returns the summary by default once the reply
  passes 20 000 characters, saying so; pinned by `tests/tools.rs`.
- [ ] Post-implementation review of phases 1 to 3, with the security pass
  over `## Security`'s rows, recorded in `docs/research/01-first-version.md`:
  scope, findings or none, fixes and the re-run checks.
- [ ] Check: `HERDR_MCP_LIVE=1 sh scripts/check.sh` passes, and
  `sh scripts/bench.sh` prints the numbers inside the budget.

### Phase 4: open the PR

- [ ] The branch pushed and its PR opened against the branch it was cut
  from (or, if one is already open for this branch, its body brought up to
  date) with a Conventional Commit title for the whole change, a body
  naming this plan, what changed and why, and the checks run with their
  result. The PR's URL printed as the evidence. Opened, never merged.
