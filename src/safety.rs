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
