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
use mcp_server::{McpServer, McpResult, SessionContext};
use mcp_session_storage::{PostgresSessionStorage, PostgresConfig};
use mcp_derive::McpTool;
use serde_json::{json, Value};
use tracing::{info, error, debug};

/// Tool that stores user preferences in session state
#[derive(McpTool, Default)]
#[tool(name = "store_preference", description = "Store a user preference that persists across server restarts")]
struct StorePreferenceTool {
    #[param(description = "Preference key (e.g., 'theme', 'language')")]
    key: String,
    #[param(description = "Preference value")]
    value: Value,
}

impl StorePreferenceTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<Value> {
        let session = session.ok_or_else(|| mcp_protocol::McpError::SessionError("Session required".to_string()))?;

        debug!("Storing preference: {} = {}", self.key, self.value);

        // Store preference in PostgreSQL-backed session
        (session.set_state)(&format!("pref_{}", self.key), self.value.clone());

        // Send progress notification
        (session.send_notification)(mcp_server::SessionEvent::Notification(json!({
            "jsonrpc": "2.0",
            "method": "notifications/progress",
            "params": {
                "progressToken": format!("pref_{}", self.key),
                "progress": 1,
                "total": 1
            }
        })));

        Ok(json!({
            "stored": true,
            "key": self.key,
            "value": self.value,
            "message": format!("Stored preference '{}' in PostgreSQL session storage", self.key),
            "timestamp": chrono::Utc::now()
        }))
    }
}

/// Tool that retrieves user preferences from session state
#[derive(McpTool, Default)]
#[tool(name = "get_preference", description = "Retrieve a user preference from persistent session storage")]
struct GetPreferenceTool {
    #[param(description = "Preference key to retrieve")]
    key: String,
}

impl GetPreferenceTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<Value> {
        let session = session.ok_or_else(|| mcp_protocol::McpError::SessionError("Session required".to_string()))?;

        debug!("Getting preference: {}", self.key);

        // Retrieve preference from PostgreSQL-backed session
        let value = (session.get_state)(&format!("pref_{}", self.key));

        Ok(json!({
            "found": value.is_some(),
            "key": self.key,
            "value": value,
            "message": if value.is_some() {
                format!("Retrieved preference '{}' from PostgreSQL", self.key)
            } else {
                format!("Preference '{}' not found in session", self.key)
            },
            "timestamp": chrono::Utc::now()
        }))
    }
}

/// Tool that lists all stored preferences
#[derive(McpTool, Default)]
#[tool(name = "list_preferences", description = "List all stored user preferences")]
struct ListPreferencesTool {}

impl ListPreferencesTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<Value> {
        let session = session.ok_or_else(|| mcp_protocol::McpError::SessionError("Session required".to_string()))?;

        debug!("Listing all preferences");

        // This is a simplified example - in practice you'd need a way to enumerate keys
        // For now, just show that session is available
        let demo_keys = vec!["theme", "language", "timezone"];
        let mut preferences = serde_json::Map::new();

        for key in demo_keys {
            if let Some(value) = (session.get_state)(&format!("pref_{}", key)) {
                preferences.insert(key.to_string(), value);
            }
        }

        Ok(json!({
            "preferences": preferences,
            "session_id": session.session_id,
            "storage_backend": "PostgreSQL",
            "message": "Retrieved preferences from PostgreSQL session storage",
            "timestamp": chrono::Utc::now()
        }))
    }
}

/// Tool that demonstrates session info
#[derive(McpTool, Default)]
#[tool(name = "session_info", description = "Get information about the current session")]
struct SessionInfoTool {}

impl SessionInfoTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<Value> {
        let session = session.ok_or_else(|| mcp_protocol::McpError::SessionError("Session required".to_string()))?;

        Ok(json!({
            "session_id": session.session_id,
            "storage_backend": "PostgreSQL",
            "persistent": true,
            "shared_across_instances": true,
            "features": [
                "Session state persistence",
                "Multi-instance sharing",
                "Automatic cleanup",
                "Transaction support"
            ],
            "message": "Session backed by PostgreSQL - state persists across server restarts",
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
        .instructions("Demonstrates PostgreSQL-backed session storage for MCP servers. Use the tools to store and retrieve preferences that persist across server restarts.")
        .with_session_storage(postgres_storage)
        .tool(StorePreferenceTool::default())
        .tool(GetPreferenceTool::default())
        .tool(ListPreferencesTool::default())
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
    info!("  • store_preference  - Store user preference in PostgreSQL");
    info!("  • get_preference    - Retrieve user preference from PostgreSQL");  
    info!("  • list_preferences  - List all stored preferences");
    info!("  • session_info      - View session storage information");
    info!("");
    info!("Example usage:");
    info!("  1. store_preference(key='theme', value='dark')");
    info!("  2. Restart the server");
    info!("  3. get_preference(key='theme')  // Returns 'dark' - persisted!");
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