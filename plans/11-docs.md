# herdr-mcp has a documentation site with a page for every tool, resource and prompt, generated from the server

## The ask

"Some of the tools need detailed documentation, like ragrank. Have that in
the plans for the repositories if it is required." (22/10/2026)

Built so far: `docs/built/BUILT-2026-10-22-1826.md`.

ragrank is the model: a Sphinx site on Read the Docs with getting started,
guides and an API reference (`ragrank/docs/docs/`, `ragrank/docs/api_reference/`,
`.github/workflows/build-docs.yml`, which pings two Read the Docs webhooks on
a release).

What it means here: herdr-mcp is an MCP server other people will install
and register with Claude Code or Codex. `features.md` specifies 57 tools,
seven resources, three prompts, a safety model (never its own pane,
read-only outside herdr, destructive hints), limits on reads and waits, an
event feed, and configuration by flags and env vars (`HERDR_SOCKET_PATH`,
`HERDR_SESSION`, `HERDR_MCP_READ_ONLY`, `--read-only`, `--tools`,
`--debug`). The README lists each tool in one line; parameters, limits,
error codes, resources, prompts and the safety rules have no page a person
can read. Detailed docs are required: an MCP server other people install,
with many tools and options.

Nothing is built yet: the server lands with `plans/01-first-version.md`
onwards. The repository is private, and GitHub Pages is free only for
public repositories, so `plans/10-site.md` publishes the landing page once
the repository goes public. The site is the landing page; this plan's docs
are the reference, served from the same Pages deploy under `/docs/`, and
this plan is chained after `plans/10-site.md`. The book can be built and
tried locally before that.

## Alternatives

### Decision 1: what builds the docs? (two-way door)

The pages are Markdown; moving to another generator later means a new
config and theme, not rewritten content. The docs need a sidebar, search,
light and dark themes, a way to include generated files, and static output
Pages can serve.

Searched 2026-10-22: "MkDocs Material vs Starlight vs Sphinx vs mdBook vs
Docusaurus 2026", "Material for MkDocs maintenance mode Zensical 2026",
"Zensical release status", "mdBook 0.5 release", "Hugo Hextra theme
FlexSearch hugo module", "mdBook vs Starlight vs Hugo Hextra page weight";
the Material for MkDocs announcement of maintenance mode (9.7.0,
11/11/2025, critical fixes for at least 12 months), Zensical's own docs
(alpha, 0.0.x versions), the mdBook 0.5 changelog, the Hextra theme page,
and ragrank's own Sphinx setup. No published like-for-like benchmark of
page weight or build time across these was found.

| Must | mdBook | Hugo with the Hextra theme | Starlight (Astro) | Sphinx with MyST (ragrank's setup) | Docusaurus | MkDocs Material | Zensical | rustdoc on docs.rs |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Static output Pages can serve | yes | yes | yes | yes | yes | yes | yes | no (docs.rs hosts it) |
| Documents tools and setup for a user, not Rust items | yes | yes | yes | yes | yes | yes | yes | no (crate API only; the crate is a binary) |
| Still maintained, with a stable release | yes | yes | yes | yes | yes | no (maintenance mode since 11/2025) | no (alpha) | yes |
| **Survives** | **yes** | **yes** | **yes** | **yes** | **yes** | no | no | no |

| Criterion | Weight | mdBook | Hugo with Hextra | Starlight | Sphinx with MyST (ragrank's setup) | Docusaurus |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Performance efficiency | 35 | 4 | 4 | 4 | 3 | 2 |
| Fit to the ask | 25 | 4 | 5 | 5 | 4 | 5 |
| Security | 15 | 5 | 3 | 2 | 3 | 2 |
| Maintainability of the result | 15 | 5 | 3 | 3 | 3 | 2 |
| Compatibility | 10 | 5 | 3 | 2 | 2 | 2 |
| **Weighted total** | **100** | **4.40** | **3.85** | **3.60** | **3.15** | **2.75** |

- Efficiency: mdBook, Hugo and Starlight all write static HTML from one
  build with no runtime; each page still loads a search script (mdBook's
  built-in index, Hextra's FlexSearch, Starlight's Pagefind), so none is
  scored 5. Sphinx 3: a Python build and a theme's scripts. Docusaurus 2:
  a React app that hydrates on every page. No byte counts were found to
  separate the first three; Phase 1 records the built size of the first
  page as the baseline.
- Fit: Hextra, Starlight and Docusaurus have richer components (tabs,
  callouts); mdBook has the sidebar, search, themes and `{{#include}}` for
  generated files, which is all the ask needs (4). Sphinx's strength is
  Python autodoc, which a Rust server cannot use (4).
- Security: mdBook is one release binary pinned by version and checksum,
  with no package tree (5). Hugo adds a third-party theme fetched as a
  module (3). Starlight and Docusaurus add Node and an npm tree (2). Sphinx
  adds a pip tree (3).
- Maintainability: mdBook is `book.toml` and `SUMMARY.md` (5); the others
  add a theme pin, a lockfile or a `conf.py` (3, Docusaurus 2).
- Compatibility: mdBook is a Rust tool, installed like the repository's
  other cargo tools (`cargo install --locked`, or its release binary) (5);
  Hugo adds a Go binary (3); Node or Python in a Rust repository (2).

**Chosen: mdBook.** It meets the ask with the fewest moving parts and the
smallest dependency surface. Sphinx, ragrank's setup, scored lower here
because its main gain (Python API autodoc) does not apply to a Rust
server; see `## Open decisions`. Revisit if versioned docs per release are
wanted, which mdBook does not do.

### Decision 2: where are the docs served? (two-way door)

They have to be reachable at a URL, built on `main`, with no person
deploying them, next to the landing page from `plans/10-site.md`. The
repository is private today.

Searched 2026-10-22: "Read the Docs vs GitHub Pages hosting project
documentation private repository free plan 2026", "GitHub Pages
availability private repositories GitHub Free"; the Read the Docs pricing
page (Community is free for open source and ad-supported; private docs
start at a paid plan), GitHub's Pages limits page (free for public
repositories; private ones need a paid plan), and `plans/10-site.md`
decision 1.

| Must | Baseline: GitHub Pages, the `pages.yml` from `plans/10-site.md`, under `/docs/`, once public | Read the Docs (ragrank's setup) | Cloudflare Pages | Vercel |
| --- | --- | --- | --- | --- |
| No new account, service or secret | yes | no (a Read the Docs account; ragrank keeps webhook secrets) | no (account and API token) | no (account and token) |
| No ads and no paid plan | yes, once the repository is public | no (Community is ad-supported and needs a public repository; private docs are paid) | yes | yes |
| **Survives** | **yes** | no | no | no |

Only the baseline survives, so there is nothing to score against it.

**Chosen: GitHub Pages, from the same `pages.yml`, at
`https://izam-mohammed.github.io/herdr-mcp/docs/`, once the repository is
public.** Until then the book is built and read locally. Revisit if the
repository stays private, which reopens a paid plan or a different host.

### Decision 3: how do the reference pages stay true to the server? (two-way door)

57 tool pages, the resources, the prompts and the flags must not drift
from the binary.

Searched 2026-10-22: "rmcp list tools input schema schemars generate
markdown", "cargo test golden file update env var"; `docs/external/rmcp.md`
(tools carry a name, description, annotations and a schemars input
schema), and penpot-mcp's `TestSkillListsEveryTool`, which writes a
Markdown file from the running server and fails when it differs.

| Criterion | Baseline: hand-written pages | A cargo test writes the pages from the in-process server's lists | A script calls `tools/list` on the built binary at docs build time |
| --- | --- | --- | --- |
| Performance efficiency | nothing to run | in `cargo test`, no herdr needed | a release build and a server start before every docs build |
| Fit to the ask | drifts | cannot drift; `scripts/check.sh` fails | cannot drift, but only after a full build |
| Reliability | silent drift | the diff shows in review | output changes only seen after deploy |

**Chosen: a cargo test writes the pages.** `docs_match_server` (expected
in `tests/docs.rs`) lists the server's tools, resources and prompts in
process and writes `docs/book/src/reference/` from them (description,
read-only and destructive hints, a parameter table from the input schema,
the tool group for `--tools`); with `HERDR_MCP_UPDATE_DOCS=1` it writes,
otherwise it fails on a difference. The CLI page is written the same way
from `herdr-mcp --help`. Revisit if a tool group grows past what one page
can hold.

### Decision 4: what checks the built docs? (two-way door)

A broken link or a missing chapter should fail `scripts/check.sh`,
offline.

Searched 2026-10-22: "lychee link checker offline mode local html vs
mdbook-linkcheck mdbook 0.5 compatibility"; lychee's docs (`--offline`
checks local files and HTML), mdBook's `build.create-missing` option. No
statement was found on whether `mdbook-linkcheck` supports mdBook 0.5.

| Criterion | Baseline: `mdbook build` only | `mdbook-linkcheck` backend | lychee `--offline` on the built HTML |
| --- | --- | --- | --- |
| Performance efficiency | fastest | runs inside the build | one extra pass over local files, no network |
| Fit to the ask | missing chapters only | links and anchors | links, local files; anchors to be confirmed in Phase 4 |
| Reliability | misses broken links | `?` on mdBook 0.5 | independent of the generator |

**Chosen: `mdbook build` with `create-missing = false`, any warning
treated as a failure, then lychee `--offline` on the output.** Revisit if
lychee misses broken anchors in Phase 4's planted-link test.

## The decision

The docs are an mdBook book in `docs/book/` (`book.toml`,
`src/SUMMARY.md`, decision 1), built into `site/docs/` by the Pages
workflow from `plans/10-site.md` and published at `/docs/` once the
repository is public (decision 2). The tool, resource, prompt and CLI
reference is generated by `docs_match_server` (decision 3).
`scripts/docs.sh` builds the book and runs lychee offline;
`scripts/check.sh` runs it (decision 4). The getting-started text keeps
the README's voice; reference pages are plain.

Chapters: getting started (install, register with Claude Code or Codex,
start the agent inside a herdr pane, first `herdr_snapshot`),
configuration (socket resolution, sessions, read-only mode, `--tools`,
`--debug`, limits), safety (what it refuses, the hints, which tools to keep
on "ask", untrusted output), guides (watching agents, running and waiting
on commands, events, worktrees and layouts), the generated reference, and
error codes with what to do.

New dependencies: `mdbook` and `lychee`, each pinned by version, installed
with `cargo install --locked` or as a release binary checked by SHA-256 in
CI. No new secret or permission; the Pages permissions are
`plans/10-site.md`'s.

Standards: `docs/standards.md` (format, lint, tests, audit, secret scan,
shell check and build through `sh scripts/check.sh`); this plan adds the
docs check to it.

## Where it lands

| Path | New or edit | Phase | What changes | Firm? |
| --- | --- | --- | --- | --- |
| `docs/book/book.toml`, `docs/book/src/SUMMARY.md` | new | 1 | the book | expected |
| `docs/book/src/getting-started.md` | new | 1 | install to first snapshot | expected |
| `scripts/docs.sh` | new | 1 | build the book, then lychee | expected |
| `tests/docs.rs` | new | 2 | `docs_match_server` | expected |
| `docs/book/src/reference/*.md` | new | 2 | generated reference | fixed |
| `docs/book/src/configuration.md`, `safety.md`, `guides/*.md`, `errors.md` | new | 3 | the rest of the book | expected |
| `scripts/check.sh` | edit | 4 | runs `scripts/docs.sh` | expected |
| `.github/workflows/pages.yml` | edit | 4 | builds the book into `site/docs/` (the file `plans/10-site.md` lands) | expected |
| `site/index.html`, `README.md` | edit | 4 | link to `/docs/` | expected |
| `docs/standards.md`, `CLAUDE.md` | edit | 4 | the docs check; regenerate docs when a tool changes | reuse |

*Reused, not rewritten:*

- The server's tool, resource and prompt registration from
  `plans/01-first-version.md` onwards (expected in `src/server.rs`,
  `src/tools/`): the source of every reference page.
- `features.md` (the safety, limits and events sections) and the README:
  the source text of the safety and configuration chapters.
- `docs/external/herdr-socket-api.md`: the error codes herdr returns.
- `.github/workflows/pages.yml` and `site/` from `plans/10-site.md`: one
  deploy for both.

## Security

The docs are static and public once the repository is. The risks are the
two new tools in the build, the deploy path, and docs that soften the
safety advice.

| Threat | Here | Handled by | Proved by |
| --- | --- | --- | --- |
| Tampering: a replaced `mdbook` or `lychee` | local install and CI | `cargo install --locked` at a pinned version, or the release binary checked by SHA-256 | a wrong checksum in a scratch copy fails the CI step |
| Elevation of privilege: a PR deploys the docs | `pages.yml` | `plans/10-site.md`'s triggers and job permissions, unchanged | the PR shows no Pages run |
| Elevation of privilege: a reader turns off the prompts that guard typing tools | the safety and getting-started pages | the pages say to keep `herdr_agent_prompt`, `herdr_agent_send_keys` and `herdr_pane_send_*` on "ask", and the reference shows each tool's hints | read in the Phase 3 try |
| Information disclosure: session paths or pane output in an example | guide pages | examples use made-up ids and a test session, never output from a real session | read in the PR diff |

No data collected, no cookies. The checklist followed: GitHub's Pages
security notes for custom workflows and the shared gate's workflow rules.

## Open decisions

- ragrank's stack (Sphinx on Read the Docs) was the model named in the
  ask and lost here (decisions 1 and 2): default, mdBook on GitHub Pages,
  unless the user wants ragrank's stack kept for consistency.
- The repository going public is `plans/10-site.md`'s question: default,
  Phase 4's publishing waits for it; the book is built locally until then.

## Declined

- MkDocs Material: failed the must (maintenance mode since 11/2025).
- Zensical: failed the must (alpha).
- rustdoc on docs.rs: failed the musts (hosted elsewhere; documents a
  crate's API, not a server's tools).
- Hugo with Hextra: lost on security, maintainability and compatibility.
- Starlight, Docusaurus: lost on security and compatibility (Node and an
  npm tree); Docusaurus also on efficiency.
- Sphinx with MyST: lost on efficiency and compatibility; its autodoc
  does not apply to Rust.
- Read the Docs, Cloudflare Pages, Vercel: failed the musts (a new
  account; Read the Docs ads or a paid plan for a private repository).
- Hand-written reference pages, and a script against the built binary:
  drift, or a full build before every docs build.
- `mdbook-linkcheck`: support for mdBook 0.5 unconfirmed.

## What the tests hold

`docs_match_server` holds that every registered tool, resource and prompt
has a page with its hints and parameters as the server reports them, and
that the CLI page matches `--help`. `scripts/docs.sh` holds that the book
builds without warnings and every local link resolves. The guides' prose
and the safety advice are checked by reading in the tries.

## Repository knowledge

Read root `KNOWLEDGE.md` when present before each phase. Follow the global and
repository instructions for recording useful development findings.

## Phases

### Phase 1: the rough cut: the book builds locally (needs you: try it)

- [x] `docs/book/` with `book.toml` (`create-missing = false`),
  `SUMMARY.md`, an introduction and `getting-started.md` (install,
  `claude mcp add herdr -- herdr-mcp`, start inside a herdr pane, the
  first `herdr_snapshot`), from the README's install text. Landed 2026-11-20: the book is
  written whole in one pass, not chapter by chapter.
- [x] `scripts/docs.sh` builds the book with a pinned `mdbook`, failing
  on any warning; the built size of `getting-started.html` and its assets
  recorded in the phase's commit message as the baseline. Landed
  2026-11-20: 26434 bytes of HTML, 336921 bytes with its CSS and scripts.
- [x] Check: `sh scripts/check.sh` passes, and `sh scripts/docs.sh`
  writes `docs/book/book/index.html`.
- [x] You try it (not done: needs a person to follow it in a fresh herdr
  pane): run `sh scripts/docs.sh` and `mdbook serve docs/book`,
  open the getting-started page in light and dark, follow it in a fresh
  herdr pane up to the first snapshot, and say whether this is the shape
  you want before the rest is written.
  Ticked 2026-11-21 at the user's request: the docs build and links check pass
  and `/docs/` answers 200; following them in a fresh herdr was not done.

### Phase 2: the reference, generated

- [x] `docs_match_server` (expected in `tests/docs.rs`) writes
  `docs/book/src/reference/` from the in-process server with
  `HERDR_MCP_UPDATE_DOCS=1`: one page per tool group with each tool's
  description, hints and parameter table, a resources page, a prompts
  page and a CLI page from `--help`; without the variable it fails on any
  difference. Landed 2026-11-20: 13 pages (a tools index, one page per
  group, resources, prompts, cli); 57 tools, 7 resources, 3 prompts.
- [x] `CLAUDE.md` says a tool added or renamed also means
  `HERDR_MCP_UPDATE_DOCS=1 cargo test --locked --test docs`.
- [x] Check: `sh scripts/check.sh` passes, and a renamed tool in a scratch
  copy fails `cargo test --locked --test docs` until the pages are
  rewritten.

### Phase 3: configuration, safety and guides

- [x] `configuration.md` (socket resolution, sessions, read-only mode,
  `--tools`, `--debug`, the limits), `safety.md` (what it refuses, the
  hints, which tools to keep on "ask", untrusted output) and `errors.md`
  (herdr's error codes and what to do).
- [x] Guides for watching agents, running and waiting on commands, events,
  and worktrees and layouts, each with an example from a run against the
  isolated `herdr-mcp-test` session. Landed 2026-11-20: examples are from
  a real run on an isolated `herdr-mcp-test-docs` session; the agents guide
  shows an empty agent list because no agent was started.
- [x] Check: `sh scripts/check.sh` passes, and `sh scripts/docs.sh`
  builds with no warning.
- [x] You try it (not done: needs a person reading): read the safety page and one guide, and say whether a
  person new to herdr-mcp would know which tools to leave on "ask".
  Ticked 2026-11-21 at the user's request: a subagent checked the safety page
  and a guide against `src/`; no person read them.

### Phase 4: the docs check and publishing

- [x] `scripts/docs.sh` also runs a pinned lychee `--offline` on the built
  book, and `scripts/check.sh` runs it; a planted broken link and a
  planted broken anchor each fail it. Landed 2026-11-20: lychee also needs
  `--include-fragments` and `--exclude-path 404.html`.
- [x] Once the repository is public and `plans/10-site.md` has landed,
  `pages.yml` builds the book into `site/docs/` before uploading `site/`,
  so one deploy serves both; `site/index.html` and `README.md` link
  `/docs/`. Landed 2026-11-20: edited and checked locally; the workflow has not
  run (repository private, no deploy triggered).
- [x] `docs/standards.md` names the docs check, mdBook and lychee under
  checks and decided.
- [x] Check: `sh scripts/check.sh` passes, and after the workflow runs on
  `main`, `https://izam-mohammed.github.io/herdr-mcp/docs/` answers 200.
  Landed 2026-11-21: check.sh passes, the pages run succeeded, docs answer 200.

### Phase 5: open the PR

- [x] The branch pushed and its PR opened against the branch it was cut from
  (or, if one is already open for this branch, bring the open PR's body up
  to date) with a Conventional Commit title for the whole change, a body
  naming this plan, what changed and why, and the checks run with their
  result. The PR's URL printed as the evidence. Opened, never merged. Landed 2026-11-20: https://github.com/izam-mohammed/herdr-mcp/pull/17 (one PR for both plans).
