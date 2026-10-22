# herdr-mcp says how to report a hole and is shared under the MIT licence

## The ask

The owner asked, in their own words, to "have dependabot in all of my
repos", to "have proper security.md file for all of my repos", and for a
"proper licence for all of the repos ... which is a product in nature ...
even for private ones". herdr-mcp is a tool meant to be installed by others, so it gets a
security policy and an MIT licence. Dependabot is skipped for now: see
below.

What is there on `main` (read 22/10/2026):

- No `Cargo.toml`, no other manifest and no `.github/workflows/`: `main`
  holds the plans, the docs and the README, and the server itself lands
  with `plans/01-first-version.md`. Dependabot has nothing to watch, so
  the brief's rule applies: no config, and this is reported.
- No `SECURITY.md`, no `LICENSE`, no licence line in the README.
- No repository topics.

No built snapshot: this change adds repository settings and policy files
and touches no code.

## Alternatives

### Decision 1: which licence (two-way door while private)

Searched 22/10/2026: "MIT vs Apache-2.0 for a developer tool"; the texts
at opensource.org and choosealicense.com.

| Criterion | MIT | Apache-2.0 |
| --- | --- | --- |
| Fit to the ask | short, permissive, the owner's default for tools and apps | permissive, adds a patent grant and NOTICE rules |
| Compatibility | matches `penpot-mcp` and the owner's other tools | matches `ragrank` |
| Reliability | one paragraph, nothing to keep in step | NOTICE file and change notes to keep |

**Chosen: MIT.** The owner's brief names MIT for this repository, and a
tool with one maintainer gains little from the patent clause. Revisit if a
contributor or company asks for a patent grant.

## The decision

Topics are a repository setting, set from the README and description.
`SECURITY.md` follows `decisionsmith`'s, written for this private
repository and its MCP tools. `LICENSE` is MIT (Decision 1), and one line
is added at the end of the README. There is no manifest yet to carry a
`license` field. No Dependabot config: there is no manifest and no
workflow for it to watch.

New dependencies or permissions: none.

Security: no code, input, secret or permission changes; the change only
asks for update PRs and says where to report a vulnerability.

## Where it lands

| Path | New or edit | Phase | What changes | Firm? |
| --- | --- | --- | --- | --- |
| `SECURITY.md` | new | 2 | the security policy | fixed |
| `LICENSE` | new | 3 | the MIT text | fixed |
| `README.md` | edit | 3 | one licence line at the end | expected |

*Reused, not rewritten:*

- `decisionsmith`'s `SECURITY.md` and `penpot-mcp`'s `LICENSE`: the
  shape and the text copied.

## Declined

- Apache-2.0: lost the licence decision; the brief names MIT for this
  repository.
- Dependabot now: no manifest or workflow exists for it to watch. The
  plan that adds `Cargo.toml` or the first workflow should add
  `.github/dependabot.yml` (`cargo`, `github-actions`) with it.
- Renovate: nothing to watch either, for the same reason.

## Phases

### Phase 1: topics

- [x] The repository topics set from the README and description, recorded
  here: `ai-agents`, `claude-code`, `codex`, `herdr`, `mcp`,
  `mcp-server`, `rust`, `terminal`; check: `gh repo view izam-mohammed/herdr-mcp --json repositoryTopics`
  lists them.

### Phase 2: security policy

- [x] `SECURITY.md` says only the latest commit on `main` gets fixes, how
  to report privately to `@izam-mohammed`, what happens next, and the
  scope (the `herdr-mcp` server, its tools and its refusals, the install
  path).

### Phase 3: licence

- [ ] The MIT text lands (expected in `LICENSE`) with
  `Copyright (c) 2026 Izam Mohammed`, and the README ends
  with `MIT, see [LICENSE](LICENSE).`.

### Phase 4: open the PR

- [ ] The branch pushed and its PR opened against the branch it was cut
  from (or, if one is already open for this branch, its body brought up to
  date) with a Conventional Commit title for the whole change, a body naming
  this plan, what changed and why, and the checks run with their result. The
  PR's URL printed as the evidence. Opened, never merged.
