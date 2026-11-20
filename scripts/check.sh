#!/bin/sh
# The standard check (docs/standards.md). Stops at the first failure.
# HERDR_MCP_LIVE=1 adds the live tests and the bench (needs herdr).
set -eu
cd "$(dirname "$0")/.."
step() { printf '\n== %s\n' "$1"; }

step format;  cargo fmt --all -- --check
step lint;    cargo clippy --all-targets --locked -- -D warnings
step test;    cargo test --locked
if [ "${HERDR_MCP_LIVE:-}" = 1 ]; then
  step live;  cargo test --locked --test live -- --test-threads=1
fi
step audit;   cargo deny check advisories bans sources licenses
step secrets; gitleaks git --no-banner
step shell;   shellcheck scripts/*.sh scripts/prove/*.sh
step build;   cargo build --release --locked
if [ "${HERDR_MCP_LIVE:-}" = 1 ] && [ -f scripts/bench.sh ]; then
  step bench; sh scripts/bench.sh
fi
printf '\nOK\n'
