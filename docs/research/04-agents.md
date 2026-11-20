# Plan 04 review: agent tools

## Post-implementation review (20/11/2026)

Scope: phases 1 and 2 of `plans/04-agents.md`: `src/tools/agents.rs`,
`refuse_self_agent` in `src/safety.rs`, `tests/lane04_tools.rs`,
`tests/lane04_live.rs`, the README text. Each row of the plan's
`## Security` table was read against the code and run.

| Threat | Result | Proof |
| --- | --- | --- |
| Elevation: steering the server's own agent, by id or name | Held. `refuse_self_agent` resolves the target with `agent.get`, compares its pane id and `terminal_id` with the own pane, and fails closed if herdr cannot resolve either. Input is then sent to the resolved pane id, not the name, so a rename between check and send cannot redirect it | `own_agent_is_refused_by_pane_id_by_name_and_after_a_move`, `an_unresolvable_target_fails_closed`, the live test |
| Elevation: approving another agent's prompt | Held as far as the server can: `_send_keys` and `_prompt` are not read-only, so the client asks per the user's rules; the README says to keep them on "ask"; read-only mode drops them | `annotations_split_reads_from_input_tools`, `read_only_mode_drops_the_input_tools` |
| Prompt injection: agent output steering this one | Held as far as text can. Descriptions of `_get`, `_read`, `_explain`, `_prompt` say the output is untrusted data; server instructions say so too | `instructions_say_output_is_untrusted` (plan 01) |
| Denial of service: wait on an agent that never settles | Held. Every wait sends a `timeout_ms` (default 60 s, at most 600 s) and the client limit is that plus 5 s; herdr's `timeout` becomes `timed_out: true` | `waits_end_at_the_cap_with_timed_out` |

Findings:

- Fixed in this plan: an earlier draft sent the caller's `target` string
  to herdr after the check; a name reassigned in between would have
  bypassed it. Input now goes to the resolved pane id.
- The own-agent check is skipped when `HERDR_PANE_ID` is unset (the server
  is outside herdr, so `--allow-outside-herdr` is explicit). Nothing to
  compare against; accepted.
- Plan 03's `call_wait` and caps are not in this lane's base. The waits use
  a private client with a longer limit and local constants
  (`WAIT_MAX_MS`, `READ_MAX_LINES` in `src/tools/agents.rs`); the join
  should swap these for plan 03's.
- `herdr_agent_start` also maps herdr's `timeout` to `timed_out: true`
  through the same helper; harmless.
