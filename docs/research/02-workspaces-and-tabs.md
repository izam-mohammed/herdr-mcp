# Research 02: workspaces and tabs

## Post-implementation review (20/11/2026)

Scope: phases 1 and 2 of `plans/02-workspaces-and-tabs.md`: the nine new
tools in `src/tools/workspaces.rs` and `src/tools/tabs.rs`, `refuse_self`
and `require_confirmation` in `src/safety.rs`, `--confirm-destructive` in
`src/config.rs`, `tests/lane02_tools.rs` and `tests/lane02_live.rs`. Each
row of the plan's `## Security` table was read against the code and run.

| Threat | Result | Proof |
| --- | --- | --- |
| Elevation: close of the own tab or workspace | Held. `refuse_self` finds the own pane's `terminal_id` (from `pane.current`, cached) in `pane.list` and compares its tab or workspace id with the target; `allow_self: true` skips the check and is never forwarded to herdr | `own_tab_and_workspace_are_refused_and_others_close`, `allow_self_lets_the_close_through_without_extra_params`, `the_refusal_reads_clearly` |
| Elevation: own pane moved, new workspace closed | Held. The lookup is by `terminal_id`, so the new place is the one checked | `a_moved_own_pane_protects_its_new_workspace` (fake), `a_moved_own_pane_is_still_protected_in_its_new_workspace` (live, real `pane.move`) |
| Elevation: herdr cannot say where the pane is | Held, fails closed: a failed `pane.list` or `pane.current`, or a pane without `terminal_id`, refuses the close | `a_failed_pane_list_refuses_the_close`, `no_terminal_id_for_the_own_pane_refuses_the_close` |
| Tampering: metadata tokens outside herdr's limits | Held. 17 keys, bad or empty or 33-character keys, ttl 0 and above 86 400 000 are refused as `invalid_params` before herdr; 16 keys of 32 characters and the maximum ttl pass | `metadata_limits_are_refused_before_herdr` |
| Repudiation: untraceable close | Held as far as the log goes. stderr names the method and request id of each call (`workspace.close`) and no reply body | `the_log_names_the_close_but_not_pane_text` |
| Write gating | Held. Every new write tool has `readOnlyHint: false` and drops in read-only mode; the closes alone carry `destructiveHint`, and `--confirm-destructive` adds `requiresUserInteraction` to those only | `read_only_mode_drops_the_new_writes`, `closes_are_destructive_and_other_writes_are_not`, `confirm_destructive_marks_only_the_closes` |

Findings, none needing a fix in this plan:

- The check and the close are two calls, so a pane moved in between (by a
  person or another client) can slip past. herdr offers no atomic
  "close unless it holds this pane". The window is a few milliseconds and
  the move needs another actor; left as is.
- `workspace_close` with `close_group: true` closes every workspace of a
  worktree group, and the group's membership is not in `pane.list`, so the
  check only covers the named workspace. Mitigations: `close_group` is sent
  only when the caller sets it, the tool description says never to set it
  to get past `workspace_group_close_required`, and the close is
  `destructiveHint`. Plan 05 (worktrees) should revisit this when it knows
  group membership.
- The own pane's `terminal_id` is captured at the first close, not at
  start. A start-up call would break plan 01's tests that count requests and
  costs a call for sessions that never close anything. If the pane moved
  before that first close, `pane.current` by the stale `HERDR_PANE_ID` fails
  and the close is refused (fail closed); the caller can use `allow_self`
  once the user agrees.
- The plan's repudiation row says the log line has `workspace.close w2`;
  the log has the method and the request id (as in plan 01), not the target
  id. Adding the target id is a one-line change if wanted.

Re-run after the review: `HERDR_MCP_LIVE=1 sh scripts/check.sh` passes, and
`sh scripts/bench.sh` prints PASS.
