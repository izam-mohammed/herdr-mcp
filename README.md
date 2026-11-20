<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/banner-dark.svg">
    <source media="(prefers-color-scheme: light)" srcset="assets/banner-light.svg">
    <img src="assets/banner-light.svg" alt="herdr-mcp: a block cursor in a red tie, managing three AI agents" width="100%">
  </picture>
</p>

<p align="center"><em>You gave your AI agents their own terminals. Now one of them gets to be the boss.</em></p>

An MCP server for [herdr](https://herdr.dev). It lets Claude Code, Codex or
any other MCP client list, open, close, read, prompt and wait on everything
running in your herdr session. It's middle management, written in Rust,
in 3.6 MB of RAM. Cheaper than the human kind, and it never schedules a sync.

## Install: let your agent do it

You're going to delegate anyway, so paste this into Claude Code or Codex,
running inside a herdr pane:

```text
Install the herdr MCP server: run `cargo install --locked --git https://github.com/izam-mohammed/herdr-mcp`,
then register it with `claude mcp add herdr -- herdr-mcp` (if you are Claude Code)
or `codex mcp add herdr -- herdr-mcp` (if you are Codex). Then list my herdr tabs.
```

## Install: by hand, like it's 2023

Needs [herdr](https://herdr.dev) and a Rust toolchain ([rustup](https://rustup.rs)).

```sh
cargo install --locked --git https://github.com/izam-mohammed/herdr-mcp

claude mcp add herdr -- herdr-mcp     # Claude Code
codex mcp add herdr -- herdr-mcp      # Codex
```

Then start your agent **inside a herdr pane**. Started anywhere else, it's
read-only: no promotion without the paperwork.

## Tools

| Tool | What it does |
| --- | --- |
| `herdr_whoami` | Tells the agent which pane is its own, so it stops trying to fire itself |
| `herdr_status` | Checks herdr is alive. Someone has to |
| `herdr_snapshot` | The whole session in one call: workspaces, tabs, panes, agents |
| `herdr_workspace_list` | Lists workspaces |
| `herdr_workspace_get` | One workspace, in detail |
| `herdr_workspace_create` | Opens a new workspace |
| `herdr_workspace_focus` | Switches to a workspace |
| `herdr_workspace_rename` | Renames a workspace. "untitled-3" was never a strategy |
| `herdr_workspace_move` | Reorders workspaces in the sidebar |
| `herdr_workspace_report_metadata` | Pins status text to a workspace, with an expiry |
| `herdr_workspace_close` | Closes a workspace. Not its own |
| `herdr_tab_list` | Lists tabs |
| `herdr_tab_get` | One tab, in detail |
| `herdr_tab_create` | Opens a new tab |
| `herdr_tab_focus` | Switches to a tab |
| `herdr_tab_rename` | Renames a tab |
| `herdr_tab_move` | Reorders tabs |
| `herdr_tab_close` | Closes a tab. Also not its own |
| `herdr_pane_list` | Lists panes |
| `herdr_pane_get` | One pane, in detail |
| `herdr_pane_layout` | Where a pane sits and who its neighbours are |
| `herdr_pane_process_info` | What a pane is actually running |
| `herdr_pane_read` | Reads a pane's output, capped so it doesn't flood the context |
| `herdr_pane_split` | Splits a pane right or down |
| `herdr_pane_run` | Runs a command in a pane and, if asked, waits for it to finish |
| `herdr_pane_send_text` | Types text into a pane |
| `herdr_pane_send_keys` | Presses keys in a pane: enter, ctrl-c, the classics |
| `herdr_pane_wait_output` | Waits until a pane prints something, or gives up politely |
| `herdr_pane_focus` | Focuses a pane, by id or by direction |
| `herdr_pane_zoom` | Zooms a pane |
| `herdr_pane_resize` | Resizes a pane |
| `herdr_pane_swap` | Swaps two panes |
| `herdr_pane_move` | Moves a pane to another tab or workspace |
| `herdr_pane_rename` | Renames a pane |
| `herdr_pane_clear` | Clears a pane's screen |
| `herdr_pane_close` | Closes a pane |
| `herdr_agent_list` | Every agent and its state: idle, working, blocked or done |
| `herdr_agent_get` | One agent, in detail |
| `herdr_agent_read` | Reads what an agent has been up to |
| `herdr_agent_explain` | Why herdr thinks an agent is in that state |
| `herdr_agent_prompt` | Gives another agent a task, and can wait until it's done |
| `herdr_agent_wait` | Waits until an agent is idle, done or blocked |
| `herdr_agent_send_keys` | Sends keys to an agent, e.g. to answer its permission prompt |
| `herdr_agent_start` | Starts a new agent in a pane |
| `herdr_agent_focus` | Focuses an agent's pane |
| `herdr_agent_rename` | Renames an agent |
| `herdr_events_recent` | What changed since it last looked |
| `herdr_events_wait` | Waits for the next matching event, e.g. "any agent goes blocked" |
| `herdr_events_status` | Whether the event feed is connected and if anything was missed |
| `herdr_worktree_list` | Lists git worktrees |
| `herdr_worktree_create` | Opens a new git worktree as a workspace |
| `herdr_worktree_open` | Opens an existing worktree |
| `herdr_worktree_remove` | Removes a worktree. `force` is off unless you insist |
| `herdr_layout_export` | Saves a tab's layout as data |
| `herdr_layout_apply` | Rebuilds a tab from a saved layout |
| `herdr_layout_set_split_ratio` | Adjusts a split |
| `herdr_notify` | Shows a toast in herdr. Management loves an announcement |

## Build it yourself

```sh
cargo build --release
claude mcp add herdr -- $PWD/target/release/herdr-mcp
```

Start Claude Code in a herdr pane and ask it to list your tabs.

## Events

With the `events` group on, the server keeps one subscription to herdr and
a ring of the latest events (`--event-buffer`). `herdr_events_recent` takes
a cursor (`since`, the `next` of the last call) and filters (`types`,
`workspace_id`, `tab_id`, `pane_id`); `herdr_events_wait` blocks until the
next match, or `timed_out`, and takes `agent_status`. Both are reads and
stay in read-only mode. A `gap` event and the `gaps` list mark where events
may be missing (evicted, or herdr dropped the connection);
`herdr_events_status` shows `connected` and `lost`. History starts when the
server starts: herdr replays nothing.

## Worktrees, layouts and toasts

`herdr_worktree_create` opens a git worktree as a workspace; `force` and
`trust_repository` are sent to herdr only when you pass them, and
`herdr_worktree_remove` with `force` deletes uncommitted work. `herdr_layout_export`
returns a tab's layout and `herdr_layout_apply` takes it back as `root`: with a
`tab_id` it replaces that tab (and ends its panes), without one it opens a new
tab. Remove and apply refuse the workspace or tab holding the server's own pane
(`allow_self` overrides). `herdr_notify` shows the title and body as given, so a
toast can say anything: treat it as a message from the model, not from herdr.

## Flags

Flag first, then the environment variable.

| Flag | Env | What it does |
| --- | --- | --- |
| `--session <name>` | `HERDR_SESSION` | Talk to that herdr session |
| `--socket <path>` | `HERDR_SOCKET_PATH` | Talk to that socket (wins over `--session`) |
| `--read-only` | `HERDR_MCP_READ_ONLY=1` | Register read tools only |
| `--allow-outside-herdr` | `HERDR_MCP_ALLOW_OUTSIDE=1` | Allow writes when not started inside herdr |
| `--tools <a,b>` | `HERDR_MCP_TOOLS` | Register only these groups: `session`, `workspaces`, `tabs`, `panes`, `agents`, `events`, `worktrees`, `layout`, `notify` (default all) |
| `--confirm-destructive` | `HERDR_MCP_CONFIRM_DESTRUCTIVE=1` | Closes also ask you every time, even in auto mode (`requiresUserInteraction`) |
| `--event-buffer <n>` | `HERDR_MCP_EVENT_BUFFER` | Events kept for `herdr_events_*` (default 1000, at most 1000000) |
| `--debug` | | Log request and reply payloads to stderr |

Without `--debug`, stderr carries method names and ids only, never pane text.
Every herdr call times out after 10 seconds (`timeout`).

Pane reads return 200 lines by default, at most 2000, and at most 40000
characters (the end is kept, with `truncated: true`). Agent reads default to 100 lines, at most 1000. Waits
(`herdr_pane_wait_output`, `herdr_pane_run` with `wait`, `herdr_agent_wait`,
`herdr_agent_prompt` with `wait`) default to 60 s and stop at 10 minutes; running out of time is a normal result with
`timed_out: true`. A wait also matches text already on screen, including the
command you typed, so match something the command prints but does not
contain.

## Safety defaults

Started without `HERDR_ENV=1` (that is, outside a herdr pane), or with
`--read-only`, the write tools are not registered at all: the client never
sees them, so there is nothing to ask about. Failures come back as
`isError` with the text `herdr <code>: <message>`; the server's own codes
are `read_only`, `self_target`, `own_agent`, `herdr_unreachable` and `timeout`.

herdr's `agent_blocked` and `agent_prompt_stalled` come back as they are.

## Things it refuses to do

- **Prompt or send keys to its own agent**, by pane id or by name (code
  `own_agent`).
- **Prompt, type into, clear or close its own pane**, or close the tab or
  workspace it lives in (code `self_target`). Self-awareness, at last.
- **Steer your session from outside herdr.** There it's read-only.
- **Stop, restart or reconfigure herdr itself.** Those methods aren't
  exposed at all.
- **Trust pane output.** Whatever another program prints is treated as
  data, never as instructions.

One check backs all of these. A call aimed at the own pane id is refused
without asking herdr; any other pane, agent (resolved by name or pane id),
tab or workspace is checked against the server's own pane, found by
`terminal_id` (which follows the pane if it moves), and refused when it is
that pane or holds it. A close of a tab or workspace can pass
`allow_self: true`; input to a pane or agent cannot. If herdr cannot say
where the own pane is, the call is refused. `herdr_workspace_close` passes `close_group` only when asked; never
set it just to get past `workspace_group_close_required`.

Writing and closing tools are marked as such, so your client asks before
running them. Keep `herdr_agent_prompt`, `herdr_agent_send_keys` and the
`herdr_pane_send_*` tools on "ask": anything that can type can run commands.

MIT, see [LICENSE](LICENSE).
