//! The tool registry. A new group is one `mod` line and one `GROUPS` line;
//! the group lives in its own file and exposes `router()`.
use crate::server::Server;
use rmcp::handler::server::router::tool::ToolRouter;

mod admin;
mod agents;
mod events;
mod layout;
mod notify;
mod panes;
mod session;
mod tabs;
mod workspaces;
mod worktrees;

/// Group name (as in `--tools`) and the function that builds its router.
type Build = fn() -> ToolRouter<Server>;

const GROUPS: &[(&str, Build)] = &[
    ("session", session::router),
    ("workspaces", workspaces::router),
    ("tabs", tabs::router),
    ("panes", panes::router),
    ("agents", agents::router),
    ("events", events::router),
    ("worktrees", worktrees::router),
    ("layout", layout::router),
    ("notify", notify::router),
    ("admin", admin::router),
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
