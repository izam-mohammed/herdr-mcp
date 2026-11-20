#!/bin/sh
# Proof: the rmcp example server answers initialize and tools/call, and the
# call returns herdr's pong. Records the negotiated MCP protocolVersion.
set -eu
cd "$(dirname "$0")/../.."
cargo build --locked --example prove_rmcp >/dev/null
out=$( {
  printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"prove","version":"0"}}}'
  printf '%s\n' '{"jsonrpc":"2.0","method":"notifications/initialized"}'
  printf '%s\n' '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"ping","arguments":{}}}'
  sleep 1
} | target/debug/examples/prove_rmcp )
echo "$out" | sed -n 's/.*"protocolVersion":"\([^"]*\)".*/negotiated protocolVersion: \1/p'
case $out in
  *'pong'*) echo "PASS" ;;
  *) echo "FAIL: $out"; exit 1 ;;
esac
