# herdr-mcp: what every change passes

Run everything: `sh scripts/check.sh`. A change is done when it passes,
locally and in CI. Chosen in `plans/01-first-version.md` (decision 6);
`scripts/check.sh` and its config land in that plan's Phase 1.

## Toolchain

| What | Version | Pinned in |
| --- | --- | --- |
| Rust | stable 1.100.0 (current on 20/11/2026; was 1.99 on 22/10/2026), edition 2024 | `rust-toolchain.toml` |
| cargo | with the toolchain | `Cargo.lock`, committed (a binary) |
| herdr, for live tests | 0.9.5, protocol 22 (re-checked 20/11/2026) | the live tests check `ping` |

## Checks

| Check | Tool | Config | Command | Passing means |
| --- | --- | --- | --- | --- |
| Format | rustfmt | `rustfmt.toml` | `cargo fmt --all -- --check` | no diff |
| Lint | Clippy | `Cargo.toml` `[lints]` | `cargo clippy --all-targets --locked -- -D warnings` | no errors, no warnings |
| Types | rustc | | (part of clippy and test) | compiles |
| Unit and fake-socket tests, with coverage | `cargo test` under cargo-llvm-cov 0.9.1 | `llvm-tools-preview` in `rust-toolchain.toml` | `cargo llvm-cov --locked --fail-under-lines 90` | all pass, 90 % of lines or more (target 95 %); the per-file table is printed |
| Live tests | `cargo test` | | `HERDR_MCP_LIVE=1 cargo test --locked --test <file> -- --test-threads=1`, for `live`, `lane02_live`, `lane03_live`, `lane04_live`, `lane05_live`, `lane06_live`, `lane07_live` | all pass, each file against its own isolated session (`herdr-mcp-test`, `-02` to `-07`; the bench has `-bench`) |
| Performance budget | `scripts/bench.sh` | the budget in the script | `sh scripts/bench.sh` | inside the budget (from plan 01 Phase 3) |
| Dependency audit | cargo-deny | `deny.toml` | `cargo deny check advisories bans sources licenses` | no advisory, no banned or unknown source |
| Secret scan | gitleaks | `.gitleaks.toml` (defaults) | `gitleaks git --no-banner` | no findings |
| Shell scripts | ShellCheck | | `shellcheck scripts/*.sh scripts/prove/*.sh` | no findings |
| Build | cargo | `[profile.release]` | `cargo build --release --locked` | builds |

`scripts/check.sh` runs format, lint, tests with coverage, audit, secret scan, shell
check and build, in that order, stopping at the first failure. The live
tests and the budget run only when `HERDR_MCP_LIVE=1`, since CI has no
herdr; locally every phase's check sets it.

## Tests

- Unit tests sit beside the code (`#[cfg(test)] mod tests`).
- Fake-socket tests in `tests/`: a `tokio::net::UnixListener` that replays
  recorded herdr responses (`tests/fixtures/*.jsonl`), so tool behaviour,
  errors and safety refusals are tested without herdr.
- Live tests in `tests/live.rs` and `tests/lane0N_live.rs`: start `herdr
  --session herdr-mcp-test[-0N] server` with `HERDR_*` unset, run the real
  binary over stdio against it, stop and delete the session at the end. They
  never touch the default session. Each file owns one session name (the bench
  has `herdr-mcp-test-bench`); `tests/session_names.rs` fails on a duplicate,
  so two files can never stop each other's server, and `scripts/check.sh`
  runs them one file at a time with `--test-threads=1`.
- Test files that count requests or list tools name their groups with
  `HERDR_MCP_TOOLS` (the harness sets no default): the events hub sends its
  own requests when `events` is on.
- One test: `cargo test --locked <name>`.

## Constraints

- **Performance-critical**: every Claude Code session runs its own copy, so
  start time, memory and CPU per call have a budget, held by
  `scripts/bench.sh` (plan 01 Phase 3).
- **Steers other programs**: tools can type into other terminals, so the
  safety tests (read-only mode, own-pane refusal) are part of every check.
- **Untrusted output**: pane and agent text is never logged at the default
  level.

## Conventions

Conventional Commits, lowercase subjects, no attribution lines. Branches
`feat/<topic>`, `fix/<topic>`, `plan/<topic>`. A PR names its plan, what
changed and why, and the checks run with their result. Tool names are
`herdr_<area>_<verb>` and are a contract (plan 01, `## Contract`).

## CI

`.github/workflows/check.yml` runs `sh scripts/check.sh` on every PR and
on pushes to `main` (no live tests: no herdr in CI); every action is pinned by
SHA, `permissions: {}` at the top. A red check blocks merge. It is also a
reusable workflow (`workflow_call`), which the release calls.

`.github/workflows/security.yml` calls the shared gate
`izam-mohammed/.github/.github/workflows/security-scan.yml` (pinned to its
commit SHA, `ecosystem: rust`) on every PR and weekly: gitleaks blocks;
osv-scanner, semgrep and zizmor report to the PR.

`.github/workflows/release.yml` (made by `dist`, config in
`dist-workspace.toml`, see `docs/releases.md`) starts on a pushed version tag.
Its `check` job runs the same `scripts/check.sh` first; nothing builds if it
fails. The token is read-only except on the `host` job (`contents: write`),
which creates the GitHub release. On pull requests it runs `dist plan` only.
Locally: `dist plan`, and `dist build --artifacts=local --target <host>`.
Two marked hand edits (`allow-dirty = ["ci"]`) keep it from being regenerated
blindly: read `docs/releases.md` before running `dist generate`.

## Decided

- Rust over Go, TypeScript and Python: plan 01, decision 1.
- `rmcp` over Go's SDK and hand-rolled JSON-RPC: plan 01, decision 2.
- cargo-deny over cargo-audit: one tool for advisories, sources, bans and
  licences (plan 01, decision 6).
- `cargo test` over cargo-nextest: no extra binary; revisit if the suite
  passes 30 s (plan 01, decision 6).
- cargo-llvm-cov over tarpaulin and grcov, a fixed 90 % line floor over a
  ratchet, and the shared gate with `ecosystem: rust` over CodeQL: plan 09.
  The test harness forwards `LLVM_PROFILE_FILE` to the spawned binary and
  lets it exit on drop, so its lines count.
- gitleaks over TruffleHog, and `scripts/check.sh` over a Makefile or
  justfile: plan 01, decision 6.
