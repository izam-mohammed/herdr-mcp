//! Group `notify`: a toast in herdr. Title and body are passed as given; herdr
//! shows them as a notification, so the user sees who wrote them (the model).
use crate::server::Server;
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::{tool, tool_router};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub fn router() -> ToolRouter<Server> {
    Server::notify_router()
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
enum Position {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
enum Sound {
    None,
    Done,
    Request,
}

#[derive(Serialize, Deserialize, JsonSchema)]
struct Notify {
    /// The toast's title, e.g. `tests passed in w3`.
    title: String,
    /// More text under the title.
    #[serde(skip_serializing_if = "Option::is_none")]
    body: Option<String>,
    /// Where the toast shows. Omit for herdr's default.
    #[serde(skip_serializing_if = "Option::is_none")]
    position: Option<Position>,
    /// A sound to play: `none`, `done` or `request`. Omit for herdr's default.
    #[serde(skip_serializing_if = "Option::is_none")]
    sound: Option<Sound>,
}

#[tool_router(router = notify_router)]
impl Server {
    #[tool(
        description = "Show a toast in herdr, e.g. when a long task is ready. It changes no \
panes, tabs or files. Keep it short and honest: the user reads it as a message from you.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    async fn herdr_notify(&self, Parameters(p): Parameters<Notify>) -> CallToolResult {
        self.herdr("notification.show", p).await
    }
}
