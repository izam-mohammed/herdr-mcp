# Claude Code reads, prompts and waits on the other coding agents in herdr

## The ask

`features.md` § Agents: see each agent's state and output, send it a
prompt, wait until it is `idle`, `done` or `blocked`, answer its
permission prompts, and start new ones. Plan 01 landed `herdr_agent_list`;
this plan lands the rest, with the server's own agent off limits.

## Alternatives

### Decision 1: how is an agent `target` checked against the server's own pane? (two-way door)

A `target` is a pane id or an agent name (`docs/external/herdr-socket-api.md`
§ Agents), so a string compare misses names.

Searched 2026-10-22: "herdr agent target name or pane id"; herdr's socket
API page and `herdr --skill`, the schema's `AgentInfo` (it carries
`pane_id` and `terminal_id`).

| Criterion | Compare `target` to the own pane id | `agent.get` first, compare its `terminal_id` with the own one |
| --- | --- | --- |
| Performance efficiency | no call | one call, ~0.3 ms |
| Fit to the ask | misses a target given by name | catches names and ids, and moved panes |
| Reliability | silently wrong for names | fails closed if `agent.get` fails |

**Chosen: `agent.get` first.** 0.3 ms buys a check that cannot be dodged
by naming the agent. Revisit never; this is a must in practice.

### Decision 2: prompt then wait, in one request or two? (two-way door)

Searched 2026-10-22: "herdr agent prompt wait atomic stalled"; `herdr
--skill` on `--wait`, the schema's `AgentPromptParams.wait`.

| Criterion | `agent.prompt` with `wait` | `agent.prompt`, then `agent.wait` |
| --- | --- | --- |
| Performance efficiency | one connection | two |
| Reliability | herdr watches from submission: a quick `working` then `idle` is not missed; `agent_prompt_stalled` if nothing starts in 5 s | a fast agent can finish between the calls, so the wait hangs |
| Fit to the ask | features.md: "which herdr does atomically" | |

**Chosen: `agent.prompt` with `wait`.** Revisit if herdr deprecates it.

## The decision

Feature context. The `agents` group gains `herdr_agent_get`, `_read`,
`_explain`, `_prompt`, `_wait`, `_send_keys`, `_start`, `_focus`,
`_rename`. Input tools (`_prompt`, `_send_keys`) resolve the target with
`agent.get` and refuse the server's own agent (decision 1). `_prompt` with
`wait` sends herdr's atomic form (decision 2) under plan 03's `call_wait`
bounds; `agent_blocked` and `agent_prompt_stalled` pass through as
herdr's errors. `_read` uses plan 03's caps.

Dependencies and permissions: none new.

Standards: `docs/standards.md` (format, lint, tests, audit, secret scan,
shell check and build through `sh scripts/check.sh`).

## Contract

Adds to plan 01's: `herdr_agent_prompt` takes `target`, `text`, `wait?`
(`{"until": [states], "timeout_ms"?}`); `herdr_agent_wait` takes `target`,
`until` (default `["idle", "done", "blocked"]`), `timeout_ms?`. Waits
return `timed_out: true` at the cap, as in plan 03.

## Where it lands

| Path | New or edit | Phase | What changes | Firm? |
| --- | --- | --- | --- | --- |
| `src/tools/agents.rs` | edit | 1, 2 | the agent tools | reuse |
| `src/safety.rs` | edit | 1 | `refuse_self_agent` via `agent.get` | reuse |
| `tests/tools.rs`, `tests/fixtures/` | edit | 1, 2 | agent tools and refusals | reuse |
| `tests/live.rs` | edit | 2 | `agent.start` with a stub agent, if herdr allows one | reuse |
| `README.md` | edit | 1 | agent tools and their safety | reuse |

*Reused, not rewritten:* `call_wait` (`src/herdr.rs`) and the caps
(`src/limits.rs`) from plan 03; `own_pane()` (`src/safety.rs`) from plan
02; `src/tools/agents.rs` from plan 01. Searched for a target resolver:
none, so `refuse_self_agent` is new.

## Security

What is protected: the other agents, which act on prompts with their own
permissions, and the server's own client. Who can reach it: the MCP
client, behind the read-only gate.

| Threat | Here | Handled by | Proved by |
| --- | --- | --- | --- |
| Elevation of privilege | prompting or sending keys to the server's own agent, by id or by name | `refuse_self_agent` | `tests/tools.rs`: own agent refused by pane id and by name |
| Elevation of privilege | answering another agent's permission prompt to approve something | `_send_keys` is non-read-only, so the client asks per the user's rules; the README says to keep it on "ask" | `tests/tools.rs`: annotations on `_send_keys` |
| Prompt injection | one agent's output steers this one into prompting a third | output returned as data; descriptions say so | review item in Phase 3 |
| Denial of service | a wait on an agent that never settles | plan 03's bounds | `tests/tools.rs`: `_wait` returns `timed_out` |

Checklist: OWASP Top 10 for LLM Applications (2025), LLM01 Prompt
Injection, LLM06 Excessive Agency.

## Declined

- Comparing the target string (decision 1): misses agents named by name.
- Prompt then wait in two calls (decision 2): races a fast agent.

## What the tests hold

Each tool sends the documented method and params; the own agent is refused
by id and by name; `_prompt` with `wait` sends one request; herdr's
`agent_blocked` and `agent_prompt_stalled` pass through; waits end at the
cap.

## Repository knowledge

Read root `KNOWLEDGE.md` when present before each phase. Follow the global and
repository instructions for recording useful development findings.

## Phases

### Phase 1: the rough cut: prompt another agent and wait (needs you: try it)

- [x] `herdr_agent_get`, `_read`, `_prompt` (with `wait`), `_wait` and
  `_send_keys` (expected in `src/tools/agents.rs`), with
  `refuse_self_agent` in `src/safety.rs`; pinned by `tests/tools.rs` for
  each tool and every `## Security` row; `README.md` updated.
  Landed 2026-11-20: tests are in `tests/lane04_tools.rs` with
  `tests/fixtures/lane04.jsonl` (lane file, `tests/tools.rs` only has its
  tool-name lists extended). `refuse_self_agent` returns the resolved pane
  id and input goes there, so a rename cannot dodge the check. Waits use a
  private client and local caps in `agents.rs` until plan 03's `call_wait`
  and `limits` are joined.
- [x] Check: `HERDR_MCP_LIVE=1 sh scripts/check.sh` passes.
- [ ] You try it: with a second Claude Code running in another herdr pane,
  ask the first to give it a small task and tell you when it is done; then
  ask it to prompt itself, and say whether both behave as you expect.
  Landed 2026-11-21 (partial, box stays open): `herdr_agent_list`, `_get`,
  `_explain` and `_read` worked through real Claude Code on the pane's own
  Claude agent (state `idle`, matched rule `live_prompt_box`). Not done:
  prompting a second agent or the self-prompt refusal, since
  `herdr_agent_prompt` is hidden outside a herdr pane.

### Phase 2: the rest of the agents

- [x] `herdr_agent_explain`, `_start`, `_focus` and `_rename` (expected in
  `src/tools/agents.rs`), `_start` checking herdr's 3 001 to 300 000 ms
  timeout range before the call; pinned by `tests/tools.rs`, and by
  `tests/live.rs` starting an agent when the isolated session can run one.
  Landed 2026-11-20: the live test is `tests/lane04_live.rs` (session
  `herdr-mcp-test-04`, a stub `claude` script on PATH; herdr detects it).
  `HERDR_MCP_LIVE=1 cargo test --test lane04_live -- --test-threads=1`;
  `scripts/check.sh` only runs `--test live`, so the join adds it.
- [x] Check: `HERDR_MCP_LIVE=1 sh scripts/check.sh` passes.

### Phase 3: review

- [x] Post-implementation review of phases 1 and 2 with the security pass
  over `## Security`, recorded in `docs/research/04-agents.md`: scope,
  findings or none, fixes and re-run checks.
- [x] Check: `HERDR_MCP_LIVE=1 sh scripts/check.sh` passes, and
  `sh scripts/bench.sh` stays inside the budget.

### Phase 4: open the PR

- [x] The branch pushed and its PR opened against the branch it was cut
  from (or, if one is already open for this branch, its body brought up to
  date) with a Conventional Commit title for the whole change, a body
  naming this plan, what changed and why, and the checks run with their
  result. The PR's URL printed as the evidence. Opened, never merged.
  PR: https://github.com/izam-mohammed/herdr-mcp/pull/7

Landed 2026-11-20 (join): `refuse_self_agent` stays (it returns the resolved pane
id) but its comparison is `safety::Target::Agent`, the shared own-pane check
(own terminal from `pane.current` and `pane.list`, no longer `pane.get`); the
code stays `own_agent`. The private wait client is replaced by
`HerdrClient::call_wait` (local guard 2 s past the wait, was 5 s) and the wait
constants by `limits::wait_ms` (60 s default, 10 min max, as before). Agent reads
keep 100 default and 1000 max lines, through `limits::lines_within`.
`scripts/check.sh` now runs `lane04_live`.
