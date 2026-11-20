# herdr-mcp

An MCP server, in Rust, that lets Claude Code and other MCP clients see and
control a running herdr session. What it should do is `features.md`; what
is left to build, in order, is `PLAN.md`.

Standards: docs/standards.md

- herdr's API, as this project uses it: `docs/external/herdr-socket-api.md`.
- Never test against the live herdr session's destructive methods; use an
  isolated `herdr --session herdr-mcp-test server` (see the API doc's
  Gotchas). Never run `herdr server stop` on the default session.

Releases: docs/releases.md
- A tool, resource or prompt added, renamed or changed also means
  `HERDR_MCP_UPDATE_DOCS=1 cargo test --locked --test docs` (it rewrites
  `docs/book/src/reference/`; the plain test fails when they are stale).
  `sh scripts/docs.sh` builds the book (`docs/book/`) and checks its links.
