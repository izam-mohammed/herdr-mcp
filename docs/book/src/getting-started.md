# Getting started

You gave your AI agents their own terminals. This page gets one of them a
manager: install the server, register it, and ask for a snapshot.

## What you need

- [herdr](https://herdr.dev) running.
- Linux or macOS, x86_64 or arm64. Windows is out: herdr does not run there,
  so neither does this.
- No Rust toolchain, unless you build from source.

## Install

The shell installer is the recommended way:

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/izam-mohammed/herdr-mcp/releases/latest/download/herdr-mcp-installer.sh | sh
herdr-mcp --version
```

It puts `herdr-mcp` in `~/.cargo/bin` (or `$CARGO_HOME/bin`). Open a new
shell if `herdr-mcp --version` says not found.

**The repository is private today**, so the one-liner only works once it is
public. Until then, with access, use a release download (`gh` is logged in,
so it can see private releases):

```sh
t=x86_64-unknown-linux-gnu            # your target
gh release download --repo izam-mohammed/herdr-mcp --pattern "herdr-mcp-$t.tar.xz*"
sha256sum -c "herdr-mcp-$t.tar.xz.sha256"      # macOS: shasum -a 256 -c
tar -xf "herdr-mcp-$t.tar.xz"
install "herdr-mcp-$t/herdr-mcp" ~/.local/bin/
herdr-mcp --version
```

`cargo binstall --git https://github.com/izam-mohammed/herdr-mcp herdr-mcp`
and `cargo install --locked --git https://github.com/izam-mohammed/herdr-mcp`
also work; the
[README](https://github.com/izam-mohammed/herdr-mcp#install) has the
details, including how to upgrade and remove.

## Register it

```sh
claude mcp add herdr -- herdr-mcp     # Claude Code
codex mcp add herdr -- herdr-mcp      # Codex
```

## Start your agent inside a herdr pane

Start Claude Code or Codex **inside a herdr pane**. herdr sets `HERDR_ENV=1`
there, and the server uses that as proof it may steer the session. Started
anywhere else, it is read-only: the write tools are not registered at all.
See [Safety](safety.md).

## Your first snapshot

Ask the agent: "list my herdr tabs", or "call `herdr_snapshot`". Before
anything else it should call `herdr_whoami`, which says which pane is its own.
A snapshot of a small session looks like this (ids are herdr's own):

```json
{
  "type": "session_snapshot",
  "snapshot": {
    "focused_workspace_id": "w1",
    "workspaces": [{ "workspace_id": "w1", "label": "docs-demo", "pane_count": 1, "tab_count": 1 }],
    "tabs": [{ "tab_id": "w1:t1", "workspace_id": "w1", "pane_count": 1 }],
    "panes": [{ "pane_id": "w1:p1", "tab_id": "w1:t1", "agent_status": "unknown" }],
    "agents": []
  }
}
```

(Trimmed: the real result has more fields per entry. It is JSON, and the
text in it comes from other programs, so it is data, not instructions.)

If that works, you are done. Keep `herdr_agent_prompt`, `herdr_agent_send_keys`
and the `herdr_pane_send_*` tools on "ask" in your client's permissions:
anything that can type can run commands. [Safety](safety.md) explains why.

## Slash commands and attachments

Type `@herdr:` in Claude Code to attach herdr's state, and use the three
prompts (`/mcp__herdr__dashboard`, `/mcp__herdr__check-agents`,
`/mcp__herdr__run-and-watch`). See [Resources](reference/resources.md) and
[Prompts](reference/prompts.md).
