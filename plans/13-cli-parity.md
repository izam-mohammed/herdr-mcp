# every herdr command is also an MCP tool, and a check says when herdr gets a new one

## The ask

"create a plan for syncing and updating with the default cli and have all of
the possible commands in cli in the mcp also." The herdr app is out of scope
("no need to cover the app"). Checked 21/11/2026 against herdr 0.9.5
(protocol 22): `herdr api schema` lists about 80 socket methods, of which
the 57 tools use most; the CLI adds commands that are not socket methods
(`config`, `channel`, `session`, `machine`, `update`, `completion`) and the
socket has methods the tools never call (`pane.scroll`,
`pane.selection.read`, `pane.input.set`, `pane.report_*`, `agent.view.*`,
`workspace.move_block`, `integration.*`, `server.*`).

Two things come out of it: every command that an agent can use gets a tool,
and a check that fails when herdr gains a method or command the MCP has not
decided about, so the next herdr release cannot leave a silent gap.

Built so far: `plans/01-first-version.md` to `plans/12-repo-hygiene.md`, all
merged, `v0.1.0` released (`docs/releases.md`). This work lands on `main`
unreleased; the next release is cut by hand when wanted.

## Alternatives

### Decision 1: how do the commands that are not socket methods reach the MCP? (two-way door)

Searched 21/11/2026: `herdr --help` and each subcommand's help,
`herdr api schema --json` (110 names, 80 methods), the existing client
(`src/herdr.rs`); measured `herdr pane list`, `session list`, `config check`
and `channel show` at 5 to 6 ms each as a subprocess, against 0.2 to 0.4 ms
for a socket call (plan 07's measurement, `docs/research/07-resources-and-prompts.md`).

| Must | Socket where a method exists, `herdr` subprocess otherwise | Subprocess for everything | Cover socket methods only |
| --- | --- | --- | --- |
| Every command an agent can use has a tool | yes | yes | no (`config`, `channel`, `session`, `machine`, `update` have no method) |
| **Survives** | **yes** | yes | no |

| Criterion | Weight | Socket where possible | Subprocess for all |
| --- | ---: | ---: | ---: |
| Performance efficiency | 50 | 5 | 2 |
| Reliability (typed replies, no text parsing) | 30 | 5 | 3 |
| Security (no argv built from model text where a typed call exists) | 20 | 4 | 3 |
| **Weighted total** | **100** | **4.80** | **2.50** |

- Efficiency: a method call costs a fraction of a millisecond; each
  subprocess costs 5 to 6 ms and a fork (measured 21/11/2026).
- Reliability: socket replies are JSON with a schema; the CLI prints text
  that changes between herdr versions.

Sensitivity: equal weights give 4.67 against 2.67; it holds.
Risk: the subprocess commands print text; each tool passes it through
capped and untrusted rather than parsing it.

**Chosen: socket where a method exists, the `herdr` binary as a subprocess
otherwise**, with argv items, no shell, a timeout and a capped reply.

### Decision 2: are the new tools on by default? (one-way door, names are what people write rules against)

Searched 21/11/2026: `features.md` § Non-functional (safe by default),
`docs/book/src/safety.md`, the tool counts in the README. The admin
commands change configs, stop servers, replace the herdr binary and let an
agent report another agent's state.

| Must | Admin group, off until asked | Admin tools in the default groups | A separate binary |
| --- | --- | --- | --- |
| Safe by default | yes | no (a default session could stop herdr) | yes |
| One server, one install | yes | yes | no |
| **Survives** | **yes** | no | no |

Only one survives; no weights needed. Tools in a group that is off cost
nothing: a registered tool adds its schema to every session's context.

**Chosen: a new `admin` group, off by default**, enabled with
`--tools all,admin` or `HERDR_MCP_TOOLS`. Harmless additions (scroll, read
a selection, an agent's view, reorder a block, pane metadata) join the
existing groups. Revisit the default after a release of use.

### Decision 3: how does the MCP stay in sync with herdr? (two-way door)

Searched 21/11/2026: `herdr api schema --json`, `herdr --help`; the
project's reference test (`tests/docs.rs`, which already regenerates
`docs/book/src/reference/` from the server).

| Must | A coverage manifest and checked-in lists, a test, a sync script | Generate the tools from the schema | Review by hand |
| --- | --- | --- | --- |
| Works offline in CI (no herdr there) | yes | yes | yes |
| Keeps per-tool safety, caps and descriptions | yes | no (generated tools have none) | yes |
| Fails when herdr adds something | yes | yes | no |
| **Survives** | **yes** | no | no |

**Chosen: a manifest.** `docs/external/herdr-methods.txt` and
`docs/external/herdr-commands.txt` hold herdr's methods and CLI commands as
of the version they name; `docs/herdr-coverage.toml` maps each to a tool or
to `skip = "<reason>"`; a test fails on any entry without a row, and any
row for something herdr no longer has. `sh scripts/sync-herdr.sh`
regenerates the two lists from the installed herdr and prints what is new.

## The decision

- **Sync (decision 3).** `scripts/sync-herdr.sh`, the two lists, the
  manifest and `tests/coverage.rs`, run by the normal test step.
- **Harmless additions** to existing groups, over the socket: panes
  `herdr_pane_scroll`, `herdr_pane_selection_read` (untrusted text, capped)
  and `herdr_pane_report_metadata`; agents `herdr_agent_view_set` and
  `herdr_agent_view_clear`; workspaces `herdr_workspace_move_block`.
- **The `admin` group (decision 2), socket-backed:** `herdr_integration_list`,
  `_install`, `_uninstall`; `herdr_server_reload_config`,
  `herdr_server_agent_manifests`, `herdr_server_reload_agent_manifests`,
  `herdr_server_stop`; `herdr_pane_set_input`, `herdr_pane_report_agent`,
  `herdr_pane_report_agent_session`, `herdr_pane_release_agent`,
  `herdr_pane_clear_agent_authority`.
- **The `admin` group, subprocess-backed (decision 1):** `herdr_config_check`,
  `herdr_config_reset_keys`, `herdr_channel_show`, `herdr_channel_set`,
  `herdr_session_list`, `herdr_session_stop`, `herdr_session_delete`,
  `herdr_machine_list`, `_status`, `_rename`, `_remove`, `_enable`,
  `_disable`, `herdr_update`.
- **Skipped, each with its reason in the manifest:** `session attach`,
  `machine add` and `machine reconnect` (interactive, they ask for a
  password or a terminal), `completion` (a shell script for a person),
  `pane.copy_*`, `pane.edit_scrollback`, `pane.link.*` (copy mode, an
  editor and opening links: interactive), `herdr --remote` and bare
  `herdr` (they start a client).
- **Safety.** Admin tools that change something are marked destructive, so
  the client asks; `--read-only` hides every mutating one; `herdr_server_stop`
  and `herdr_session_stop` and `_delete` refuse the caller's own session
  (`self_target`, `allow_self` overrides); `herdr_update` and
  `herdr_config_reset_keys` also need `confirm: true`; subprocess tools pass
  argv items only, reject anything that starts with `-` as a value, and cap
  and time out the output.

Dependencies and permissions: none new. The subprocess tools run the user's
own installed `herdr`.

Standards: `docs/standards.md` (format, lint, tests with the 90 percent
floor, audit, secret scan, shell check and build, through
`sh scripts/check.sh`; the docs reference test too, since tools change).

## Integrations

| Integration | Local docs | Proof script | Env vars |
| --- | --- | --- | --- |
| herdr CLI and schema | `docs/external/herdr-socket-api.md` (extend) | `scripts/prove/herdr-cli.sh` | `HERDR_SESSION` |

## Where it lands

| Path | New or edit | Phase | What changes | Firm? |
| --- | --- | --- | --- | --- |
| `scripts/sync-herdr.sh`, `scripts/prove/herdr-cli.sh` | new | 1 | regenerate the lists, prove the CLI calls | expected |
| `docs/external/herdr-methods.txt`, `herdr-commands.txt` | new | 1 | herdr's surface, dated, with its version | fixed |
| `docs/herdr-coverage.toml` | new | 1 | method or command to tool, or skip and why | fixed |
| `tests/coverage.rs` | new | 1 | fails on an unmapped or stale entry | expected |
| `src/tools/panes.rs` | edit | 2 | scroll, selection read, pane metadata | reuse |
| `src/tools/agents.rs` | edit | 2 | agent view set and clear | reuse |
| `src/tools/workspaces.rs` | edit | 2 | move block | reuse |
| `src/tools/admin.rs` | new | 3 | the socket-backed admin tools | expected |
| `src/tools/admin_cli.rs`, `src/cli.rs` | new | 4 | the subprocess runner and its tools | expected |
| `src/tools/mod.rs`, `src/config.rs` | edit | 3 | the `admin` group, off by default | reuse |
| `src/safety.rs` | edit | 3, 4 | own session and `confirm` rules | reuse |
| `README.md`, `docs/book/src/` | edit | 5 | counts, the admin group, the sync flow | reuse |

*Reused, not rewritten:*

- `src/tools/mod.rs` `GROUPS`: a new group is one `mod` line and one entry.
- `src/herdr.rs` `HerdrClient::call`: every socket-backed tool uses it.
- `src/safety.rs`: own-pane and read-only checks, extended to the own
  session.
- `tests/docs.rs`: the reference regeneration that already runs in the test
  step.
- Searched for an existing subprocess helper: none, since every call so far
  is a socket call; `src/cli.rs` is the one new place.

## Contract

- Tool names above are binding; every one starts `herdr_`.
- Group name `admin`, off by default; `HERDR_MCP_TOOLS` and `--tools`
  accept it.
- Subprocess tools return `{ "stdout": ..., "stderr": ..., "exit": n,
  "truncated": bool }`, capped, marked untrusted.
- New error kinds: none. Refusals reuse `self_target`, `read_only` and
  `invalid_params`; a missing `herdr` binary is `herdr_unreachable`.
- `docs/herdr-coverage.toml` rows: `"<method or command>" = "<tool>"` or
  `"<method or command>" = { skip = "<reason>" }`.

## Edge cases

- A value that starts with `-` in a subprocess tool: `invalid_params`.
- `herdr` not on `PATH` or a timeout (default 10 s, `update` 120 s):
  `herdr_unreachable` or a `timed_out` result, never a hang.
- `herdr_update` while the server's own herdr is the one updating: refused
  without `confirm`, and the reply says the session restarts.
- A herdr newer than the lists: the test passes offline (it checks the
  checked-in lists), and `sync-herdr.sh` shows the new entries.
- `--read-only`: every mutating admin tool is absent from `tools/list`.

## Security

What is protected: the user's herdr configs, sessions and binary, which the
admin tools can change, and other agents' states. Reached by any MCP client
of the server, only when the `admin` group is on.

| Threat | Here | Handled by | Proved by |
| --- | --- | --- | --- |
| Elevation of privilege | an agent stops the server it runs in, or its own session | own-session refusal, `allow_self` | a fake-socket test per tool |
| Tampering | argv built from model text injects a flag or a command | argv items, no shell, a value starting `-` refused | a test with `--help`, `; rm` and quotes |
| Tampering | `herdr update` or `config reset-keys` runs on a prompt-injected request | marked destructive, `confirm: true` required | a test without `confirm` |
| Spoofing | `report_agent*` makes herdr show a false state | admin group only, destructive, hidden in read-only | the read-only list test |
| Information disclosure | `session list` or `machine list` leaks hosts | admin only, output marked untrusted and capped | a cap test |
| Denial of service | a command that never ends | timeout per tool | a stub that sleeps |

Data: no secrets read or written; no env var holds one. Logged: payloads
only under `--debug`. Checklist followed: OWASP ASVS V5 (input handling)
for the argv path, and the project's `docs/book/src/safety.md`.

## Risks

- herdr's CLI text changes between versions: the tools pass it through and
  never parse it.
- The admin group is a bigger blast radius: off by default and destructive
  marks, and the docs say what each does.
- More schemas in a session's context: only for those who turn `admin` on;
  the harmless six add a few hundred tokens.

## Open decisions

- Whether to cut a release for this: default, no; it lands on `main` and a
  release is cut by hand when wanted.
- `herdr_update` at all: default, included behind `confirm`; drop it if
  replacing the binary under a running server proves flaky in Phase 4.

## Declined

- Covering only socket methods (decision 1): leaves `config`, `channel`,
  `session`, `machine` and `update` with no tool.
- A subprocess for everything (decision 1): 5 to 6 ms against 0.3 ms, and
  text instead of typed replies.
- Admin tools in the default groups, or a second binary (decision 2): the
  first is unsafe by default, the second splits the install.
- Generating tools from the schema (decision 3): generated tools carry no
  safety rules, caps or descriptions.

## What the tests hold

Every method and command in the checked-in lists has a manifest row, and no
row is stale. Each new tool refuses what it must (own session, a flag-like
value, a missing `confirm`, read-only) and passes what it should. The
default `tools/list` is unchanged apart from the six harmless tools; the
admin ones appear only with the group on. The existing suite and its live
files stay green.

## Verification

| Behaviour | Surface | Driven by | Evidence |
| --- | --- | --- | --- |
| the sync script lists every herdr entry with no tool | CLI | `scripts/verify/uncovered.sh` | `docs/verified/13-cli-parity/uncovered.log` |
| the six harmless tools work against a real herdr | MCP over stdio | `scripts/verify/harmless-tools.sh` | `docs/verified/13-cli-parity/harmless-tools.log` |
| the admin group is absent by default and present when asked | MCP over stdio | `scripts/verify/admin-off-by-default.sh` | `docs/verified/13-cli-parity/admin-off-by-default.log` |
| the admin tools refuse what they must | MCP over stdio | `tests/admin_*.rs` | the test run in `scripts/check.sh` |
| `herdr_config_check` and `herdr_session_list` read a real session | MCP over stdio | the live file in Phase 4 | `docs/verified/13-cli-parity/admin-cli.log` |

The run's outcome (`passed`, `failed`, `blocked` or `cannot_verify`) and the
date go in `docs/verified/13-cli-parity/RESULT.md`.

## Repository knowledge

Read root `KNOWLEDGE.md` when present before each phase. Follow the global and
repository instructions for recording useful development findings.

## Phases

### Phase 1: the rough cut: see what is not covered

- [ ] `scripts/prove/herdr-cli.sh` runs `herdr api schema --json`,
  `herdr --help` and one read-only subcommand, and prints PASS; nothing
  below is wired until it does.
- [ ] `scripts/sync-herdr.sh`, the two lists, `docs/herdr-coverage.toml`
  with a row for every method and command (a tool, or a skip with a reason)
  and `tests/coverage.rs` (expected in `scripts/`, `docs/`, `tests/`),
  pinned by a test that fails on an unmapped entry and on a stale row.
- [ ] Check: `sh scripts/check.sh` passes, and `sh scripts/sync-herdr.sh`
  prints the entries with no tool yet.
- [ ] Verify: `scripts/verify/uncovered.sh` runs `sh scripts/sync-herdr.sh`
  against the installed herdr and asserts it prints every entry that has no
  tool yet and exits 0; its output is saved under
  `docs/verified/13-cli-parity/`.

### Phase 2: the harmless additions

- [ ] `herdr_pane_scroll`, `herdr_pane_selection_read`,
  `herdr_pane_report_metadata`, `herdr_agent_view_set`, `_clear` and
  `herdr_workspace_move_block` over the socket (expected in
  `src/tools/panes.rs`, `agents.rs`, `workspaces.rs`), pinned by
  fake-socket tests, with the manifest rows pointing at them.
- [ ] Check: `sh scripts/check.sh` passes and the coverage test is green.
- [ ] Verify: `scripts/verify/harmless-tools.sh` starts an isolated
  `herdr --session herdr-mcp-test-13 server`, calls each of the six tools
  over stdio and prints PASS; output saved under `docs/verified/13-cli-parity/`.

### Phase 3: the admin group, over the socket

- [ ] The `admin` group, off by default, with the socket-backed tools and
  the own-session and read-only rules (expected in `src/tools/admin.rs`,
  `src/tools/mod.rs`, `src/config.rs`, `src/safety.rs`), pinned by tests
  that the default `tools/list` has none and `--tools all,admin` has them.
- [ ] Check: `sh scripts/check.sh` passes.
- [ ] Verify: `scripts/verify/admin-off-by-default.sh` lists tools with and
  without the group and prints PASS; output saved under
  `docs/verified/13-cli-parity/`.

### Phase 4: the admin group, through the herdr binary

- [ ] The subprocess runner (argv only, a value starting `-` refused, a
  timeout, a capped reply) and the subprocess-backed tools (expected in
  `src/cli.rs`, `src/tools/admin_cli.rs`), pinned by a stub `herdr` script
  in the tests, including the `confirm` rules and the own-session refusal.
- [ ] Check: `sh scripts/check.sh` passes, and with `HERDR_MCP_LIVE=1` the
  live file reads `herdr_config_check` and `herdr_session_list` against an
  isolated session.

### Phase 5: docs and the counts

- [ ] README tool counts, the `admin` group and its warning, the sync flow
  (`sh scripts/sync-herdr.sh`, then add a row), the book's safety page and
  `docs/standards.md` updated; the reference regenerated with
  `HERDR_MCP_UPDATE_DOCS=1 cargo test --locked --test docs`.
- [ ] Check: `sh scripts/check.sh` passes, `sh scripts/docs.sh` passes.

### Phase 6: open the PR

- [ ] The branch pushed and its PR opened against the branch it was cut
  from (or, if one is already open for this branch, its body brought up to
  date) with a Conventional Commit title for the whole change, a body naming
  this plan, what changed and why, and the checks run with their result. The
  PR's URL printed as the evidence. Opened, never merged.
