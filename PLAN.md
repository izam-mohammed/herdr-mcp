# herdr-mcp: what is left to build, in order

The spec is `features.md`. Each plan below is built phase by phase and
ticked here when all its boxes are.

### Phase 1: v0.1.0, the first version you can try

- [x] `plans/12-repo-hygiene.md` - a security policy and an MIT licence.
- [ ] `plans/01-first-version.md` - the Rust server, the safety defaults,
  and tab management from Claude Code.
- [ ] `plans/09-quality-gates.md` - a 90 % coverage floor in the check and
  the shared security gate on every PR, before any later plan lands.
- [ ] `plans/08-release-v0.1.0.md` - one-command install without a Rust
  toolchain, and `v0.1.0` released.

### Phase 2: v0.2.0, control the rest of herdr

- [ ] `plans/02-workspaces-and-tabs.md` - workspaces and tabs in full, and
  never closing its own.
- [ ] `plans/03-panes.md` - split, run, read and wait on panes.
- [ ] `plans/04-agents.md` - read, prompt and wait on other agents.
- [ ] `plans/05-events.md` - what changed since last time, and waiting for
  the next change.
- [ ] `plans/13-release-v0.2.0.md` - `v0.2.0` released.

### Phase 3: v0.3.0, the remaining surface

- [ ] `plans/06-worktrees-layout-notify.md` - worktrees, layouts and
  toasts.
- [ ] `plans/07-resources-and-prompts.md` - `@herdr:` resources and the
  dashboard prompts.
- [ ] `plans/14-release-v0.3.0.md` - `v0.3.0` released.

### Phase 4: v0.4.0, a page and docs for people

- [ ] `plans/10-site.md` - a one-page site on GitHub Pages, once there is
  something to install and the repository is public.
- [ ] `plans/11-docs.md` - an mdBook documentation site at `/docs/`, with
  the tool, resource and prompt reference generated from the server.
- [ ] `plans/15-release-v0.4.0.md` - `v0.4.0` released.
