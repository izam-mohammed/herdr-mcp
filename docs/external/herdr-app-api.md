# herdr-app: the desktop window's API

Fetched: 21/11/2026 from `herdr-app --help`, `herdr-app --skill`,
`herdr-app commands` (75 commands), `herdr-app ls`, `herdr-app status` and
`~/.config/herdr-app/AGENTS.md`, herdr-app 0.1.16 on top of herdr-core
0.1.21. Regenerate the command list with `herdr-app commands`.

## Authentication

None. The window listens on `~/.config/herdr-app/herdr-app.sock` (mode
`srwxrwxr-x`, the directory is the user's). `HERDR_APP_HOME` picks another
app home and so another running window. Never stop, signal or restart the
person's app or core (its own skill says so).

## What it holds

Spaces (with `folder` and `machine`), each with tabs, each tab with
terminals and a `pane` id. `herdr-app ls` prints them as JSON:
`spaces[].{id,name,folder,machine,active,tabs[].{id,name,active,settled,
terminals[].{terminal,pane,status,app}}}`. Ids look like `sp...`, `tb...`,
`p...`; terminal addresses (`t6`) are herdr-core's.

## Commands (75), by use

- spaces: `space.new {name?,folder?,tab?,argv?,task?,machine?,focus?}`,
  `space.close {id?,confirmed?}`, `space.rename`, `space.move`,
  `space.select`, `space.next`, `space.prev`.
- tabs: `tab.new`, `tab.close`, `tab.rename`, `tab.move`, `tab.select`,
  `tab.settle`, `tab.unsettle`, `tab.next`, `tab.prev`.
- panes and terminals: `pane.split {direction,pane?}`, `pane.close`,
  `pane.zoom`, `pane.focus`, `pane.go`, `pane.swap`, `terminal.open`,
  `terminal.peek`, `terminal.show`, `terminal.scroll`, `terminal.paste`,
  `terminal.search`, `terminal.kill`, `terminal.keep`, `terminal.park`.
- agents: `agent.focus`, `agent.next`, `agent.prev`.
- window and look: `layout.*`, `mod.*`, `overlay.close`, `app.*`,
  `window.*`. These change what the person sees; not part of the MCP.

`herdr-app do <command> '<json>'` runs one. Unknown arguments fail and list
the valid ones. The reply is the created ids or `{ "error": ... }`, exit 1.
`space.close` asks the person unless `confirmed` is passed.

Measured 21/11/2026: `herdr-app ls` and `herdr-app status` each took
4 to 5 ms as a subprocess.

## Not found yet

The app socket's wire protocol. Only the `herdr-app` CLI is documented, so
the plan's proof phase decides between the CLI and the socket
(`plans/16-herdr-app-backend.md` decision 2).
