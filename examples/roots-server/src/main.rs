//! # MCP Roots Test Server
//!
//! Test server providing roots for E2E testing of the MCP roots/list endpoint.
//! This server demonstrates root directory listing and follows MCP 2025-11-25 spec.
//!
//! ## Test Roots Available:
//! - `file:///workspace` - Project Workspace (read/write)
//! - `file:///data` - Data Storage (read/write)
//! - `file:///tmp` - Temporary Files (read/write with auto-cleanup)
//! - `file:///config` - Configuration Files (read-only)
//! - `file:///logs` - Log Files (read-only)
//!
//! ## Usage:
//! ```bash
//! # Start server on random port
//! cargo run --bin roots-server
//!
//! # Start server on specific port
//! cargo run --bin roots-server -- --port 8050
//!
//! # Test with curl
//! curl -X POST http://127.0.0.1:PORT/mcp \
//!   -H "Content-Type: application/json" \
//!   -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}'
//!
//! curl -X POST http://127.0.0.1:PORT/mcp \
//!   -H "Content-Type: application/json" \
//!   -H "Mcp-Session-Id: SESSION_ID" \
//!   -d '{"jsonrpc":"2.0","id":2,"method":"roots/list","params":{}}'
//! ```

use clap::Parser;
use tracing::info;
use turul_mcp_protocol::roots::Root;
use turul_mcp_server::McpServer;

#[derive(Parser)]
#[command(name = "roots-server")]
#[command(about = "MCP Roots Test Server - Provides root directories for E2E testing")]
struct Args {
    /// Port to run the server on (0 = random port assigned by OS)
    #[arg(short, long, default_value = "0")]
    port: u16,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let args = Args::parse();

    // Use specified port or OS ephemeral allocation if 0
    let port = if args.port == 0 {
        // Use OS ephemeral port allocation - reliable for parallel testing
        let listener = std::net::TcpListener::bind("127.0.0.1:0")
            .map_err(|e| format!("Failed to bind to ephemeral port: {}", e))?;
        let port = listener.local_addr()?.port();
        drop(listener); // Release immediately so server can bind to it
        port
    } else {
        args.port
    };

    info!("🚀 Starting MCP Roots Test Server on port {}", port);
    info!("📡 Server URL: http://127.0.0.1:{}/mcp", port);
    info!("");
    info!("🧪 Test Roots Available:");
    info!("   📁 Basic Resources (Coverage):");
    info!("      • file:///workspace - Project Workspace (RW)");
    info!("      • file:///data - Data Storage (RW)");
    info!("      • file:///tmp - Temporary Files (RW, auto-cleanup)");
    info!("      • file:///config - Configuration Files (RO)");
    info!("      • file:///logs - Log Files (RO)");
    info!("");
    info!("💡 Quick Test Commands:");
    info!("   curl -X POST http://127.0.0.1:{}/mcp \\", port);
    info!("     -H 'Content-Type: application/json' \\");
    info!("     -d '{{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{{\"protocolVersion\":\"2025-11-25\",\"capabilities\":{{}},\"clientInfo\":{{\"name\":\"test\",\"version\":\"1.0\"}}}}}}'");
    info!("");
    info!("   curl -X POST http://127.0.0.1:{}/mcp \\", port);
    info!("     -H 'Content-Type: application/json' \\");
    info!("     -H 'Mcp-Session-Id: SESSION_ID' \\");
    info!("     -d '{{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"roots/list\",\"params\":{{}}}}'");
    info!("");

    let server = McpServer::builder()
        .name("roots-server")
        .version("0.2.0")
        .title("MCP Roots Test Server")
        .instructions("Test server demonstrating MCP roots functionality for file system access control and directory discovery. Provides root directories for E2E testing.")
        // Add root directories the server can access
        .root(Root::new("file:///workspace").with_name("Project Workspace"))
        .root(Root::new("file:///data").with_name("Data Storage"))
        .root(Root::new("file:///tmp").with_name("Temporary Files"))
        .root(Root::new("file:///config").with_name("Configuration Files"))
        .root(Root::new("file:///logs").with_name("Log Files"))
        .bind_address(format!("127.0.0.1:{}", port).parse()?)
        .build()?;

    info!("🚀 Roots server listening on http://127.0.0.1:{}/mcp", port);
    info!("📋 Server provides 5 root directories via roots/list endpoint");
    info!("✅ Ready for E2E testing");

    server.run().await?;
    Ok(())
}
