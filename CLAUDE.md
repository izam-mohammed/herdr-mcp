# herdr-mcp

An MCP server, in Rust, that lets Claude Code and other MCP clients see and
control a running herdr session. What it should do is `features.md`; what
is left to build, in order, is `PLAN.md`.

Standards: docs/standards.md

- herdr's API, as this project uses it: `docs/external/herdr-socket-api.md`.
- Never test against the live herdr session's destructive methods; use an
  isolated `herdr --session herdr-mcp-test server` (see the API doc's
  Gotchas). Never run `herdr server stop` on the default session.
