# Configuration

Flags and environment variables only; there is no config file. A flag wins
over its environment variable. The full list is on the
[command line](reference/cli.md) page.

## Which herdr it talks to

The socket is chosen in this order, first match wins:

1. `--socket <path>`
2. `--session <name>`, which means `~/.config/herdr/sessions/<name>/herdr.sock`
3. `HERDR_SOCKET_PATH`
4. `HERDR_SESSION=<name>`, the same session path as 2
5. the default, `~/.config/herdr/herdr.sock`

Inside a herdr pane, herdr sets `HERDR_SOCKET_PATH` for you, so the server
finds the right session without any flag. To drive another session, for
example a test one, pass `--session herdr-mcp-test`.

Every call opens its own connection and times out after 10 seconds
(`HERDR_MCP_TIMEOUT_MS` changes that, in milliseconds). The failure is the
error code `timeout`.

## Read-only mode

`--read-only` or `HERDR_MCP_READ_ONLY=1` registers the read tools only. The
client never sees a write tool, so there is nothing to ask about.

It is also the default whenever `HERDR_ENV` is not `1`, that is, when the
server was not started inside a herdr pane. `--allow-outside-herdr` (or
`HERDR_MCP_ALLOW_OUTSIDE=1`) lifts that. Use it only for a session you mean
to steer from outside, such as a test session.

## Choosing tool groups

`--tools session,tabs` or `HERDR_MCP_TOOLS=session,tabs` registers only the
named groups. The default is nine: `session`, `workspaces`, `tabs`,
`panes`, `agents`, `events`, `worktrees`, `layout`, `notify`. A tenth,
`admin`, is off until you name it: `--tools all,admin` (`all` is the nine
defaults). An unknown group name stops the server at start. Fewer groups means fewer tool names
in your client's context. The [tools reference](reference/tools.md) lists
which tool is in which group.

The `events` group also controls the event feed: the subscription to herdr
only starts with it, and `herdr://events/recent` is listed only with it.

## Destructive tools

`--confirm-destructive` (or `HERDR_MCP_CONFIRM_DESTRUCTIVE=1`) makes every
destructive tool ask you each time, even in a client's auto mode.

## The event buffer

`--event-buffer <n>` (`HERDR_MCP_EVENT_BUFFER`) sets how many events are
kept for `herdr_events_recent` and `herdr_events_wait`: default 1000, from 1
to 1000000. See [Events](guides/events.md).

## Debugging

`--debug` logs request and reply payloads to stderr. Without it, stderr
carries method names and ids only, never pane text. A debug log can contain
terminal output, so do not paste it into a bug report unread.

## Limits

| What | Default | Most |
| --- | --- | --- |
| Pane read (`herdr_pane_read`, the pane resource) | 200 lines | 2000 lines, 40000 characters |
| Agent read (`herdr_agent_read`) | 100 lines | 1000 lines |
| Any wait (`herdr_pane_wait_output`, `herdr_pane_run` with `wait`, `herdr_agent_wait`, `herdr_agent_prompt` with `wait`, `herdr_events_wait`) | 60 seconds | 10 minutes |

When a read passes the character cap, the end is kept (that is where a
command's result is) and the result says `truncated: true`. Running out of
time on a wait is a normal result with `timed_out: true`, not an error.

A wait also matches text already on screen, including the command you typed.
Match something the command prints but does not contain.
