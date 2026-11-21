# Review of plans/06-worktrees-layout-notify.md, phases 1 and 2

Reviewed 20/11/2026 on herdr 0.9.5 (protocol 22), after the code landed.

## Scope

`src/tools/{worktrees,layout,notify}.rs`, `tests/lane06_tools.rs` (fake
socket, `tests/fixtures/lane06.jsonl`) and `tests/lane06_live.rs` (isolated
session `herdr-mcp-test-06`, a throwaway git repository under the temp dir).

## Measured on the real herdr

- `worktree.create` and `worktree.remove` work without `trust_repository` on
  a repository with no setup scripts (decision 1 stands: exposed, default
  false, never sent unless asked).
- `worktree.create` without `path` puts the checkout under
  `~/.herdr/worktrees/<repo>/<branch>`; `worktree.remove` deletes that
  checkout and answers `forced: false`.
- `layout.apply` with no `tab_id` opens a new tab and touches nothing else.
  With `tab_id` it replaces that tab: the old tab id is gone and a new one
  (with the old label) holds the layout. Only that case can end a running
  pane, so only it is checked with `refuse_self`; a `workspace_id` alone is
  not.
- `layout.export` returns `layout.root`, which `layout.apply` accepts as
  `root` unchanged (the pane ids in it are ignored); `ratio` is a float and
  `layout.set_split_ratio` takes `path` as a list of booleans, `[]` being the
  root split.
- `notification.show` on a server with no attached client answers
  `{"shown":false,"reason":"no_foreground_client"}`, not an error. The tool
  passes that through.

## Security pass

| Threat | Result |
| --- | --- |
| `force` deletes uncommitted work | absent unless given; asserted in `tests/lane06_tools.rs` |
| Repository trusted without the user | `trust_repository` absent unless given; asserted |
| Remove or rebuild on the server's own place | refused with `self_target` through `refuse_self`; `allow_self` as for closes |
| Destructive hint | `herdr_worktree_remove` and `herdr_layout_apply` only; `--confirm-destructive` marks them; read-only mode drops all writes including `herdr_notify` |
| A toast imitating herdr | passed as given; noted in the README |

## Findings

| Finding | Fix |
| --- | --- |
| The plan said `_apply` refuses a target holding the own pane; herdr only replaces panes when `tab_id` is given. | Refuse `tab_id` only; a new tab in the own workspace is allowed. |
| herdr's `worktree.create` default path is under the user's home. | Live tests pass `path` inside their scratch directory. |
