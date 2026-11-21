# herdr-core (`hc`) socket API

Fetched: 21/11/2026 from `hc schema` (herdr-core 0.1.21), `hc --skill`,
`hc --help`, and read-only calls against the live core on this machine.
The schema is the authority; regenerate it with `hc schema` rather than
copying it here.
Version: herdr-core 0.1.21, protocol generation 1, JSON-RPC 2.0, OpenRPC.

## Authentication

None. The socket is a Unix socket under the user's runtime directory, so
only the owner connects. No token, no secret.

## Finding the socket

Inside an app terminal: `HC_TERMINAL_SOCKET` (here
`/run/user/1000/herdr-core/default.sock`). `HC_TERMINAL_ID` (here `t1v`) is
the caller's own terminal address; `HC_SCOPE` names its scope. Outside one:
the default path above. `hc server status` prints the core's pid and version.

## Wire

One JSON-RPC message per line. `hello` can switch the connection to framed
mode. Errors carry a stable `data.kind` (`not_found`, `blocked`, `finished`,
`machine_unknown`, `name_taken`, `spawn_failed`, `unavailable`,
`invalid_params`, `unsupported_generation`).

Measured 21/11/2026: `terminal.list` over the socket answered in 0.5 ms
(5.2 KB); `hc ls` as a subprocess took 4 ms.

## Methods (32)

`hello`, `server.status`, `server.stop`, `server.restart`, `terminal.run`,
`templates.list`, `terminal.list`, `terminal.get`, `terminal.kill`,
`terminal.rm`, `terminal.keep`, `terminal.label`, `terminal.read`,
`terminal.attach`, `terminal.send`, `terminal.wait`, `terminal.keys`,
`terminal.interrupt`, `terminal.put`, `machine.exec`, `file.read`,
`file.write`, `file.edit`, `doc.get`, `doc.set`, `events.subscribe`,
`machine.add`, `machine.list`, `machine.connect`, `machine.rename`,
`machine.rm`, `rpc.discover`.

Behaviours that matter here:

- `terminal.list {all?, labels?}`: terminals with `address`, `argv`, `cwd`,
  `cols`, `foreground`, `exit_code`, status fields and labels.
- `terminal.read {terminal, lines?}`: screen plus `lines` of history.
- `terminal.send`: pastes and presses Enter after the person's draft settles
  (up to 60 s) or about 3 s of quiet; refuses a blocked program unless
  `force`. This is the safe "prompt an agent" call.
- `terminal.wait {terminal, output?, children?}`: until the program stops
  working; `ended: false` on a timeout is a check-in, not an error.
- `events.subscribe`: snapshot, then `event` notifications (created, lost,
  restarted, keep, finished, removed, status, turn, title, labels, size,
  doc), a `heartbeat` every 15 s, and `missed` plus a fresh snapshot when
  the subscriber fell behind.
- A terminal started with the label `herdr.space=<id>` (and optionally
  `herdr.tab=<id>`) shows in that app space; an unlabelled one stays hidden.

## Not found yet

Workspace, space, tab and pane methods: none exist in `hc`. They are the
app's (`herdr-app-api.md`).
