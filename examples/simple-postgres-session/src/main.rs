//! # Simple PostgreSQL Session Storage Example
//!
//! This example demonstrates PostgreSQL-backed session storage for MCP servers.
//! It shows how session state persists across server restarts and can be shared
//! across multiple server instances.
//!
//! ## Setup
//!
//! Start PostgreSQL with Docker:
//! ```bash
//! docker run -d --name postgres-session \
//!   -e POSTGRES_DB=mcp_sessions \
//!   -e POSTGRES_USER=mcp \
//!   -e POSTGRES_PASSWORD=mcp_pass \
//!   -p 5432:5432 \
//!   postgres:15
//! ```
//!
//! ## Features Demonstrated
//!
//! - Session state persistence across server restarts
//! - Multi-instance session sharing
//! - PostgreSQL-backed state management

use std::sync::Arc;
use turul_mcp_server::{McpServer, McpResult, SessionContext};
use turul_mcp_session_storage::{PostgresSessionStorage, PostgresConfig};
use turul_mcp_derive::McpTool;
use serde_json::{json, Value};
use tracing::{info, error, debug};

/// Tool that stores a key-value pair in this session's PostgreSQL storage
#[derive(McpTool, Default)]
#[tool(name = "store_value", description = "Store a value in this session's PostgreSQL storage (session-scoped)")]
struct StoreValueTool {
    #[param(description = "Key to store in session")]
    key: String,
    #[param(description = "Value to store in session")]
    value: Value,
}

impl StoreValueTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<Value> {
        let session = session.ok_or_else(|| turul_mcp_protocol::McpError::SessionError("Session required".to_string()))?;

        debug!("Storing value in PostgreSQL: {} = {}", self.key, self.value);

        // Store value in this session's PostgreSQL storage
        (session.set_state)(&self.key, self.value.clone()).await;

        Ok(json!({
            "stored": true,
            "session_id": session.session_id,
            "key": self.key,
            "value": self.value,
            "storage": "PostgreSQL (session-scoped)",
            "message": format!("Stored '{}' in session {} (PostgreSQL)", self.key, session.session_id),
            "timestamp": chrono::Utc::now()
        }))
    }
}

/// Tool that retrieves a value from this session's PostgreSQL storage
#[derive(McpTool, Default)]
#[tool(name = "get_value", description = "Retrieve a value from this session's PostgreSQL storage (session-scoped)")]
struct GetValueTool {
    #[param(description = "Key to retrieve from session")]
    key: String,
}

impl GetValueTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<Value> {
        let session = session.ok_or_else(|| turul_mcp_protocol::McpError::SessionError("Session required".to_string()))?;

        debug!("Getting value from PostgreSQL: {}", self.key);

        // Retrieve value from this session's PostgreSQL storage
        let value = (session.get_state)(&self.key).await;

        Ok(json!({
            "found": value.is_some(),
            "session_id": session.session_id,
            "key": self.key,
            "value": value,
            "storage": "PostgreSQL (session-scoped)",
            "message": if value.is_some() {
                format!("Retrieved '{}' from session {} (PostgreSQL)", self.key, session.session_id)
            } else {
                format!("Key '{}' not found in session {} (PostgreSQL)", self.key, session.session_id)
            },
            "timestamp": chrono::Utc::now()
        }))
    }
}

/// Tool that shows session information
#[derive(McpTool, Default)]
#[tool(name = "session_info", description = "Get information about the PostgreSQL session")]
struct SessionInfoTool {}

impl SessionInfoTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<Value> {
        let session = session.ok_or_else(|| turul_mcp_protocol::McpError::SessionError("Session required".to_string()))?;

        Ok(json!({
            "session_id": session.session_id,
            "storage_backend": "PostgreSQL",
            "features": [
                "Session state persistence",
                "Multi-instance sharing",
                "Automatic cleanup",
                "Transaction support"
            ],
            "message": "Session data isolated per session, backed by PostgreSQL - persists across server restarts",
            "timestamp": chrono::Utc::now()
        }))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("Starting Simple PostgreSQL Session Storage Example");

    // PostgreSQL configuration
    let postgres_config = PostgresConfig {
        database_url: std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://mcp:mcp_pass@localhost:5432/mcp_sessions".to_string()),
        max_connections: 10,
        min_connections: 2,
        connection_timeout_secs: 30,
        session_timeout_minutes: 60,
        cleanup_interval_minutes: 10,
        max_events_per_session: 1000,
        enable_pooling_optimizations: true,
        statement_timeout_secs: 30,
        create_tables_if_missing: true,
    };

    info!("Connecting to PostgreSQL at {}", mask_db_url(&postgres_config.database_url));

    // Create PostgreSQL session storage
    let postgres_storage = match PostgresSessionStorage::with_config(postgres_config).await {
        Ok(storage) => {
            info!("✅ PostgreSQL session storage connected successfully");
            Arc::new(storage)
        }
        Err(e) => {
            error!("❌ Failed to connect to PostgreSQL: {}", e);
            error!("Make sure PostgreSQL is running and accessible:");
            error!("docker run -d --name postgres-session \\");
            error!("  -e POSTGRES_DB=mcp_sessions \\");
            error!("  -e POSTGRES_USER=mcp \\");
            error!("  -e POSTGRES_PASSWORD=mcp_pass \\");
            error!("  -p 5432:5432 \\");
            error!("  postgres:15");
            return Err(e.into());
        }
    };

    // Build MCP server with PostgreSQL session storage
    let server = McpServer::builder()
        .name("simple-postgres-session")
        .version("1.0.0")
        .title("PostgreSQL Session Storage Example")
        .instructions("Demonstrates PostgreSQL-backed session storage for MCP servers. Use the tools to store and retrieve values that persist across server restarts.")
        .with_session_storage(postgres_storage)
        .tool(StoreValueTool::default())
        .tool(GetValueTool::default())
        .tool(SessionInfoTool::default())
        .bind_address("127.0.0.1:8060".parse()?)
        .sse(true)
        .build()?;

    info!("🎉 PostgreSQL session storage example server ready!");
    info!("🚀 Server running at: http://127.0.0.1:8060/mcp");
    info!("📊 Session Storage: PostgreSQL (Multi-instance support)");
    info!("🔄 SSE Notifications: Enabled");
    info!("");
    info!("Available tools:");
    info!("  • store_value    - Store value in PostgreSQL");
    info!("  • get_value      - Retrieve value from PostgreSQL");  
    info!("  • session_info   - View session storage information");
    info!("");
    info!("Example usage:");
    info!("  1. store_value(key='theme', value='dark')");
    info!("  2. Restart the server");
    info!("  3. get_value(key='theme')  // Returns 'dark' - persisted!");
    info!("  4. session_info()  // View PostgreSQL backend info");
    info!("");
    info!("🔧 Multi-instance: Start multiple servers with same DATABASE_URL to share sessions");

    server.run().await?;
    Ok(())
}

/// Mask sensitive information in database URL for logging
fn mask_db_url(url: &str) -> String {
    if let Some(at_pos) = url.find('@') {
        let (prefix, suffix) = url.split_at(at_pos);
        if let Some(colon_pos) = prefix.rfind(':') {
            format!("{}:***{}", &prefix[..colon_pos], suffix)
        } else {
            url.to_string()
        }
    } else {
        url.to_string()
    }
}