//! # Bilingual Fleet Client
//!
//! One client binary talking to a mixed 2025-11-25 / 2026-07-28 fleet — the
//! rolling-upgrade story. For each URL the bilingual `McpClient` probes
//! `server/discover`; a 2026 server answers, a 2025 server returns `-32601`
//! (or `-32004`) and the client falls back to the `initialize` handshake.
//! The negotiated wire spec then locks for that connection's lifetime.
//!
//! Upgrade sequencing this enables: upgrade servers first (the client
//! negotiates each one independently), clients are never blocked.
//!
//! ```bash
//! # Start one server from each generation:
//! cargo run -p minimal-server               # 2026-07-28, port 8641
//! cargo run -p client-initialise-server -- --port 52950   # 2025-11-25
//!
//! # Then sweep the fleet:
//! cargo run -p bilingual-fleet-client -- \
//!     http://127.0.0.1:8641/mcp http://127.0.0.1:52950/mcp
//! ```

use turul_mcp_client::transport::HttpTransport;
use turul_mcp_client::{McpClient, McpVersion};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "warn".into()),
        )
        .init();

    let urls: Vec<String> = std::env::args().skip(1).collect();
    let urls = if urls.is_empty() {
        println!("No URLs given — using the default demo fleet.");
        println!("(start them with: cargo run -p minimal-server  and");
        println!("  cargo run -p client-initialise-server -- --port 52950)\n");
        vec![
            "http://127.0.0.1:8641/mcp".to_string(),
            "http://127.0.0.1:52950/mcp".to_string(),
        ]
    } else {
        urls
    };

    for url in &urls {
        println!("── {url}");
        match probe(url).await {
            Ok(()) => {}
            Err(e) => println!("   unreachable: {e}"),
        }
        println!();
    }

    Ok(())
}

/// Connect, report the negotiated spec, and run one version-appropriate call.
async fn probe(url: &str) -> Result<(), Box<dyn std::error::Error>> {
    let transport = Box::new(HttpTransport::new(url)?);
    let client = McpClient::new(transport, Default::default());
    client.connect().await?;

    let version = client.negotiated_version().await;
    match version {
        Some(McpVersion::V2026_07_28) => {
            println!("   negotiated: 2026-07-28 (server/discover answered — stateless wire)");
            // 2026-only surface: the retained discover body.
            if let Some(d) = client.discovered_server().await {
                println!("   serverInfo: {:?}", d.server_info);
                println!("   supported : {:?}", d.supported_versions);
            }
        }
        Some(other) => {
            println!(
                "   negotiated: {other:?} (discover refused — fell back to the initialize handshake; Mcp-Session-Id session is live)"
            );
        }
        None => {
            println!("   negotiation failed — no common spec");
            return Ok(());
        }
    }

    // Version-neutral surface: the same list_tools call works on both wires.
    let tools = client.list_tools().await?;
    let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
    println!("   tools ({}): {names:?}", names.len());

    // disconnect() is a no-op on 2026 (nothing to tear down) and a DELETE on
    // a 2025 session — the client routes by the negotiated version.
    client.disconnect().await?;
    Ok(())
}
