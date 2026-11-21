# the check holds coverage at 90 % and every PR passes the shared security gate

## The ask

Every repository should have proper lint and format, tests, a test
coverage floor of 90 % (target 95 %) and security checks (dependency audit,
secret scan, static analysis), all run by one local check command and by CI
on every pull request, reusing the shared security gate in
`izam-mohammed/.github` rather than a parallel one.

Built so far: `docs/built/BUILT-2026-10-22-1826.md`.

What it means here: there is no code yet, but `plans/01-first-version.md`
Phase 1 already lands `scripts/check.sh` (rustfmt, Clippy `-D warnings`,
`cargo test`, cargo-deny, gitleaks, ShellCheck, the build) and
`.github/workflows/check.yml`, as `docs/standards.md` records. Two parts of
the ask are in no plan: a coverage floor, and the shared gate (semgrep for
SAST and zizmor for workflows are missing from plan 01's set). This plan
runs right after plan 01, so every later plan is built under the floor and
the gate.

## Alternatives

### Decision 1: which tool measures coverage? (two-way door)

It has to run inside `scripts/check.sh` on Linux CI and on the user's
machines, and fail the check under a floor.

Searched 2026-10-22: "cargo-llvm-cov vs cargo-tarpaulin vs grcov rust
coverage fail-under 2026"; cargo-llvm-cov's docs (`--fail-under-lines`,
`--fail-under-regions`, wraps `cargo test` and nextest, source-based
`-C instrument-coverage`), the Microsoft Rust training book's coverage
chapter, tarpaulin's README (ptrace on Linux x86-64), a 2025 project's move
from tarpaulin to cargo-llvm-cov.

| Must | cargo-llvm-cov | cargo-tarpaulin | grcov |
| --- | --- | --- | --- |
| Runs on Linux and macOS, x86_64 and arm64 | yes | no (ptrace engine Linux x86-64 only) | yes |
| **Survives** | **yes** | no | **yes** |

| Criterion | Weight | cargo-llvm-cov | grcov |
| --- | ---: | ---: | ---: |
| Performance efficiency | 40 | 4 | 4 |
| Reliability | 30 | 5 | 3 |
| Compatibility | 30 | 5 | 3 |
| **Weighted total** | **100** | **4.60** | **3.40** |

- Both efficiency 4: the same rustc instrumentation, one instrumented build
  and test run.
- cargo-llvm-cov reliability 5: `--fail-under-lines` exits 1 under the
  floor; grcov 3: an aggregator, the threshold scripted around it.
- cargo-llvm-cov compatibility 5: runs the `cargo test` suite as it is;
  grcov 3: `RUSTFLAGS` and `.profraw` files handled by hand.

**Chosen: cargo-llvm-cov.** Revisit if the suite moves to nextest and its
coverage mode changes.

### Decision 2: what floor, from when? (two-way door)

The project is new, so the floor can hold from the first tested code
rather than ratchet up later.

Searched 2026-10-22: "rust coverage floor ratchet vs fixed threshold new
project"; cargo-llvm-cov's `--fail-under-lines` docs, `mac-notch`'s
`scripts/coverage-floor` (one number, raised only).

| Criterion | Weight | 90 % from this plan on | A ratchet from the measured value |
| --- | ---: | ---: | ---: |
| Reliability | 40 | 5 | 4 |
| Fit to the ask | 35 | 5 | 4 |
| Compatibility | 25 | 4 | 5 |
| **Weighted total** | **100** | **4.75** | **4.25** |

Performance efficiency is left out: both run the same command.

- 90 % reliability 5 and fit 5: the ask's floor holds for every plan from
  02 on; a ratchet lets plans 02 to 08 land under it.
- 90 % compatibility 4: plan 01's rough cut may land under 90 %, so this
  plan's first phase adds the tests that close the gap before the floor
  goes on.

**Chosen: 90 % from this plan on**, lines, with 95 % the target. Revisit
if the live-only paths (herdr socket edge cases) keep a plan under 90 %
with nothing a fake socket can reach.

### Decision 3: how does the shared gate cover Rust? (two-way door)

The gate's `ecosystem` input lists `python | node | go | shell`; any other
value still runs gitleaks (blocking), osv-scanner on `Cargo.lock`, the
default semgrep packs and zizmor.

Searched 2026-10-22: "GitHub reusable workflow security scan gitleaks
osv-scanner semgrep zizmor vs CodeQL default setup", "semgrep p/rust
ruleset"; the gate's `security-scan.yml` (the `case` on `ecosystem`),
its `README.md`.

| Criterion | Baseline: `scripts/check.sh` only (cargo-deny, gitleaks) | `security.yml` calling the gate with `ecosystem: rust`, check kept | CodeQL default setup added |
| --- | --- | --- | --- |
| Performance efficiency | one job | four parallel jobs on PRs and weekly | a CodeQL build per PR |
| Fit to the ask | no SAST, no workflow audit | secrets, deps, generic SAST, workflows | SAST only |
| Compatibility | as plan 01 | the same caller file as the Go repositories | a second SAST beside semgrep |

**Chosen: `security.yml` calling the gate with `ecosystem: rust`, the
check kept.** The gate needs no change to run; adding `p/rust` to its
`case` is a follow-up in `izam-mohammed/.github`. Revisit if semgrep's
default packs report nothing useful on Rust.

## The decision

`scripts/check.sh` runs the tests under `cargo llvm-cov --locked
--fail-under-lines 90` in place of plain `cargo test`, printing the
per-file table (decision 1, decision 2). `.github/workflows/security.yml`
calls `izam-mohammed/.github/.github/workflows/security-scan.yml` pinned to
its commit SHA with `ecosystem: rust` (decision 3). `check.yml` installs
cargo-llvm-cov at a pinned version and pins every action by SHA with
`permissions: {}` at the top. `docs/standards.md` gains the coverage row
and the gate.

New dependencies: cargo-llvm-cov (a dev tool, pinned in `check.yml` and
named in `docs/standards.md`) and the `llvm-tools-preview` rustup
component in `rust-toolchain.toml`. Nothing new in the binary. New
permission: `pull-requests: write` on the gate's job only.

Standards: `docs/standards.md` (format, lint, tests, audit, secret scan,
shell check and build through `sh scripts/check.sh`; this plan adds
coverage and the gate).

## Where it lands

Every edited path below lands in `plans/01-first-version.md` first.

| Path | New or edit | Phase | What changes | Firm? |
| --- | --- | --- | --- | --- |
| `scripts/check.sh` | edit | 1 | tests through cargo-llvm-cov with the floor | fixed |
| `rust-toolchain.toml` | edit | 1 | `llvm-tools-preview` component | expected |
| `.github/workflows/check.yml` | edit | 1 | cargo-llvm-cov installed; SHA pins | expected |
| `.github/workflows/security.yml` | new | 2 | the shared gate | expected |
| `docs/standards.md` | edit | 1 | coverage row, the gate | reuse |

*Reused, not rewritten:*

- `scripts/check.sh` from plan 01: extended, not replaced.
- `tests/support/fake_herdr.rs` from plan 01: the fake socket that new
  tests use to reach the floor.
- `izam-mohammed/.github/.github/workflows/security-scan.yml`: called,
  not copied; `claim-your-code`'s `security.yml` is the caller to copy.

## Security

The change touches CI permissions and adds a workflow and a dev tool.

| Threat | Here | Handled by | Proved by |
| --- | --- | --- | --- |
| Tampering: an action replaced under its tag | the workflows' actions | full SHA pins, version in a comment | the gate's zizmor stage reports no unpinned action |
| Tampering: a compromised coverage tool | cargo-llvm-cov installed in CI | exact version pinned, `--locked` install | `check.yml` shows the pinned version |
| Elevation of privilege: a workflow with more than it needs | PRs from forks | `permissions: {}` at the top, grants per job, `persist-credentials: false` | zizmor clean on the PR |
| Information disclosure: a secret committed | any change | gitleaks in the check and stage 1 of the gate (blocking) | the gate's secrets stage green |

No secret is passed to the gate. The checklist followed: the gate's
workflow conventions.

## Open decisions

- Add `rust` (`p/rust`) to the gate's `case` in `izam-mohammed/.github`:
  default, a separate PR there after this lands; until then the default
  packs run.
- `herdr-mcp` is private, so Actions minutes count against the account's
  quota: default, the gate runs on PRs and weekly as in the other
  repositories.

## Declined

- cargo-tarpaulin: failed the must (Linux x86-64 only).
- grcov: lost on reliability and compatibility.
- A ratchet floor: lets the first plans land under the ask's floor.
- `scripts/check.sh` only, and CodeQL: no SAST or workflow audit, or a
  second SAST.

## What the tests hold

The floor holds that every plan from 02 on lands its tests. The gate holds
secrets (blocking), dependencies, SAST and workflow findings (reported).
The floor is proved by setting it to 100 in a scratch copy and seeing the
check fail.

## Repository knowledge

Read root `KNOWLEDGE.md` when present before each phase. Follow the global and
repository instructions for recording useful development findings.

## Phases

### Phase 1: the rough cut: the floor in the check (needs you: try it)

- [x] Tests over `tests/support/fake_herdr.rs` added until line coverage
  is at least 90 %, then `scripts/check.sh` runs `cargo llvm-cov --locked
  --fail-under-lines 90` in place of `cargo test`, printing the per-file
  table, pinned by the check failing with the floor at 100 in a scratch
  copy.
  Landed 2026-11-20: no new tests were needed. The suite already held 95.9 %
  of lines once the harness (`tests/support/mod.rs`) forwarded
  `LLVM_PROFILE_FILE` to the spawned binary (it used `env_clear`, so the
  child's lines counted as 0 %: 37 % total) and let the child exit on drop
  instead of being killed (a killed child loses its counters: 52 %). The
  live-test list in `scripts/check.sh` is one entry per line. Floor at 100
  exits 1 (checked in place).
- [x] `rust-toolchain.toml` gains `llvm-tools-preview`, `check.yml`
  installs cargo-llvm-cov at a pinned version with every action pinned by
  SHA, and `docs/standards.md` gains the coverage row.
- [x] Check: `HERDR_MCP_LIVE=1 sh scripts/check.sh` passes and prints a
  line total of 90 % or more.
  Landed 2026-11-20: passed, line total 95.74 % (non-live suite; the live
  files run after it, uncounted, since CI has no herdr).
- [ ] You try it: run `sh scripts/check.sh` and say whether the coverage
  table and the run time are acceptable on every phase.

### Phase 2: the shared gate

- [x] `.github/workflows/security.yml` calls the shared gate with
  `ecosystem: rust`, pinned to its commit SHA, on PRs and weekly, and
  `docs/standards.md`'s CI and Decided sections name it.
  Landed 2026-11-20: pinned to the gate's current main commit, not the
  sibling repo's older pin; zizmor 1.32 reports no findings on the three
  workflows.
- [x] Check: `sh scripts/check.sh` passes (95.77 % lines, 2026-11-21), and the PR's Security run (green on PR 17)
  reports stage 1 green and its comment lists osv-scanner, semgrep and
  zizmor results.

### Phase 3: open the PR

- [x] The branch pushed and its PR opened against the branch it was cut from
  (or, if one is already open for this branch, its body brought up to date)
  with a Conventional Commit title for the whole change, a body naming this
  plan, what changed and why, and the checks run with their result. The PR's
  URL printed as the evidence. Opened, never merged.
  Landed 2026-11-20: https://github.com/izam-mohammed/herdr-mcp/pull/14

Landed 2026-11-20 (join): on the merged tree the floor held without new tests: 96.50 % of lines
(lanes 06 and 07 included), `check.sh` runs `lane06_live` and `lane07_live` too, and
`tests/session_names.rs` now counts exactly 8 names (7 live files and the bench).
