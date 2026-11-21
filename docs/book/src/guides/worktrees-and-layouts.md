# Worktrees and layouts

## Worktrees

`herdr_worktree_create` creates a git worktree on a new branch and opens it
as a herdr workspace. It writes to the repository's `.git` and to disk, so it
is not marked read-only.

```json
{ "cwd": "/path/to/repo", "branch": "docs-demo", "base": "main", "path": "/path/to/tree" }
```

The result has the new workspace (labelled after the checkout, here `tree`),
its root pane and a `worktree` block with the branch and path. Without `path`,
herdr puts the checkout in `~/.herdr/worktrees/<repo>/<branch>`.

`herdr_worktree_list` lists the worktrees of a repository and which
workspace each is open in, and `herdr_worktree_open` opens an existing one.

`force` and `trust_repository` are sent to herdr only when you pass them.
`herdr_worktree_remove` removes the worktree and closes its workspace; with
`force` it deletes uncommitted work, so leave `force` off unless you asked
for exactly that. It refuses the workspace that holds the server's own pane
(`self_target`).

## Layouts

`herdr_layout_export` returns a tab's layout as data. A tab with two panes
side by side exported as:

```json
{
  "layout": {
    "tab_id": "w1:t1",
    "root": {
      "type": "split", "direction": "right", "ratio": 0.5,
      "first":  { "type": "pane", "pane_id": "w1:p1", "cwd": "/home/me" },
      "second": { "type": "pane", "pane_id": "w1:p2", "cwd": "/home/me" }
    }
  }
}
```

(Trimmed.) `herdr_layout_apply` takes that `root` back. With a `tab_id` it
replaces that tab, and the tab's panes end. Without one it opens a new tab.
Because it can end running processes it is marked destructive, and it
refuses the tab that holds the server's own pane unless `allow_self` is set.

`herdr_layout_set_split_ratio` changes one split: `path` is a list of
booleans that walks down the tree (`[]` is the root split), and `ratio` is
the new share.

## Toasts

`herdr_notify` shows a toast in herdr. With no herdr window attached it
answers `shown: false` and a reason, which is not an error. The title and body
are shown as given.
