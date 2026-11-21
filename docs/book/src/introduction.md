# herdr-mcp

An MCP server for [herdr](https://herdr.dev). It lets Claude Code, Codex or
any other MCP client list, open, close, read, prompt and wait on everything
running in your herdr session. Middle management, written in Rust.

It offers 57 tools, 7 resources and 3 prompts. The reference chapters list
all of them and are generated from the server itself, so they match the
binary you installed.

## Where to start

- New here: [Getting started](getting-started.md) takes you from install to
  your first `herdr_snapshot`.
- Deciding what to allow: read [Safety](safety.md) before you turn on
  auto mode.
- Changing how it runs: [Configuration](configuration.md) has every flag,
  environment variable and limit.
- Something failed: [Error codes](errors.md).
- Looking up one tool: the [reference](reference/tools.md).

The source and the releases are on
[GitHub](https://github.com/izam-mohammed/herdr-mcp).
