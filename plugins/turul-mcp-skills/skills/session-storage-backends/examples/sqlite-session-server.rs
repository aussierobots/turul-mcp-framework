// turul-mcp-server v0.3
// SQLite session storage backend with custom configuration

use std::path::PathBuf;
use std::sync::Arc;
use turul_mcp_derive::mcp_tool;
use turul_mcp_server::prelude::*;
use turul_mcp_session_storage::{SqliteConfig, SqliteSessionStorage};

#[mcp_tool(name = "ping", description = "Simple ping tool")]
async fn ping() -> McpResult<String> {
    Ok("pong".to_string())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure SQLite with custom settings
    let config = SqliteConfig {
        database_path: PathBuf::from("./my-sessions.db"),
        max_connections: 5,             // Connection pool size (default: 10)
        session_timeout_minutes: 60,    // Session TTL in minutes (default: 30)
        cleanup_interval_minutes: 10,   // Background cleanup interval (default: 5)
        max_events_per_session: 500,    // Max SSE events stored per session (default: 1000)
        verify_tables: true,
        create_tables: true, // Auto-create schema on first run (default: true)
        create_database_if_missing: true, // Auto-create .db file (default: true)
        ..Default::default()
    };

    let storage = Arc::new(SqliteSessionStorage::with_config(config).await?);

    let server = McpServer::builder()
        .name("sqlite-session-server")
        .version("1.0.0")
        .with_session_storage(storage)
        .tool_fn(ping)
        .build()?;

    server.run().await?;
    Ok(())
}
