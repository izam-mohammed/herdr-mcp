# Review of plans/08-release-v0.1.0.md, phases 1 and 2 (pre-tag)

Reviewed 20/11/2026 with dist 0.34.0, before any tag exists. The release
workflow, the installer against a real release and `cargo binstall` have not
run; this records what was checked locally and what the try must still prove.
Tool facts: `docs/external/dist.md`.

## Scope

`dist-workspace.toml`, `.github/workflows/release.yml`,
`.github/workflows/check.yml` (gained `workflow_call`), `Cargo.toml`
(`repository`, `readme`, `[profile.dist]`, `[package.metadata.binstall]`),
`CHANGELOG.md`, the README's Install section.

## Measured

- `dist plan` lists the six targets, the shell installer, `sha256.sum`, a
  `.sha256` per archive and the source tarball; the release body is the
  `0.1.0` section of `CHANGELOG.md`.
- `dist build --artifacts=local --target x86_64-unknown-linux-gnu` built the
  archive; its `.sha256` verified with `sha256sum -c`; the extracted binary
  answers `herdr-mcp --version`. `dist build --artifacts=global` built the
  installer and `sha256.sum`.
- `release.yml` and `check.yml` parse as YAML. `cargo binstall --manifest-path
  . --dry-run` reads the binstall metadata (the download itself needs a
  release).

## Review of permissions

- The generated file had `contents: write` for every job. `dist` 0.34.0
  cannot narrow it, and its custom plan jobs render `uses: $/...`, which is
  invalid. The workflow therefore carries two hand edits with
  `allow-dirty = ["ci"]`: a `check` job first, and `contents: write` on the
  `host` job only (`docs/releases.md`, "Hand edits to release.yml").
- Reading the file: `plan`, `build-local-artifacts`, `build-global-artifacts`
  and `announce` need no write (they upload workflow artifacts and read the
  repository); `host` creates the release with `gh release create`, which
  needs `contents: write`. Checkouts use `persist-credentials: false`.
- The `host` job's `if` requires `plan` to have succeeded, and `plan` needs
  `check`, so a red check stops the release before any build.
- Risk: the hand edits are not guarded by `dist generate --check`. A
  `dist` upgrade must be diffed by hand.
- Third-party actions are the ones `dist` emits (`actions/checkout`,
  `upload-artifact`, `download-artifact`), pinned by tag, not commit. `dist`'s
  `github-action-commits` could pin them; not done for 0.1.0.

## Review of the installer

- The shell installer downloads the archive for the host from the release
  URL and installs to `~/.cargo/bin`. The copy built locally had no checksum
  embedded (it prints "no checksums to verify"), because only one target's
  archive existed; in CI the global job runs after every local build and
  should embed them. Unproven until the try: read the installer's output
  for a checksum line, and if it still says none, the README must not claim
  the installer verifies anything (it does not today).
- It cannot tell musl from glibc and does not work against a private
  repository (no token). Documented in the README.

## Still to prove (needs a tag)

The `v0.1.0-rc.1` run, a release listing of every target with checksums, the
installer on a machine without Rust, and `claude mcp add herdr --
herdr-mcp` after it.
