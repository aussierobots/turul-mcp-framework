//! Serves a Server Card (**experimental** upstream; ADR-038) at
//! `GET /mcp/server-card` beside an ordinary MCP endpoint.
//!
//! ```text
//! cargo run -p server-card-server
//! curl -i http://127.0.0.1:8665/mcp/server-card
//! ```
//!
//! `version`, `title` and the remote's `supportedProtocolVersions` come from the
//! server itself; the card adds the reverse-DNS name and the public URL.

use turul_mcp_derive::mcp_tool;
use turul_mcp_ext_server_card::v1::{Remote, RemoteType, ServerCard};
use turul_mcp_server::prelude::*;

#[mcp_tool(name = "forecast", description = "Forecast for a city")]
async fn forecast(city: String) -> McpResult<String> {
    Ok(format!("Sunny in {city}"))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    // The public URL clients connect to. Behind a proxy or gateway this is the
    // external URL, which the server cannot discover for itself.
    let public_url =
        std::env::var("PUBLIC_MCP_URL").unwrap_or_else(|_| "http://127.0.0.1:8665/mcp".into());

    let card = ServerCard::new("io.github.example/weather", "Weather forecasts by city")
        .with_remote(Remote::new(RemoteType::StreamableHttp, public_url));

    let server = McpServer::builder()
        .name("server-card-server")
        .version(env!("CARGO_PKG_VERSION"))
        .title("Server Card Example")
        .tool_fn(forecast)
        .with_server_card(card)
        .bind_address("127.0.0.1:8665".parse()?)
        .build()?;

    tracing::info!("MCP at http://127.0.0.1:8665/mcp, card at /mcp/server-card");
    server.run().await?;
    Ok(())
}
