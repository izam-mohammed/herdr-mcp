# rmcp, the official Rust MCP SDK

Fetched: 22/10/2026 from https://github.com/modelcontextprotocol/rust-sdk
(README), https://docs.rs/rmcp/latest/rmcp/, crates.io
(`max_stable_version` 3.6.0, published 20/10/2026), and the crate source
`rmcp-3.6.0` / `rmcp-macros-3.6.0` downloaded by cargo.
Version: rmcp 3.6.0.

## Authentication

None for stdio. The OAuth and Streamable HTTP features are not used here.

## Cargo features this project uses

```toml
rmcp = { version = "3.6", features = ["server", "transport-io"] }
```

`server` (default): server and tool system. `macros` (default): `#[tool]`,
`#[tool_router]`, `#[prompt]`, `#[prompt_router]`. `transport-io`: stdio
transport (`rmcp::transport::stdio`). Needs `tokio`, `serde`, `schemars` 1.

## What this project calls

### A stdio server with tools

From the README, built and run 22/10/2026 (it answered `initialize` and
`tools/call`):

```rust
use rmcp::{tool, tool_router, ServiceExt, transport::stdio};

#[derive(Clone)]
struct Bench;

#[tool_router(server_handler)]
impl Bench {
    #[tool(description = "ping herdr")]
    async fn ping(&self) -> String { /* ... */ }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let service = Bench.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
```

Tool params: a struct deriving `serde::Deserialize` and
`schemars::JsonSchema`, taken as `Parameters(T)`
(`rmcp::handler::server::wrapper::Parameters`); the input schema is derived
from it.

### Tool attributes

From `rmcp-macros-3.6.0/src/tool.rs`:

```rust
#[tool(name = "my_tool", description = "...",
       annotations(title = "...", read_only_hint = true,
                   destructive_hint = false, idempotent_hint = true,
                   open_world_hint = false),
       meta = /* an expression giving the tool's _meta */)]
```

### Structured output

`rmcp::handler::server::wrapper::Json<T>` (`pub struct Json<T>(pub T)`)
returns `T` as `structuredContent`. For a pass-through result,
`CallToolResult` can be built directly.

### Server info and instructions

`ServerHandler::get_info` returns `ServerInfo` (`InitializeResult`), whose
`instructions: Option<String>` reaches the client.

### Resources, prompts, notifications

- `ServerHandler::list_resources`, `read_resource`, `subscribe`.
- `#[prompt]`, `#[prompt_router]` for prompts.
- On the peer (`rmcp/src/service/server.rs`): `notify_tool_list_changed`,
  `notify_prompt_list_changed`, `notify_resource_list_changed`,
  `notify_resource_updated`.

## Errors

Tool errors return `CallToolResult` with `is_error: true` (the client shows
the text to the model), or an `rmcp::ErrorData` for protocol errors.

## Limits

Not stated in the docs.

## Gotchas

- Advisories (OSV, 22/10/2026): GHSA-33f5-2c5q-wgwj, GHSA-89vp-x53w-74fx /
  RUSTSEC-2026-0189, GHSA-9g45-5xwm-f3wc, GHSA-9pj6-vhgr-3mwh,
  GHSA-c9xm-49cp-xcr9, all in the OAuth or Streamable HTTP code, all fixed
  at or before 2.1.0. 3.6.0 is clear, and stdio uses none of that code.
- The published HTTP benchmark that found a 40 ms latency floor in rmcp was
  in its Streamable HTTP transport (SSE content type), not stdio.
- Several unofficial crates share the name space (`rust-mcp-sdk`, `mcpkit`,
  `mcp-protocol-sdk`); this project uses only `rmcp` from
  `modelcontextprotocol/rust-sdk`.
