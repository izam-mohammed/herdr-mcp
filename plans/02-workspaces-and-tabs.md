# Claude Code creates, moves and closes herdr workspaces and tabs, and never closes its own

## The ask

`features.md` § Workspaces and § Tabs: the full set, including the
destructive closes, under § Safety's rule that the server never acts on its
own pane, nor closes the tab or workspace holding it, unless
`allow_self: true`. Plan 01 landed the reads plus tab create, focus and
rename; this plan lands the rest of both areas.

## Alternatives

### Decision 1: how does the server know which pane is its own when a close arrives? (two-way door)

herdr's docs say a pane moved to another workspace gets "a new public pane
id", and a process's environment cannot change after it starts, so
`HERDR_PANE_ID` can go stale.

Searched 2026-10-22: "herdr pane id stable move workspace"; herdr's socket
API page (v0.9.5) on `pane.swap` and `pane.move`, the schema's
`pane_current` result.

| Criterion | `HERDR_*` env ids, read once | `pane.current` with `caller_pane_id` from env | `terminal_id` captured at start, found in `pane.list` per check |
| --- | --- | --- | --- |
| Performance efficiency | no call | one call, ~0.3 ms | one call, ~0.3 ms (`pane.list`) |
| Fit to the ask | wrong after a cross-workspace move: protects the old place | fails `not_found` after the move | follows the pane wherever it moves |
| Reliability | silently wrong | fails closed (refuses) | correct; fails closed if the pane is gone |

- `terminal_id` (`term_...`) comes back from `pane.current` and `pane.list`
  and does not encode a place (`docs/external/herdr-socket-api.md`).

**Chosen: `terminal_id` captured at start.** One 0.3 ms call per
destructive tool, and it stays right after moves. Revisit if herdr adds an
id for the caller that survives moves.

### Decision 2: what extra confirmation do destructive tools get? (two-way door)

Searched 2026-10-22: "MCP destructiveHint client behaviour", "Claude Code
requiresUserInteraction"; the MCP tool annotations spec, Claude Code's MCP
docs (`docs/external/claude-code-mcp.md` § Per-tool `_meta` keys).

| Criterion | `destructiveHint` only | `requiresUserInteraction` always | `destructiveHint`, plus `--confirm-destructive` setting `requiresUserInteraction` |
| --- | --- | --- | --- |
| Performance efficiency | no prompt the user did not configure | a prompt on every close, even in auto mode | the user chooses |
| Fit to the ask | permission rules decide, per features.md | blocks unattended runs the user wants | both |
| Security | relies on the user's rules | strongest | strongest when asked for |

**Chosen: `destructiveHint`, plus `--confirm-destructive`.** Revisit if
users ask for it on by default.

## The decision

Feature context. The `workspaces` and `tabs` groups gain
`herdr_workspace_get`, `_create`, `_focus`, `_rename`, `_move`,
`_report_metadata`, `_close`, and `herdr_tab_move`, `herdr_tab_close`, on
plan 01's `HerdrClient::call` and pass-through results. The server records
its `terminal_id` at start and, before any close, finds its pane in
`pane.list` (decision 1); closing the tab or workspace that holds it is
refused with `self_target` unless `allow_self: true`. Destructive tools
carry `destructiveHint`, and `--confirm-destructive` adds
`requiresUserInteraction` (decision 2). Usable first: Phase 1 is the rough
cut.

Dependencies and permissions: none new.

Standards: `docs/standards.md` (format, lint, tests, audit, secret scan,
shell check and build through `sh scripts/check.sh`).

## Contract

Adds to plan 01's `## Contract`: flag `--confirm-destructive` /
`HERDR_MCP_CONFIRM_DESTRUCTIVE=1`; argument `allow_self: bool` (default
false) on every close; refusal code `self_target`. `close_group` on
`herdr_workspace_close` is passed only when the caller sets it, and its
description says never to set it just to get past
`workspace_group_close_required`.

## Where it lands

| Path | New or edit | Phase | What changes | Firm? |
| --- | --- | --- | --- | --- |
| `src/tools/workspaces.rs` | edit | 1 | the workspace tools | expected |
| `src/tools/tabs.rs` | edit | 1 | `move`, `close` | expected |
| `src/safety.rs` | edit | 1 | `own_pane()` by `terminal_id`, `refuse_self` | reuse |
| `src/config.rs` | edit | 1 | `--confirm-destructive` | reuse |
| `tests/tools.rs`, `tests/fixtures/` | edit | 1 | the new tools and refusals | reuse |
| `tests/live.rs` | edit | 1, 2 | close in the isolated session | reuse |
| `README.md` | edit | 1 | the closes and their safety | reuse |

*Reused, not rewritten:* `HerdrClient::call` (`src/herdr.rs`), the error
mapping (`src/error.rs`), the read-only gate (`src/safety.rs`), the fake
socket (`tests/support/fake_herdr.rs`) and the live-session harness
(`tests/live.rs`), all from plan 01. Searched for an existing
self-protection helper: plan 01 lands only the read-only gate, so
`refuse_self` is new beside it.

## Security

What is protected: every workspace and tab, above all the one the server
runs in. Who can reach it: the MCP client over stdio, behind plan 01's
read-only gate.

| Threat | Here | Handled by | Proved by |
| --- | --- | --- | --- |
| Elevation of privilege | a close of the server's own tab or workspace | `refuse_self` by `terminal_id` | `tests/tools.rs`: own tab and workspace refused with `self_target`; allowed with `allow_self: true` |
| Elevation of privilege | own pane moved to another workspace, then its new workspace closed | the `terminal_id` lookup | `tests/live.rs`: move the pane, close attempt refused |
| Tampering | metadata tokens outside herdr's limits | validated before the call (16 keys, `^[A-Za-z0-9_-]{1,32}$`, TTL 1 to 86 400 000) | `tests/tools.rs`: each limit refused |
| Repudiation | a close nobody can trace | the stderr log line names method and id | `tests/tools.rs`: captured log has `workspace.close w2` |

No new data, secret or dependency. Checklist: OWASP Top 10 for LLM
Applications (2025), LLM06 Excessive Agency.

## Edge cases

- Own pane gone (closed from elsewhere): no self to protect; closes go
  ahead.
- `pane.list` fails before a close: the close is refused (fail closed).
- `workspace_group_close_required`: passed through as herdr's error.

## Declined

- Env ids and `pane.current` by id (decision 1): wrong or failing after a
  cross-workspace move.
- `requiresUserInteraction` always (decision 2): blocks unattended use the
  user may want.

## What the tests hold

Each tool sends the documented method and params and passes the result
through; every close is refused for the server's own tab or workspace and
allowed with `allow_self`; metadata limits are checked before herdr; the
live test proves protection follows a moved pane.

## Repository knowledge

Read root `KNOWLEDGE.md` when present before each phase. Follow the global and
repository instructions for recording useful development findings.

## Phases

### Phase 1: the rough cut: the rest of workspaces and tabs (needs you: try it)

- [x] The nine tools (expected in `src/tools/workspaces.rs` and
  `src/tools/tabs.rs`), `own_pane()` and `refuse_self` in
  `src/safety.rs`, and `--confirm-destructive` in `src/config.rs`; pinned
  by `tests/tools.rs` for every tool and the `## Security` rows, and by
  `tests/live.rs` creating and closing a workspace and a tab in the
  isolated session; `README.md` updated.
  Landed 2026-11-20: the new tests live in `tests/lane02_tools.rs`,
  `tests/lane02_live.rs` (isolated session `herdr-mcp-test-02`) and
  `tests/fixtures/lane02.jsonl`, not in `tests/tools.rs` and
  `tests/live.rs` (parallel lanes), apart from the tool-name lists in
  `tests/tools.rs`. The own pane's `terminal_id` is captured at the first
  close, not at start, so a start sends no call (plan 01's tests count
  requests); a stale id fails closed.
- [x] Check: `HERDR_MCP_LIVE=1 sh scripts/check.sh` passes.
- [ ] You try it: in Claude Code in a herdr pane, ask it to make a
  workspace called `sandbox` with two tabs, move the second tab first,
  then close `sandbox`; then ask it to close its own tab, and say whether
  the refusal reads clearly.

### Phase 2: edge cases and the moved pane

- [x] The edge cases above hold: a moved own pane is still protected
  (`tests/live.rs`), a failed `pane.list` refuses the close, and a group
  close error passes through (`tests/tools.rs`).
- [x] Post-implementation review of phases 1 and 2 with the security pass
  over `## Security`, recorded in `docs/research/02-workspaces-and-tabs.md`:
  scope, findings or none, fixes and re-run checks.
- [x] Check: `HERDR_MCP_LIVE=1 sh scripts/check.sh` passes, and
  `sh scripts/bench.sh` stays inside the budget.

### Phase 3: open the PR

- [ ] The branch pushed and its PR opened against the branch it was cut
  from (or, if one is already open for this branch, its body brought up to
  date) with a Conventional Commit title for the whole change, a body
  naming this plan, what changed and why, and the checks run with their
  result. The PR's URL printed as the evidence. Opened, never merged.
