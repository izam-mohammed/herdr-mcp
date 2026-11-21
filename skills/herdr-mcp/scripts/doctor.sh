#!/bin/sh
# herdr-mcp doctor: one line per check, `ok`, `warn` or `FAIL`; exit 1 when any
# line is FAIL. POSIX sh and standard tools only (curl, nc, sed, grep), no setup.
# It reads environment variable names and one public URL; it never prints the
# value of a token or of any variable other than socket paths.
#   HERDR_MCP_DOCTOR_RELEASE_URL  override the latest-release URL (tests)
set -u
fails=0
ok() { printf 'ok: %s\n' "$*"; }
warn() { printf 'warn: %s\n' "$*"; }
fail() { printf 'FAIL: %s\n' "$*"; fails=$((fails + 1)); }

# 1. the binary and its version
installed=
if command -v herdr-mcp >/dev/null 2>&1; then
  installed=$(herdr-mcp --version 2>/dev/null | awk '{print $2}')
  if [ -n "$installed" ]; then
    ok "herdr-mcp $installed at $(command -v herdr-mcp)"
  else
    fail "herdr-mcp is on PATH but --version printed nothing: reinstall it (README, Get started)"
  fi
else
  fail "herdr-mcp not on PATH: install it (README, Get started), then open a new shell"
fi

# 2. the latest release (read-only request to the public GitHub API)
url=${HERDR_MCP_DOCTOR_RELEASE_URL:-https://api.github.com/repos/izam-mohammed/herdr-mcp/releases/latest}
latest=
if command -v curl >/dev/null 2>&1; then
  latest=$(curl -fsS --max-time 5 -H 'Accept: application/vnd.github+json' "$url" 2>/dev/null |
    sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"v\{0,1\}\([^"]*\)".*/\1/p' | head -n 1)
fi
if [ -z "$latest" ]; then
  warn "offline, skipped: latest release not checked (no network, no curl, or no release visible)"
elif [ -z "$installed" ]; then
  warn "latest release is $latest"
elif [ "$installed" = "$latest" ]; then
  ok "version $installed is the latest release"
else
  warn "installed $installed, latest release $latest: upgrade by running the install again (README)"
fi

# 3. registered in the agent CLIs, when they exist (names only, never the line)
for cli in claude codex; do
  if command -v "$cli" >/dev/null 2>&1; then
    if "$cli" mcp list 2>/dev/null | grep -qi herdr; then
      ok "registered in $cli"
    else
      warn "not registered in $cli: run '$cli mcp add herdr -- herdr-mcp'"
    fi
  else
    ok "$cli not installed, skipped"
  fi
done

# 4. herdr reachable, same socket order as the server (docs: Configuration)
if [ -n "${HERDR_SOCKET_PATH:-}" ]; then
  sock=$HERDR_SOCKET_PATH
elif [ -n "${HERDR_SESSION:-}" ]; then
  sock=$HOME/.config/herdr/sessions/$HERDR_SESSION/herdr.sock
else
  sock=$HOME/.config/herdr/herdr.sock
fi
if [ ! -S "$sock" ]; then
  fail "herdr not reachable at $sock: start herdr, or set HERDR_SESSION to the right session name"
elif ! command -v nc >/dev/null 2>&1; then
  warn "socket $sock exists, but nc is missing so ping was skipped"
else
  reply=$(printf '{"id":"d1","method":"ping","params":{}}\n' | nc -U -w 2 "$sock" 2>/dev/null)
  case $reply in
    *'"type":"pong"'*) ok "herdr answers ping at $sock" ;;
    *) fail "herdr not reachable at $sock: the socket is there but did not answer ping; restart that herdr" ;;
  esac
fi

# 5. inside a herdr pane? (writes need it)
if [ "${HERDR_ENV:-}" = 1 ]; then
  ok "inside a herdr pane (HERDR_ENV=1): write tools are available"
elif [ "${HERDR_MCP_ALLOW_OUTSIDE:-}" = 1 ]; then
  warn "outside a herdr pane, but HERDR_MCP_ALLOW_OUTSIDE=1 lets writes through"
else
  warn "not inside a herdr pane (HERDR_ENV is not 1): herdr-mcp is read-only here"
fi
if [ "${HERDR_MCP_READ_ONLY:-}" = 1 ]; then
  warn "HERDR_MCP_READ_ONLY=1 is set: write tools are off"
fi
if [ -n "${HERDR_MCP_TOOLS:-}" ]; then
  warn "HERDR_MCP_TOOLS is set: a tool group may be switched off"
fi

[ "$fails" -eq 0 ]
