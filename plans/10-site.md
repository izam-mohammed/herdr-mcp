# herdr-mcp has a one-page site that says what it does and how to install it

## The ask

Every repository should have a `site/` folder holding a presentable
one-page site for users, modelled on `mac-notch/site/index.html`, and a
recorded way it is published.

Built so far: `docs/built/BUILT-2026-10-22-1826.md`.

What it means here: there is nothing to install yet, and the repository is
private. A page that says "install with one command" is only true after
`plans/08-release-v0.1.0.md` ships binaries, so this plan is chained after it.
The reference page is one self-contained HTML file (inline CSS, no
scripts, no fetched fonts): a header, one pitch, one button to the latest
release, a footer. How mac-notch's page is published is not recorded in
that repository (no Pages workflow; GitHub's Pages API answered 404 for it
on 22/10/2026), so this plan decides publishing for itself. The README's
voice (middle management jokes, "Cheaper than the human kind") carries over
to the page.

## Alternatives

### Decision 1: where is the page published? (two-way door)

The page has to be reachable at a URL, built from `site/` on `main`, with
no person deploying it.

Searched 2026-10-22: "GitHub Pages deploy static site from folder
actions/deploy-pages vs Cloudflare Pages vs Netlify 2026", "GitHub Pages
publishing source custom workflow upload-pages-artifact"; GitHub's docs on
custom Pages workflows (`actions/upload-pages-artifact@v4`,
`actions/configure-pages@v5`, the `github-pages` environment) and on Pages
availability (public repositories on free plans; private ones on paid
plans), 2026 comparisons of Cloudflare Pages and Netlify.

| Must | Baseline: no site | GitHub Pages, deployed by a workflow | Cloudflare Pages | Netlify |
| --- | --- | --- | --- | --- |
| Reachable at a URL | no | yes, once the repository is public or on a paid plan | yes | yes |
| No new account, service or secret | yes | yes | no (a Cloudflare account and API token) | no (a Netlify account and token) |
| **Survives** | no | **yes** | no | no |

Only GitHub Pages survives, so there is nothing to score against it.

**Chosen: GitHub Pages, deployed from `site/` by a workflow on push to
`main`**, switched on when the repository goes public (see
`## Open decisions`). Revisit if the repository stays private on a free
plan.

### Decision 2: how is the page made? (two-way door)

One page, no content pipeline; it has to load fast and match the reference.

Searched 2026-10-22: "single html landing page vs hugo vs jekyll for a
one-page project site"; the reference `mac-notch/site/index.html`
(140 lines, inline CSS, no script), Hugo's and Jekyll's docs on Pages.

| Criterion | Weight | Hand-written single HTML | Hugo | Jekyll rendering the README |
| --- | ---: | ---: | ---: | ---: |
| Performance efficiency | 45 | 5 | 4 | 3 |
| Fit to the ask | 30 | 4 | 5 | 2 |
| Maintainability of the result | 25 | 4 | 3 | 5 |
| **Weighted total** | **100** | **4.45** | **4.05** | **3.20** |

- Single HTML efficiency 5: one request, nothing fetched; Hugo 4: a build
  step in CI; Jekyll 3: a theme's CSS and fonts.
- Jekyll fit 2: the README's 60-row tool table as a page is not a
  one-pager.
- Maintainability: the single file copies the install line from the
  README (4); Hugo adds a toolchain (3).

**Chosen: hand-written single HTML.** Revisit if a second page is wanted.

## The decision

`site/index.html`, one self-contained page in the reference's structure: a
header with the name and links to Releases and GitHub, the README's
tagline ("You gave your AI agents their own terminals. Now one of them gets
to be the boss."), the dark banner from `assets/banner-dark.svg` inlined,
the one-command install from `plans/08-release-v0.1.0.md` and the `claude mcp add`
and `codex mcp add` lines, the tool areas in one line each (workspaces,
tabs, panes, agents, events), the safety defaults in one line (read-only
outside herdr, never its own pane), a "latest release" link, and a footer
(decision 2). `.github/workflows/pages.yml` uploads `site/` and deploys it
on push to `main` and by hand (decision 1).

New dependencies: `actions/checkout`, `actions/configure-pages`,
`actions/upload-pages-artifact`, `actions/deploy-pages`, pinned by SHA.
New permission: `pages: write` and `id-token: write` on the deploy job
only. A person turns Pages on with the source set to GitHub Actions.

Standards: `docs/standards.md` (format, lint, tests, audit, secret scan,
shell check and build through `sh scripts/check.sh`).

## Where it lands

| Path | New or edit | Phase | What changes | Firm? |
| --- | --- | --- | --- | --- |
| `site/index.html` | new | 1 | the page | fixed |
| `.github/workflows/pages.yml` | new | 1 | deploy `site/` | expected |
| `README.md` | edit | 2 | a link to the site | reuse |

*Reused, not rewritten:*

- `mac-notch/site/index.html`: the structure and approach, not its content
  or colours.
- `README.md`'s tagline and install lines, and `assets/banner-dark.svg`,
  inlined.

## Security

The page is static and public; the risk is the deploy path and the install
command it shows.

| Threat | Here | Handled by | Proved by |
| --- | --- | --- | --- |
| Elevation of privilege: a PR deploys the site | `pages.yml` with `pages: write` | triggers are `push` to `main` and `workflow_dispatch`; write permissions on the deploy job, `permissions: {}` at the top | the PR shows no Pages run |
| Tampering: an action replaced under its tag | four actions | pinned by full SHA | the shared gate's zizmor stage (`plans/09-quality-gates.md`) |
| Spoofing: the page's install command differs from the release's | hand-copied | copied from `README.md` after `plans/08-release-v0.1.0.md`, both in the PR diff | read side by side in the try |
| Information disclosure: a third-party request from the page | fonts or scripts | none fetched; inline CSS only | the browser's network panel shows one request |

No data collected, no cookies.

## Open decisions

- The repository is private: default, the page and workflow land, and
  Pages is switched on when the repository goes public; the user says if
  it should go up earlier on a paid plan.
- A custom domain: default, none.

## Declined

- No site: failed the must.
- Cloudflare Pages and Netlify: failed the must (a new account and token).
- Hugo: lost on efficiency and maintainability for one page.
- Jekyll rendering the README: lost on fit.

## What the tests hold

`sh scripts/check.sh` holds that no code changed. The try checks the page at
phone width with no horizontal scroll, every link resolving, and one network
request.

## Repository knowledge

Read root `KNOWLEDGE.md` when present before each phase. Follow the global and
repository instructions for recording useful development findings.

## Phases

### Phase 1: the rough cut: the page, published (needs you: try it)

- [x] `site/index.html` with the header, tagline, banner, install and
  register lines, tool areas, safety line, latest-release link and footer,
  self-contained, readable at 360 px wide. Landed 2026-11-20: also
  a docs link (plan 11); copy states 57 tools, 7 resources and 3 prompts, as the
  server reports; checked at 360 px in headless Chrome.
- [x] `.github/workflows/pages.yml` uploads `site/` and deploys it on push
  to `main` and by hand, write permissions on the deploy job only, actions
  pinned by SHA; `docs/standards.md` names it under CI. Landed 2026-11-20: SHAs are
  the commits of the `v5` and `v4` tags of the three pages actions; zizmor was
  not installed here, so the shared gate's zizmor stage is the check.
- [x] Check: `sh scripts/check.sh` passes, and opening `site/index.html`
  from disk shows the page with one network request. Landed 2026-11-20:
  headless Chrome loads it from disk with no sub-resource requests (the
  document is the one request).
- [ ] You try it (not done: the repository is private and Pages stays off,
  and this needs a person looking): with the repository public, turn Pages on (Settings,
  Pages, source GitHub Actions), run the workflow by hand, open the URL on
  phone and laptop, and say whether it reads as herdr-mcp's page.

### Phase 2: point people at it

- [x] `README.md` links the site under the banner, and the repository's
  homepage is set to the Pages URL (`gh repo edit --homepage`).
  Landed 2026-11-21: repository made public, Pages on with source GitHub Actions.
- [x] Check: `sh scripts/check.sh` passes, and the site URL answers 200.
  Landed 2026-11-21: `https://izam-mohammed.github.io/herdr-mcp/` answers 200.

### Phase 3: open the PR

- [x] The branch pushed and its PR opened against the branch it was cut from
  (or, if one is already open for this branch, its body brought up to date)
  with a Conventional Commit title for the whole change, a body naming this
  plan, what changed and why, and the checks run with their result. The PR's
  URL printed as the evidence. Opened, never merged. Landed 2026-11-20: https://github.com/izam-mohammed/herdr-mcp/pull/17 (one PR for both plans).
