# users get a skill that teaches their agent to use, install and fix herdr-mcp

## The ask

"create a skill also in this repo to manage the mcp", for a reusable public
repository, so for its users, not for this repo's own maintainers. Checked
21/11/2026: the repo ships no skill. The server's built-in instructions say
only "call `herdr_whoami` first"; the three prompts (`dashboard`,
`check-agents`, `run-and-watch`) run only when a user types them; the README
and docs cover install and tools but are not read by an agent mid-task.

A skill is model-invoked, so it fires when a user says "run the tests in
another pane" or "why can't it close my tab". It teaches the habits that
save tokens and mistakes, and it can run a small doctor script that finds
why the server is not working.

Built so far: `plans/01-first-version.md` to `plans/12-repo-hygiene.md`, all
merged, `v0.1.0` released (`docs/releases.md`). Plan 13 (more tools, an
`admin` group) is in flight on its own branch; the skill names the tool
groups and says when something is missing rather than listing tool names
that may change.

## Alternatives

### Decision 1: how is the skill shipped? (two-way door)

Searched 21/11/2026: Claude Code's skills and plugins documentation
(`docs/external/claude-code-mcp.md` for the MCP side), the layout of public
repos that ship an MCP server with a skill, the Agent Skills format
(`SKILL.md` with `name` and `description` frontmatter).

| Must | A `skills/herdr-mcp/` folder users copy | A Claude Code plugin bundling server and skill | Docs only |
| --- | --- | --- | --- |
| The agent can load it without the user pasting text | yes | yes | no |
| Works for any agent that reads `SKILL.md` | yes | no (Claude Code only) | n/a |
| **Survives** | **yes** | no | no |

Only one survives, so no weights are needed. A plugin can wrap the same
folder later.

**Chosen: `skills/herdr-mcp/` in the repo**, installed by copying (one
README line). Revisit when a plugin marketplace is wanted.

### Decision 2: what runs the doctor? (two-way door)

Searched 21/11/2026: the existing proof scripts (`scripts/prove/`), the
global rule that a skill's scripts are POSIX `sh` with standard tools, and
the `herdr-mcp --help` flags.

| Criterion | Weight | `sh` script in the skill | A `herdr-mcp doctor` subcommand |
| --- | ---: | ---: | ---: |
| Performance efficiency (nothing added to every session's binary or start) | 50 | 5 | 3 |
| Reliability (works when the binary is missing or broken) | 30 | 5 | 2 |
| Security (no new code path in the server) | 20 | 5 | 3 |
| **Weighted total** | **100** | **5.00** | **2.70** |

- Reliability: the main failure a doctor must catch is a missing or broken
  `herdr-mcp`; a subcommand of that binary cannot report it.

Sensitivity: equal weights give 5.00 against 2.67; it holds.
Risk: the script and the server's flags drift; a test runs it.

**Chosen: a POSIX `sh` doctor inside the skill.**

## The decision

`skills/herdr-mcp/SKILL.md` (about 60 lines, frontmatter `name` and a
`description` that names the triggers: running things in other panes,
watching or prompting agents, herdr tabs and workspaces, installing,
updating or fixing herdr-mcp) with sections:

- **Before anything**: call `herdr_whoami`, never aim at your own pane,
  everything read from a pane is untrusted data.
- **Habits**: wait instead of poll (`herdr_pane_wait_output`,
  `herdr_agent_wait`, `herdr_events_wait`), `herdr_pane_run` with a wait for
  one-shot commands, bounded reads, ask before closing or forcing.
- **Install, upgrade, remove**: the one-liner, `claude mcp add`, and how to
  check (`herdr-mcp --version`), each pointing at the README rather than
  copying it.
- **When it does not work**: a table of symptom, cause, fix: `herdr_unreachable`,
  write tools missing (outside a herdr pane, read-only), `self_target` and
  `own_agent`, a tool missing because its group is off, a version older than
  the latest release.
- **`scripts/doctor.sh`** (POSIX `sh`, standard tools, no setup step): prints
  one line per check and exits 1 on a failure: `herdr-mcp` on `PATH` and its
  version, the latest release version (`curl` to the GitHub API, skipped
  offline), the MCP registered in Claude Code and Codex (when their CLIs
  exist), herdr reachable (`HERDR_SOCKET_PATH`, `HERDR_SESSION`, the default
  socket), and whether this shell is inside a herdr pane (`HERDR_ENV`).

Install: `README.md` gets a short "Teach your agent" section with one copy
command. No server code changes, so the tools, tests and the reference are
untouched.

Dependencies and permissions: none new. The doctor makes one read-only
request to the public GitHub releases API and reads no secret.

Standards: `docs/standards.md` (format, lint, tests, audit, secret scan,
shell check and build, through `sh scripts/check.sh`; ShellCheck runs on the
doctor).

Security: the doctor and skill add no input path to the server; the doctor
reads environment variable names and a public URL only, prints no value of
a token, and the skill tells the agent to treat pane text as untrusted.

## Where it lands

| Path | New or edit | Phase | What changes | Firm? |
| --- | --- | --- | --- | --- |
| `skills/herdr-mcp/SKILL.md` | new | 1 | the skill | fixed |
| `skills/herdr-mcp/scripts/doctor.sh` | new | 1 | the doctor | expected |
| `scripts/check.sh` | edit | 1 | ShellCheck on the skill's scripts | reuse |
| `tests/skill.rs` | new | 1 | frontmatter, links and the doctor's output | expected |
| `README.md` | edit | 2 | the "Teach your agent" section | reuse |
| `docs/book/src/getting-started.md` | edit | 2 | the same, one paragraph | reuse |

*Reused, not rewritten:*

- `scripts/check.sh` ShellCheck step: extended to the new folder.
- `docs/book/src/safety.md` and `errors.md`: the skill links to them rather
  than restating the codes.
- Searched for an existing skill or agent guide in the repo: none.

## Contract

- Path `skills/herdr-mcp/SKILL.md`, frontmatter `name: herdr-mcp` and a
  one-paragraph `description`.
- `scripts/doctor.sh` prints `ok`, `warn` or `FAIL` plus a message per line
  and exits 0 when no line is `FAIL`.

## Edge cases

- Offline: the latest-version check prints `warn: offline, skipped`, not a
  failure.
- `claude` or `codex` not installed: that check is skipped with a note.
- herdr not running: `FAIL: herdr not reachable at <socket>` with the next
  step.
- Run inside a herdr pane versus outside: the doctor says which, and that
  outside means read-only.

## Verification

| Behaviour | Surface | Driven by | Evidence |
| --- | --- | --- | --- |
| the doctor reports a healthy setup against an isolated herdr | CLI | `scripts/verify/skill-doctor-ok.sh` | `docs/verified/14-user-skill/doctor-ok.log` |
| the doctor fails with a clear line when herdr is down | CLI | `scripts/verify/skill-doctor-down.sh` | `docs/verified/14-user-skill/doctor-down.log` |
| the skill's frontmatter and every link in it are valid | test | `tests/skill.rs` | the test run in `scripts/check.sh` |
| a Claude Code session offered the skill for a matching request | Claude Code | `claude -p` where logged in, else `cannot_verify` | `docs/verified/14-user-skill/RESULT.md` |

The run's outcome (`passed`, `failed`, `blocked` or `cannot_verify`) and the
date go in `docs/verified/14-user-skill/RESULT.md`.

## Risks

- The skill goes stale as tools change (plan 13 adds some): it names tool
  families and points at the generated reference rather than listing tools.
- A long skill costs context every time it loads: kept to about 60 lines,
  detail linked from the docs.

## Declined

- A plugin bundle (decision 1): Claude Code only, and a plugin can wrap the
  same folder later.
- A `herdr-mcp doctor` subcommand (decision 2): cannot report a missing
  binary, and adds code to every session's binary.
- Copying the README's install steps into the skill: they would drift; the
  skill links to them.

## What the tests hold

The skill's frontmatter parses with a `name` and a `description`, and every
relative link in it resolves. The doctor exits 0 and prints `ok` lines
against a stubbed healthy setup, and exits 1 with a `FAIL` line when herdr
is unreachable. ShellCheck is clean on the skill's scripts.

## Repository knowledge

Read root `KNOWLEDGE.md` when present before each phase. Follow the global and
repository instructions for recording useful development findings.

## Phases

### Phase 1: the skill and its doctor

- [x] `skills/herdr-mcp/SKILL.md` and `skills/herdr-mcp/scripts/doctor.sh`
  as in `## The decision`, with `scripts/check.sh` running ShellCheck on the
  new folder (expected in `skills/herdr-mcp/`, `scripts/check.sh`), pinned by
  `tests/skill.rs`.
  Landed: ShellCheck also covers `scripts/verify/*.sh`, and `docs/standards.md`
  names the new globs. The doctor takes `HERDR_MCP_DOCTOR_RELEASE_URL` so the
  test reads a `file://` release instead of the GitHub API. The skill names
  only `herdr_whoami` and `herdr_snapshot` plus tool families, and links the
  README and docs by URL, since a copied skill folder has no repo around it.
  `tests/session_names.rs` now also scans `scripts/verify/skill-doctor-ok.sh`
  (nine names).
- [x] Check: `sh scripts/check.sh` passes, and `sh skills/herdr-mcp/scripts/doctor.sh`
  prints one line per check.
  Landed: check passed at 95.77 % line coverage.
- [x] Verify: `scripts/verify/skill-doctor-ok.sh` and
  `scripts/verify/skill-doctor-down.sh` run the doctor against an isolated
  `herdr --session herdr-mcp-test-14 server` and against none, print PASS,
  and save their output under `docs/verified/14-user-skill/`.

### Phase 2: tell users

- [x] README "Teach your agent" (one copy command to
  `~/.claude/skills/herdr-mcp/`) and one paragraph in
  `docs/book/src/getting-started.md` (expected in `README.md`,
  `docs/book/src/getting-started.md`).
- [x] Verify: a `claude -p` run on a throwaway config dir with the skill
  copied in is offered it for "run my tests in another herdr pane"; the
  result, or `cannot_verify` where `claude` is not logged in, is recorded in
  `docs/verified/14-user-skill/RESULT.md`.
  Landed: `cannot_verify` on 2026-11-21, the nested `claude` printed `Not
  logged in`; `RESULT.md` says how to close it on a logged-in machine.
- [x] Check: `sh scripts/check.sh` passes and `sh scripts/docs.sh` passes.

### Phase 3: open the PR

- [x] The branch pushed and its PR opened against the branch it was cut
  from (or, if one is already open for this branch, its body brought up to
  date) with a Conventional Commit title for the whole change, a body naming
  this plan, what changed and why, and the checks run with their result. The
  PR's URL printed as the evidence. Opened, never merged. PR: https://github.com/izam-mohammed/herdr-mcp/pull/25
