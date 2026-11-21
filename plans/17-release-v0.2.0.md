# v0.2.0 is released: the herdr app as well as the herdr TUI

## The ask

`docs/releases.md` puts support for the herdr desktop app in `v0.2.0`. The pipeline exists since
`plans/08-release.md`, so this plan only confirms the previous release,
bumps the version and opens the PR.

## The decision

Bump `Cargo.toml` and `CHANGELOG.md` to `0.2.0`, merge, then push tag
`v0.2.0`; `release.yml` builds the release. No new dependency, permission or
channel. No rough cut: the release is the usable thing, and the installer's
`--version` is its try.

Standards: `docs/standards.md` (`sh scripts/check.sh`).

Security: no new surface; the release job's permissions are unchanged from
`plans/08-release.md`.

## Repository knowledge

Read root `KNOWLEDGE.md` when present before each phase. Follow the global and
repository instructions for recording useful development findings.

## Phases

### Phase 1: bump and check

- [ ] The `v0.1.0` release exists on GitHub with every target and `SHA256SUMS`
  (`gh release view v0.1.0`), and the plans listed for `v0.2.0` in
  `docs/releases.md` are all ticked in `PLAN.md`.
- [ ] `Cargo.toml` and `CHANGELOG.md` bumped to `0.2.0`, and `docs/releases.md`
  marks `v0.2.0` as released once the tag is pushed.
- [ ] Check: `sh scripts/check.sh` passes.

### Phase 2: open the PR

- [ ] The branch pushed and its PR opened against the branch it was cut
  from (or, if one is already open for this branch, its body brought up to
  date) with a Conventional Commit title for the whole change, a body naming
  this plan, what changed and why, and the checks run with their result. The
  PR's URL printed as the evidence. Opened, never merged.
