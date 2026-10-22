# herdr-mcp: what every change passes

Run everything: `sh scripts/check.sh`. A change is done when it passes,
locally and in CI. Chosen in `plans/01-first-version.md` (decision 6);
`scripts/check.sh` and its config land in that plan's Phase 2.

## Toolchain

| What | Version | Pinned in |
| --- | --- | --- |
| Rust | stable 1.99 (current on 22/10/2026), edition 2024 | `rust-toolchain.toml` |
| cargo | with the toolchain | `Cargo.lock`, committed (a binary) |
| herdr, for live tests | 0.9.5, protocol 22 | the live tests check `ping` |

## Checks

| Check | Tool | Config | Command | Passing means |
| --- | --- | --- | --- | --- |
| Format | rustfmt | `rustfmt.toml` | `cargo fmt --all -- --check` | no diff |
| Lint | Clippy | `Cargo.toml` `[lints]` | `cargo clippy --all-targets --locked -- -D warnings` | no errors, no warnings |
| Types | rustc | | (part of clippy and test) | compiles |
| Unit and fake-socket tests | `cargo test` | | `cargo test --locked` | all pass |
| Live tests | `cargo test` | | `HERDR_MCP_LIVE=1 cargo test --locked --test live -- --test-threads=1` | all pass against an isolated `herdr-mcp-test` session |
| Performance budget | `scripts/bench.sh` | the budget in the script | `sh scripts/bench.sh` | inside the budget (from plan 01 Phase 3) |
| Dependency audit | cargo-deny | `deny.toml` | `cargo deny check advisories bans sources licenses` | no advisory, no banned or unknown source |
| Secret scan | gitleaks | `.gitleaks.toml` (defaults) | `gitleaks git --no-banner` | no findings |
| Shell scripts | ShellCheck | | `shellcheck scripts/*.sh scripts/prove/*.sh` | no findings |
| Build | cargo | `[profile.release]` | `cargo build --release --locked` | builds |

`scripts/check.sh` runs format, lint, tests, audit, secret scan, shell
check and build, in that order, stopping at the first failure. The live
tests and the budget run only when `HERDR_MCP_LIVE=1`, since CI has no
herdr; locally every phase's check sets it.

## Tests

- Unit tests sit beside the code (`#[cfg(test)] mod tests`).
- Fake-socket tests in `tests/`: a `tokio::net::UnixListener` that replays
  recorded herdr responses (`tests/fixtures/*.jsonl`), so tool behaviour,
  errors and safety refusals are tested without herdr.
- Live tests in `tests/live.rs`: start `herdr --session herdr-mcp-test
  server` with `HERDR_*` unset, run the real binary over stdio against it,
  stop and delete the session at the end. They never touch the default
  session.
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
on pushes to `main` (no live tests: no herdr in CI). A red check blocks
merge.

## Decided

- Rust over Go, TypeScript and Python: plan 01, decision 1.
- `rmcp` over Go's SDK and hand-rolled JSON-RPC: plan 01, decision 2.
- cargo-deny over cargo-audit: one tool for advisories, sources, bans and
  licences (plan 01, decision 6).
- `cargo test` over cargo-nextest: no extra binary; revisit if the suite
  passes 30 s (plan 01, decision 6).
- gitleaks over TruffleHog, and `scripts/check.sh` over a Makefile or
  justfile: plan 01, decision 6.
