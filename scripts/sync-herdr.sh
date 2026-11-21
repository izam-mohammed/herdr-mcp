#!/bin/sh
# Keep the MCP honest about herdr (plan 13, decision 3). Reads the installed
# herdr's socket methods (`herdr api schema --json`) and CLI commands
# (`herdr --help` and each group's help), rewrites the two checked-in lists
#   docs/external/herdr-methods.txt   docs/external/herdr-commands.txt
# when something changed, and prints:
#   new       in herdr but not in the checked-in list (a new herdr release)
#   gone      in the list but no longer in herdr
#   no row    in a list but with no row in docs/herdr-coverage.toml
#   no tool   a row names a tool that src/tools does not define yet
#   stale     a row for something not in a list
# The fix for each is a row in docs/herdr-coverage.toml: a tool, or
# { skip = "<reason>" }. Always exits 0 when herdr answers; tests/coverage.rs
# is the check that fails. Read-only for herdr: help, schema and version only.
# POSIX sh, standard tools only.
set -eu
cd "$(dirname "$0")/.."
for v in $(env | sed -n 's/^\(HERDR_[A-Za-z0-9_]*\)=.*/\1/p'); do unset "$v"; done
command -v herdr >/dev/null 2>&1 || { echo "sync-herdr: herdr is not on PATH" >&2; exit 1; }
export LC_ALL=C

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
methods_file=docs/external/herdr-methods.txt
commands_file=docs/external/herdr-commands.txt
manifest=docs/herdr-coverage.toml

version=$(herdr --version | awk '{print $2}')
schema=$(herdr api schema --json)
protocol=$(printf '%s\n' "$schema" | sed -n 's/^  "protocol": \([0-9]*\),*$/\1/p' | head -n 1)

printf '%s\n' "$schema" |
  awk '/^    "request": \{/{r=1;next} r&&/^    "[a-z_]+": \{/{r=0}
       r&&/^ +"method": \{/{m=1;next} m&&/"const":/{gsub(/.*"const": "|".*/,"");print;m=0}' |
  sort -u >"$tmp/methods"
[ -s "$tmp/methods" ] || { echo "sync-herdr: no methods in the schema" >&2; exit 1; }

# Commands: each group's "Commands:" block, plus the few bare forms and flags
# that run something. The group words come from the top-level help.
herdr --help >"$tmp/help"
{
  sed -n 's/^ *herdr \([a-z][a-z-]*\)\( .*\)\{0,1\}$/\1/p' "$tmp/help" | sort -u |
    while read -r group; do
      subs=$(herdr "$group" --help 2>/dev/null | awk '/^Commands:/{c=1;next} c&&/^$/{exit} c{print $1}')
      if [ -n "$subs" ]; then
        for s in $subs; do echo "$group $s"; done
      else
        echo "$group"
      fi
    done
  echo herdr
  for flag in --remote --machine --skill --default-config; do
    if grep -q -- "^ *$flag" "$tmp/help"; then echo "$flag"; fi
  done
  echo status
  echo server
} | sort -u >"$tmp/commands"

# One list: rewrite when the entries or the version moved, print the diff.
sync_list() { # file kind entries
  file=$1 kind=$2 new=$3
  old="$tmp/old.$kind"
  { grep -v '^#' "$file" 2>/dev/null || true; } | sort -u >"$old"
  comm -13 "$old" "$new" | sed "s/^/new $kind: /"
  comm -23 "$old" "$new" | sed "s/^/gone $kind: /"
  head="# herdr $version, protocol $protocol"
  if ! cmp -s "$old" "$new" || ! grep -q "^$head," "$file" 2>/dev/null; then
    {
      echo "$head, checked $(date +%Y-%m-%d)"
      echo "# one $kind per line; regenerate with: sh scripts/sync-herdr.sh"
      cat "$new"
    } >"$file"
  fi
}
sync_list "$methods_file" method "$tmp/methods"
sync_list "$commands_file" command "$tmp/commands"

# The manifest: [methods] and [commands] tables of "key" = "tool" or
# "key" = { skip = "reason" }, one row per line (tests/coverage.rs holds the shape).
if [ -f "$manifest" ]; then
  awk '/^\[methods\]/{s="method";next} /^\[commands\]/{s="command";next}
       /^"/{k=$0; sub(/^"/,"",k); sub(/" = .*/,"",k); v=$0; sub(/^"[^"]*" = /,"",v)
            if (v ~ /^\{/) t="-"; else {t=v; gsub(/"/,"",t)}
            print s "\t" k "\t" t}' "$manifest" >"$tmp/rows"
else
  : >"$tmp/rows"
fi
for kind in method command; do
  awk -F'\t' -v k="$kind" '$1==k{print $2}' "$tmp/rows" | sort -u >"$tmp/rows.$kind"
  comm -23 "$tmp/${kind}s" "$tmp/rows.$kind" | sed "s/^/no row $kind: /"
  comm -13 "$tmp/${kind}s" "$tmp/rows.$kind" | sed "s/^/stale $kind: /"
done
awk -F'\t' '$3!="-"{print $3 "\t" $1 " " $2}' "$tmp/rows" | sort -u |
  while IFS="$(printf '\t')" read -r tool entry; do
    grep -rqE "fn $tool\(" src/tools/ || echo "no tool $entry -> $tool"
  done
exit 0
