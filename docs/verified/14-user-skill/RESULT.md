# plan 14 verification

Run on 2026-11-21, herdr 0.9.5, `herdr-mcp 0.1.0`.

| Behaviour | Driven by | Outcome | Date | Evidence |
| --- | --- | --- | --- | --- |
| the doctor reports a healthy setup against an isolated herdr | `scripts/verify/skill-doctor-ok.sh` | passed | 2026-11-21 | `doctor-ok.log` |
| the doctor fails with a clear line when herdr is down | `scripts/verify/skill-doctor-down.sh` | passed | 2026-11-21 | `doctor-down.log` |
| the skill's frontmatter and every link in it are valid | `tests/skill.rs` | passed | 2026-11-21 | `cargo test --test skill`, inside `scripts/check.sh` |
| a Claude Code session is offered the skill for "run my tests in another herdr pane" | `claude -p` with a throwaway `CLAUDE_CONFIG_DIR` holding the skill | cannot_verify | 2026-11-21 | the nested `claude` printed `Not logged in · Please run /login`; no login was copied or searched for |

The isolated session `herdr-mcp-test-14` was stopped and deleted after the
ok run; the default session was not touched.

To close the last row: on a machine where `claude` is logged in, copy
`skills/herdr-mcp/` into a fresh config dir's `skills/` and run
`CLAUDE_CONFIG_DIR=<dir> claude -p "run my tests in another herdr pane"`, then
check the answer or the transcript names the `herdr-mcp` skill.
