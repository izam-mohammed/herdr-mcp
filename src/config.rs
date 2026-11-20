//! Flags and environment, resolved once at start. Flag first, then env.
use std::path::PathBuf;
use std::time::Duration;

pub const GROUPS: &[&str] = &[
    "session",
    "workspaces",
    "tabs",
    "panes",
    "agents",
    "events",
    "worktrees",
    "layout",
    "notify",
];

const USAGE: &str = "herdr-mcp: an MCP server for herdr, over stdio

  --session <name>        herdr session (HERDR_SESSION)
  --socket <path>         herdr socket (HERDR_SOCKET_PATH)
  --read-only             register read tools only (HERDR_MCP_READ_ONLY=1)
  --allow-outside-herdr   allow writes when HERDR_ENV is not 1 (HERDR_MCP_ALLOW_OUTSIDE=1)
  --tools <a,b,...>       tool groups to register, default all (HERDR_MCP_TOOLS)
  --debug                 log payloads to stderr
  --version, --help";

#[derive(Debug, Clone)]
pub struct Config {
    pub socket: PathBuf,
    pub read_only: bool,
    pub groups: Vec<String>,
    pub debug: bool,
    pub timeout: Duration,
    pub pane_id: Option<String>,
    pub tab_id: Option<String>,
    pub workspace_id: Option<String>,
}

/// What the caller asked for on the command line and in the environment.
#[derive(Debug, Default)]
pub struct Env {
    pub vars: Vec<(String, String)>,
    pub home: PathBuf,
}

impl Env {
    #[must_use]
    pub fn from_process() -> Self {
        Self {
            vars: std::env::vars().collect(),
            home: std::env::var_os("HOME").map_or_else(PathBuf::new, PathBuf::from),
        }
    }

    fn get(&self, key: &str) -> Option<&str> {
        self.vars
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
            .filter(|v| !v.is_empty())
    }

    fn flag(&self, key: &str) -> bool {
        self.get(key) == Some("1")
    }
}

pub enum Parsed {
    Run(Box<Config>),
    Exit(String),
}

/// Parse `args` (without the program name) against `env`.
pub fn parse(args: &[String], env: &Env) -> Result<Parsed, String> {
    let (mut session, mut socket) = (None, None);
    let mut read_only = env.flag("HERDR_MCP_READ_ONLY");
    let mut allow_outside = env.flag("HERDR_MCP_ALLOW_OUTSIDE");
    let mut tools = env.get("HERDR_MCP_TOOLS").map(str::to_owned);
    let mut debug = false;

    let mut it = args.iter();
    while let Some(arg) = it.next() {
        let (name, inline) = match arg.split_once('=') {
            Some((n, v)) if n.starts_with("--") => (n, Some(v.to_owned())),
            _ => (arg.as_str(), None),
        };
        let value = |it: &mut std::slice::Iter<String>| {
            inline
                .clone()
                .or_else(|| it.next().cloned())
                .ok_or_else(|| format!("{name} needs a value"))
        };
        match name {
            "--session" => session = Some(value(&mut it)?),
            "--socket" => socket = Some(value(&mut it)?),
            "--tools" => tools = Some(value(&mut it)?),
            "--read-only" => read_only = true,
            "--allow-outside-herdr" => allow_outside = true,
            "--debug" => debug = true,
            "--help" | "-h" => return Ok(Parsed::Exit(USAGE.to_owned())),
            "--version" | "-V" => {
                return Ok(Parsed::Exit(format!(
                    "herdr-mcp {}",
                    env!("CARGO_PKG_VERSION")
                )));
            }
            other => return Err(format!("unknown argument {other}\n\n{USAGE}")),
        }
    }

    let groups = match tools {
        None => GROUPS.iter().map(|g| (*g).to_owned()).collect(),
        Some(list) => {
            let mut groups = Vec::new();
            for g in list.split(',').map(str::trim).filter(|g| !g.is_empty()) {
                if !GROUPS.contains(&g) {
                    return Err(format!(
                        "unknown tool group {g} (groups: {})",
                        GROUPS.join(", ")
                    ));
                }
                groups.push(g.to_owned());
            }
            groups
        }
    };

    let inside = env.flag("HERDR_ENV");
    Ok(Parsed::Run(Box::new(Config {
        socket: resolve_socket(session, socket, env),
        read_only: read_only || (!inside && !allow_outside),
        groups,
        debug,
        timeout: Duration::from_millis(
            env.get("HERDR_MCP_TIMEOUT_MS")
                .and_then(|v| v.parse().ok())
                .unwrap_or(10_000),
        ),
        pane_id: env.get("HERDR_PANE_ID").map(str::to_owned),
        tab_id: env.get("HERDR_TAB_ID").map(str::to_owned),
        workspace_id: env.get("HERDR_WORKSPACE_ID").map(str::to_owned),
    })))
}

/// herdr's order, with the flags first.
fn resolve_socket(session: Option<String>, socket: Option<String>, env: &Env) -> PathBuf {
    let sessions = |name: &str| {
        env.home
            .join(".config/herdr/sessions")
            .join(name)
            .join("herdr.sock")
    };
    if let Some(path) = socket {
        PathBuf::from(path)
    } else if let Some(name) = session {
        sessions(&name)
    } else if let Some(path) = env.get("HERDR_SOCKET_PATH") {
        PathBuf::from(path)
    } else if let Some(name) = env.get("HERDR_SESSION") {
        sessions(name)
    } else {
        env.home.join(".config/herdr/herdr.sock")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(pairs: &[(&str, &str)]) -> Env {
        Env {
            vars: pairs
                .iter()
                .map(|(k, v)| ((*k).into(), (*v).into()))
                .collect(),
            home: PathBuf::from("/h"),
        }
    }

    fn run(args: &[&str], e: &Env) -> Config {
        let args: Vec<String> = args.iter().map(|a| (*a).to_owned()).collect();
        match parse(&args, e) {
            Ok(Parsed::Run(c)) => *c,
            _ => panic!("expected a config"),
        }
    }

    #[test]
    fn socket_order_is_flags_then_herdrs_own() {
        let e = env(&[("HERDR_SOCKET_PATH", "/s/env.sock"), ("HERDR_SESSION", "x")]);
        assert_eq!(
            run(&["--socket", "/s/a.sock"], &e).socket,
            PathBuf::from("/s/a.sock")
        );
        assert_eq!(
            run(&["--session=t"], &e).socket,
            PathBuf::from("/h/.config/herdr/sessions/t/herdr.sock")
        );
        assert_eq!(run(&[], &e).socket, PathBuf::from("/s/env.sock"));
        assert_eq!(
            run(&[], &env(&[("HERDR_SESSION", "x")])).socket,
            PathBuf::from("/h/.config/herdr/sessions/x/herdr.sock")
        );
        assert_eq!(
            run(&[], &env(&[])).socket,
            PathBuf::from("/h/.config/herdr/herdr.sock")
        );
    }

    #[test]
    fn read_only_unless_inside_herdr() {
        assert!(run(&[], &env(&[])).read_only);
        assert!(!run(&[], &env(&[("HERDR_ENV", "1")])).read_only);
        assert!(!run(&["--allow-outside-herdr"], &env(&[])).read_only);
        assert!(run(&["--read-only"], &env(&[("HERDR_ENV", "1")])).read_only);
        assert!(
            run(
                &[],
                &env(&[("HERDR_ENV", "1"), ("HERDR_MCP_READ_ONLY", "1")])
            )
            .read_only
        );
    }

    #[test]
    fn tool_groups_are_checked() {
        assert_eq!(
            run(&["--tools", "tabs,session"], &env(&[])).groups,
            ["tabs", "session"]
        );
        assert_eq!(run(&[], &env(&[])).groups.len(), GROUPS.len());
        assert!(parse(&["--tools".into(), "nope".into()], &env(&[])).is_err());
        assert!(parse(&["--bogus".into()], &env(&[])).is_err());
        assert!(parse(&["--session".into()], &env(&[])).is_err());
    }
}
