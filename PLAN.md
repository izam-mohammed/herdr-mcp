# herdr-mcp: what is left to build, in order

The spec is `features.md`. Each plan below is built phase by phase and
ticked here when all its boxes are.

### Phase 1: the first version you can try

- [ ] `plans/01-first-version.md` - the Rust server, the safety defaults,
  and tab management from Claude Code.
- [ ] `plans/09-quality-gates.md` - a 90 % coverage floor in the check and
  the shared security gate on every PR, before any later plan lands.

### Phase 2: control the rest of herdr

- [ ] `plans/02-workspaces-and-tabs.md` - workspaces and tabs in full, and
  never closing its own.
- [ ] `plans/03-panes.md` - split, run, read and wait on panes.
- [ ] `plans/04-agents.md` - read, prompt and wait on other agents.

### Phase 3: events

- [ ] `plans/05-events.md` - what changed since last time, and waiting for
  the next change.

### Phase 4: the remaining surface

- [ ] `plans/06-worktrees-layout-notify.md` - worktrees, layouts and
  toasts.
- [ ] `plans/07-resources-and-prompts.md` - `@herdr:` resources and the
  dashboard prompts.

### Phase 5: ship it

- [ ] `plans/08-release.md` - one-command install without a Rust
  toolchain.

### Phase 6: a page for people

- [ ] `plans/10-site.md` - a one-page site on GitHub Pages, once there is
  something to install and the repository is public.
