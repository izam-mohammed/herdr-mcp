<!-- Generated from the server by `tests/docs.rs`. Do not edit: run `HERDR_MCP_UPDATE_DOCS=1 cargo test --locked --test docs`. -->

# Tools: admin

## `herdr_integration_install`

Install herdr's integration for an agent: writes a hook or plugin into that agent's own config directory. The reply lists what was written.

Group `admin` (destructive). Read-only: no. Destructive: yes. Idempotent: yes. Reaches beyond herdr: no.

| Parameter | Type | Required | What it is |
| --- | --- | --- | --- |
| `target` | string | yes | The agent to integrate with, as `herdr_integration_list` names it, e.g. `claude`, `codex`, `pi`. |

## `herdr_integration_list`

List herdr's built-in agent integrations and whether each is installed, current or outdated.

Group `admin` (read). Read-only: yes. Destructive: no. Idempotent: yes. Reaches beyond herdr: no.

No parameters.

## `herdr_integration_uninstall`

Remove herdr's integration for an agent from that agent's config directory. The reply lists what was removed.

Group `admin` (destructive). Read-only: no. Destructive: yes. Idempotent: yes. Reaches beyond herdr: no.

| Parameter | Type | Required | What it is |
| --- | --- | --- | --- |
| `target` | string | yes | The agent to integrate with, as `herdr_integration_list` names it, e.g. `claude`, `codex`, `pi`. |

## `herdr_pane_clear_agent_authority`

Clear the authority over a pane's agent state that reporting sources hold.

Group `admin` (destructive). Read-only: no. Destructive: yes. Idempotent: yes. Reaches beyond herdr: no.

| Parameter | Type | Required | What it is |
| --- | --- | --- | --- |
| `pane_id` | string | yes | Pane id, e.g. `wA:p2`. |
| `seq` | integer | no | Sequence number. |
| `source` | string | no | Clear only what this source holds. Omit to clear all. |

## `herdr_pane_release_agent`

Release the agent state a source reported for a pane, back to herdr's own detection.

Group `admin` (destructive). Read-only: no. Destructive: yes. Idempotent: yes. Reaches beyond herdr: no.

| Parameter | Type | Required | What it is |
| --- | --- | --- | --- |
| `agent` | string | yes | The agent's name. |
| `pane_id` | string | yes | Pane id, e.g. `wA:p2`. |
| `seq` | integer | no | Sequence number; herdr ignores a release older than a report it has. |
| `source` | string | yes | The source that reported the agent. |

## `herdr_pane_report_agent`

Report an agent's state for a pane, as an agent's own hook would. herdr then shows that state, so a wrong report is a false display. Use only for an agent you run.

Group `admin` (destructive). Read-only: no. Destructive: yes. Idempotent: yes. Reaches beyond herdr: no.

| Parameter | Type | Required | What it is |
| --- | --- | --- | --- |
| `agent` | string | yes | The agent's name, e.g. `claude`. |
| `agent_session_id` | string | no | The agent's own session id. |
| `agent_session_path` | string | no | The path of the agent's session file. |
| `message` | string | no | A message shown with the state. |
| `pane_id` | string | yes | Pane id, e.g. `wA:p2`. |
| `resume_argv` | array of string | no | Command that resumes the agent after a herdr restart; the first item must be a plain command name. |
| `seq` | integer | no | Sequence number; herdr ignores a report older than one it has. |
| `source` | string | yes | Who reports, e.g. `herdr-mcp`. The same source must release it later. |
| `state` | `idle`, `working`, `blocked`, `unknown` | yes | The state to show for the agent. |

## `herdr_pane_report_agent_session`

Report an agent's session identity for a pane (session id, file, how to resume it after a herdr restart).

Group `admin` (destructive). Read-only: no. Destructive: yes. Idempotent: yes. Reaches beyond herdr: no.

| Parameter | Type | Required | What it is |
| --- | --- | --- | --- |
| `agent` | string | yes | The agent's name, e.g. `claude`. |
| `agent_session_id` | string | no | The agent's own session id. |
| `agent_session_path` | string | no | The path of the agent's session file. |
| `pane_id` | string | yes | Pane id, e.g. `wA:p2`. |
| `resume_argv` | array of string | no | Command that resumes the agent after a herdr restart; the first item must be a plain command name. |
| `seq` | integer | no | Sequence number; herdr ignores a report older than one it has. |
| `session_start_source` | string | no | How the session started, as the agent names it. |
| `source` | string | yes | Who reports, e.g. `herdr-mcp`. |

## `herdr_pane_set_input`

Choose where a right click in a pane goes: to herdr's menu or to the program in the pane.

Group `admin` (destructive). Read-only: no. Destructive: yes. Idempotent: yes. Reaches beyond herdr: no.

| Parameter | Type | Required | What it is |
| --- | --- | --- | --- |
| `pane_id` | string | yes | Pane id, e.g. `wA:p2`. |
| `right_click` | `herdr` or `pane` | yes | Where a right click in the pane is routed. |

## `herdr_server_agent_manifests`

Show the agent detection manifests herdr uses: which agent, from which source, which version.

Group `admin` (read). Read-only: yes. Destructive: no. Idempotent: yes. Reaches beyond herdr: no.

No parameters.

## `herdr_server_reload_agent_manifests`

Reload herdr's local agent detection manifest overrides, which changes how agents in every pane are detected.

Group `admin` (destructive). Read-only: no. Destructive: yes. Idempotent: yes. Reaches beyond herdr: no.

No parameters.

## `herdr_server_reload_config`

Reload config.toml in the running herdr server. A broken file is reported, not applied.

Group `admin` (destructive). Read-only: no. Destructive: yes. Idempotent: yes. Reaches beyond herdr: no.

No parameters.

## `herdr_server_stop`

Stop the herdr server this tool talks to: every pane and the programs in them end. Refused with self_target when that server is the one this MCP server runs in, unless allow_self is true (only if the user asked for exactly that).

Group `admin` (destructive). Read-only: no. Destructive: yes. Idempotent: no. Reaches beyond herdr: no.

| Parameter | Type | Required | What it is |
| --- | --- | --- | --- |
| `allow_self` | boolean | no | Stop the server even though it is the one this server runs in, which ends every pane in that session, yours included. Only when the user asked for exactly that. |
