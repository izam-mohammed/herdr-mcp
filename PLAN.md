# herdr-mcp: what is left to build, in order

The spec is `features.md`. Each plan below is built phase by phase and
ticked here when all its boxes are.

### Phase 1: v0.1.0, the first version you can try

- [x] `plans/12-repo-hygiene.md` - a security policy and an MIT licence.
- [x] `plans/01-first-version.md` - the Rust server, the safety defaults,
  and tab management from Claude Code.
- [x] `plans/09-quality-gates.md` - a 90 % coverage floor in the check and
  the shared security gate on every PR, before any later plan lands.
- [x] `plans/08-release-v0.1.0.md` - one-command install without a Rust
  toolchain, and `v0.1.0` released.

### Phase 2: v0.2.0, control the rest of herdr

- [x] `plans/02-workspaces-and-tabs.md` - workspaces and tabs in full, and
  never closing its own.
- [x] `plans/03-panes.md` - split, run, read and wait on panes.
- [x] `plans/04-agents.md` - read, prompt and wait on other agents.
- [x] `plans/05-events.md` - what changed since last time, and waiting for
  the next change.
- [x] `plans/13-release-v0.2.0.md` - `v0.2.0` released. (folded into `v0.1.0`, no release cut)

### Phase 3: v0.3.0, the remaining surface

- [x] `plans/06-worktrees-layout-notify.md` - worktrees, layouts and
  toasts.
- [x] `plans/07-resources-and-prompts.md` - `@herdr:` resources and the
  dashboard prompts.
- [x] `plans/14-release-v0.3.0.md` - `v0.3.0` released. (folded into `v0.1.0`, no release cut)

### Phase 4: v0.4.0, a page and docs for people

- [x] `plans/10-site.md` - a one-page site on GitHub Pages, once there is
  something to install and the repository is public.
- [x] `plans/11-docs.md` - an mdBook documentation site at `/docs/`, with
  the tool, resource and prompt reference generated from the server.
- [x] `plans/15-release-v0.4.0.md` - `v0.4.0` released. (folded into `v0.1.0`, no release cut)

### Phase 5: v0.2.0, the herdr app as well as the herdr TUI

- [ ] `plans/16-herdr-app-backend.md` - one server drives the herdr desktop
  app and the herdr TUI, ids routed by shape.
- [ ] `plans/17-release-v0.2.0.md` - `v0.2.0` released.
