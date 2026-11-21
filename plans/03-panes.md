# Claude Code splits panes, runs commands in them and reads what they print

## The ask

`features.md` § Panes: list, inspect, read, split, run, type into, wait on,
arrange and close panes. "Run a command in a new pane and report when it
finishes or fails" is the use the user named. Reading is capped (§ Limits),
waits are bounded, and typing into or closing the server's own pane is
refused (§ Safety).

## Alternatives

### Decision 1: how does `herdr_pane_run` type the command and press enter? (two-way door)

Searched 2026-10-22: "herdr pane run send_input"; herdr's socket API page
(v0.9.5), the schema's `PaneSendInputParams`, `herdr pane --help`.

| Criterion | `pane.send_text` with `\n` | `pane.send_text`, then `pane.send_keys` | `pane.send_input` with `text` and `keys` |
| --- | --- | --- | --- |
| Performance efficiency | one call | two calls | one call |
| Reliability | a newline inside pasted text may not submit | another write can land between the two | text then keys in one request |
| Fit to the ask | | | the schema's own combined form |

**Chosen: `pane.send_input`.** One request, nothing can interleave. The key
name for enter is not stated in the docs; Phase 1's live test settles it
and records it in `docs/external/herdr-socket-api.md`. Revisit if
`send_input` rejects a key the CLI accepts.

### Decision 2: how are waits bounded? (two-way door)

`pane.wait_for_output` can block for as long as asked; MCP clients can
cancel a call.

Searched 2026-10-22: "tokio timeout drop UnixStream cancel", "MCP
cancellation notifications/cancelled"; tokio docs, the MCP spec on
cancellation, rmcp's request context.

| Criterion | herdr's `timeout_ms` only | a tokio timeout only | both: herdr's `timeout_ms` at the cap, a tokio guard 2 s past it, the connection dropped on cancel |
| --- | --- | --- | --- |
| Performance efficiency | a hung herdr holds the task | herdr keeps waiting after the client gave up | nothing left waiting |
| Reliability | trusts herdr | herdr not told | both ends bounded |
| Fit to the ask | | | a clean "timed out" result, per § Limits |

**Chosen: both.** Default 60 s, maximum 10 minutes; a timeout is a normal
result with `timed_out: true`, not an error. Revisit if herdr adds
server-side cancellation.

### Decision 3: how is read output kept small? (two-way door)

Every character returned is context the model pays for on every later
turn.

Searched 2026-10-22: "Claude Code MCP output limit"; Claude Code's MCP docs
(`docs/external/claude-code-mcp.md` § Output limits).

| Criterion | herdr's `lines` only | lines capped, plus a character cap keeping the tail | leave it to Claude Code's 50 000-character file spill |
| --- | --- | --- | --- |
| Performance efficiency | one long line can still be huge | bounded in lines and bytes | 25 000 tokens can land in context |
| Fit to the ask | | default 200 lines, max 2000; 40 000 characters, tail kept, truncation said | |

**Chosen: lines capped plus a character cap keeping the tail.** The tail
is where a command's result is. Revisit if a 40 000-character cap cuts
useful output in the try.

## The decision

Feature context. The `panes` group: `herdr_pane_list`, `_get`, `_read`,
`_split`, `_run`, `_send_text`, `_send_keys`, `_wait_output`, `_close` in
the rough cut; then `_layout` (with `pane.edges`, `pane.neighbor`),
`_process_info`, `_focus` (with `pane.focus_direction`), `_zoom`,
`_resize`, `_swap`, `_move`, `_rename`, `_clear`. `herdr_pane_run` splits a
new pane when no `pane_id` is given, types the command through
`pane.send_input` (decision 1), and with `wait` set calls
`pane.wait_for_output` bounded as decision 2 says. Reads are capped by
decision 3. Every input tool and `_clear`, `_close` refuse the server's
own pane through plan 02's `refuse_self`.

Dependencies and permissions: none new.

Standards: `docs/standards.md` (format, lint, tests, audit, secret scan,
shell check and build through `sh scripts/check.sh`).

## Contract

Adds to plan 01's: `herdr_pane_run` takes `command`, `pane_id?`,
`direction?` (`right`, `down`), `cwd?`, `wait?` (`{"match": {"type":
"substring"|"regex", "value"}, "timeout_ms"?}`); returns herdr's results
plus `pane_id` and, with `wait`, `matched` or `timed_out: true`. Read
tools take `source` (default `recent_unwrapped`) and `lines` (default 200,
max 2000) and add `truncated: true` when the character cap applied.

## Where it lands

| Path | New or edit | Phase | What changes | Firm? |
| --- | --- | --- | --- | --- |
| `src/tools/panes.rs` | new | 1, 2 | the pane tools | expected |
| `src/herdr.rs` | edit | 1 | `call_wait` with the double bound and cancel | reuse |
| `src/limits.rs` | new | 1 | line and character caps | expected |
| `tests/tools.rs`, `tests/fixtures/` | edit | 1, 2 | pane tools, caps, refusals | reuse |
| `tests/live.rs` | edit | 1 | split, run, wait, read, close | reuse |
| `docs/external/herdr-socket-api.md` | edit | 1 | the enter key's name | reuse |
| `README.md` | edit | 1 | pane tools and their safety | reuse |

*Reused, not rewritten:* `HerdrClient::call` and the timeout in
`src/herdr.rs`, `refuse_self` and `own_pane()` in `src/safety.rs` (plan
02), the fake socket and live harness. Searched for an existing cap or
truncation helper: none, so `src/limits.rs` is new and plan 04's agent
reads use it too.

## Security

What is protected: every other terminal, since `_run`, `_send_text` and
`_send_keys` can run any command there. Who can reach it: the MCP client,
behind plan 01's read-only gate (none of these is registered in read-only
mode).

| Threat | Here | Handled by | Proved by |
| --- | --- | --- | --- |
| Elevation of privilege | typing into the server's own pane, i.e. its own client's prompt | `refuse_self` | `tests/tools.rs`: send, run, clear and close on own pane refused |
| Tampering | a command built from strings | the command is one JSON string field to herdr; no shell runs in this process | `tests/tools.rs`: the request body is the exact text |
| Prompt injection | pane output tells the model what to do | descriptions say output is untrusted data; output is returned as data, never as instructions | review item in Phase 3 |
| Denial of service | a huge read, or a wait that never ends | decisions 2 and 3 | `tests/tools.rs`: caps applied; a wait past the cap returns `timed_out` |
| Information disclosure | secrets printed in a pane copied into logs | output never logged without `--debug` | `tests/tools.rs`: captured stderr has no pane text |

Tool annotations: `_run`, `_send_*` `readOnlyHint: false`, `openWorldHint:
true` (they reach a shell); `_close`, `_clear` `destructiveHint: true`.
Checklist: OWASP Top 10 for LLM Applications (2025), LLM01 Prompt
Injection, LLM06 Excessive Agency.

## Edge cases

- `_run` with no `pane_id` and no focused workspace: split fails; herdr's
  error passed through.
- Regex that herdr rejects: herdr's error passed through, before any wait.
- Pane exits while waited on: herdr's result passed through.
- The MCP client cancels a wait: the herdr connection is dropped.

## Declined

- `send_text` with `\n`, and two calls (decision 1): may not submit, or can
  interleave.
- One-sided timeouts (decision 2): leave one end waiting.
- herdr's `lines` alone, or Claude Code's spill (decision 3): unbounded
  characters reach the model.

## What the tests hold

Each tool sends the documented method and params; `_run` makes one
`send_input` call; caps apply and say so; waits end at the cap with
`timed_out`; own-pane input is refused; the live test runs `echo` in a new
pane, waits for it and reads it back.

## Repository knowledge

Read root `KNOWLEDGE.md` when present before each phase. Follow the global and
repository instructions for recording useful development findings.

## Phases

### Phase 1: the rough cut: run, wait, read (needs you: try it)

- [x] `herdr_pane_list`, `_get`, `_read`, `_split`, `_run`, `_send_text`,
  `_send_keys`, `_wait_output` and `_close` (expected in
  `src/tools/panes.rs`), `call_wait` in `src/herdr.rs` and the caps in
  `src/limits.rs`; pinned by `tests/tools.rs` for each tool and every
  `## Security` row, and by `tests/live.rs` running `echo herdr-mcp-ok` in
  a new pane, waiting for it and reading it; the enter key's name recorded
  in `docs/external/herdr-socket-api.md`; `README.md` updated.
  Landed 2026-11-20: tests are in `tests/lane03_tools.rs` and
  `tests/lane03_live.rs` (isolated session `herdr-mcp-test-03`), fixtures in
  `tests/fixtures/lane03.jsonl`; `call_wait` is in `src/herdr.rs` with a 2 s
  guard and a 16 MiB reply cap (`reply_too_large`); the own-pane check is
  private to `src/tools/panes.rs` (`refuse_own_pane`, code `self_target`)
  because plan 02's `refuse_self` is not on this branch; the enter key is
  `enter`; `herdr_pane_list` filters by tab on this side since herdr takes
  only a workspace.
- [x] Check: `HERDR_MCP_LIVE=1 sh scripts/check.sh` passes.
- [ ] You try it: ask Claude Code in a herdr pane to run `cargo test` (or
  any slow command) in a new pane to the right and tell you when it
  finishes; say whether it waits and reports the way you expect.

### Phase 2: the rest of the panes

- [x] `herdr_pane_layout`, `_process_info`, `_focus`, `_zoom`, `_resize`,
  `_swap`, `_move`, `_rename` and `_clear` (expected in
  `src/tools/panes.rs`), `_clear` refusing the own pane; pinned by
  `tests/tools.rs`, with the edge cases above.
  Landed 2026-11-20: pinned by `tests/lane03_tools.rs`; `herdr_pane_swap`
  takes `source_pane_id` with `target_pane_id` or `direction` (herdr rejects
  `pane_id` alone); `tests/tools.rs` only had its tool-name lists and the
  open-world check updated.
- [x] Check: `HERDR_MCP_LIVE=1 sh scripts/check.sh` passes.

### Phase 3: review

- [x] Post-implementation review of phases 1 and 2 with the security pass
  over `## Security`, recorded in `docs/research/03-panes.md`: scope,
  findings or none, fixes and re-run checks.
- [x] Check: `HERDR_MCP_LIVE=1 sh scripts/check.sh` passes, and
  `sh scripts/bench.sh` stays inside the budget.

### Phase 4: open the PR

- [x] The branch pushed and its PR opened against the branch it was cut
  from (or, if one is already open for this branch, its body brought up to
  date) with a Conventional Commit title for the whole change, a body
  naming this plan, what changed and why, and the checks run with their
  result. The PR's URL printed as the evidence. Opened, never merged.

Landed 2026-11-20 (join): the private `refuse_own_pane` is gone. Input to a pane
now goes through `safety::Target::Pane`: the own pane id is refused without a
call, any other pane costs a `pane.list` (and one `pane.current`, cached) to catch
the own pane after a move. `DEFAULT_WAIT_MS` and `MAX_WAIT_MS` moved to
`src/limits.rs`. `tests/lane03_tools.rs` names its group (`panes`) and ignores
the check's lookups.
