#!/bin/sh
# Verify (plan 13, Phase 2): the six harmless tools against a real herdr. Starts the
# isolated session herdr-mcp-test-13 (HERDR_* unset, never the default session),
# drives the built binary over stdio, stops and deletes the session. Prints PASS.
#   herdr_pane_scroll  herdr_pane_selection_read  herdr_pane_report_metadata
#   herdr_agent_view_set  herdr_agent_view_clear  herdr_workspace_move_block
# Needs a built binary: cargo build --release (BIN overrides the path).
# Output: docs/verified/13-cli-parity/harmless-tools.log
set -eu
cd "$(dirname "$0")/../.."
SESSION=herdr-mcp-test-13
BIN=${BIN:-${CARGO_TARGET_DIR:-target}/release/herdr-mcp}
out=docs/verified/13-cli-parity/harmless-tools.log
mkdir -p docs/verified/13-cli-parity
[ -x "$BIN" ] || { echo "FAIL: no binary at $BIN (cargo build --release --locked)"; exit 1; }
for v in $(env | sed -n 's/^\(HERDR_[A-Za-z0-9_]*\)=.*/\1/p'); do unset "$v"; done
sock="$HOME/.config/herdr/sessions/$SESSION/herdr.sock"
work=$(mktemp -d)
echo "date: $(date +%Y-%m-%d)  herdr: $(herdr --version)  session: $SESSION" >"$out"

cleanup() {
  herdr --session "$SESSION" server stop >/dev/null 2>&1 || true
  [ -n "${srv:-}" ] && kill "$srv" 2>/dev/null || true
  i=0; while [ -S "$sock" ] && [ "$i" -lt 50 ]; do sleep 0.1; i=$((i + 1)); done
  herdr session delete "$SESSION" >/dev/null 2>&1 || true
  rm -rf "$work"
}
trap cleanup EXIT
fail() { echo "FAIL: $1" | tee -a "$out"; exit 1; }

herdr --session "$SESSION" server >/dev/null 2>&1 &
srv=$!
i=0; until [ -S "$sock" ] && printf '{"id":"p","method":"ping","params":{}}\n' | nc -U -w 2 "$sock" | grep -q pong; do
  i=$((i + 1)); [ "$i" -lt 100 ] || fail "the isolated server did not come up"; sleep 0.1
done

# One stdio session: the handshake, then each request line; stdin closes after.
mcp() { # file-with-calls -> replies on stdout
  {
    printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"verify","version":"0"}}}'
    printf '%s\n' '{"jsonrpc":"2.0","method":"notifications/initialized"}'
    cat "$1"
    sleep 2
  } | HERDR_ENV=1 HERDR_SOCKET_PATH="$sock" HERDR_MCP_TOOLS=panes,agents,workspaces "$BIN" 2>/dev/null
}
call() { # id tool args-json
  printf '{"jsonrpc":"2.0","id":%s,"method":"tools/call","params":{"name":"%s","arguments":%s}}\n' "$1" "$2" "$3"
}

# 1: a workspace and its pane (a fresh session has none).
{
  call 2 herdr_workspace_create '{"label":"verify-13","focus":true}'
  call 3 herdr_pane_list '{}'
} >"$work/in1"
mcp "$work/in1" >"$work/out1"
ws=$(sed -n 's/.*"workspace_id":"\([^"]*\)".*/\1/p' "$work/out1" | head -n 1)
pane=$(grep '"id":3' "$work/out1" | sed -n 's/.*"pane_id":"\([^"]*\)".*/\1/p' | head -n 1)
[ -n "$ws" ] && [ -n "$pane" ] || fail "no workspace or pane: $(cat "$work/out1")"
echo "workspace $ws pane $pane" >>"$out"

# 2: a second workspace to move, then the six tools.
{
  call 2 herdr_workspace_create '{"label":"verify-13-b"}'
} >"$work/in2"
mcp "$work/in2" >"$work/out2"
ws2=$(sed -n 's/.*"workspace_id":"\([^"]*\)".*/\1/p' "$work/out2" | head -n 1)
[ -n "$ws2" ] || fail "no second workspace: $(cat "$work/out2")"

{
  call 10 herdr_pane_scroll "{\"pane_id\":\"$pane\",\"offset_from_bottom\":0}"
  call 11 herdr_pane_selection_read "{\"pane_id\":\"$pane\",\"anchor\":{\"row\":0,\"col\":0},\"cursor\":{\"row\":0,\"col\":10}}"
  call 12 herdr_pane_report_metadata "{\"pane_id\":\"$pane\",\"source\":\"herdr-mcp-verify\",\"title\":\"verify\",\"tokens\":{\"k\":\"v\"},\"ttl_ms\":60000}"
  call 13 herdr_agent_view_set '{"source":"herdr-mcp-verify","label":"verify","filter":{"op":"eq","field":"status","value":"blocked"},"sort":[{"field":"attention"}]}'
  call 14 herdr_agent_view_clear '{"source":"herdr-mcp-verify"}'
  call 15 herdr_workspace_move_block "{\"workspace_ids\":[\"$ws2\"],\"before_workspace_id\":\"$ws\"}"
} >"$work/in3"
mcp "$work/in3" >"$work/out3"
cat "$work/out3" >>"$out"
for id in 10 11 12 13 14 15; do
  line=$(grep "\"id\":$id," "$work/out3" || true)
  [ -n "$line" ] || fail "no reply for call $id"
  case $line in
    *'"isError":false'*) echo "ok call $id" >>"$out" ;;
    *) fail "call $id: $line" ;;
  esac
done
echo "PASS: six tools answered without error on $SESSION" | tee -a "$out"
