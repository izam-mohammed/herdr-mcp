# herdr socket API

Fetched: 22/10/2026 from `herdr api schema --json` (herdr 0.9.5, bundled with
the binary), `herdr --skill`, the socket API page
(https://raw.githubusercontent.com/herdrdev/herdr/v0.9.5/docs/next/website/src/content/docs/socket-api.mdx),
and read-only calls against the live socket on this machine. The full schema is 272 KB; regenerate it with
`herdr api schema --json` rather than copying it here.
Version: herdr 0.9.5, protocol 22, API schema version 1. Re-checked 20/11/2026: `herdr --version` is still 0.9.5 and `ping` on an isolated session answers protocol 22.
`nc -U -w 2 <socket>` works for a one-shot request on this machine (OpenBSD netcat); `-q` is also accepted.

## Authentication

None. The socket is a Unix domain socket created with mode `srw-------`, so
only the owning user can connect. No token, no env var holds a secret.

## Finding the socket

In herdr's own order:

1. an explicit session name: `~/.config/herdr/sessions/<name>/herdr.sock`;
2. `HERDR_SOCKET_PATH` (set inside every herdr pane, e.g.
   `~/.config/herdr/herdr.sock`);
3. `HERDR_SESSION=<name>`, giving the sessions path above;
4. the default, `~/.config/herdr/herdr.sock`.

Inside a herdr pane these are also set: `HERDR_ENV=1`, `HERDR_PANE_ID`
(e.g. `wA:p1`), `HERDR_TAB_ID` (`wA:t1`), `HERDR_WORKSPACE_ID` (`wA`).
Per `herdr --skill`: when `HERDR_ENV` is not `1`, a client must not inspect or
control the focused session from outside herdr.

## Wire format

Newline-delimited JSON. Request: `{"id":"<string>","method":"<name>","params":{...}}`.

- Success: `{"id":"<same>","result":{"type":"<result type>", ...}}`.
- Error: `{"id":"<same>","error":{"code":"<string>","message":"<string>"}}`.

The docs say "Send one request per line" and recommend the CLI wrappers
for most automation, the raw socket "when you need direct request/response
control or long-lived event subscriptions". JSON clients should "ignore
unknown fields and handle unsupported methods as normal errors".

**One request per connection.** Not stated in the docs; measured 22/10/2026: two requests written on
one connection get one response; the second is never answered. A client
opens a connection per call. `events.subscribe` is the exception: the same
connection then streams events until it is closed.

Measured cost of connect + request + response on this machine (100 calls
each, new connection per call): `ping` p50 0.14 ms, `tab.list` 0.18 ms,
`agent.list` 0.26 ms, `session.snapshot` 0.41 ms (3.7 KB with 3 workspaces).

## What this project calls

Method names and params are from the schema. Ids are strings like `wA`
(workspace), `wA:t1` (tab), `wA:p1` (pane). Per the docs, a same-tab
`pane.swap` "preserves ... pane ids", but a move to another workspace
assigns "a new public pane id in the destination workspace"; nothing is
stated about ids after closes. A pane's `terminal_id` (`term_...`) is the
handle that does not encode its place. Agent `target` accepts a pane id
or an agent name.

### Session

| Method | Params | Result `type` |
| --- | --- | --- |
| `ping` | `{}` | `pong`: `version`, `protocol`, `capabilities` |
| `session.snapshot` | `{}` | `session_snapshot`: `snapshot.{version, protocol, focused_workspace_id, focused_tab_id, focused_pane_id, workspaces[], tabs[], panes[], layouts[], agents[]}` |
| `pane.current` | `caller_pane_id?` | `pane_current`: `pane.{pane_id, workspace_id, tab_id, focused, cwd, agent, agent_status, ...}` |

Example, live:

```json
{"id":"x1","method":"ping","params":{}}
{"id":"x1","result":{"type":"pong","version":"0.9.5","protocol":22,"capabilities":{"live_handoff":true,"detached_server_daemon":true,"endpoint_protocol_generation":1,"surface_interest":true,"health_check":true,"ssh_agent_registration":true}}}
```

### Workspaces

| Method | Params |
| --- | --- |
| `workspace.list` | `{}` |
| `workspace.get`, `workspace.focus` | `workspace_id` |
| `workspace.create` | `label?`, `cwd?`, `env?` (string map), `focus` (default false), `source_workspace_id?` |
| `workspace.rename` | `workspace_id`, `label` |
| `workspace.move` | `workspace_id`, `insert_index` |
| `workspace.report_metadata` | `workspace_id`, `source`, `tokens` (max 16 keys, `^[A-Za-z0-9_-]{1,32}$`), `ttl_ms?` (1 to 86 400 000), `seq?` |
| `workspace.close` | `workspace_id`, `close_group?` |

### Tabs

| Method | Params |
| --- | --- |
| `tab.list` | `workspace_id?` |
| `tab.get`, `tab.focus`, `tab.close` | `tab_id` |
| `tab.create` | `workspace_id?`, `label?`, `cwd?`, `env?`, `focus` (default false) |
| `tab.rename` | `tab_id`, `label` |
| `tab.move` | `tab_id`, `insert_index` |

Live `tab.list` result: `{"type":"tab_list","tabs":[{"tab_id":"w8:t1","workspace_id":"w8","number":1,"label":"1","focused":false,"pane_count":1,"agent_status":"idle"}]}`.

### Panes

| Method | Params |
| --- | --- |
| `pane.list` | `workspace_id?` |
| `pane.get`, `pane.focus`, `pane.clear`, `pane.close` | `pane_id` |
| `pane.read` | `pane_id`, `source` (`visible`, `recent`, `recent_unwrapped`, `detection`), `lines?`, `format` (`text`, `ansi`), `strip_ansi` (default true) |
| `pane.split` | `direction` (`right`, `down`), `target_pane_id?`, `workspace_id?`, `ratio?`, `cwd?`, `env?`, `focus` |
| `pane.send_text` | `pane_id`, `text` |
| `pane.send_keys` | `pane_id`, `keys[]` |
| `pane.send_input` | `pane_id`, `text?`, `keys[]?` (text then keys, one request) |
| `pane.wait_for_output` | `pane_id`, `source`, `match` (`{"type":"substring"|"regex","value"}`), `lines?`, `timeout_ms?`, `strip_ansi` |
| `pane.layout`, `pane.edges` | `pane_id?` |
| `pane.neighbor`, `pane.focus_direction`, `pane.resize` | `direction`, `pane_id?` (resize also `amount?`) |
| `pane.zoom` | `pane_id?`, `mode` (default `toggle`) |
| `pane.swap` | `pane_id?`, `source_pane_id?`, `target_pane_id?`, `direction?` |
| `pane.move` | `pane_id`, `destination` (`{"type":"tab","tab_id","target_pane_id?","split":"right"\|"down"}`, `{"type":"new_tab","workspace_id","label"}` or `{"type":"new_workspace","label","tab_label"}`), `focus` |
| `pane.rename` | `pane_id`, `label?` |
| `pane.process_info` | per schema `PaneProcessInfoParams` |

### Agents

States: `idle`, `working`, `blocked`, `done`, `unknown`.

| Method | Params |
| --- | --- |
| `agent.list` | `{}` |
| `agent.get`, `agent.explain`, `agent.focus` | `target` |
| `agent.read` | `target`, `source`, `lines?`, `format`, `strip_ansi` |
| `agent.prompt` | `target`, `text`, `wait?` (`until[]`, `timeout_ms?`) |
| `agent.wait` | `target`, `until[]`, `timeout_ms?` |
| `agent.send_keys` | `target`, `keys[]` |
| `agent.start` | `name`, `kind`, `pane_id`, `args[]?`, `timeout_ms?` (over 3 000, at most 300 000) |
| `agent.rename` | `target`, `name?` |

`agent.prompt` with `wait`, per `herdr --skill`: a prompt sent from a
non-working state must produce `working` or `blocked` within five seconds,
else `agent_prompt_stalled`; `timeout` if the caller's timeout expires
first. Without a timeout the settled-state wait is indefinite.

### Worktrees, layout, notifications

| Method | Params |
| --- | --- |
| `worktree.list` | per `WorktreeListParams` |
| `worktree.create` | `workspace_id?`, `cwd?`, `branch?`, `base?`, `path?`, `label?`, `focus`, `trust_repository?` |
| `worktree.open` | per `WorktreeOpenParams` |
| `worktree.remove` | `workspace_id`, `force` (default false), `trust_repository?` |
| `layout.export` | `tab_id?`, `pane_id?` |
| `layout.apply` | `root` (a `LayoutNode`), `tab_id?`, `workspace_id?`, `tab_label?`, `focus` |
| `layout.set_split_ratio` | per `LayoutSetSplitRatioParams` |
| `notification.show` | `title`, `body?`, `position?`, `sound?` |

### Events

`events.subscribe` takes `subscriptions[]`, each `{"type":"<dotted name>"}`.
Reply: `{"id":"sub","result":{"type":"subscription_started"}}`, then one
envelope per line, `{"event":"<snake name>","data":{...}}`.

Lifecycle types need only `type`: `workspace.{created, updated,
metadata_updated, renamed, moved, reordered, closed, focused}`,
`worktree.{created, opened, removed}`, `tab.{created, closed, focused,
renamed, moved}`, `pane.{created, closed, updated, focused, moved, exited,
agent_detected}`, `layout.updated`.

**Pane-scoped types need a `pane_id`** (measured: omitting it returns
`invalid_request: missing field pane_id`):

- `pane.agent_status_changed`: `pane_id`, `agent_status?`
- `pane.scroll_changed`: `pane_id`
- `pane.output_matched`: `pane_id`, `source`, `match`, `lines?`, `strip_ansi?`

So "any agent goes blocked" needs one `pane.agent_status_changed` entry per
pane, and a new subscription when panes are created.

`events.wait` takes `match_event` (an `EventMatch`, e.g.
`{"event":"tab_closed","tab_id":"w1:t2"}`) and `timeout_ms?`, and returns
once.

## Errors

The schema types the code as a free string; the docs give no complete
list. Codes named in the docs, the skill and live calls: `not_found`,
`invalid_request`, `invalid_params`, `agent_blocked`,
`agent_prompt_stalled`, `timeout`, `events_lost`,
`workspace_group_close_required`, `platform_unsupported`,
`plugin_disabled`, `popup_not_open`. Pass any other code through
unchanged.

## Limits

- Subscriptions do not replay history.
- On `events_lost` (the subscriber fell behind), herdr closes the
  subscription; the client resubscribes and refreshes from
  `session.snapshot`.
- If any pane in a subscription does not exist, herdr rejects the whole
  request and closes the connection.
- "Ordering is guaranteed within each subscription entry, not across
  entries or relative to a snapshot." Events "invalidate cached state";
  "serialize refreshes and refresh again if events arrive while a read is
  in flight".
- Rate limits, maximum request size, maximum `lines`: not stated.

## Gotchas

- Isolated test servers: `herdr --session <name> server` runs a headless
  server at `~/.config/herdr/sessions/<name>/herdr.sock`, which starts with
  no workspaces. Stop it with `herdr --session <name> server stop`, then
  `herdr session delete <name>`. Verified 22/10/2026. Unset `HERDR_*` vars
  first, or the CLI targets the inherited session.
- Never run `herdr server stop` against the live session: it kills every
  pane process.
- Pane input (measured live 20/11/2026, herdr 0.9.5): `pane.send_input` with
  `{"text":"echo hi","keys":["enter"]}` types and submits in one request;
  the key name is `enter` (`Enter` is accepted too). `pane.read` returns
  `result.read.{text, source, truncated, ...}`; `pane.wait_for_output`
  matches text already on screen (so the typed command line itself can
  match), returns `output_matched` with `matched_line` and `read.text`, and a
  miss is the error `{"code":"timeout"}`; a bad regex is `invalid_regex`.
  `pane.swap` needs `source_pane_id` (with `target_pane_id` or `direction`);
  `pane_id` alone fails with `invalid_pane_swap`. `pane.move` returns
  `result.move_result.pane`. `pane.edges` and `pane.neighbor` embed the whole
  layout in their replies.
- Pane and agent output is arbitrary terminal text from another program.
- `workspace.close` without `close_group` on a worktree group returns
  `workspace_group_close_required`; never add `close_group` just to get past
  it.
- Worktrees, layout, notifications (measured live 20/11/2026, herdr 0.9.5):
  `worktree.create` and `worktree.remove` succeed without `trust_repository`
  on a repository with no setup scripts; the docs do not say what it unlocks,
  so herdr-mcp sends it only when asked. Without `path` the checkout lands in
  `~/.herdr/worktrees/<repo>/<branch>`. `layout.apply` without `tab_id` opens a
  new tab; with `tab_id` it replaces that tab and the result has a new tab id.
  `layout.export` gives `layout.root`, which is valid as `root` for apply.
  `layout.set_split_ratio` takes `path` (booleans, `[]` is the root split) and
  `ratio`. `notification.show` with no attached client returns
  `shown: false, reason: no_foreground_client`, not an error.
