# Claude Code sees what changed in herdr since it last looked, and can wait for the next change

## The ask

The first question was about "the active tabs and other events".
`features.md` § Events: one long-lived subscription per server, a bounded
buffer, and three tools: `herdr_events_recent` (since a cursor),
`herdr_events_wait` (block until a matching event) and
`herdr_events_status`. Claude acts only in a turn, so events are something
it asks for or waits on, not something pushed to it (push is plan 07's
question).

The research changed one part: `pane.agent_status_changed` needs a
`pane_id` per subscription entry, so "until any agent goes `blocked`"
means one entry per pane, kept in step as panes come and go
(`docs/external/herdr-socket-api.md` § Events).

## Alternatives

### Decision 1: where do the events come from? (two-way door)

Searched 2026-10-22: "herdr events.subscribe events.wait"; herdr's socket
API page (v0.9.5) on subscriptions and `events.wait`, the schema's
`EventsWaitParams`.

| Must | One subscription per server, buffered | herdr's `events.wait` per tool call | A new subscription per tool call |
| --- | --- | --- | --- |
| "Events since a cursor" (`herdr_events_recent`) | yes | no (no history) | no (no history) |
| **Survives** | **yes** | no | no |

- Cost of the survivor: one idle connection per server, and events parsed
  only as they arrive.

**Chosen: one subscription per server, buffered.** `herdr_events_wait`
waits on the same buffer, so a wait and a later `recent` agree. Revisit if
herdr adds history replay.

### Decision 2: what holds the buffer? (two-way door)

Searched 2026-10-22: "tokio broadcast vs VecDeque ring buffer cursor",
"tokio Notify"; tokio's `sync::broadcast` and `sync::Notify` docs.

| Criterion | `VecDeque` ring with a sequence number, `tokio::sync::Notify` to wake waiters | `tokio::sync::broadcast` channel | Append-only `Vec`, trimmed |
| --- | --- | --- | --- |
| Performance efficiency | O(1) push and evict; a cursor read is a binary search on sequence | O(1), but no reading the past by cursor | trimming shifts the whole vector |
| Fit to the ask | cursor reads, filters, waits, a gap marker | only events after a receiver joined | the same as the ring, slower |
| Memory | fixed by capacity; each entry the raw line plus parsed ids | fixed by capacity | grows until trimmed |

**Chosen: `VecDeque` ring with `Notify`.** Default 1000 entries
(`--event-buffer`); entries keep the raw JSON line once, plus the kind and
ids used for filtering, so nothing is parsed twice. Revisit if the
measured memory at a full buffer breaks the budget.

### Decision 3: how does the server watch every agent's status? (two-way door)

Searched 2026-10-22: "herdr pane.agent_status_changed pane_id required";
the schema's `Subscription` list, a live `events.subscribe` that omitted
`pane_id` (refused with `invalid_request`).

| Criterion | One entry per pane, the subscription rebuilt when panes change | Watch `pane.updated` and re-read `agent.list` | Poll `agent.list` on a timer |
| --- | --- | --- | --- |
| Performance efficiency | herdr pushes only changes; a rebuild per pane created or closed | an `agent.list` per pane update | calls every tick, idle or not |
| Fit to the ask | the documented event with the new state | ? (whether `pane.updated` fires on a status change is not stated) | misses changes between ticks |
| Reliability | ordering per entry, as documented | depends on the unknown | gaps by design |

**Chosen: one entry per pane, rebuilt.** On `pane.created` or
`pane.closed`, the server validates the pane set with `pane.list` (herdr
rejects a whole request naming a missing pane), opens the new subscription,
waits for `subscription_started`, then closes the old one, dropping
duplicates in the overlap. Revisit if `pane.updated` turns out to carry
status changes.

## The decision

Feature context. With the `events` group on, the server starts an
`EventHub` at launch: one subscription to every lifecycle type plus
`pane.agent_status_changed` per pane (decisions 1 and 3), feeding a ring
(decision 2). On `events_lost` or a dropped connection it resubscribes
with backoff (100 ms doubling to 5 s), refreshes from `session.snapshot` on
a second connection as herdr's docs advise, and writes a gap marker so
`herdr_events_recent` can say events were missed. Usable first: Phase 1's
rough cut is lifecycle events only.

Dependencies and permissions: none new. One more long-lived connection to
the user's own herdr socket.

Standards: `docs/standards.md` (format, lint, tests, audit, secret scan,
shell check and build through `sh scripts/check.sh`).

## Contract

Adds to plan 01's: flag `--event-buffer <n>` / `HERDR_MCP_EVENT_BUFFER`
(default 1000). Tools:

- `herdr_events_recent`: `since?` (cursor), `limit?` (default 50, max
  500), `types?`, `workspace_id?`, `tab_id?`, `pane_id?` → `events` (each
  `{"seq", "event", "data"}` with herdr's `event` and `data` unchanged),
  `next` (cursor), `gaps` (sequence ranges lost).
- `herdr_events_wait`: the same filters plus `agent_status?`,
  `timeout_ms?` (default 60 s, max 10 min) → the first matching event
  after the call (or after `since`), or `timed_out: true`.
- `herdr_events_status`: `connected`, `buffered`, `capacity`, `last_seq`,
  `last_event_at`, `lost` (count), `panes_watched`.

## Where it lands

| Path | New or edit | Phase | What changes | Firm? |
| --- | --- | --- | --- | --- |
| `src/events/hub.rs` | new | 1, 2, 3 | subscription task, rebuild, reconnect | expected |
| `src/events/ring.rs` | new | 1 | ring, cursor reads, filters, `Notify` | expected |
| `src/tools/events.rs` | new | 1 | the three tools | expected |
| `src/herdr.rs` | edit | 1 | `subscribe()` returning a line stream | reuse |
| `tests/events.rs`, `tests/fixtures/` | new | 1, 2, 3 | ring and hub on the fake socket | expected |
| `tests/support/fake_herdr.rs` | edit | 1 | streams events after `subscription_started` | reuse |
| `tests/live.rs` | edit | 1, 3 | real events; the isolated server restarted | reuse |
| `README.md` | edit | 1 | the event tools | reuse |

*Reused, not rewritten:* socket resolution and `call` in `src/herdr.rs`,
`call_wait`'s bounds (plan 03), the fake socket and live harness. Searched
for an existing event or buffer type: none in the tree.

## Edge cases

- No panes yet: the subscription has lifecycle entries only.
- A pane closes between `pane.list` and the subscribe: herdr rejects and
  closes; the hub retries once with a fresh list.
- herdr restarts (`herdr update --handoff`, server stop): reconnect with
  backoff; `herdr_events_status` shows `connected: false` meanwhile.
- A wait's filter names a pane that does not exist: the wait still runs;
  it ends at its timeout.
- Buffer full: oldest evicted; a cursor older than the oldest entry
  returns from the oldest with a gap marked.

## Security

Security: no new input surface beyond plan 01's tool arguments, which are
typed and bounded (limits, timeouts, buffer size), no new data stored past
the process's life, no secret, dependency or permission. Event payloads
(titles, labels) are untrusted text and are returned as data, under plan
01's descriptions.

## Declined

- herdr's `events.wait` per call, and a subscription per call (decision
  1): no history for `herdr_events_recent`.
- `broadcast` and a trimmed `Vec` (decision 2): no cursor reads, or O(n)
  trims.
- `pane.updated` with re-reads, and polling (decision 3): unknown, or
  gaps by design.

## What the tests hold

The ring evicts, reads by cursor, filters and wakes waiters; the hub
rebuilds on pane changes without losing or doubling an event; `events_lost`
and a dropped socket produce a gap marker and a reconnect; the live test
sees a real `tab.created` and an agent status change in the isolated
session, and survives the isolated server's restart.

## Repository knowledge

Read root `KNOWLEDGE.md` when present before each phase. Follow the global and
repository instructions for recording useful development findings.

## Phases

### Phase 1: the rough cut: recent and wait on lifecycle events (needs you: try it)

- [ ] `EventHub` subscribing to every lifecycle type (expected in
  `src/events/hub.rs`), the ring (`src/events/ring.rs`) and
  `herdr_events_recent`, `herdr_events_wait`, `herdr_events_status`
  (`src/tools/events.rs`); pinned by `tests/events.rs` (eviction, cursor,
  filters, wake-up) and `tests/live.rs` (a tab created in the isolated
  session shows up); `README.md` updated.
- [ ] Check: `HERDR_MCP_LIVE=1 sh scripts/check.sh` passes.
- [ ] You try it: in Claude Code, ask what changed in herdr in the last
  minute after opening and closing a tab by hand; then ask it to wait until
  you focus another workspace, and say whether the answers are useful.

### Phase 2: every agent's status

- [ ] The hub adds `pane.agent_status_changed` per pane and rebuilds on
  `pane.created` and `pane.closed` as decision 3 says, and
  `herdr_events_wait` takes `agent_status`; pinned by `tests/events.rs`
  (rebuild without loss or duplicates) and `tests/live.rs` (an agent's
  status change seen).
- [ ] Check: `HERDR_MCP_LIVE=1 sh scripts/check.sh` passes.

### Phase 3: lost events and restarts

- [ ] `events_lost`, a rejected pane set and a dropped connection each
  lead to a gap marker, a snapshot refresh and a resubscribe with backoff
  (expected in `src/events/hub.rs`); pinned by `tests/events.rs` on the
  fake socket and by `tests/live.rs` stopping and restarting the isolated
  server.
- [ ] Post-implementation review of phases 1 to 3, recorded in
  `docs/research/05-events.md`: scope, findings or none, fixes and re-run
  checks, and the RSS measured with a full 1000-event buffer.
- [ ] Check: `HERDR_MCP_LIVE=1 sh scripts/check.sh` passes, and
  `sh scripts/bench.sh` stays inside the budget with the hub running.

### Phase 4: open the PR

- [ ] The branch pushed and its PR opened against the branch it was cut
  from (or, if one is already open for this branch, its body brought up to
  date) with a Conventional Commit title for the whole change, a body
  naming this plan, what changed and why, and the checks run with their
  result. The PR's URL printed as the evidence. Opened, never merged.
