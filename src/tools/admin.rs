//! Group `admin` (off by default): the socket-backed herdr administration methods.
//! Integrations, config and manifest reloads, stopping the server, pane input
//! routing and agent state reports. Every tool that changes something is marked
//! destructive, so the client asks and read-only mode hides it. Stopping the
//! server refuses the caller's own session unless `allow_self` is set.
use crate::server::{NoParams, Server};
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::{tool, tool_router};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub fn router() -> ToolRouter<Server> {
    let mut all = Server::admin_router();
    all += super::admin_cli::router();
    all
}

#[derive(Serialize, Deserialize, JsonSchema)]
struct Integration {
    /// The agent to integrate with, as `herdr_integration_list` names it, e.g. `claude`, `codex`, `pi`.
    target: String,
}

#[derive(Deserialize, JsonSchema)]
struct ServerStop {
    /// Stop the server even though it is the one this server runs in, which ends every
    /// pane in that session, yours included. Only when the user asked for exactly that.
    allow_self: Option<bool>,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Copy)]
#[serde(rename_all = "snake_case")]
enum Click {
    /// A right click opens herdr's own menu.
    Herdr,
    /// A right click goes to the program in the pane.
    Pane,
}

#[derive(Serialize, Deserialize, JsonSchema)]
struct SetInput {
    /// Pane id, e.g. `wA:p2`.
    pane_id: String,
    /// Where a right click in the pane is routed.
    right_click: Click,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Copy)]
#[serde(rename_all = "snake_case")]
enum AgentState {
    Idle,
    Working,
    Blocked,
    Unknown,
}

#[derive(Serialize, Deserialize, JsonSchema)]
struct ReportAgent {
    /// Pane id, e.g. `wA:p2`.
    pane_id: String,
    /// Who reports, e.g. `herdr-mcp`. The same source must release it later.
    source: String,
    /// The agent's name, e.g. `claude`.
    agent: String,
    /// The state to show for the agent.
    state: AgentState,
    /// A message shown with the state.
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
    /// The agent's own session id.
    #[serde(skip_serializing_if = "Option::is_none")]
    agent_session_id: Option<String>,
    /// The path of the agent's session file.
    #[serde(skip_serializing_if = "Option::is_none")]
    agent_session_path: Option<String>,
    /// Command that resumes the agent after a herdr restart; the first item must be a plain command name.
    #[serde(skip_serializing_if = "Option::is_none")]
    resume_argv: Option<Vec<String>>,
    /// Sequence number; herdr ignores a report older than one it has.
    #[serde(skip_serializing_if = "Option::is_none")]
    seq: Option<u64>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
struct ReportSession {
    /// Pane id, e.g. `wA:p2`.
    pane_id: String,
    /// Who reports, e.g. `herdr-mcp`.
    source: String,
    /// The agent's name, e.g. `claude`.
    agent: String,
    /// The agent's own session id.
    #[serde(skip_serializing_if = "Option::is_none")]
    agent_session_id: Option<String>,
    /// The path of the agent's session file.
    #[serde(skip_serializing_if = "Option::is_none")]
    agent_session_path: Option<String>,
    /// Command that resumes the agent after a herdr restart; the first item must be a plain command name.
    #[serde(skip_serializing_if = "Option::is_none")]
    resume_argv: Option<Vec<String>>,
    /// How the session started, as the agent names it.
    #[serde(skip_serializing_if = "Option::is_none")]
    session_start_source: Option<String>,
    /// Sequence number; herdr ignores a report older than one it has.
    #[serde(skip_serializing_if = "Option::is_none")]
    seq: Option<u64>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
struct ReleaseAgent {
    /// Pane id, e.g. `wA:p2`.
    pane_id: String,
    /// The source that reported the agent.
    source: String,
    /// The agent's name.
    agent: String,
    /// Sequence number; herdr ignores a release older than a report it has.
    #[serde(skip_serializing_if = "Option::is_none")]
    seq: Option<u64>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
struct ClearAuthority {
    /// Pane id, e.g. `wA:p2`.
    pane_id: String,
    /// Clear only what this source holds. Omit to clear all.
    #[serde(skip_serializing_if = "Option::is_none")]
    source: Option<String>,
    /// Sequence number.
    #[serde(skip_serializing_if = "Option::is_none")]
    seq: Option<u64>,
}

#[tool_router(router = admin_router)]
impl Server {
    #[tool(
        description = "List herdr's built-in agent integrations and whether each is installed, \
current or outdated.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn herdr_integration_list(&self) -> CallToolResult {
        self.herdr("integration.list", NoParams {}).await
    }

    #[tool(
        description = "Install herdr's integration for an agent: writes a hook or plugin into \
that agent's own config directory. The reply lists what was written.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn herdr_integration_install(
        &self,
        Parameters(p): Parameters<Integration>,
    ) -> CallToolResult {
        self.herdr("integration.install", p).await
    }

    #[tool(
        description = "Remove herdr's integration for an agent from that agent's config \
directory. The reply lists what was removed.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn herdr_integration_uninstall(
        &self,
        Parameters(p): Parameters<Integration>,
    ) -> CallToolResult {
        self.herdr("integration.uninstall", p).await
    }

    #[tool(
        description = "Reload config.toml in the running herdr server. A broken file is \
reported, not applied.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn herdr_server_reload_config(&self) -> CallToolResult {
        self.herdr("server.reload_config", NoParams {}).await
    }

    #[tool(
        description = "Show the agent detection manifests herdr uses: which agent, from which \
source, which version.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn herdr_server_agent_manifests(&self) -> CallToolResult {
        self.herdr("server.agent_manifests", NoParams {}).await
    }

    #[tool(
        description = "Reload herdr's local agent detection manifest overrides, which changes \
how agents in every pane are detected.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn herdr_server_reload_agent_manifests(&self) -> CallToolResult {
        self.herdr("server.reload_agent_manifests", NoParams {})
            .await
    }

    #[tool(
        description = "Stop the herdr server this tool talks to: every pane and the programs \
in them end. Refused with self_target when that server is the one this MCP server runs in, \
unless allow_self is true (only if the user asked for exactly that).",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    async fn herdr_server_stop(&self, Parameters(p): Parameters<ServerStop>) -> CallToolResult {
        if let Err(refusal) = self.session_refusal(None, p.allow_self == Some(true)) {
            return refusal.into_result();
        }
        self.herdr("server.stop", NoParams {}).await
    }

    #[tool(
        description = "Choose where a right click in a pane goes: to herdr's menu or to the \
program in the pane.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn herdr_pane_set_input(&self, Parameters(p): Parameters<SetInput>) -> CallToolResult {
        self.herdr("pane.input.set", p).await
    }

    #[tool(
        description = "Report an agent's state for a pane, as an agent's own hook would. herdr \
then shows that state, so a wrong report is a false display. Use only for an agent you run.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn herdr_pane_report_agent(
        &self,
        Parameters(p): Parameters<ReportAgent>,
    ) -> CallToolResult {
        self.herdr("pane.report_agent", p).await
    }

    #[tool(
        description = "Report an agent's session identity for a pane (session id, file, how to \
resume it after a herdr restart).",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn herdr_pane_report_agent_session(
        &self,
        Parameters(p): Parameters<ReportSession>,
    ) -> CallToolResult {
        self.herdr("pane.report_agent_session", p).await
    }

    #[tool(
        description = "Release the agent state a source reported for a pane, back to herdr's own detection.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn herdr_pane_release_agent(
        &self,
        Parameters(p): Parameters<ReleaseAgent>,
    ) -> CallToolResult {
        self.herdr("pane.release_agent", p).await
    }

    #[tool(
        description = "Clear the authority over a pane's agent state that reporting sources hold.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn herdr_pane_clear_agent_authority(
        &self,
        Parameters(p): Parameters<ClearAuthority>,
    ) -> CallToolResult {
        self.herdr("pane.clear_agent_authority", p).await
    }
}
