//! # Simple SQLite Session Storage Example
//!
//! This example demonstrates SQLite-backed session storage for MCP servers.
//! Perfect for single-instance deployments that need persistent session state
//! without the complexity of a database server.
//!
//! ## Features Demonstrated
//!
//! - File-based session persistence using SQLite
//! - Session state survives server restarts
//! - Lightweight, zero-configuration storage
//! - Automatic database creation and schema migration

use std::sync::Arc;
use std::path::PathBuf;
use turul_mcp_server::{McpServer, McpResult, SessionContext};
use turul_mcp_session_storage::{SqliteSessionStorage, SqliteConfig};
use turul_mcp_derive::McpTool;
use serde_json::{json, Value};
use tracing::{info, error, debug};

/// Tool that stores a key-value pair in this session's SQLite storage
#[derive(McpTool, Default)]
#[tool(name = "store_value", description = "Store a value in this session's SQLite storage (session-scoped)", field = "value")]
struct StoreValueTool {
    #[param(description = "Key to store in session")]
    key: String,
    #[param(description = "Value to store in session")]
    value: Value,
}

impl StoreValueTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<Value> {
        let session = session.ok_or_else(|| turul_mcp_protocol::McpError::SessionError("Session required".to_string()))?;

        debug!("Storing value in SQLite: {} = {}", self.key, self.value);

        // Store value in this session's SQLite storage
        (session.set_state)(&self.key, self.value.clone()).await;

        Ok(json!({
            "stored": true,
            "session_id": session.session_id,
            "key": self.key,
            "value": self.value,
            "storage": "SQLite (session-scoped)",
            "message": format!("Stored '{}' in session {} (SQLite)", self.key, session.session_id),
            "timestamp": chrono::Utc::now()
        }))
    }
}

/// Tool that retrieves a value from this session's SQLite storage
#[derive(McpTool, Default)]
#[tool(name = "get_value", description = "Retrieve a value from this session's SQLite storage (session-scoped)", field = "value")]
struct GetValueTool {
    #[param(description = "Key to retrieve from session")]
    key: String,
}

impl GetValueTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<Value> {
        let session = session.ok_or_else(|| turul_mcp_protocol::McpError::SessionError("Session required".to_string()))?;

        debug!("Getting value from SQLite: {}", self.key);

        // Retrieve value from this session's SQLite storage
        let value = (session.get_state)(&self.key).await;

        Ok(json!({
            "found": value.is_some(),
            "session_id": session.session_id,
            "key": self.key,
            "value": value,
            "storage": "SQLite (session-scoped)",
            "message": if value.is_some() {
                format!("Retrieved '{}' from session {} (SQLite)", self.key, session.session_id)
            } else {
                format!("Key '{}' not found in session {} (SQLite)", self.key, session.session_id)
            },
            "timestamp": chrono::Utc::now()
        }))
    }
}

/// Tool that shows session information
#[derive(McpTool, Default)]
#[tool(name = "session_info", description = "Get information about the SQLite session", field = "value")]
struct SessionInfoTool {}

impl SessionInfoTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<Value> {
        let session = session.ok_or_else(|| turul_mcp_protocol::McpError::SessionError("Session required".to_string()))?;

        Ok(json!({
            "session_id": session.session_id,
            "storage_backend": "SQLite",
            "storage_type": "File-based database",
            "database_file": "./sessions.db",
            "features": [
                "File-based persistence",
                "ACID transactions", 
                "Lightweight & embedded",
                "Zero configuration",
                "Automatic schema creation"
            ],
            "use_cases": [
                "Single-instance deployments",
                "Development environments",
                "Desktop applications",
                "Local persistence needs"
            ],
            "persistence": "Data survives server restarts",
            "concurrency": "Single process access (SQLite limitation)",
            "message": "Session data isolated per session, stored in local SQLite database file",
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

    info!("Starting Simple SQLite Session Storage Example");

    // SQLite configuration
    let sqlite_config = SqliteConfig {
        database_path: std::env::var("SQLITE_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("./sessions.db")),
        max_connections: 5,
        connection_timeout_secs: 30,
        session_timeout_minutes: 30,
        cleanup_interval_minutes: 5,
        max_events_per_session: 500,
        create_tables_if_missing: true,
        create_database_if_missing: true,
    };

    info!("Using SQLite database: {}", sqlite_config.database_path.display());

    // Create SQLite session storage
    let sqlite_storage = match SqliteSessionStorage::with_config(sqlite_config.clone()).await {
        Ok(storage) => {
            info!("✅ SQLite session storage initialized successfully");
            info!("📁 Database file: {}", sqlite_config.database_path.display());
            Arc::new(storage)
        }
        Err(e) => {
            error!("❌ Failed to initialize SQLite session storage: {}", e);
            error!("Check that the database path is writable");
            return Err(e.into());
        }
    };

    // Build MCP server with SQLite session storage
    let server = McpServer::builder()
        .name("simple-sqlite-session")
        .version("1.0.0")
        .title("SQLite Session Storage Example")
        .instructions("Demonstrates SQLite-backed session storage for single-instance MCP servers. Perfect for development and desktop applications.")
        .with_session_storage(sqlite_storage)
        .tool(StoreValueTool::default())
        .tool(GetValueTool::default())
        .tool(SessionInfoTool::default())
        .bind_address("127.0.0.1:8061".parse()?)
        .sse(true)
        .build()?;

    info!("🎉 SQLite session storage example server ready!");
    info!("🚀 Server running at: http://127.0.0.1:8061/mcp");
    info!("📊 Session Storage: SQLite (File-based persistence)");
    info!("🔄 SSE Notifications: Enabled"); 
    info!("📁 Database File: {}", sqlite_config.database_path.display());
    info!("");
    info!("Available tools:");
    info!("  • store_value    - Store value in SQLite");
    info!("  • get_value      - Retrieve value from SQLite");
    info!("  • session_info   - View session storage information");
    info!("");
    info!("Example usage:");
    info!("  1. store_value(key='theme', value='dark')");
    info!("  2. Restart the server (Ctrl+C, then restart)");
    info!("  3. get_value(key='theme')  // Returns 'dark' - persisted in SQLite!");
    info!("  4. session_info()  // View SQLite backend info");
    info!("");
    info!("🔧 Persistence: Data stored in {} survives server restarts", sqlite_config.database_path.display());

    server.run().await?;
    Ok(())
}