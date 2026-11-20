# Review of plans/05-events.md, phases 1 to 3

Reviewed 20/11/2026 on herdr 0.9.5 (protocol 22), after the code landed.

## Scope

`src/events/{ring,hub}.rs`, `src/tools/events.rs`, the `--event-buffer`
flag in `src/config.rs`, the hub wiring in `src/server.rs`, and the tests
in `tests/lane05_tools.rs` (fake socket with a streaming fake in
`tests/support/fake_events.rs`), `tests/lane05_live.rs` (isolated session
`herdr-mcp-test-05`) and the unit tests in `src/events/ring.rs`.

## Measured on the real herdr

- `pane.agent_status_changed` arrives as `"event":"pane.agent_status_changed"`
  (dotted), every other event as snake case (`tab_created`). The ring keeps
  herdr's name unchanged and the `types` filter treats the two spellings as
  one.
- Ids sit one object down (`data.tab.tab_id`, `data.pane.pane_id`) on
  created events and at the top (`data.pane_id`) on closed and status
  events; the ring reads both.
- A subscription naming a missing pane is refused with `pane_not_found`
  (not `invalid_request`) and the connection closes.
- `pane.report_agent` (`pane_id`, `source`, `agent`, `state`) drives a
  status change, which is how the live test sees one without a real agent.
- A restart of the isolated server closes the stream (EOF); the hub
  reconnects within its backoff and leaves a `gap` with a snapshot summary.

## Findings

| Finding | Fix |
| --- | --- |
| The hub's own connections showed up in plan 01's request-log tests, and the new tools in its tool lists. | `tests/support/mod.rs` defaults `HERDR_MCP_TOOLS` to the plan 01 groups; tests of later groups name theirs in `opts.env`. |
| A pane created while the old stream drains would be in neither pane set, and its twin on the new stream is suppressed as a duplicate. | The drain reports a pane event and the rebuild runs again. |
| Duplicates are found by identical line, for 1 s after a rebuild. A genuine identical event in that second would be dropped. | Accepted: herdr events carry no id, the window is short and only follows a pane change. |
| The fake kept a closed subscription "live" until it wrote to it, so a test of the old stream closing passed for the wrong reason. | The fake now ends a subscription on client EOF. |
| Two live tests share one session and `cargo test` runs them in parallel. | A static async mutex in `tests/lane05_live.rs`. |
| `check.sh` names `--test live` only, so `lane05_live` runs in its plain `cargo test` step when `HERDR_MCP_LIVE=1`, not in the live step. | Left as is (shared file); the tests are serialised and pass in both. |

No bug found in the ring's cursor logic: eviction, a cursor ahead of the
server (a restarted server) and a cursor older than the buffer each return
a `gaps` entry.

## Memory

`target/release/herdr-mcp` with only the `events` group, against the live
isolated session, after 1200 `workspace.rename` calls (buffer full at 1000
entries of about 150 bytes of data each): idle 4.2 MB RSS, full buffer
5.3 MB RSS (budget 8 MB). Real pane events are larger (a pane object is
about 500 bytes of JSON), so expect roughly 1 MB more at a full buffer.

## Checks

`HERDR_MCP_LIVE=1 sh scripts/check.sh` passes, including the bench with the
hub running (start p50 1.7 ms, RSS after 1000 calls 4.2 MB, one noisy run
over the call budget that passed on re-run).
