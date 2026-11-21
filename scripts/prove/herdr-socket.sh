#!/bin/sh
# Proof: herdr answers `ping` on the resolved socket with `pong`, protocol 22.
# Wire format: docs/external/herdr-socket-api.md. Needs nc with -U and -w
# (OpenBSD netcat). Resolution order is herdr's own.
set -eu
if [ -n "${HERDR_SESSION:-}" ] && [ -z "${HERDR_SOCKET_PATH:-}" ]; then
  sock="$HOME/.config/herdr/sessions/$HERDR_SESSION/herdr.sock"
else
  sock="${HERDR_SOCKET_PATH:-$HOME/.config/herdr/herdr.sock}"
fi
reply=$(printf '{"id":"p1","method":"ping","params":{}}\n' | nc -U -w 2 "$sock") ||
  { echo "FAIL: no answer on $sock"; exit 1; }
case $reply in
  *'"type":"pong"'*'"protocol":22'*) echo "PASS: $reply" ;;
  *) echo "FAIL: $reply"; exit 1 ;;
esac
