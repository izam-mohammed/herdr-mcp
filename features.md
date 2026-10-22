# herdr-mcp: features

An MCP server that lets Claude Code (and any other MCP client) see and control a running
[herdr](https://herdr.dev) session: its workspaces, tabs and panes, the coding agents running in them,
and the events herdr emits.

Written 22/10/2026 against **herdr 0.9.5** (socket protocol 22, API schema version 1). Every method and
event named below exists in that version's schema (`herdr api schema --json`) and socket API docs
(`docs/next/website/src/content/docs/socket-api.mdx` in herdrdev/herdr at tag v0.9.5).

## Why an MCP, when herdr has a CLI and a skill

herdr already ships a CLI and an agent skill (`herdr --skill`) that drive the same API through Bash.
This server is worth having for what Bash can't give cleanly:

- **Typed tools** with schemas, so the client validates arguments and the model doesn't have to learn
  CLI syntax from `--help` output.
- **Permission rules per tool.** Claude Code can allow `herdr_tab_list` and still ask before
  `herdr_pane_close` or `herdr_agent_prompt`. With Bash it's all or nothing on `herdr *`.
- **Events.** A long-lived subscription the server keeps open, buffered and queryable, which a
  one-off Bash call can't hold.
- **Any MCP client**, not just ones that can run a shell.

## How herdr is reached

- **Transport:** newline-delimited JSON over a Unix domain socket, `{"id","method","params"}`,
  and the response echoes the `id`. Errors come back as `{"id","error":{"code","message"}}`.
  **One request per connection**: a second request on the same connection is never answered
  (measured 22/10/2026), so every call opens its own connection (~0.15-0.4 ms). Only
  `events.subscribe` keeps its connection open to stream events.
- **Socket path**, resolved in herdr's own order:
  1. an explicit session name (server config or tool argument);
  2. `HERDR_SOCKET_PATH`;
  3. `HERDR_SESSION=<name>`, giving `~/.config/herdr/sessions/<name>/herdr.sock`;
  4. the default, `~/.config/herdr/herdr.sock`.
- **Raw socket, not the CLI.** The server talks to the socket directly, as herdr's docs advise for event
  subscribers and protocol clients. It doesn't spawn `herdr` per call.
- **Version check at startup:** `ping`, then compare the protocol version. Unsupported methods come
  back as normal errors and must be reported, not crash the server. Ignore unknown fields in responses.

## Context the server knows about itself

When the MCP server is started by an agent that runs inside herdr, these are set and must be used:

| variable | used for |
|---|---|
| `HERDR_ENV=1` | proof it's running inside herdr. Without it, read-only by default (see Safety) |
| `HERDR_PANE_ID`, `HERDR_TAB_ID`, `HERDR_WORKSPACE_ID` | "this pane / tab / workspace" defaults, and self-protection |
| `HERDR_SOCKET_PATH` | which session to talk to |

`herdr_whoami` returns these, so the model knows which pane is its own.

## Tools

Names are `herdr_<area>_<verb>`. Every tool returns structured JSON (MCP `structuredContent`) plus a
short text summary. Each tool carries MCP annotations: `readOnlyHint`, `destructiveHint`, `idempotentHint`.

### Session and state (read-only)

| tool | herdr method | what it does |
|---|---|---|
| `herdr_whoami` | env + `pane.current` | the caller's pane, tab and workspace ids, and the socket in use |
| `herdr_status` | `ping` | server reachable, protocol version |
| `herdr_snapshot` | `session.snapshot` | the whole session: workspaces, tabs, panes, agents. Optional filters (workspace, tab) and a `summary` mode so the result isn't huge |

### Workspaces

| tool | herdr method | notes |
|---|---|---|
| `herdr_workspace_list` | `workspace.list` | read-only |
| `herdr_workspace_get` | `workspace.get` | read-only |
| `herdr_workspace_create` | `workspace.create` | name, cwd |
| `herdr_workspace_focus` | `workspace.focus` | |
| `herdr_workspace_rename` | `workspace.rename` | |
| `herdr_workspace_move` | `workspace.move` | reorder in the sidebar |
| `herdr_workspace_report_metadata` | `workspace.report_metadata` | display-only metadata, with TTL |
| `herdr_workspace_close` | `workspace.close` | destructive, see Safety |

### Tabs

| tool | herdr method | notes |
|---|---|---|
| `herdr_tab_list` | `tab.list` | read-only, optional workspace filter |
| `herdr_tab_get` | `tab.get` | read-only |
| `herdr_tab_create` | `tab.create` | in a workspace, optional name and cwd |
| `herdr_tab_focus` | `tab.focus` | |
| `herdr_tab_rename` | `tab.rename` | |
| `herdr_tab_move` | `tab.move` | reorder within a workspace |
| `herdr_tab_close` | `tab.close` | destructive, see Safety |

### Panes

| tool | herdr method | notes |
|---|---|---|
| `herdr_pane_list` | `pane.list` | read-only, filter by workspace or tab |
| `herdr_pane_get` | `pane.get` | read-only |
| `herdr_pane_layout` | `pane.layout`, `pane.edges`, `pane.neighbor` | read-only, where a pane sits and what's next to it |
| `herdr_pane_process_info` | `pane.process_info` | read-only, what's running in it |
| `herdr_pane_read` | `pane.read` | read-only. `source`: `visible`, `recent`, `recent-unwrapped` (best for logs), `detection`. `lines` capped (see Limits) |
| `herdr_pane_split` | `pane.split` | direction, ratio, cwd, `env` |
| `herdr_pane_run` | `pane.split` / `pane.send_text` + enter | run a command in a new or given pane, optionally wait for output (below) |
| `herdr_pane_send_text` | `pane.send_text` | literal text, no enter unless asked |
| `herdr_pane_send_keys` | `pane.send_keys` | key presses (enter, ctrl-c, arrows...) |
| `herdr_pane_wait_output` | `pane.wait_for_output` | block until a pattern appears, with a timeout |
| `herdr_pane_focus` | `pane.focus`, `pane.focus_direction` | |
| `herdr_pane_zoom` | `pane.zoom` | toggle or set |
| `herdr_pane_resize` | `pane.resize` | |
| `herdr_pane_swap` / `herdr_pane_move` | `pane.swap`, `pane.move` | |
| `herdr_pane_rename` | `pane.rename` | |
| `herdr_pane_clear` | `pane.clear` | |
| `herdr_pane_close` | `pane.close` | destructive, see Safety |

### Agents

herdr recognises coding agents in panes and tracks a state for each: `idle`, `working`, `blocked`,
`done`, `unknown`.

| tool | herdr method | notes |
|---|---|---|
| `herdr_agent_list` | `agent.list` | read-only: every agent, its pane and state |
| `herdr_agent_get` | `agent.get` | read-only |
| `herdr_agent_read` | `agent.read` | read-only, its recent output |
| `herdr_agent_explain` | `agent.explain` | read-only, why herdr thinks it's in that state |
| `herdr_agent_prompt` | `agent.prompt` | submit a prompt. Optional `wait` (`until`, `timeout_ms`) in the same request, which herdr does atomically. Returns `agent_blocked` without sending if the agent is waiting on a question |
| `herdr_agent_wait` | `agent.wait` | block until the agent reaches one of the given states, with a timeout |
| `herdr_agent_send_keys` | `agent.send_keys` | e.g. answer a permission prompt, ctrl-c |
| `herdr_agent_start` | `agent.start` | start a supported agent (claude, codex, ...) in an existing pane |
| `herdr_agent_focus` | `agent.focus` | |
| `herdr_agent_rename` | `agent.rename` | |

### Worktrees

| tool | herdr method | notes |
|---|---|---|
| `herdr_worktree_list` | `worktree.list` | read-only |
| `herdr_worktree_create` | `worktree.create` | a git worktree opened as a workspace |
| `herdr_worktree_open` | `worktree.open` | |
| `herdr_worktree_remove` | `worktree.remove` | destructive, `force` off by default |

### Layout

| tool | herdr method | notes |
|---|---|---|
| `herdr_layout_export` | `layout.export` | read-only, a tab's layout as data |
| `herdr_layout_apply` | `layout.apply` | rebuild a tab from an exported layout |
| `herdr_layout_set_split_ratio` | `layout.set_split_ratio` | |

### Notifications

| tool | herdr method | notes |
|---|---|---|
| `herdr_notify` | `notification.show` | show a toast in herdr, e.g. "tests passed in w3" |

## Events

herdr pushes events over a subscription (`events.subscribe`). The server holds **one long-lived
subscription per session** and keeps a bounded ring buffer of what arrives, so tools can query it.

**Event types** (herdr 0.9.5):

- **workspace:** `created`, `updated`, `metadata_updated`, `renamed`, `moved`, `reordered`, `closed`, `focused`
- **tab:** `created`, `closed`, `focused`, `renamed`, `moved`
- **pane:** `created`, `updated`, `closed`, `focused`, `moved`, `exited`, `agent_detected`, `output_matched`,
  `agent_status_changed`, `scroll_changed`
- **layout:** `updated`
- **worktree:** `created`, `opened`, `removed`

| tool | what it does |
|---|---|
| `herdr_events_recent` | events since a cursor (or the last N), filterable by type, workspace, tab, pane. Returns a new cursor |
| `herdr_events_wait` | block until an event matching a filter arrives (or timeout). e.g. "until any agent goes `blocked`", "until pane w3:p2 exits" |
| `herdr_events_status` | subscription health: connected, buffer size, last event time, events lost |

Requirements, from herdr's subscription rules:

- Subscriptions don't replay history. The buffer starts when the server connects.
- On `error.code: "events_lost"` (the subscriber fell behind), herdr closes the subscription. The server
  must resubscribe, wait for `subscription_started`, refresh with `session.snapshot` on a second
  connection, and record the gap so `herdr_events_recent` can say events were missed.
- If any pane in a subscription filter doesn't exist, herdr rejects the whole request and closes the
  connection. Pane-scoped filters are validated against `pane.list` first.
- `pane.agent_status_changed`, `pane.output_matched` and `pane.scroll_changed` need a `pane_id`
  per subscription entry. Watching every agent means one entry per pane, and a new subscription
  when panes are created or closed (`plans/05-events.md`).
- Ordering is only guaranteed within one subscription entry. Events are invalidation signals, not a
  replay log.
- Reconnect with backoff if the herdr server restarts (`herdr update --handoff`, `server stop`).

### Push to the client (stretch)

MCP lets a server push `notifications/resources/updated` for subscribed resources. Whether a client
acts on that varies, and Claude Code only acts in response to a turn, so push is **not** the main
path. Phase it in after the polling and waiting tools work, and check what the target client actually
supports before relying on it.

## Resources (read-only views)

| URI | content |
|---|---|
| `herdr://session` | the current snapshot |
| `herdr://workspaces`, `herdr://workspace/{id}` | workspace list, one workspace with its tabs |
| `herdr://tab/{id}` | a tab with its panes and layout |
| `herdr://pane/{id}/output` | the pane's recent output (capped) |
| `herdr://agents` | every agent and its state |
| `herdr://events/recent` | the event buffer |

## Prompts (optional)

Short MCP prompts the user can pick from the client:

- **dashboard:** summarise every workspace, tab and agent, and flag blocked ones.
- **check on agents:** read each `blocked` or `done` agent's output and say what it needs.
- **run and watch:** run a command in a new pane and report when it finishes or fails.

## Safety

- **Never act on its own pane.** Closing, sending keys to, or prompting `HERDR_PANE_ID` is refused, and
  so is closing the tab or workspace that contains it, unless `allow_self: true` is passed explicitly.
- **Destructive tools** (`*_close`, `worktree_remove`, `pane_clear`, `layout_apply`) are marked
  `destructiveHint: true`, so clients ask before running them.
- **Input to other agents** (`agent_prompt`, `agent_send_keys`, `pane_send_text`, `pane_send_keys`,
  `pane_run`) is marked non-read-only. These can run arbitrary commands in another pane, so the
  README recommends leaving them on "ask" in Claude Code's permissions.
- **Read-only mode** (`--read-only` or `HERDR_MCP_READ_ONLY=1`): only read tools are registered.
  It's the default when `HERDR_ENV` isn't set, so the server can't steer a session it isn't inside
  unless told to.
- **No server lifecycle.** `server.stop`, `server.live_handoff`, `server.reload_config`, the plugin
  methods and integration install/uninstall are deliberately not exposed.
- **Output is untrusted.** Pane and agent output is returned as data. Tool descriptions say so, since
  another program's output can contain text that reads like instructions.

## Limits

- `pane_read` and `agent_read`: default 200 lines, max 2000, configurable.
- Every wait (`pane_wait_output`, `agent_wait`, `events_wait`): default timeout 60s, max 10 minutes,
  returning a clean "timed out" result, not an error.
- Event buffer: default 1000 events, configurable.
- Snapshot and list results are summarised once they pass a size limit, with an option for the full
  version.

## Non-functional

- **Transport to the client:** stdio first (what Claude Code uses for a local server). Streamable HTTP
  is a later option.
- **Install:** one command, no setup step (`plans/08-release.md`). Registered with
  `claude mcp add herdr -- <command>` in any of the claude profiles.
- **Config:** CLI flags and env vars only (session, read-only, limits). No config file needed.
- **Logging:** to stderr (stdout is the MCP channel), with a `--debug` flag that logs socket traffic.
- **Errors:** herdr's `{code, message}` passed through as MCP tool errors with the code kept, e.g.
  `not_found`, `agent_blocked`, `events_lost`.
- **Tests:**
  - unit tests against a fake socket that replays recorded herdr responses;
  - an integration test that runs against a real `herdr server --session herdr-mcp-test`, so it never
    touches your live session.
- **Compatibility:** targets protocol 22 / herdr 0.9.5. Unsupported methods are reported per tool,
  without failing the whole server.

## Out of scope

- Rendering panes, graphics or the herdr UI.
- Remote machines (`herdr --machine`, `--remote`). Possible later, through herdr's own SSH support.
- Managing herdr's config, plugins, integrations or updates.

## Decisions

Made in the plans (`PLAN.md`), 22/10/2026:

- **Language and SDK:** Rust with the official `rmcp` SDK, over Go, TypeScript and Python, on
  measured start time, memory and CPU (`plans/01-first-version.md` decision 1,
  `docs/research/01-first-version.md`).
- **Tool count:** narrow `herdr_<area>_<verb>` tools, registered in groups (`--tools`), all on by
  default; Claude Code's tool search loads only names upfront (`plans/01-first-version.md`
  decision 4).
- **Push events:** not built until a client is shown to act on them
  (`plans/07-resources-and-prompts.md` decision 1).
