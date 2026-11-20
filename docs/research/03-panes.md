# Review of plans/03-panes.md, phases 1 and 2

Reviewed 20/11/2026 against herdr 0.9.5 (protocol 22), live in the isolated
`herdr-mcp-test-03` session and with the fake socket.

## Scope

`src/tools/panes.rs` (all 18 pane tools), `src/limits.rs`, `HerdrClient::call_wait`
and the reply-size cap in `src/herdr.rs`, plus `tests/lane03_tools.rs`,
`tests/lane03_live.rs`.

## Security pass over `## Security`

| Threat | Finding | Proved by |
| --- | --- | --- |
| Elevation of privilege | `_send_text`, `_send_keys`, `_run` with `pane_id`, `_clear`, `_close` refuse `HERDR_PANE_ID` with `self_target` and make no herdr call. A new-pane `_run` splits next to the caller, which is another pane. `_swap`, `_move`, `_focus` act on layout, not input, and are allowed on the own pane. | `own_pane_input_clear_and_close_are_refused_without_calling_herdr`, live refusal |
| Tampering | the command is one JSON string in `pane.send_input`; no shell runs in this process; quotes, `;`, backticks and `\n` arrive unchanged | `run_splits_types_the_exact_command_and_presses_enter_in_one_call` |
| Prompt injection | read and wait descriptions say output is untrusted data; the server instructions already say so; output is returned in `structuredContent` only | `input_tools_are_not_read_only...` (description check) |
| Denial of service | `lines` is clamped to 2000, text to 40000 characters keeping the tail; waits are capped at 10 minutes with herdr's `timeout_ms` plus a 2 s local guard; a reply over 16 MiB is refused (`reply_too_large`), closing the plan 01 review item | `read_is_capped...`, `wait_timeout_is_capped...`, `a_hung_herdr_is_cut_off...`, `an_oversized_herdr_reply...` |
| Information disclosure | stderr carries method names and ids only | `stderr_never_carries_pane_text` |

## Findings

- None blocking. Notes, not defects:
  - `self_target` only matches the exact `HERDR_PANE_ID` string. A client
    could name the same pane by another id form (herdr's `terminal_id`); the
    pane tools take pane ids only, and herdr returns `not_found` for other
    forms, so this is not reachable today.
  - A wait matches text already on screen, including the typed command, so a
    careless `match` returns at once. Documented in the tool descriptions and
    the README rather than hidden.
  - `herdr_pane_wait_output` is marked read-only: it never changes herdr.

## Fixes and re-run checks

None needed after the review; `HERDR_MCP_LIVE=1 sh scripts/check.sh` and
`sh scripts/bench.sh` were run at the end (see the PR).
