// turul-mcp-server v0.3
// DynamoDB session storage backend with production TTL overrides

use std::sync::Arc;
use turul_mcp_derive::mcp_tool;
use turul_mcp_server::prelude::*;
use turul_mcp_session_storage::{DynamoDbConfig, DynamoDbSessionStorage};

#[mcp_tool(name = "ping", description = "Simple ping tool")]
async fn ping() -> McpResult<String> {
    Ok("pong".to_string())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // IMPORTANT: Default TTL is 5 minutes (for testing).
    // Production deployments MUST override session_ttl_minutes and event_ttl_minutes.
    let config = DynamoDbConfig {
        table_name: "my-mcp-sessions".to_string(), // Default: "mcp-sessions"
        region: std::env::var("AWS_REGION").unwrap_or_else(|_| "us-east-1".to_string()),
        session_ttl_minutes: 1440,      // 24 hours (default: 5 minutes!)
        event_ttl_minutes: 1440,        // 24 hours (default: 5 minutes!)
        max_events_per_session: 1000,   // Max SSE events per session (default: 1000)
        enable_backup: true,            // Point-in-time recovery (default: true)
        enable_encryption: true,        // Server-side encryption (default: true)
        create_tables_if_missing: true, // Auto-create table + GSIs (default: true)
    };

    let storage = Arc::new(DynamoDbSessionStorage::with_config(config).await?);

    let server = McpServer::builder()
        .name("dynamodb-session-server")
        .version("1.0.0")
        .with_session_storage(storage)
        .tool_fn(ping)
        .build()?;

    server.run().await?;
    Ok(())
}
