# Plan 13 verification: every herdr command is also an MCP tool

Run 21/11/2026 against herdr 0.9.5 (protocol 22), isolated session `herdr-mcp-test-13`
only; the default session and the herdr app were never touched.

| Behaviour | Driven by | Outcome | Evidence |
| --- | --- | --- | --- |
| the sync script lists every herdr entry with no tool | `scripts/verify/uncovered.sh` | passed | `uncovered.log` (the Phase 1 run listed 32 tools not built yet; the final run lists 0) |
| the six harmless tools work against a real herdr | `scripts/verify/harmless-tools.sh` | passed | `harmless-tools.log` |
| the admin group is absent by default and present when asked | `scripts/verify/admin-off-by-default.sh` | passed | `admin-off-by-default.log` (63 tools by default, 89 with the group, 31 read-only with it) |
| the admin tools refuse what they must | `tests/admin_tools.rs`, `tests/admin_cli.rs` (stub `herdr`) | passed | the test run in `scripts/check.sh` |
| `herdr_config_check` and `herdr_session_list` read a real session | `tests/lane13_live.rs` | passed | `admin-cli.log` |

Outcome of the whole run: **passed**. Not verified against a real herdr, on purpose:
`herdr_update`, `herdr_config_reset_keys`, `herdr_channel_set`, the integration installs,
the machine changes and the session and server stops; those run only through the stub
`herdr` script in `tests/admin_cli.rs` and the fake socket in `tests/admin_tools.rs`.
