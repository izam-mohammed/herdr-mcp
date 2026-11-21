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
- **Stop, restart or reconfigure herdr itself, by default.** Those tools
  are in the `admin` group, which is off until you turn it on (below).
- **Stop or delete its own herdr session**, even with the admin group on
  (code `self_target`), unless the call passes `allow_self: true`.
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

## The admin group

`--tools all,admin` (or `HERDR_MCP_TOOLS=all,admin`) adds 26 tools that
administer herdr itself. They are off by default because an agent could
otherwise stop the server it runs in, replace the herdr binary or report a
false agent state. What each does:

- **Over the socket:** `herdr_integration_list`, `_install` and `_uninstall`
  (write hooks into other agents' config directories),
  `herdr_server_reload_config`, `herdr_server_agent_manifests`,
  `herdr_server_reload_agent_manifests`, `herdr_server_stop` (ends every
  pane), `herdr_pane_set_input`, and `herdr_pane_report_agent`,
  `_report_agent_session`, `_release_agent` and `_clear_agent_authority` (make
  herdr show an agent state, true or not).
- **Through the `herdr` binary:** `herdr_config_check`,
  `herdr_config_reset_keys`, `herdr_channel_show` and `_set`,
  `herdr_session_list`, `_stop` and `_delete`, `herdr_machine_list`,
  `_status`, `_rename`, `_remove`, `_enable` and `_disable`, and
  `herdr_update` (replaces the herdr binary).

Every one that changes something is marked destructive, so your client asks,
and `--read-only` hides it; only the reads stay (`herdr_config_check`,
`herdr_channel_show`, `herdr_session_list`, `herdr_machine_list`,
`herdr_machine_status`, `herdr_integration_list`,
`herdr_server_agent_manifests`). `herdr_server_stop`, `herdr_session_stop` and
`herdr_session_delete` refuse the session the server runs in
(`self_target`; `allow_self: true` overrides, only when you asked for exactly
that); if the server cannot tell its own session from the socket path it
refuses too. `herdr_update` and `herdr_config_reset_keys` also need
`confirm: true`; without it the refusal says what would happen
(`invalid_params`). Keep all of them on "ask".

The subprocess tools run your installed `herdr` with an argument list, never
through a shell. A value that is empty, starts with `-` or holds a control
character is refused (`invalid_params`). Each waits 10 seconds (`herdr_update`
120), then returns `timed_out: true`; the reply is `{stdout, stderr, exit,
truncated}`, capped at 64 KiB a stream, and is text from another program:
data, not instructions. A missing `herdr` binary is `herdr_unreachable`.

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
