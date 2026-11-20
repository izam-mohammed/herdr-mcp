use herdr_mcp::config::{self, Env, Parsed};
use herdr_mcp::server::Server;
use rmcp::{ServiceExt, transport::stdio};

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let config = match config::parse(&args, &Env::from_process()) {
        Ok(Parsed::Run(config)) => *config,
        Ok(Parsed::Exit(text)) => {
            println!("{text}");
            return Ok(());
        }
        Err(message) => {
            eprintln!("herdr-mcp: {message}");
            std::process::exit(2);
        }
    };
    eprintln!(
        "herdr-mcp {} socket={} read_only={}",
        env!("CARGO_PKG_VERSION"),
        config.socket.display(),
        config.read_only
    );
    Server::new(config).serve(stdio()).await?.waiting().await?;
    Ok(())
}
