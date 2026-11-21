#!/bin/sh
# Build the book and check its links offline (plans/11-docs.md).
#   mdbook 0.5.2   cargo install --locked mdbook --version 0.5.2
#   lychee 0.24.2  cargo install --locked lychee --version 0.24.2
# Any warning from mdbook fails the build. The output is docs/book/book/
# (git-ignored). Set DOCS_DEST to build somewhere else, and skip the links
# with DOCS_LINKS=0.
set -eu
cd "$(dirname "$0")/.."
MDBOOK=0.5.2
LYCHEE=0.24.2

need() { # tool version
  have=$("$1" --version 2>/dev/null | awk '{print $2}' | sed 's/^v//')
  if [ "$have" != "$2" ]; then
    echo "docs: need $1 $2, found '${have:-none}': cargo install --locked $1 --version $2" >&2
    exit 1
  fi
}
need mdbook "$MDBOOK"
[ "${DOCS_LINKS:-1}" = 1 ] && need lychee "$LYCHEE"

dest=${DOCS_DEST:-docs/book/book}
log=$(mktemp)
trap 'rm -f "$log"' EXIT
mdbook build docs/book --dest-dir "$(cd "$(dirname "$dest")" && pwd)/$(basename "$dest")" >"$log" 2>&1 || { cat "$log" >&2; exit 1; }
if grep -qi 'warn' "$log"; then
  echo "docs: mdbook warned:" >&2
  cat "$log" >&2
  exit 1
fi

[ -f "$dest/index.html" ] || { echo "docs: no $dest/index.html" >&2; exit 1; }

if [ "${DOCS_LINKS:-1}" = 1 ]; then
  # Local files and anchors only: --offline never touches the network.
  lychee --offline --include-fragments --exclude-path "$dest/404.html" --no-progress --root-dir "$(cd "$dest" && pwd)" "$dest" >"$log" 2>&1 || { cat "$log" >&2; exit 1; }
fi
echo "docs: built $dest"
