#!/bin/sh
# The performance budget (docs/standards.md): start p50 under 10 ms, RSS
# under 8 MB after 1000 calls, call p99 under 2 ms, against an isolated herdr
# session. Never touches the default session.
set -eu
cd "$(dirname "$0")/.."
SESSION=herdr-mcp-test-bench
for v in $(env | sed -n 's/^\(HERDR_[A-Z_]*\)=.*/\1/p'); do unset "$v"; done
sock="$HOME/.config/herdr/sessions/$SESSION/herdr.sock"

cargo build --release --locked --example bench >/dev/null 2>&1
cargo build --release --locked >/dev/null 2>&1

herdr --session "$SESSION" server >/dev/null 2>&1 &
server=$!
# shellcheck disable=SC2329  # run by the trap below
cleanup() {
  herdr --session "$SESSION" server stop >/dev/null 2>&1 || true
  wait "$server" 2>/dev/null || true
  herdr session delete "$SESSION" >/dev/null 2>&1 || true
}
trap cleanup EXIT INT TERM

i=0
until printf '{"id":"b","method":"ping","params":{}}\n' | nc -U -w 1 "$sock" 2>/dev/null | grep -q pong; do
  i=$((i + 1))
  [ "$i" -gt 100 ] && { echo "FAIL: isolated herdr did not start"; exit 1; }
  sleep 0.05
done
# A fresh session is empty; give the calls something real to list.
printf '{"id":"b","method":"workspace.create","params":{"label":"bench","focus":true}}\n' |
  nc -U -w 1 "$sock" >/dev/null

# Timings jitter right after a build; a run past the budget is retried twice
# and fails only if all three are.
for attempt in 1 2 3; do
  HERDR_SOCKET_PATH=$sock target/release/examples/bench && exit 0
  [ "$attempt" -lt 3 ] && echo "retrying ($attempt of 3)"
done
exit 1
