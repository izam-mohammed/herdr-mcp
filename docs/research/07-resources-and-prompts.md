# Plan 07 review: resources and prompts

Reviewed 2026-11-20 after phases 1 and 2.

- Resources: seven URIs through `ServerHandler::{list_resources,
  list_resource_templates, read_resource}` in `src/resources.rs`, each a
  read-through call; `herdr://events/recent` reads plan 05's ring and is
  listed only when the hub runs. An id is matched against `[A-Za-z0-9:_.-]`
  up to 64 characters before any herdr call; herdr's `*not_found` codes map
  to MCP `resource_not_found` (-32002).
- Caps: pane output uses `limits::cap_read`; any other JSON over 40000
  characters becomes a note instead of cut JSON; `herdr://session` uses the
  snapshot summary rule.
- Prompts: written by hand in `src/prompts.rs` (`list_prompts` and
  `get_prompt`), not `#[prompt_router]`: three static texts do not need a
  router field and its extra wiring in `src/server.rs`. A missing `command`
  is `invalid_params`.
- Subscriptions (decision 1): not built, as chosen. The capability
  advertises no `subscribe` and no `listChanged`. What Claude Code does with
  a subscription has not been tried by a human; the "you try it" box stays
  open.
- rmcp 3.11 detail: `read_resource` and `get_prompt` return
  `ReadResourceResponse` and `GetPromptResponse`; `ReadResourceResult::new(..).into()`
  and `GetPromptResult::new(..).into()` convert.
