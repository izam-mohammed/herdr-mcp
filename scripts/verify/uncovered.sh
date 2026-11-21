#!/bin/sh
# Verify (plan 13): `sh scripts/sync-herdr.sh` against the installed herdr
# prints every manifest entry whose tool src/tools does not define yet, and
# exits 0. The expected set is worked out here by a different path than the
# script's own (a plain grep over the manifest), so the two must agree.
# Output: docs/verified/13-cli-parity/uncovered.log. Read-only for herdr.
set -eu
cd "$(dirname "$0")/../.."
out=docs/verified/13-cli-parity/uncovered.log
mkdir -p docs/verified/13-cli-parity
t=$(mktemp)
trap 'rm -f "$t" "$t.tools"' EXIT
fail() { echo "FAIL: $1" | tee -a "$out"; exit 1; }
echo "date: $(date +%Y-%m-%d)  herdr: $(herdr --version)" >"$out"
sh scripts/sync-herdr.sh >"$t" 2>&1 || { cat "$t" >>"$out"; fail "sync-herdr.sh exited non-zero"; }
cat "$t" >>"$out"
# Expected: every tool named in a row that has no `fn <tool>(` in src/tools.
want=0
grep -E '^"[^"]*" = "herdr_[a-z_]*"$' docs/herdr-coverage.toml | sed 's/.* = "\(.*\)"$/\1/' | sort -u >"$t.tools"
while read -r tool; do
  if ! grep -rqE "fn $tool\(" src/tools/; then
    want=$((want + 1))
    grep -q -- "-> $tool\$" "$t" || fail "$tool has no tool yet but the script did not print it"
  fi
done <"$t.tools"
if grep -qE '^(no row|stale) ' "$t"; then fail "the manifest has a missing or stale row"; fi
got=$(grep -c '^no tool ' "$t" || true)
echo "PASS: $want tools not built yet, $got lines printed, exit 0" | tee -a "$out"
