//! The tool registry. A new group is one `mod` line and one `GROUPS` line;
//! the group lives in its own file and exposes `router()`.
use crate::server::Server;
use rmcp::handler::server::router::tool::ToolRouter;

mod agents;
mod events;
mod session;
mod tabs;
mod workspaces;

/// Group name (as in `--tools`) and the function that builds its router.
type Build = fn() -> ToolRouter<Server>;

const GROUPS: &[(&str, Build)] = &[
    ("session", session::router),
    ("workspaces", workspaces::router),
    ("tabs", tabs::router),
    ("agents", agents::router),
    ("events", events::router),
];

/// The routers of the groups asked for.
pub fn router(groups: &[String]) -> ToolRouter<Server> {
    let mut all = ToolRouter::new();
    for (name, build) in GROUPS {
        if groups.iter().any(|g| g == name) {
            all += build();
        }
    }
    all
}
