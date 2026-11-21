#!/bin/sh
# Verify (plan 13, Phase 3): the admin tool group is absent from `tools/list` by default,
# present with `--tools all,admin`, and its mutating tools are absent under
# `--read-only`. Drives the built binary over stdio; it never calls a tool, and the
# socket path points at nothing, so no herdr is touched. Prints PASS.
# Needs a built binary: cargo build --release (BIN overrides the path).
# Output: docs/verified/13-cli-parity/admin-off-by-default.log
set -eu
cd "$(dirname "$0")/../.."
BIN=${BIN:-${CARGO_TARGET_DIR:-target}/release/herdr-mcp}
out=docs/verified/13-cli-parity/admin-off-by-default.log
mkdir -p docs/verified/13-cli-parity
[ -x "$BIN" ] || { echo "FAIL: no binary at $BIN (cargo build --release --locked)"; exit 1; }
for v in $(env | sed -n 's/^\(HERDR_[A-Za-z0-9_]*\)=.*/\1/p'); do unset "$v"; done
trap 'rm -f "$out.default" "$out.admin" "$out.ro"' EXIT
echo "date: $(date +%Y-%m-%d)" >"$out"
fail() { echo "FAIL: $1" | tee -a "$out"; exit 1; }

tools() { # args... -> tool names, one per line
  {
    printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"verify","version":"0"}}}'
    printf '%s\n' '{"jsonrpc":"2.0","method":"notifications/initialized"}'
    printf '%s\n' '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}'
    sleep 1
  } | HERDR_ENV=1 HERDR_SOCKET_PATH=/nonexistent/herdr.sock "$BIN" "$@" 2>/dev/null |
    grep '"id":2,' | grep -o '"name":"herdr_[a-z_]*"' | sed 's/"name":"//;s/"$//' | sort
}

tools >"$out.default"
tools --tools all,admin >"$out.admin"
tools --tools all,admin --read-only >"$out.ro"
default=$(wc -l <"$out.default"); admin=$(wc -l <"$out.admin"); ro=$(wc -l <"$out.ro")
{
  echo "default: $default tools, with admin: $admin tools, read-only with admin: $ro tools"
  echo "admin-only tools:"
  comm -13 "$out.default" "$out.admin" | sed 's/^/  /'
} >>"$out"

[ "$default" -gt 0 ] || fail "no tools listed by default"
grep -q '^herdr_server_stop$' "$out.default" && fail "herdr_server_stop is in the default list"
grep -q '^herdr_update$' "$out.default" && fail "herdr_update is in the default list"
grep -q '^herdr_server_stop$' "$out.admin" || fail "herdr_server_stop missing with --tools all,admin"
[ "$admin" -gt "$default" ] || fail "the admin group added nothing"
for t in herdr_server_stop herdr_pane_report_agent herdr_integration_install; do
  ! grep -q "^$t\$" "$out.ro" || fail "$t is in the read-only list"
done
grep -q '^herdr_integration_list$' "$out.ro" || fail "herdr_integration_list (a read) missing in read-only"
echo "PASS: absent by default ($default tools), $((admin - default)) admin tools with the group, mutating ones hidden read-only" | tee -a "$out"
