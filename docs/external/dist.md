# dist (cargo-dist), as this project uses it

Checked 20/11/2026 against dist 0.34.0 (released 2026-11-02, installed with
its shell installer). Sources: the book at
`github.com/axodotdev/cargo-dist/tree/v0.34.0/book/src` (`reference/config.md`,
`installers/shell.md`, `ci/customizing.md`), the CLI's own `--help`, the
template `cargo-dist/templates/ci/github/release.yml.j2` at v0.34.0, and
runs in this repo. `cargo binstall` 1.27.0: `SUPPORT.md` in
`github.com/cargo-bins/cargo-binstall`.

## What we rely on

- Config: `dist-workspace.toml` (`[workspace] members = ["cargo:."]` and a
  `[dist]` table). `dist init --yes` writes it and runs `dist generate`; it
  also adds `[profile.dist]` (inherits release, `lto = "thin"`) to `Cargo.toml`.
- `targets`: the six we ship are `aarch64-apple-darwin`,
  `x86_64-apple-darwin`, `aarch64-unknown-linux-gnu`,
  `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-musl`,
  `x86_64-unknown-linux-musl`.
- `installers = ["shell"]`: `herdr-mcp-installer.sh`, which fetches the
  archive for the host from the release, installs into `install-path`
  (`CARGO_HOME`, so `~/.cargo/bin`) and edits shell rc files for `PATH`.
  It needs `curl` or `wget` and `tar`; it cannot tell musl from glibc.
- `dist = true`: needed because `Cargo.toml` has `publish = false`, which
  dist otherwise reads as "do not distribute".
- `checksum = "sha256"` (the default): one `<archive>.sha256` per archive plus
  a combined `sha256.sum`. There is no file called `SHA256SUMS`.
- Archives are `.tar.xz` named `herdr-mcp-<target>.tar.xz`, holding
  `herdr-mcp-<target>/herdr-mcp`, `LICENSE` and `README.md`.
- Trigger: a tag matching `**[0-9]+.[0-9]+.[0-9]+*`; a version with a
  suffix (`v0.1.0-rc.1`) makes a GitHub pre-release. On pull requests the
  workflow only runs `dist plan` (`pr-run-mode = "plan"`, the default).
- Release body comes from the `CHANGELOG.md` section for the version when it
  has one (dist reads `CHANGELOG.md` headings like `## 0.1.0`).
- Local checks: `dist plan` (lists artifacts), `dist build --artifacts=local
  --target <host triple>`, `dist build --artifacts=global` (installer,
  `sha256.sum`, source tarball), `dist generate --check`.

## Known limits in 0.34.0 (found by running it)

- `dist generate` writes `permissions: contents: write` at the top of
  `release.yml`, so every job's token can write. There is no setting to
  narrow it (`github-custom-job-permissions` only applies to custom jobs).
- `plan-jobs = ["./x"]` (a custom job before the build) renders
  `uses: $/.github/workflows/x.yml`: a literal `$` that GitHub rejects. The
  template has that text hard-coded (`release.yml.j2`, lines for
  `plan_jobs`). So custom jobs cannot run `scripts/check.sh` first.
- Both limits are why `allow-dirty = ["ci"]` is set and `release.yml` has two
  hand edits; see `docs/releases.md`, "Hand edits to release.yml". Revisit
  on each dist upgrade: regenerate into a scratch copy, diff, re-apply.

## cargo binstall

- `herdr-mcp` is not on crates.io (checked 20/11/2026: crate does not
  exist), so users run `cargo binstall --git <repo> herdr-mcp`.
- `[package.metadata.binstall]` in `Cargo.toml` points `pkg-url` at the dist
  archive (`{ repo }/releases/download/v{ version }/{ name }-{ target
  }.tar.xz`, `pkg-fmt = "txz"`, `bin-dir = "{ name }-{ target }/{ bin }{
  binary-ext }"`). `cargo binstall --manifest-path . --dry-run` parses it;
  resolving the download needs a real release.
- For a private repository binstall needs a `GITHUB_TOKEN`.

## Not verified here

The release workflow, the installer against a real release, and binstall's
download have not run: they need a pushed tag (see `plans/08-release-v0.1.0.md`).
