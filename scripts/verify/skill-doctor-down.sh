#!/bin/sh
# Verify (plans/14-user-skill.md): the skill's doctor fails with a clear line
# when herdr is down. Points it at a socket that does not exist; starts no
# herdr and touches no session. Saves the output to
# docs/verified/14-user-skill/doctor-down.log and prints PASS.
set -eu
cd "$(dirname "$0")/../.."
for v in $(env | sed -n 's/^\(HERDR_[A-Z_]*\)=.*/\1/p'); do unset "$v"; done
log=docs/verified/14-user-skill/doctor-down.log
mkdir -p "$(dirname "$log")"
sock=${TMPDIR:-/tmp}/herdr-mcp-doctor-down-$$/herdr.sock

set +e
HERDR_SOCKET_PATH=$sock sh skills/herdr-mcp/scripts/doctor.sh >"$log" 2>&1
code=$?
set -e
cat "$log"
if [ "$code" -eq 1 ] && grep -q "^FAIL: herdr not reachable at $sock" "$log"; then
  echo "PASS: doctor exits 1 with a FAIL line when herdr is down"
else
  echo "FAIL: doctor exit $code, expected 1 with a 'herdr not reachable' line"
  exit 1
fi
