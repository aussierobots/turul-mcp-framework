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
use turul_mcp_server::{McpServer, McpResult, SessionContext};
use turul_mcp_session_storage::{SqliteSessionStorage, SqliteConfig};
use turul_mcp_derive::McpTool;
use serde_json::{json, Value};
use tracing::{info, error, debug, warn};

/// Tool that saves user settings to SQLite-backed session storage
#[derive(McpTool, Default)]
#[tool(name = "save_setting", description = "Save a user setting that persists in SQLite database")]
struct SaveSettingTool {
    #[param(description = "Setting name (e.g., 'ui_mode', 'auto_save')")]
    name: String,
    #[param(description = "Setting value")]
    value: Value,
}

impl SaveSettingTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<Value> {
        let session = session.ok_or_else(|| turul_mcp_protocol::McpError::SessionError("Session required".to_string()))?;

        debug!("Saving setting to SQLite: {} = {}", self.name, self.value);

        // Save setting in SQLite-backed session storage
        (session.set_state)(&format!("setting_{}", self.name), self.value.clone());

        Ok(json!({
            "saved": true,
            "name": self.name,
            "value": self.value,
            "storage": "SQLite file-based storage",
            "message": format!("Setting '{}' saved to SQLite database", self.name),
            "timestamp": chrono::Utc::now()
        }))
    }
}

/// Tool that loads user settings from SQLite-backed session storage
#[derive(McpTool, Default)]
#[tool(name = "load_setting", description = "Load a user setting from SQLite database")]
struct LoadSettingTool {
    #[param(description = "Setting name to load")]
    name: String,
}

impl LoadSettingTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<Value> {
        let session = session.ok_or_else(|| turul_mcp_protocol::McpError::SessionError("Session required".to_string()))?;

        debug!("Loading setting from SQLite: {}", self.name);

        // Load setting from SQLite-backed session storage
        let value = (session.get_state)(&format!("setting_{}", self.name));

        Ok(json!({
            "found": value.is_some(),
            "name": self.name,
            "value": value,
            "storage": "SQLite file-based storage",
            "message": if value.is_some() {
                format!("Setting '{}' loaded from SQLite database", self.name)
            } else {
                format!("Setting '{}' not found in SQLite", self.name)
            },
            "timestamp": chrono::Utc::now()
        }))
    }
}

/// Tool that demonstrates persistence by incrementing a counter
#[derive(McpTool, Default)]
#[tool(name = "increment_counter", description = "Increment a persistent counter stored in SQLite")]
struct IncrementCounterTool {
    #[param(description = "Counter name")]
    counter_name: String,
}

impl IncrementCounterTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<Value> {
        let session = session.ok_or_else(|| turul_mcp_protocol::McpError::SessionError("Session required".to_string()))?;

        debug!("Incrementing counter in SQLite: {}", self.counter_name);

        // Get current counter value (or 0 if not exists)
        let key = format!("counter_{}", self.counter_name);
        let current = (session.get_state)(&key)
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        
        let new_value = current + 1;
        (session.set_state)(&key, json!(new_value));

        // Send notification about the increment
        (session.send_notification)(turul_mcp_server::SessionEvent::Notification(json!({
            "jsonrpc": "2.0",
            "method": "notifications/progress", 
            "params": {
                "progressToken": format!("counter_{}", self.counter_name),
                "progress": new_value,
                "total": new_value
            }
        })));

        Ok(json!({
            "counter_name": self.counter_name,
            "previous_value": current,
            "new_value": new_value,
            "storage": "SQLite file-based storage",
            "message": format!("Counter '{}' incremented to {} in SQLite", self.counter_name, new_value),
            "timestamp": chrono::Utc::now()
        }))
    }
}

/// Tool that shows SQLite storage statistics
#[derive(McpTool, Default)]
#[tool(name = "storage_stats", description = "Show SQLite session storage statistics")]
struct StorageStatsTool {}

impl StorageStatsTool {
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
            "message": "Session data stored in local SQLite database file",
            "timestamp": chrono::Utc::now()
        }))
    }
}

/// Tool that backs up session data
#[derive(McpTool, Default)]
#[tool(name = "backup_data", description = "Create a backup of all session data")]  
struct BackupDataTool {
    #[param(description = "Backup description")]
    description: String,
}

impl BackupDataTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<Value> {
        let session = session.ok_or_else(|| turul_mcp_protocol::McpError::SessionError("Session required".to_string()))?;

        debug!("Creating data backup: {}", self.description);

        // Store backup metadata
        let backup_id = format!("backup_{}", chrono::Utc::now().timestamp());
        (session.set_state)(&backup_id, json!({
            "description": self.description,
            "created_at": chrono::Utc::now(),
            "session_id": session.session_id
        }));

        Ok(json!({
            "backup_id": backup_id,
            "description": self.description,
            "created_at": chrono::Utc::now(),
            "storage": "SQLite file can be copied for backup",
            "location": "./sessions.db",
            "message": "Backup metadata stored in SQLite. Copy sessions.db file for full backup.",
            "instructions": [
                "Stop the server",
                "Copy sessions.db to backup location", 
                "Restart the server"
            ]
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
            .unwrap_or_else(|_| "./sessions.db".to_string()),
        connection_pool_size: 5,
        session_timeout_minutes: 30,
        cleanup_interval_minutes: 5,
        max_events_per_session: 500,
        enable_wal_mode: true, // Write-Ahead Logging for better concurrency
        enable_foreign_keys: true,
        busy_timeout_ms: 5000,
    };

    info!("Using SQLite database: {}", sqlite_config.database_path);

    // Create SQLite session storage
    let sqlite_storage = match SqliteSessionStorage::with_config(sqlite_config.clone()).await {
        Ok(storage) => {
            info!("✅ SQLite session storage initialized successfully");
            info!("📁 Database file: {}", sqlite_config.database_path);
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
        .tool(SaveSettingTool::default())
        .tool(LoadSettingTool::default())
        .tool(IncrementCounterTool::default())
        .tool(StorageStatsTool::default())
        .tool(BackupDataTool::default())
        .bind_address("127.0.0.1:8061".parse()?)
        .sse(true)
        .build()?;

    info!("🎉 SQLite session storage example server ready!");
    info!("🚀 Server running at: http://127.0.0.1:8061/mcp");
    info!("📊 Session Storage: SQLite (File-based persistence)");
    info!("🔄 SSE Notifications: Enabled"); 
    info!("📁 Database File: {}", sqlite_config.database_path);
    info!("");
    info!("Available tools:");
    info!("  • save_setting      - Save user setting to SQLite");
    info!("  • load_setting      - Load user setting from SQLite");
    info!("  • increment_counter - Increment persistent counter");
    info!("  • storage_stats     - View SQLite storage information");
    info!("  • backup_data       - Create backup metadata");
    info!("");
    info!("Example usage:");
    info!("  1. save_setting(name='theme', value='dark')");
    info!("  2. increment_counter(counter_name='page_views')");
    info!("  3. Restart the server (Ctrl+C, then restart)");
    info!("  4. load_setting(name='theme')  // Returns 'dark' - persisted in SQLite!");
    info!("  5. increment_counter(counter_name='page_views')  // Continues from previous count");
    info!("");
    info!("🔧 Persistence: Data stored in {} survives server restarts", sqlite_config.database_path);

    server.run().await?;
    Ok(())
}