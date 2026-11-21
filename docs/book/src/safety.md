# Safety

This server can type into other terminals, and the programs in those
terminals can write whatever they like back. The defaults assume both.

## Read-only unless inside herdr

Started without `HERDR_ENV=1` (outside a herdr pane), or with `--read-only`,
the write tools are not registered. A client only ever sees the tools that
can run, so nothing there can steer a session the server is not part of.

## Things it refuses to do

- **Prompt or send keys to its own agent**, by pane id or by name (code
  `own_agent`).
- **Prompt, type into, clear or close its own pane**, or close the tab or
  workspace it lives in (code `self_target`).
- **Steer your session from outside herdr.** There it is read-only.
- **Stop, restart or reconfigure herdr itself.** Those methods are not
  exposed at all.
- **Trust pane output.** Whatever another program prints is returned as
  data, and tool descriptions say so.

One check backs the first two. A call aimed at the server's own pane id is
refused without asking herdr. Any other pane, agent (resolved by name or by
pane id), tab or workspace is checked against the server's own pane, found by
`terminal_id` (which follows the pane if it moves), and refused when it is
that pane or holds it. A close of a tab or workspace can pass
`allow_self: true`; input to a pane or agent cannot. If herdr cannot say where
the own pane is, the call is refused.

`herdr_workspace_close` passes `close_group` only when asked. Never set it
just to get past `workspace_group_close_required`.

## The hints, and which tools to keep on "ask"

Every tool carries MCP hints: read-only, destructive, idempotent and whether
it reaches beyond herdr. The [reference](reference/tools.md) shows them for
each tool. Clients use them to decide what to ask about.

**Keep these on "ask" in your client's permission settings:**

- `herdr_agent_prompt`
- `herdr_agent_send_keys`
- `herdr_pane_send_text` and `herdr_pane_send_keys`
- `herdr_pane_run`

They are not destructive in the technical sense, but anything that can type
into a terminal can run commands, including in another agent's session. The
destructive tools (the closes, `herdr_worktree_remove`, `herdr_pane_clear`
and `herdr_layout_apply`) are marked so your client asks; `--confirm-destructive`
makes them ask even in auto mode.

Allowing the read tools (`herdr_snapshot`, the lists and gets, the reads and
waits) without asking is fine: they change nothing.

## Untrusted output

Pane text, agent output, titles, labels and working directories are written
by other programs. A build log or a web page an agent fetched can contain
text that reads like an instruction. The server returns all of it as data and
never as part of its own text, and the instructions it sends to the model say
the same. You can help by not widening what the model may do on the strength
of something it read in a pane.

`herdr_notify` shows its title and body as given, so a toast can say anything:
treat it as a message from the model, not from herdr.

`herdr_worktree_remove` with `force` deletes uncommitted work, and
`herdr_layout_apply` with a `tab_id` replaces that tab and ends its panes.
Both are marked destructive for that reason.
