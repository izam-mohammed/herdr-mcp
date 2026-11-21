#!/bin/sh
# Verify (plans/14-user-skill.md): the skill's doctor reports a healthy setup
# against an isolated herdr. Starts `herdr --session herdr-mcp-test-14 server`
# (never the default session), runs the doctor with HERDR_SESSION naming it,
# prints PASS and saves the output to docs/verified/14-user-skill/doctor-ok.log.
set -eu
cd "$(dirname "$0")/../.."
SESSION=herdr-mcp-test-14
for v in $(env | sed -n 's/^\(HERDR_[A-Z_]*\)=.*/\1/p'); do unset "$v"; done
sock="$HOME/.config/herdr/sessions/$SESSION/herdr.sock"
log=docs/verified/14-user-skill/doctor-ok.log
mkdir -p "$(dirname "$log")"

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
until printf '{"id":"v","method":"ping","params":{}}\n' | nc -U -w 1 "$sock" 2>/dev/null | grep -q pong; do
  i=$((i + 1))
  [ "$i" -gt 100 ] && { echo "FAIL: isolated herdr did not start"; exit 1; }
  sleep 0.05
done

# HERDR_ENV=1 stands in for running inside a herdr pane.
set +e
HERDR_SESSION=$SESSION HERDR_ENV=1 sh skills/herdr-mcp/scripts/doctor.sh >"$log" 2>&1
code=$?
set -e
cat "$log"
if [ "$code" -eq 0 ] && ! grep -q '^FAIL' "$log" && grep -q '^ok: herdr answers ping' "$log"; then
  echo "PASS: doctor healthy against isolated session $SESSION"
else
  echo "FAIL: doctor exit $code"
  exit 1
fi
