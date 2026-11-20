#!/bin/sh
# The standard check (docs/standards.md). Stops at the first failure.
# HERDR_MCP_LIVE=1 adds the live tests and the bench (needs herdr).
set -eu
cd "$(dirname "$0")/.."
step() { printf '\n== %s\n' "$1"; }

step format;  cargo fmt --all -- --check
step lint;    cargo clippy --all-targets --locked -- -D warnings
# The live files skip themselves without HERDR_MCP_LIVE; unset here so the live
# step below is the only place they run.
step test;    env -u HERDR_MCP_LIVE cargo test --locked
if [ "${HERDR_MCP_LIVE:-}" = 1 ]; then
  # One file at a time, one thread each: every file owns its own session name
  # (herdr-mcp-test, -02 to -05; tests/session_names.rs keeps them unique).
  for f in live lane02_live lane03_live lane04_live lane05_live lane06_live; do
    step "live $f"; cargo test --locked --test "$f" -- --test-threads=1
  done
fi
step audit;   cargo deny check advisories bans sources licenses
step secrets; gitleaks git --no-banner
step shell;   shellcheck scripts/*.sh scripts/prove/*.sh
step build;   cargo build --release --locked
if [ "${HERDR_MCP_LIVE:-}" = 1 ] && [ -f scripts/bench.sh ]; then
  step bench; sh scripts/bench.sh
fi
printf '\nOK\n'
