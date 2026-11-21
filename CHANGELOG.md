# Changelog

All notable changes, newest first. Versions follow
[semantic versioning](https://semver.org); `0.x` until the tool names in
`plans/01-first-version.md` are declared stable.

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
