# Error codes

A failed call comes back as a tool error (`isError: true`) with the text
`herdr <code>: <message>` and structured content `{"code", "message"}`.
herdr's own codes pass through unchanged, and the schema gives no complete
list, so an unfamiliar code is not a bug in this server: read its message.

## The server's own codes

| Code | Meaning | What to do |
| --- | --- | --- |
| `read_only` | A write tool was called in read-only mode. | Start the agent inside a herdr pane, or pass `--allow-outside-herdr` for a session you mean to steer. See [Configuration](configuration.md). |
| `self_target` | The call would act on the server's own pane, or on the tab or workspace that holds it. | Pick another target. Only a close of a tab or workspace can pass `allow_self: true`, and only when you asked for exactly that. |
| `own_agent` | The call would prompt or send keys to the server's own agent. | Use another agent. |
| `herdr_unreachable` | The socket could not be reached. | Check herdr runs, and which socket is used (the start line on stderr names it). Is the session name right? |
| `timeout` | herdr did not answer within 10 seconds. | Retry. Raise `HERDR_MCP_TIMEOUT_MS` only if herdr is genuinely slow. This is not a wait that ran out of time (those return `timed_out: true`). |

## herdr's codes

| Code | Meaning | What to do |
| --- | --- | --- |
| `not_found`, `pane_not_found` and similar | The id or name no longer exists. | List again (`herdr_snapshot`); ids change when panes close or move. |
| `invalid_request`, `invalid_params` | The request was malformed. | Check the parameters against the [reference](reference/tools.md). |
| `invalid_regex` | A `regex` match did not compile. | Fix the pattern, or use `substring`. |
| `agent_blocked` | The agent is waiting on a question, so the prompt was not sent. | Read it with `herdr_agent_read` and `herdr_agent_explain`, then decide whether to answer it with `herdr_agent_send_keys`. |
| `agent_prompt_stalled` | herdr reports the prompt stalled instead of taking effect. | Read the agent with `herdr_agent_read`, then retry or step in. |
| `events_lost` | The event subscription fell behind and herdr closed it. | The server resubscribes and records a gap; take a fresh `herdr_snapshot`. See [Events](guides/events.md). |
| `workspace_group_close_required` | The workspace is part of a worktree group. | Ask the user. Never add `close_group` just to get past this. |
| `platform_unsupported`, `plugin_disabled`, `popup_not_open` | The method needs something herdr does not have here. | Nothing to fix in the call. |
| an unsupported-method error | This herdr does not know the method. | Check the herdr version: this server targets herdr 0.9.5 (protocol 22). |

## Not errors

A wait that runs out of time returns normally with `timed_out: true`.
`herdr_notify` with no herdr window open returns `shown: false` and a
`reason`.
