use rmcp::{tool, tool_router, ServiceExt, transport::stdio};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

#[derive(Clone)]
struct Bench;

#[tool_router(server_handler)]
impl Bench {
    #[tool(description = "ping herdr")]
    async fn ping(&self) -> String {
        let path = std::env::var("HERDR_SOCKET_PATH").unwrap();
        let mut s = UnixStream::connect(path).await.unwrap();
        s.write_all(b"{\"id\":\"1\",\"method\":\"ping\",\"params\":{}}\n").await.unwrap();
        let mut line = String::new();
        BufReader::new(s).read_line(&mut line).await.unwrap();
        line.trim_end().to_string()
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let service = Bench.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
