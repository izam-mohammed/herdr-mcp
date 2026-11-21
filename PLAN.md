# herdr-mcp: what is left to build, in order

The spec is `features.md`. Each plan below is built phase by phase and
ticked here when all its boxes are.

### Phase 1: v0.1.0, the one release

- [x] `plans/12-repo-hygiene.md` - a security policy and an MIT licence.
- [x] `plans/01-first-version.md` - the Rust server, the safety defaults,
  and tab management from Claude Code.
- [x] `plans/02-workspaces-and-tabs.md` - workspaces and tabs in full, and
  never closing its own.
- [x] `plans/03-panes.md` - split, run, read and wait on panes.
- [x] `plans/04-agents.md` - read, prompt and wait on other agents.
- [x] `plans/05-events.md` - what changed since last time, and waiting for
  the next change.
- [x] `plans/06-worktrees-layout-notify.md` - worktrees, layouts and
  toasts.
- [x] `plans/07-resources-and-prompts.md` - `@herdr:` resources and the
  dashboard prompts.
- [x] `plans/09-quality-gates.md` - a 90 % coverage floor in the check and
  the shared security gate on every PR.
- [x] `plans/10-site.md` - a one-page site on GitHub Pages.
- [x] `plans/11-docs.md` - an mdBook documentation site at `/docs/`, with
  the tool, resource and prompt reference generated from the server.
- [x] `plans/08-release-v0.1.0.md` - one-command install without a Rust
  toolchain, and `v0.1.0` released.

### Phase 2: next, every herdr command as a tool

- [ ] `plans/13-cli-parity.md` - every herdr command is also an MCP tool,
  and a check says when herdr gets a new one.
