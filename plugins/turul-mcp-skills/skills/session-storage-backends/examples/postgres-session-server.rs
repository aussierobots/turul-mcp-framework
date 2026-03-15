// turul-mcp-server v0.3
// PostgreSQL session storage backend with connection pool tuning

use std::sync::Arc;
use turul_mcp_derive::mcp_tool;
use turul_mcp_server::prelude::*;
use turul_mcp_session_storage::{PostgresConfig, PostgresSessionStorage};

#[mcp_tool(name = "ping", description = "Simple ping tool")]
async fn ping() -> McpResult<String> {
    Ok("pong".to_string())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Read connection URL from environment
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://mcp:mcp_pass@localhost:5432/mcp_sessions".to_string());

    // Configure PostgreSQL with production-ready settings
    let config = PostgresConfig {
        database_url,
        max_connections: 20,                // Max pool connections (default: 20)
        min_connections: 2,                 // Minimum idle connections (default: 2)
        connection_timeout_secs: 30,        // Connection timeout (default: 30)
        session_timeout_minutes: 60,        // Session TTL (default: 30)
        cleanup_interval_minutes: 5,        // Background cleanup interval (default: 5)
        max_events_per_session: 1000,       // Max SSE events per session (default: 1000)
        enable_pooling_optimizations: true,  // Pool tuning (default: true)
        statement_timeout_secs: 30,         // Query timeout (default: 30)
        verify_tables: true,
        create_tables: true,     // Auto-create schema (default: true)
    };

    let storage = Arc::new(PostgresSessionStorage::with_config(config).await?);

    let server = McpServer::builder()
        .name("postgres-session-server")
        .version("1.0.0")
        .with_session_storage(storage)
        .tool_fn(ping)
        .build()?;

    server.run().await?;
    Ok(())
}
