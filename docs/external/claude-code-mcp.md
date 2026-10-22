# Claude Code as the MCP client

Fetched: 22/10/2026 from https://code.claude.com/docs/en/mcp.
Version: Claude Code docs as of that date (features cite v2.1.2xx-2.1.30x).

## Authentication

None for a local stdio server.

## What this project relies on

### Registering a stdio server

```bash
claude mcp add [options] <name> -- <command> [args...]
claude mcp add --transport stdio --env KEY=value herdr -- herdr-mcp
```

Scopes (`-s`, `--scope`): `local` (default, this project, only you),
`project` (`.mcp.json`, shared), `user` (all projects). Everything after
`--` goes to the server untouched. An `--env` pair must not be directly
followed by the server name.

### Tool search (deferred tools)

On by default: "Only tool names and server instructions load at session
start". Tool definitions load when Claude searches for them. `ENABLE_TOOL_SEARCH`
values: unset or `true` (deferred), `auto` / `auto:N` (deferred only past N%
of context, default 10%), `false` (all upfront).

- Server instructions and an upfront tool's description are cut at 4 096
  characters; a deferred tool's description at 16 384.
- Tool search matches against the start of descriptions: put the words a
  model would search for first.
- `_meta["anthropic/alwaysLoad"]: true` on a tool loads it upfront.

### Per-tool `_meta` keys

- `anthropic/maxResultSizeChars`: raise the 50 000-character threshold past
  which a text result is saved to a file (ceiling 500 000).
- `anthropic/requiresUserInteraction: true`: a permission prompt on every
  call, even in auto and bypass modes; allow rules do not skip it.

### Output limits

Warning above 10 000 tokens; default cap 25 000 (`MAX_MCP_OUTPUT_TOKENS`).
Text results over 50 000 characters are saved to a file. Error text over
about 11 000 characters keeps the first and last 5 000.

### Timeouts

`MCP_TIMEOUT` (startup), `MCP_TOOL_TIMEOUT` (per call, about 28 hours when
unset), per-server `timeout` in `.mcp.json`. Stdio idle timeout 30 minutes
(`CLAUDE_CODE_MCP_TOOL_IDLE_TIMEOUT`).

### Resources and prompts

Resources are referenced as `@server:protocol://path` and fetched as
attachments; Claude Code also gives the model tools to list and read them.
Prompts appear as `/server:prompt (MCP)` commands, arguments split on
whitespace. On `list_changed`, an interactive session refetches the list.

### Schemas

Top-level property names must be 1 to 64 characters of `[A-Za-z0-9_.-]`,
schemas valid JSON Schema 2020-12. A root-level `oneOf`/`anyOf` is
flattened. Full tool names (`mcp__<server>__<tool>`) over 128 characters
are dropped.

## Errors

A result with `isError: true` reaches the model as the tool's error message.

## Limits

Above. Resource subscriptions (`resources/subscribe`): not stated in the
docs.

## Gotchas

- Claude acts only in response to a turn; a server notification does not
  start one.
