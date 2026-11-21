# Events

With the `events` group on (it is by default), the server keeps one
subscription to herdr and a ring of the latest events. Three tools read it,
and all three are reads, so they work in read-only mode:

- `herdr_events_recent` returns the events after a cursor (`since`), and a
  new cursor in `next`. It filters by `types`, `workspace_id`, `tab_id` and
  `pane_id`.
- `herdr_events_wait` blocks until the next matching event, or returns
  `timed_out: true`. It also takes `agent_status`, so you can wait for an
  agent to go blocked or done.
- `herdr_events_status` says whether the feed is connected and whether any
  events were missed.

## Reading what changed

Call `herdr_events_recent` once, keep `next`, and pass it as `since` the next
time. A run against a session where one pane was split and used returned:

```json
{ "events": [ { "seq": 1, "event": "pane_created" }, { "seq": 2, "event": "layout_updated" }, { "seq": 3, "event": "pane_updated" } ], "next": 5 }
```

(Trimmed: each event also has a `data` block.)

## Waiting for a state change

```text
herdr_events_wait agent_status=blocked timeout_ms=300000
```

returns the event for the first agent that goes blocked, or times out after
five minutes.

## Gaps

History starts when the server starts: herdr replays nothing. If events may
be missing, a `gap` event appears in the stream and `herdr_events_status`
lists it under `gaps`. That happens when the buffer evicted events
(`--event-buffer`, default 1000), or when herdr dropped the connection
(`events_lost`, or herdr restarting). The server reconnects with backoff.
After a gap, take a fresh `herdr_snapshot` rather than trusting the feed:
events are signals that something changed, not a replay log.

The same buffer is the resource `herdr://events/recent`. Events are not
pushed to the client: re-attach or call the tool to refresh.
