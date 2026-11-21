<!-- Generated from the server by `tests/docs.rs`. Do not edit: run `HERDR_MCP_UPDATE_DOCS=1 cargo test --locked --test docs`. -->

# Command line

`herdr-mcp --help`:

```text
herdr-mcp: an MCP server for herdr, over stdio

  --session <name>        herdr session (HERDR_SESSION)
  --socket <path>         herdr socket (HERDR_SOCKET_PATH)
  --read-only             register read tools only (HERDR_MCP_READ_ONLY=1)
  --allow-outside-herdr   allow writes when HERDR_ENV is not 1 (HERDR_MCP_ALLOW_OUTSIDE=1)
  --tools <a,b,...>       tool groups to register (HERDR_MCP_TOOLS). Default: every group but
                          admin; `all` is that default, so `all,admin` adds the admin group
  --confirm-destructive   ask the user on every close, even in auto mode
                          (HERDR_MCP_CONFIRM_DESTRUCTIVE=1)
  --event-buffer <n>      events kept for herdr_events_* (HERDR_MCP_EVENT_BUFFER, default 1000)
  --debug                 log payloads to stderr
  --version, --help
```

Flags win over the environment variable named beside them.
