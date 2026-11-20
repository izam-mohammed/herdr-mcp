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

/// Refusal code for steering the server's own agent.
pub const OWN_AGENT: &str = "own_agent";

/// What an action would hit: a close of a tab or workspace holding the own
/// pane, or input to a pane or agent that is the own pane. `Agent` carries the
/// pane id `agent.get` resolved, so a name or a pane move cannot get past.
pub enum Target<'a> {
    Tab(&'a str),
    Workspace(&'a str),
    Pane(&'a str),
    Agent { pane: &'a str, name: &'a str },
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
        self.self_refusal(target, allow_self)
            .await
            .err()
            .map(Failure::into_result)
    }

    /// `refuse_self` as a `Failure`, for the tools that already return one.
    pub(crate) async fn self_refusal(
        &self,
        target: &Target<'_>,
        allow_self: bool,
    ) -> Result<(), Failure> {
        let Some(own) = self.shared.config.pane_id.as_deref() else {
            return Ok(());
        };
        if allow_self {
            return Ok(());
        }
        // The cheap case needs no call: the target is the pane id given at start.
        let direct =
            matches!(target, Target::Pane(id) | Target::Agent { pane: id, .. } if *id == own);
        if direct || self.holds_me(target).await? {
            return Err(refusal(target));
        }
        Ok(())
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
            (Some(p), Target::Pane(id) | Target::Agent { pane: id, .. }) => p["pane_id"] == *id,
        })
    }
}

fn refusal(target: &Target<'_>) -> Failure {
    match target {
        Target::Tab(id) | Target::Workspace(id) => {
            let what = if matches!(target, Target::Tab(_)) {
                "tab"
            } else {
                "workspace"
            };
            Failure::new(
                SELF_TARGET,
                format!(
                    "{what} {id} holds the pane this server runs in; closing it would \
end this session. Pass allow_self: true only if the user asked for exactly that."
                ),
            )
        }
        Target::Pane(id) => Failure::new(
            SELF_TARGET,
            format!("{id} is your own pane; act on another pane"),
        ),
        Target::Agent { name, .. } => Failure::new(
            OWN_AGENT,
            format!("{name} is the agent running this server; refusing to steer itself"),
        ),
    }
}

/// Resolve an agent by name or pane id with `agent.get`, refuse when it is the
/// server's own, and return the resolved pane id to send to, so a rename
/// between check and send cannot redirect the input to another agent.
///
/// # Errors
/// The refusal (`own_agent`) or herdr's failure, as a tool result.
pub async fn refuse_self_agent(server: &Server, target: &str) -> Result<String, CallToolResult> {
    if server.shared.config.pane_id.is_none() {
        return Ok(target.to_owned());
    }
    let agent = server
        .shared
        .client
        .call("agent.get", json!({"target": target}))
        .await
        .map_err(Failure::into_result)?;
    let pane = agent["agent"]["pane_id"].as_str().unwrap_or(target);
    server
        .self_refusal(&Target::Agent { pane, name: target }, false)
        .await
        .map_err(Failure::into_result)?;
    Ok(pane.to_owned())
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
