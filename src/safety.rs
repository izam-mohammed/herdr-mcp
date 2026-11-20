//! The read-only gate. Write tools are not registered at all in read-only
//! mode, which is stronger than refusing the call: the client never sees them.
use crate::error::Failure;
use crate::server::Server;
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::model::CallToolResult;
use serde_json::json;

/// Remove every tool not marked `readOnlyHint = true`.
pub fn keep_reads_only<S: Send + Sync + 'static>(router: &mut ToolRouter<S>) {
    let writes: Vec<String> = router
        .list_all()
        .into_iter()
        .filter(|t| t.annotations.as_ref().and_then(|a| a.read_only_hint) != Some(true))
        .map(|t| t.name.to_string())
        .collect();
    for name in writes {
        router.remove_route(&name);
    }
}

/// Refusal code for a close that would remove the server's own place.
pub const SELF_TARGET: &str = "self_target";

/// The server's own pane, by `terminal_id` (plan 02, decision 1): captured
/// at the first close (not at start, so a start sends nothing), then looked up in `pane.list` per check, so it follows the pane
/// when it moves to another workspace.
#[derive(Default)]
pub struct OwnPane {
    terminal: std::sync::Mutex<Option<String>>,
}

/// What a close would remove.
pub enum Target<'a> {
    Tab(&'a str),
    Workspace(&'a str),
}

impl Server {
    async fn own_terminal(&self) -> Result<String, Failure> {
        let cached = self.shared.own.terminal.lock().expect("own pane").clone();
        if let Some(t) = cached {
            return Ok(t);
        }
        let id = self.shared.config.pane_id.clone().unwrap_or_default();
        let current = self
            .shared
            .client
            .call("pane.current", json!({"caller_pane_id": id}))
            .await?;
        let terminal = current["pane"]["terminal_id"].as_str().ok_or_else(|| {
            Failure::new(
                SELF_TARGET,
                "cannot tell which pane is mine (no terminal_id), so the close is refused",
            )
        })?;
        *self.shared.own.terminal.lock().expect("own pane") = Some(terminal.to_owned());
        Ok(terminal.to_owned())
    }

    /// `Some(refusal)` when a close of `target` would remove the tab or
    /// workspace holding this server's pane and `allow_self` is not set.
    /// Outside herdr (no pane id) there is no self. A pane that is gone has
    /// nothing to protect. Any failure to find out refuses (fail closed).
    pub(crate) async fn refuse_self(
        &self,
        target: &Target<'_>,
        allow_self: bool,
    ) -> Option<CallToolResult> {
        if allow_self || self.shared.config.pane_id.is_none() {
            return None;
        }
        match self.holds_me(target).await {
            Ok(false) => None,
            Ok(true) => {
                let (what, id) = match target {
                    Target::Tab(id) => ("tab", id),
                    Target::Workspace(id) => ("workspace", id),
                };
                Some(
                    Failure::new(
                        SELF_TARGET,
                        format!(
                            "{what} {id} holds the pane this server runs in; closing it would \
end this session. Pass allow_self: true only if the user asked for exactly that."
                        ),
                    )
                    .into_result(),
                )
            }
            Err(f) => Some(f.into_result()),
        }
    }

    async fn holds_me(&self, target: &Target<'_>) -> Result<bool, Failure> {
        let terminal = self.own_terminal().await?;
        let list = self.shared.client.call("pane.list", json!({})).await?;
        let mine = list["panes"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|p| p["terminal_id"] == terminal.as_str());
        Ok(match (mine, target) {
            (None, _) => false,
            (Some(p), Target::Tab(id)) => p["tab_id"] == *id,
            (Some(p), Target::Workspace(id)) => p["workspace_id"] == *id,
        })
    }
}

/// `--confirm-destructive`: every `destructiveHint` tool also asks the user
/// each time, even in auto mode (Claude Code's `requiresUserInteraction`).
pub fn require_confirmation<S: Send + Sync + 'static>(router: &mut ToolRouter<S>) {
    for route in router.map.values_mut() {
        if route
            .attr
            .annotations
            .as_ref()
            .and_then(|a| a.destructive_hint)
            == Some(true)
        {
            let mut meta = route.attr.meta.take().unwrap_or_default();
            meta.0
                .insert("anthropic/requiresUserInteraction".into(), json!(true));
            route.attr.meta = Some(meta);
        }
    }
}
