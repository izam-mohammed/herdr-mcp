# herdr-mcp drives the herdr app as well as the herdr TUI, from one server

## The ask

"herdr can be in herdr app as well. check what are the changes required",
then "is it possible to combine both in mcp?". Today the server speaks only
to the herdr TUI's socket (`docs/external/herdr-socket-api.md`). The herdr
desktop app is a different product: `herdr-app` (the window) on top of
`herdr-core` (`hc`, which runs the terminals), with its own sockets, ids
and vocabulary. Checked 21/11/2026 on this machine: a Claude Code session
inside the app has `HC_TERMINAL_ID` and `HC_TERMINAL_SOCKET` set and no
`HERDR_ENV`, and the MCP answered from a separate `herdr server` (one tab,
workspace `wV`) instead of the app's spaces. Both backends are processes of
different binaries and versions (herdr 0.9.5; herdr-core 0.1.21 and
herdr-app 0.1.16), so configuration alone cannot join them.

Built so far: `plans/01-first-version.md` to `plans/12-repo-hygiene.md` are
all merged on `main` and `v0.1.0` is released (`docs/releases.md`).

## Alternatives

### Decision 1: how do the two backends meet the one MCP surface? (two-way door)

Searched 21/11/2026: "MCP server multiple backends one process",
`docs/external/herdr-core-api.md`, `docs/external/herdr-app-api.md`, the
project's own tool surface (57 tools, `docs/book/src/reference/`).

| Must | One server, two backends | Two registered servers | A second crate and binary |
| --- | --- | --- | --- |
| Works with no `HERDR_ENV` (inside the app) | yes | yes | yes |
| Existing tool names and ids keep working for TUI users | yes | no (the second needs another name set) | no |
| **Survives** | **yes** | no | no |

Only one option survives the second must, so no weights are needed. The two
losers also double the processes each Claude Code session starts, which the
project budgets (`docs/standards.md` Constraints).

**Chosen: one server, two backends.** Revisit if the app's API and the TUI's
diverge so far that a shared tool surface hides real differences.

### Decision 2: how does the app backend talk to herdr-core? (two-way door)

Searched 21/11/2026: `hc schema` (JSON-RPC 2.0 over a Unix socket, 32
methods, protocol generation 1), `hc --skill`; measured against the live
core.

| Criterion | Weight | JSON-RPC on the `hc` socket | `hc` as a subprocess |
| --- | ---: | ---: | ---: |
| Performance efficiency | 50 | 5 | 2 |
| Reliability | 30 | 4 | 4 |
| Security | 20 | 4 | 3 |
| **Weighted total** | **100** | **4.50** | **2.80** |

- Efficiency: `terminal.list` took 0.5 ms over the socket and `hc ls` 4 ms
  as a subprocess (21/11/2026, `docs/external/herdr-core-api.md`).
- Reliability: both are the same API; the socket has `hello` with a
  protocol generation to refuse a core it does not know.
- Security: no argument parsing by another program; the socket is mode 0600.

Sensitivity: equal weights give 4.33 against 3.00; it holds.
Risk: the protocol is generation 1 and young; `hello` pins it and the
fake-socket tests hold the shapes.

**Chosen: JSON-RPC on the `hc` socket**, reusing the client's
one-connection-per-call shape (`src/herdr.rs`).

### Decision 3: how are spaces, tabs and splits driven? (two-way door)

Searched 21/11/2026: `herdr-app --skill`, `herdr-app commands`,
`~/.config/herdr-app/AGENTS.md`. `hc` has no space, tab or pane methods;
those are the window's.

| Must | The app socket directly | `herdr-app do` as a subprocess |
| --- | --- | --- |
| A documented, supported interface | no (only the CLI is documented) | yes |
| Creates spaces, tabs and splits | yes | yes |
| **Survives** | no | **yes** |

**Chosen: `herdr-app do` and `herdr-app ls` as subprocesses**, arguments
passed as separate argv items and JSON built with `serde_json`, never
through a shell. Window changes are rare, so the 4 to 5 ms per call
(measured 21/11/2026) is paid seldom; reads that agents repeat (`terminal.*`)
go over the `hc` socket. Revisit when the app socket is documented.

### Decision 4: how does a call pick its backend? (two-way door)

Searched 21/11/2026: the id shapes in both docs. herdr ids carry a colon
(`wA`, `wA:t1`, `wA:p1`); the app's are `sp...`, `tb...`, `p...` and
terminal addresses like `t6`. They cannot be mistaken for each other.

| Must | Prefix every id (`app:t6`) | Route by the id's shape, prefix optional | Pick one backend at start |
| --- | --- | --- | --- |
| Both backends live at once | yes | yes | no |
| **Survives** | **yes** | **yes** | no |

| Criterion | Weight | Prefix every id | Route by shape |
| --- | ---: | ---: | ---: |
| Performance efficiency | 40 | 5 | 5 |
| Reliability (an id is never ambiguous) | 30 | 5 | 4 |
| Compatibility (ids already in rules and scripts) | 30 | 2 | 5 |
| **Weighted total** | **100** | **4.10** | **4.70** |

- Reliability 4: shapes are unambiguous today; a future id format that
  clashes is caught by a test that feeds every documented shape to the
  router, and the `herdr:` or `app:` prefix always overrides.
- Compatibility 2 for prefixing: every id a person has written into a
  permission rule or script would change.

Sensitivity: weighting reliability at 60 and the others at 20 each ties them
at 4.40; compatibility, which prefixing loses, then decides for shape routing.
Risk: an id of unknown shape; it is refused as `invalid_params`, never
guessed.

**Chosen: route by shape, with an optional `herdr:` or `app:` prefix.**
Tools that create something (no id to route by) take an optional `backend`
argument; its default is the caller's own backend (the app when
`HC_TERMINAL_ID` is set, the TUI when `HERDR_ENV` is `1`), and with both
unset the one reachable backend.

## The decision

A `Backend` trait in `src/herdr.rs`'s place, with `HerdrBackend` (today's
code, unchanged behaviour) and `AppBackend` (the `hc` socket client plus
`herdr-app` subprocesses), and a router in front that holds one or both
(decisions 1 and 4). Detection at start: `HERDR_ENV=1` or an explicit
`--socket` or `--session` enables herdr; `HC_TERMINAL_SOCKET` or
`--app-home` enables the app; `--backend herdr|app|auto` (default `auto`)
overrides. Both reachable means both enabled.

Vocabulary: the tools keep their names. An app space is a workspace, an
app tab a tab, an app terminal a pane (its address `tNN`, its window
`pane` id kept as a field). List and get results gain `backend`.

What the app supports: sessions, workspaces, tabs, panes (list, get, read,
run, keys, send, wait, split, close), agents (list, get, prompt, wait,
read), events. What it does not map: `herdr_layout_*` (the app's layouts
are window skins), `herdr_notify`, `herdr_worktree_*`. They answer
`unsupported_on_app` and are left out of `tools/list` when only the app is
enabled. `machine/t7` addresses are refused: remote terminals are out of
scope (open decision).

Safety is the same rules on both: read-only outside the caller's backend
unless allowed, no prompting or closing the caller's own terminal, tab or
space (own terminal from `HC_TERMINAL_ID`, own tab and space found through
`herdr-app ls`), the same error codes (`own_agent`, `self_target`,
`workspace_group_close_required`), the same destructive annotations.

Dependencies and permissions: none new (a Unix socket client and
subprocesses; `herdr-app` and `hc` are the user's own installed binaries).

Standards: `docs/standards.md` (format, lint, tests with the 90 percent
floor, audit, secret scan, shell check and build, through
`sh scripts/check.sh`; the docs check and the reference test too, since
tools change).

## Integrations

| Integration | Local docs | Proof script | Env vars |
| --- | --- | --- | --- |
| herdr-core (`hc`) | `docs/external/herdr-core-api.md` | `scripts/prove/herdr-core.sh` | `HC_TERMINAL_SOCKET` |
| herdr-app | `docs/external/herdr-app-api.md` | `scripts/prove/herdr-app.sh` | `HERDR_APP_HOME` |

## Where it lands

| Path | New or edit | Phase | What changes | Firm? |
| --- | --- | --- | --- | --- |
| `scripts/prove/herdr-core.sh`, `scripts/prove/herdr-app.sh` | new | 1 | smallest real call each, prints PASS | expected |
| `src/backend.rs` | new | 2 | the trait, the router, the id shapes | expected |
| `src/herdr.rs` | edit | 2 | becomes `HerdrBackend`, behaviour unchanged | reuse |
| `src/app/` | new | 2 | `hc` client, `herdr-app` runner, mapping | expected |
| `src/config.rs`, `src/safety.rs` | edit | 2, 3 | detection, flags, own terminal and tab | reuse |
| `src/tools/*.rs` | edit | 2 to 5 | call the router, add `backend` | reuse |
| `tests/app_*.rs`, `tests/fixtures/app/` | new | 2 to 5 | fake-socket and live tests | expected |
| `docs/book/src/`, `README.md` | edit | 5 | app support, flags, the safety page | reuse |

*Reused, not rewritten:*

- `src/herdr.rs` `HerdrClient`: one connection per call and its timeout and
  error mapping; the `hc` client copies the pattern, it does not share the
  protocol.
- `src/safety.rs`: the own-pane and read-only checks, extended to take the
  own terminal and tab from either backend.
- `src/events/` hub and ring: the app's `events.subscribe` feeds the same
  hub.
- `tests/session_names.rs`: new live files register their session names.
- Searched for an existing multi-backend seam: none; every tool calls
  `HerdrClient` directly.

## Contract

- Flags and env: `--backend herdr|app|auto` (`HERDR_MCP_BACKEND`),
  `--app-home <dir>` (`HERDR_APP_HOME`).
- Ids: a herdr id keeps its shape; an app id is `sp...`, `tb...`, `p...` or
  `tNN`; either may carry `herdr:` or `app:`. No other shape is accepted.
- Results of list and get calls carry `backend: "herdr" | "app"`.
- Create tools take `backend?: "herdr" | "app"`.
- New error kind: `unsupported_on_app`. All existing codes are unchanged.

## Edge cases

- Both backends enabled and the caller's own backend unknown: create tools
  need `backend`, and refuse with `invalid_params` naming it.
- An id of no known shape: `invalid_params`, never guessed.
- `herdr-app` missing from `PATH`, or no window running: the app backend
  reports `herdr_unreachable` and the herdr backend keeps working.
- A core whose `hello` generation is not 1: the app backend refuses to
  start and says which generation it saw.
- `space.close` asks the person unless `confirmed`; the tool passes
  `confirmed` only after its own destructive confirmation.
- `machine/t7` addresses: `invalid_params`.

## Security

What is protected: other terminals and agents, which the tools can type
into or close, and the user's session. Reached by any MCP client of the
server, through the same stdio entry as today.

| Threat | Here | Handled by | Proved by |
| --- | --- | --- | --- |
| Elevation of privilege | a prompt or close aimed at the caller's own terminal, tab or space | own ids from `HC_TERMINAL_ID` and `herdr-app ls`, refused as `own_agent` or `self_target` | a fake-socket test per tool, and the live try |
| Tampering | an id or name carrying shell syntax reaches `herdr-app` | argv items, JSON from `serde_json`, no shell | a test passing `; rm` and quotes as names |
| Tampering | a title, label or cwd read from a terminal obeyed as an instruction | results marked untrusted, as today | the existing instruction test, run on app fixtures |
| Spoofing | one backend's id used on the other | shape routing, unknown shapes refused | the router test over every shape |
| Denial of service | a huge read or a never-ending wait | the caps and bounded waits of plans 03 and 04 | the cap tests, run on app fixtures |
| Information disclosure | reading a remote machine's terminal | `machine/t7` refused | a fixture test |

Data: terminal text passes through capped and is never stored. Secrets: none
read or written; no env var holds one. Logged: payloads only under
`--debug`, as today. Checklist followed: OWASP ASVS V5 (input handling) for
the id and argument paths.

## Risks

- The app has no headless mode known to this plan, so live tests and CI may
  not run it: the app backend is held by fake-socket tests with fixtures
  recorded from the live core, and its live files run only where an
  isolated app can start (Phase 1 finds out).
- `hc` is generation 1 and moves fast: `hello` pins the generation and
  `docs/external/herdr-core-api.md` is re-checked at each release.
- The vocabulary mapping hides real differences (a pane is a terminal plus a
  window id): `backend` and the kept `pane` field show them.

## Open decisions

- Remote machines (`machine/t7`): default, refused in this plan; a later
  plan adds them once a local-only version is tried.
- The `herdr-app` socket: default, used through the CLI until its protocol
  is documented.

## Declined

- Two registered servers and a second crate (decision 1): they fail the
  must that existing tool names and ids keep working, and double the
  processes per session.
- `hc` as a subprocess (decision 2): 4 ms against 0.5 ms for the same call.
- The app socket directly (decision 3): not a documented interface.
- Prefixing every id and picking one backend at start (decision 4): the
  first breaks every stored id, the second cannot serve both at once.

## What the tests hold

The router sends every documented id shape to the right backend and refuses
the rest. The herdr backend behaves exactly as before: the whole existing
suite and its live files stay green after the refactor. The app backend's
list, get, read, run, send, wait and events calls match fixtures recorded
from the live core. The own-terminal, own-tab and own-space refusals fire on
each write tool. Unsupported tools answer `unsupported_on_app` and vanish
from `tools/list` on an app-only server. A live file drives a real app,
where one can start, over stdio.

## Repository knowledge

Read root `KNOWLEDGE.md` when present before each phase. Follow the global and
repository instructions for recording useful development findings.

## Phases

### Phase 1: prove herdr-core and herdr-app

- [ ] `scripts/prove/herdr-core.sh` makes the smallest real call per
  `docs/external/herdr-core-api.md` (`hello`, then `terminal.list`, over the
  socket) against an isolated core, and prints PASS; `scripts/prove/herdr-app.sh`
  does the same for `herdr-app ls` and one `space.new` and `space.close` in an
  isolated `HERDR_APP_HOME`, never the person's window.
  Neither is wired into the server until both pass.
- [ ] The proofs record whether an isolated core and app can start headless,
  in `docs/research/16-herdr-app-backend.md`, which settles the live-test
  risk and where the live files may run.
- [ ] Check: both scripts exit 0, and `sh scripts/check.sh` passes (shell
  check runs on them).

### Phase 2: the rough cut: read the app from this session (needs you: try it)

- [ ] `Backend` trait and `HerdrBackend`, a refactor with no behaviour
  change (expected in `src/backend.rs`, `src/herdr.rs`), pinned by the whole
  existing suite, unchanged.
- [ ] `AppBackend` read side: the `hc` socket client with `hello` and the
  generation check, `herdr-app ls` parsing, and the mapping of spaces, tabs
  and terminals to workspaces, tabs and panes (expected in `src/app/`),
  pinned by fixtures recorded from the live core.
- [ ] Detection and the router: `--backend`, `--app-home`, shape routing,
  `backend` on results, and the read tools `herdr_whoami`, `herdr_status`,
  `herdr_workspace_list/get`, `herdr_tab_list/get`, `herdr_pane_list/get/read`
  and `herdr_agent_list/get` served from the app (expected in
  `src/config.rs`, `src/tools/`), pinned by router and fixture tests.
- [ ] Check: `sh scripts/check.sh` passes, and `herdr-mcp` started inside an
  app terminal answers `herdr_workspace_list` with the app's spaces.
- [ ] You try it: restart Claude Code inside the herdr app, ask how many
  tabs there are and what is in the other panes, and say whether it matches
  what the window shows. What you want changed goes into the phases below.

### Phase 3: write to the app, safely

- [ ] Write tools on the app: `herdr_pane_run`, keys, send, split, close,
  `herdr_agent_prompt`, `herdr_workspace_*` and `herdr_tab_*` create, rename,
  move and close, through `herdr-app do` with argv items (expected in
  `src/app/`), pinned by a stub `herdr-app` in the tests.
- [ ] Own terminal, tab and space refused with the same codes, on every
  write tool (expected in `src/safety.rs`), pinned by one test per tool.
- [ ] Check: `sh scripts/check.sh` passes, and the live file, where an
  isolated app starts, creates a space, runs a command and closes the space.

### Phase 4: waits, events and what the app lacks

- [ ] `herdr_pane_wait_output`, `herdr_agent_wait` and the three events tools
  on the app, over `terminal.wait` and `events.subscribe`, with the
  resubscribe on `missed` (expected in `src/app/`, `src/events/`), pinned by
  fixtures including a dropped subscription.
- [ ] `unsupported_on_app` for layouts, notify and worktrees, absent from
  `tools/list` on an app-only server, and the resources and prompts served
  from either backend (expected in `src/tools/`, `src/resources.rs`).
- [ ] Check: `sh scripts/check.sh` passes, and `HERDR_MCP_UPDATE_DOCS=1
  cargo test --locked --test docs` then the plain run both pass.

### Phase 5: both at once, and the docs

- [ ] Both backends enabled: merged list results tagged by `backend`, create
  tools with `backend`, and the unknown-caller refusal (expected in
  `src/backend.rs`), pinned by tests over a herdr fake and an app fake.
- [ ] `scripts/bench.sh` stays inside its budget with both backends, and the
  server's start time and memory are recorded in `docs/research/`.
- [ ] README, the book's safety page and the install guide name the app
  support, the flags and what is unsupported; `docs/standards.md` names the
  app's live files; the reference is regenerated (`HERDR_MCP_UPDATE_DOCS=1
  cargo test --locked --test docs`).
- [ ] Check: `sh scripts/check.sh` passes, `sh scripts/docs.sh` passes, and
  one server inside the app lists the app's spaces and a second herdr
  session's workspaces in one call.
- [ ] You try it: run the server with both enabled in the app, ask Claude
  for every tab on both, then open one in the app, and say whether the
  result is what you expect.

### Phase 6: open the PR

- [ ] The branch pushed and its PR opened against the branch it was cut
  from (or, if one is already open for this branch, its body brought up to
  date) with a Conventional Commit title for the whole change, a body naming
  this plan, what changed and why, and the checks run with their result. The
  PR's URL printed as the evidence. Opened, never merged.
