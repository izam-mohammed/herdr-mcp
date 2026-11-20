# Running and waiting on commands

## Run something and wait for it

`herdr_pane_run` types a command into a pane and presses enter. With no
`pane_id` it splits a new pane to the right (or `direction`) first. With
`wait`, it also waits for output that matches:

```json
{
  "command": "echo hello-from-herdr-mcp",
  "wait": { "match": { "type": "regex", "value": "^hello-from-herdr-mcp$" }, "timeout_ms": 5000 }
}
```

The reply holds the new pane, `input: ok` and a `matched` block: the matched
line, the pane id and the text read at that point.

```json
{
  "type": "pane_run",
  "pane_id": "w1:p2",
  "matched": { "type": "output_matched", "matched_line": "hello-from-herdr-mcp", "pane_id": "w1:p2" }
}
```

(Trimmed. The real result also carries the created pane and the text read.)

### Match what the command prints, not what you typed

A wait matches text already on screen, including the command line you just
typed. Waiting for `build-42-done` after typing `echo build-$((6*7))-done`
works because the typed line spells it differently; waiting for the same
words you typed would match at once. Pick something the command prints but
does not contain, or anchor a regex (`^...$`) so the echoed command line
cannot satisfy it.

## When it does not match

Running out of time is a normal result, not an error:

```json
{ "type": "output_wait_timed_out", "pane_id": "w1:p1", "timed_out": true, "timeout_ms": 1500 }
```

The default is 60 seconds and the most is 10 minutes. After a timeout, read
the pane to see where it got to.

## Read the result

`herdr_pane_read` returns the pane's text. By default 200 lines, at most
2000 and 40000 characters; the end is kept, with `truncated: true`. The
`source` option picks what is read: `visible`, `recent`, `recent_unwrapped`
(the default, best for logs) or `detection`.

## Wait for something that is already running

`herdr_pane_wait_output` waits on a pane without sending anything. For
"tell me when any agent goes blocked", use `herdr_events_wait` instead (see
[Events](events.md)).

The `/mcp__herdr__run-and-watch` prompt does all of this: it splits a pane,
runs your command, waits, reads and reports. Typing tools belong on "ask":
see [Safety](../safety.md).
