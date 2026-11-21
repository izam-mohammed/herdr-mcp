//! Phase 1 proof: an rmcp stdio server with one `ping` tool that asks herdr.
//! Run by `scripts/prove/rmcp-stdio.sh`; not part of the shipped binary.
use rmcp::{ServiceExt, tool, tool_router, transport::stdio};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

#[derive(Clone)]
struct Prove;

#[tool_router(server_handler)]
impl Prove {
    #[tool(description = "ping herdr")]
    async fn ping(&self) -> String {
        let path = std::env::var("HERDR_SOCKET_PATH").unwrap_or_default();
        let Ok(mut s) = UnixStream::connect(path).await else {
            return "unreachable".into();
        };
        let _ = s
            .write_all(b"{\"id\":\"1\",\"method\":\"ping\",\"params\":{}}\n")
            .await;
        let mut line = String::new();
        let _ = BufReader::new(s).read_line(&mut line).await;
        line.trim_end().to_string()
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    Prove.serve(stdio()).await?.waiting().await?;
    Ok(())
}
