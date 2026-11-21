# Claude Code opens git worktrees as workspaces, saves and rebuilds layouts, and shows toasts in herdr

## The ask

`features.md` § Worktrees, § Layout and § Notifications: the last three
tool groups. `herdr_notify` is the one with an everyday use ("tests passed
in w3"); worktrees and layouts are for agents that set up their own work.

## Alternatives

### Decision 1: what does the server do with `trust_repository`? (two-way door)

`worktree.create` and `worktree.remove` take `trust_repository`; herdr's
docs do not describe it (`docs/external/herdr-socket-api.md` § Worktrees).

Searched 2026-10-22: "herdr worktree trust_repository"; herdr's socket API
page and CLI reference (silent on it), the schema.

| Criterion | Never send it | Expose it, default false, description warns | Always send true |
| --- | --- | --- | --- |
| Security | herdr's default applies | the model must ask for it, and the client's permission prompt shows it | trusts every repository the model names |
| Fit to the ask | fails where herdr requires trust | works where the user agrees | works always |
| Performance efficiency | equal | equal | equal |

**Chosen: expose it, default false.** Phase 1's live test records what
herdr does without it, in the API doc. Revisit once herdr documents it.

### Decision 2: how is `layout.apply`'s `root` typed? (two-way door)

`root` is a recursive `LayoutNode`.

Searched 2026-10-22: "schemars recursive type $defs", "Claude Code MCP
schema $ref"; schemars docs, Claude Code's MCP docs on schema checks.

| Criterion | Typed `LayoutNode` in Rust | A JSON object passed to herdr, which validates it |
| --- | --- | --- |
| Performance efficiency | parse and re-serialise | one parse |
| Fit to the ask | the model sees the shape in the schema | the shape only in the description; `herdr_layout_export`'s output is the example |
| Compatibility | breaks when herdr adds a node kind | follows herdr |

**Chosen: a JSON object passed through.** `herdr_layout_export` gives the
exact shape to send back. Revisit if models send bad layouts often in the
try.

## The decision

Feature context. Three groups: `worktrees` (`herdr_worktree_list`,
`_create`, `_open`, `_remove` with `force` default false), `layout`
(`herdr_layout_export`, `_apply`, `_set_split_ratio`) and `notify`
(`herdr_notify`). `_remove` and `_apply` are `destructiveHint: true`;
`_remove` and `_apply` refuse a target that holds the server's own pane,
through plan 02's `refuse_self`. `trust_repository` and `root` follow
decisions 1 and 2.

Dependencies and permissions: none new.

Standards: `docs/standards.md` (format, lint, tests, audit, secret scan,
shell check and build through `sh scripts/check.sh`).

## Where it lands

| Path | New or edit | Phase | What changes | Firm? |
| --- | --- | --- | --- | --- |
| `src/tools/notify.rs` | new | 1 | `herdr_notify` | expected |
| `src/tools/worktrees.rs` | new | 1 | worktree tools | expected |
| `src/tools/layout.rs` | new | 2 | layout tools | expected |
| `tests/tools.rs`, `tests/fixtures/` | edit | 1, 2 | the new tools and refusals | reuse |
| `tests/live.rs` | edit | 1, 2 | a worktree of a scratch repo; export then apply | reuse |
| `docs/external/herdr-socket-api.md` | edit | 1 | what `trust_repository` does | reuse |
| `README.md` | edit | 1, 2 | the three groups | reuse |

*Reused, not rewritten:* `HerdrClient::call`, the gate and `refuse_self`
(`src/safety.rs`), the group registration from plan 01, the harnesses.
Searched for an existing notify or layout helper: none.

## Security

What is protected: git working trees on disk (`worktree.remove` with
`force` deletes uncommitted work) and the layout of tabs that hold running
processes. Who can reach it: the MCP client, behind the read-only gate.

| Threat | Here | Handled by | Proved by |
| --- | --- | --- | --- |
| Tampering | `force: true` deletes uncommitted changes | `force` defaults to false and its description says what it deletes; `destructiveHint` | `tests/tools.rs`: `force` absent unless given |
| Elevation of privilege | a repository trusted without the user | decision 1 | `tests/tools.rs`: `trust_repository` absent unless given |
| Elevation of privilege | `_apply` or `_remove` on the server's own tab or workspace | `refuse_self` | `tests/tools.rs`: refused with `self_target` |
| Spoofing | a toast that imitates herdr's own messages | title and body passed as given; herdr shows them as a notification | none in code; noted in the README |

Checklist: OWASP Top 10 for LLM Applications (2025), LLM06 Excessive
Agency.

## Declined

- Never sending `trust_repository`, or always true (decision 1): fails
  where trust is needed, or trusts anything.
- A typed `LayoutNode` (decision 2): re-serialises and breaks on new node
  kinds.

## What the tests hold

Each tool sends the documented method and params; `force` and
`trust_repository` are never sent unless asked; own-target refusals hold;
the live test creates and removes a worktree of a scratch repository in
the isolated session and round-trips a layout through export and apply.

## Repository knowledge

Read root `KNOWLEDGE.md` when present before each phase. Follow the global and
repository instructions for recording useful development findings.

## Phases

Landed 2026-11-20: `herdr_layout_apply` refuses only a `tab_id` holding the
own pane (herdr replaces panes only then; with no `tab_id` it opens a new
tab), and `allow_self` overrides, as for closes. Live tests take an optional
`HERDR_MCP_SCRATCH` dir for the throwaway repository. The bench stayed inside
its budget in `scripts/check.sh`.

### Phase 1: the rough cut: toasts and worktrees (needs you: try it)

- [x] `herdr_notify` (expected in `src/tools/notify.rs`) and the four
  worktree tools (`src/tools/worktrees.rs`); pinned by `tests/tools.rs`
  and by `tests/live.rs` creating and removing a worktree of a scratch git
  repository; what herdr does without `trust_repository` recorded in
  `docs/external/herdr-socket-api.md`; `README.md` updated.
- [x] Check: `HERDR_MCP_LIVE=1 sh scripts/check.sh` passes.
- [ ] You try it: ask Claude Code to open a worktree of this repository on
  a new branch as a workspace and show a toast when it is ready; say
  whether the toast and the workspace are what you expect.
  Landed 2026-11-21 (partial, box stays open): `herdr_worktree_list` and
  `herdr_layout_export` worked through real Claude Code. Not done: opening a
  worktree as a workspace and the toast, since `herdr_worktree_create` and
  `herdr_notify` are hidden outside a herdr pane.

### Phase 2: layouts

- [x] `herdr_layout_export`, `_apply` and `_set_split_ratio` (expected in
  `src/tools/layout.rs`); pinned by `tests/tools.rs` and by
  `tests/live.rs` exporting a two-pane tab and applying it to a new tab.
- [x] Post-implementation review of phases 1 and 2 with the security pass,
  recorded in `docs/research/06-worktrees-layout-notify.md`.
- [x] Check: `HERDR_MCP_LIVE=1 sh scripts/check.sh` passes, and
  `sh scripts/bench.sh` stays inside the budget.

### Phase 3: open the PR

- [x] The branch pushed and its PR opened against the branch it was cut
  from (or, if one is already open for this branch, its body brought up to
  date) with a Conventional Commit title for the whole change, a body
  naming this plan, what changed and why, and the checks run with their
  result. The PR's URL printed as the evidence. Opened, never merged.
