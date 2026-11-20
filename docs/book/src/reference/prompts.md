<!-- Generated from the server by `tests/docs.rs`. Do not edit: run `HERDR_MCP_UPDATE_DOCS=1 cargo test --locked --test docs`. -->

# Prompts

Shown as slash commands in Claude Code, for example `/mcp__herdr__dashboard`. Each is a short instruction that names the tools to use.

## `check-agents`

Summarise what each blocked or finished agent is waiting for or did.

No arguments.

## `dashboard`

One screen: workspaces, tabs, panes and each agent's status.

No arguments.

## `run-and-watch`

Run a command in a new pane, wait for it and report the result.

| Argument | Required | What it is |
| --- | --- | --- |
| `command` | yes | The shell command to run. |
