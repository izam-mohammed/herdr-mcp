<!-- Generated from the server by `tests/docs.rs`. Do not edit: run `HERDR_MCP_UPDATE_DOCS=1 cargo test --locked --test docs`. -->

# Tools: notify

## `herdr_notify`

Show a toast in herdr, e.g. when a long task is ready. It changes no panes, tabs or files. Keep it short and honest: the user reads it as a message from you.

Group `notify` (write). Read-only: no. Destructive: no. Idempotent: no. Reaches beyond herdr: no.

| Parameter | Type | Required | What it is |
| --- | --- | --- | --- |
| `body` | string | no | More text under the title. |
| `position` | `top-left`, `top-right`, `bottom-left`, `bottom-right` | no | Where the toast shows. Omit for herdr's default. |
| `sound` | `none`, `done`, `request` | no | A sound to play: `none`, `done` or `request`. Omit for herdr's default. |
| `title` | string | yes | The toast's title, e.g. `tests passed in w3`. |
