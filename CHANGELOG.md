# Changelog

All notable changes, newest first. Versions follow
[semantic versioning](https://semver.org); `0.x` until the tool names in
`plans/01-first-version.md` are declared stable.

## 0.2.0

### Added

- Six tools in the existing groups: `herdr_pane_scroll`,
  `herdr_pane_selection_read`, `herdr_pane_report_metadata`,
  `herdr_agent_view_set`, `herdr_agent_view_clear` and
  `herdr_workspace_move_block` (63 tools by default).
- An `admin` group, off until asked (`--tools all,admin`): integrations,
  server reload and stop, config checks, the update channel, sessions, saved
  machines, `herdr_update`, and the agent-state reporting calls. Anything that
  changes something is marked destructive and hidden under `--read-only`;
  stopping the server or session it runs in is refused without `allow_self`;
  `herdr_update` and `herdr_config_reset_keys` need `confirm: true`.
- `docs/herdr-coverage.toml` and `sh scripts/sync-herdr.sh`: every herdr
  method and command is mapped to a tool or skipped with a reason, and a test
  fails when herdr gains one with no decision.
- A skill for users, `skills/herdr-mcp/`, with a doctor script that finds why
  the server is not working.
- A new README with a demo gif and a paste-in install prompt.

### Changed

- `--tools all` means the nine default groups; add `admin` to turn that group on.

## 0.1.0

The first version: an MCP server that lets Claude Code and other MCP
clients see and control a running herdr session.

### Added

- Tools for the session, workspaces, tabs, panes, agents, events,
  worktrees, layouts and toasts (`herdr_<area>_<verb>`).
- Safe by default: read-only outside herdr, no prompting or closing the
  server's own pane or agent, destructive tools marked for your client to ask.
- Flags and environment variables for session, socket, read-only, tool
  groups, confirmations and the event buffer.
- Prebuilt binaries for Linux (gnu and musl) and macOS, x86_64 and arm64,
  with a shell installer and checksums, from GitHub Releases; `cargo
  binstall --git` reads the same archives.
