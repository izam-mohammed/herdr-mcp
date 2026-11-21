# herdr's state can be attached with @herdr: mentions, and the dashboard is one slash command

## The ask

`features.md` § Resources, § Prompts and § Push to the client. Resources
are read-only views Claude Code attaches when the user types
`@herdr:herdr://...`; prompts become `/herdr:dashboard (MCP)`-style
commands. Push (`notifications/resources/updated`) is to be built only if
the client acts on it.

## Alternatives

### Decision 1: are resource-update notifications built? (two-way door)

Searched 2026-10-22: "Claude Code MCP resources subscribe
notifications/resources/updated"; Claude Code's MCP docs
(`docs/external/claude-code-mcp.md` § Resources and prompts: subscriptions
"not stated"), rmcp's `notify_resource_updated`.

| Criterion | Build `resources/subscribe` and push updates from plan 05's hub | Build none; `list_changed` only where lists change |
| --- | --- | --- |
| Performance efficiency | a message per event to a client that may drop it | nothing sent that nobody reads |
| Fit to the ask | Claude Code acts only in a turn; no documented use of updates | the documented behaviour (lists refetched) |

**Chosen: none, for now.** Phase 1's try checks what Claude Code does with
a subscription; if it acts on updates, a new plan adds them. Revisit if
Claude Code documents resource subscriptions.

### Decision 2: how are resources produced? (two-way door)

Searched 2026-10-22: "rmcp list_resources read_resource
resource templates"; rmcp's `ServerHandler` source
(`docs/external/rmcp.md` § Resources, prompts, notifications).

| Criterion | Read through to herdr on every read | Served from plan 05's hub cache |
| --- | --- | --- |
| Performance efficiency | one 0.2 to 0.4 ms call per read | no call, but the cache must be kept whole |
| Reliability | always current | stale after a gap until refreshed |
| Fit to the ask | | only works with the `events` group on |

**Chosen: read through.** Reads are rare and herdr answers in under half a
millisecond. Revisit if a resource is read more than once a second.

## The decision

Feature context. `ServerHandler::list_resources` and `read_resource`
serve `herdr://session`, `herdr://workspaces`, `herdr://workspace/{id}`,
`herdr://tab/{id}`, `herdr://pane/{id}/output` (plan 03's caps),
`herdr://agents` and `herdr://events/recent` (only with the `events`
group), each a read-through call (decision 2), as `application/json`. No
update notifications (decision 1). Prompts `dashboard`, `check-agents` and
`run-and-watch` through `#[prompt_router]`, each a short instruction that
names the tools to use.

Dependencies and permissions: none new.

Standards: `docs/standards.md` (format, lint, tests, audit, secret scan,
shell check and build through `sh scripts/check.sh`).

## Contract

Resource URIs as listed above, binding. Prompt names `dashboard`,
`check-agents`, `run-and-watch` (argument `command`).

## Where it lands

| Path | New or edit | Phase | What changes | Firm? |
| --- | --- | --- | --- | --- |
| `src/resources.rs` | new | 1 | list and read | expected |
| `src/prompts.rs` | new | 2 | the three prompts | expected |
| `src/server.rs` | edit | 1, 2 | capabilities for resources and prompts | reuse |
| `tests/resources.rs` | new | 1, 2 | resources and prompts on the fake socket | expected |
| `README.md` | edit | 1, 2 | `@herdr:` and `/herdr:` usage | reuse |

*Reused, not rewritten:* `HerdrClient::call`, the caps (`src/limits.rs`),
the event ring (`src/events/ring.rs`) for `herdr://events/recent`.
Searched for existing resource code: none.

## Security

Security: read-only views of what the read tools already return, under the
same caps; no new input beyond a URI matched against fixed patterns (an
unknown URI is `not_found`), no data stored, no secret, dependency or
permission. Pane output in `herdr://pane/{id}/output` is untrusted text,
marked so in the resource description.

## Declined

- Resource-update push (decision 1): no documented client use.
- Serving from the hub cache (decision 2): stale after gaps, and tied to
  the `events` group.

## What the tests hold

Each URI maps to the documented herdr call and returns its result; an
unknown URI is refused; `herdr://events/recent` is listed only with the
`events` group; each prompt returns its text with the argument filled.

## Repository knowledge

Read root `KNOWLEDGE.md` when present before each phase. Follow the global and
repository instructions for recording useful development findings.

## Phases

### Phase 1: the rough cut: @herdr resources (needs you: try it)

- [x] `list_resources` and `read_resource` for the seven URIs (expected in
  `src/resources.rs`), advertised in `src/server.rs`; pinned by
  `tests/resources.rs`; `README.md` updated. Landed 2026-11-20: also
  `list_resource_templates` for the three `{id}` URIs; tests are
  `tests/lane07_tools.rs` and `tests/lane07_live.rs` (own session
  `herdr-mcp-test-07`), not `tests/resources.rs`.
- [x] Check: `HERDR_MCP_LIVE=1 sh scripts/check.sh` passes.
- [ ] You try it: in Claude Code type `@herdr:` and attach
  `herdr://agents`, ask what each agent is doing; and say whether Claude
  Code offered to subscribe or showed updates (that answers decision 1).
  Landed 2026-11-21 (partial, box stays open): Claude Code listed the four
  resources (`herdr://session`, `workspaces`, `agents`, `events/recent`) and
  read `herdr://workspaces` as JSON. Not done: typing `@herdr:` in the prompt
  box and seeing whether Claude Code offers to subscribe, which needs a person
  (decision 1 stays open).

### Phase 2: prompts

- [x] `dashboard`, `check-agents` and `run-and-watch` (expected in
  `src/prompts.rs`); pinned by `tests/resources.rs`; `README.md` updated.
  Landed 2026-11-20: written by hand (`list_prompts`, `get_prompt`), not
  `#[prompt_router]`; pinned by `tests/lane07_tools.rs`.
- [x] Post-implementation review of phases 1 and 2, recorded in
  `docs/research/07-resources-and-prompts.md`, with what the try showed
  about subscriptions. Landed 2026-11-20: the try has not happened, so the
  review says subscriptions are unanswered.
- [x] Check: `HERDR_MCP_LIVE=1 sh scripts/check.sh` passes.

### Phase 3: open the PR

- [x] The branch pushed and its PR opened against the branch it was cut
  from (or, if one is already open for this branch, its body brought up to
  date) with a Conventional Commit title for the whole change, a body
  naming this plan, what changed and why, and the checks run with their
  result. The PR's URL printed as the evidence. Opened, never merged. Landed 2026-11-20: PR #12.
