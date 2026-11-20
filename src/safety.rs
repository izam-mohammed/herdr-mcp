//! The read-only gate. Write tools are not registered at all in read-only
//! mode, which is stronger than refusing the call: the client never sees them.
use rmcp::handler::server::router::tool::ToolRouter;

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

/// Code of the refusal to steer the server's own agent.
pub const OWN_AGENT: &str = "own_agent";

/// Check that `target` is not the server's own agent, by pane id, by name or
/// after a pane move (plan 04, decision 1). Resolves it with `agent.get`,
/// then compares its pane id and `terminal_id` with the own pane. Fails
/// closed: if herdr cannot resolve the target or the own pane, that failure
/// is the `Err` and nothing is sent. `Ok` is the target to send to: the
/// resolved pane id, so a rename between check and send cannot redirect the
/// input to another agent.
///
/// # Errors
/// The refusal (`own_agent`) or herdr's failure, as a tool result.
pub async fn refuse_self_agent(
    server: &crate::server::Server,
    target: &str,
) -> Result<String, rmcp::model::CallToolResult> {
    use crate::error::Failure;
    use serde_json::{Value, json};
    let Some(own) = server.shared.config.pane_id.as_deref() else {
        return Ok(target.to_owned());
    };
    let client = &server.shared.client;
    let agent = client
        .call("agent.get", json!({"target": target}))
        .await
        .map_err(Failure::into_result)?;
    let field = |k: &str| {
        agent
            .pointer(&format!("/agent/{k}"))
            .and_then(Value::as_str)
            .map(str::to_owned)
    };
    let refuse = || {
        Failure::new(
            OWN_AGENT,
            format!("{target} is the agent running this server; refusing to steer itself"),
        )
        .into_result()
    };
    let pane = field("pane_id");
    if pane.as_deref() == Some(own) {
        return Err(refuse());
    }
    if let Some(theirs) = field("terminal_id") {
        let mine = client
            .call("pane.get", json!({"pane_id": own}))
            .await
            .map_err(Failure::into_result)?;
        if mine.pointer("/pane/terminal_id").and_then(Value::as_str) == Some(theirs.as_str()) {
            return Err(refuse());
        }
    }
    Ok(pane.unwrap_or_else(|| target.to_owned()))
}
