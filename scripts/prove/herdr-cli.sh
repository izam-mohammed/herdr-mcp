#!/bin/sh
# Proof: the installed herdr prints its API schema, its help and one read-only
# subcommand, in the shapes scripts/sync-herdr.sh reads (plan 13). Nothing
# here changes anything: no server is started or stopped, and `config check`
# only validates a file. HERDR_* is unset so no session is inherited.
set -eu
for v in $(env | sed -n 's/^\(HERDR_[A-Za-z0-9_]*\)=.*/\1/p'); do unset "$v"; done
fail() { echo "FAIL: $1"; exit 1; }
command -v herdr >/dev/null 2>&1 || fail "herdr is not on PATH"

version=$(herdr --version) || fail "herdr --version"

schema=$(herdr api schema --json) || fail "herdr api schema --json"
methods=$(printf '%s\n' "$schema" |
  awk '/^    "request": \{/{r=1;next} r&&/^    "[a-z_]+": \{/{r=0}
       r&&/^ +"method": \{/{m=1;next} m&&/"const":/{gsub(/.*"const": "|".*/,"");print;m=0}' | wc -l)
[ "$methods" -gt 50 ] || fail "schema lists only $methods methods"
printf '%s\n' "$schema" | grep -q '"pane.read"' || fail "schema has no pane.read"

help=$(herdr --help) || fail "herdr --help"
printf '%s\n' "$help" | grep -q 'herdr pane <subcommand>' || fail "help has no pane group"

sub=$(herdr pane --help) || fail "herdr pane --help"
printf '%s\n' "$sub" | grep -q '^Commands:' || fail "pane help has no Commands block"

check=$(herdr config check) || fail "herdr config check: $check"
case $check in
  *ok*) ;;
  *) fail "herdr config check said: $check" ;;
esac

echo "PASS: $version, $methods schema methods, help and pane help parse, config check ok"
