//! # Simple DynamoDB Session Storage Example
//!
//! This example demonstrates DynamoDB-backed session storage for MCP servers.
//! It shows how session state persists in AWS DynamoDB with automatic TTL cleanup.
//!
//! ## Setup
//!
//! Configure AWS credentials:
//! ```bash
//! export AWS_ACCESS_KEY_ID=your_access_key
//! export AWS_SECRET_ACCESS_KEY=your_secret_key
//! export AWS_REGION=us-east-1
//! ```
//!
//! ## Features Demonstrated
//!
//! - Session state persistence in DynamoDB
//! - Automatic TTL-based cleanup
//! - AWS-native storage backend

use std::sync::Arc;
use turul_mcp_server::{McpServer, McpResult, SessionContext};
use turul_mcp_session_storage::{DynamoDbSessionStorage, DynamoDbConfig};
use turul_mcp_derive::McpTool;
use serde_json::{json, Value};
use tracing::{info, error, debug};

/// Tool that stores a key-value pair in this session's DynamoDB storage
#[derive(McpTool, Default)]
#[tool(name = "store_value", description = "Store a value in this session's DynamoDB storage (session-scoped)")]
struct StoreValueTool {
    #[param(description = "Key to store in session")]
    key: String,
    #[param(description = "Value to store in session")]
    value: Value,
}

impl StoreValueTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<Value> {
        let session = session.ok_or_else(|| turul_mcp_protocol::McpError::SessionError("Session required".to_string()))?;

        debug!("Storing value in DynamoDB: {} = {}", self.key, self.value);

        // Store value in this session's DynamoDB storage
        (session.set_state)(&self.key, self.value.clone()).await;

        Ok(json!({
            "stored": true,
            "session_id": session.session_id,
            "key": self.key,
            "value": self.value,
            "storage": "DynamoDB (session-scoped)",
            "message": format!("Stored '{}' in session {} (DynamoDB)", self.key, session.session_id),
            "timestamp": chrono::Utc::now()
        }))
    }
}

/// Tool that retrieves a value from this session's DynamoDB storage
#[derive(McpTool, Default)]
#[tool(name = "get_value", description = "Retrieve a value from this session's DynamoDB storage (session-scoped)")]
struct GetValueTool {
    #[param(description = "Key to retrieve from session")]
    key: String,
}

impl GetValueTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<Value> {
        let session = session.ok_or_else(|| turul_mcp_protocol::McpError::SessionError("Session required".to_string()))?;

        debug!("Getting value from DynamoDB: {}", self.key);

        // Retrieve value from this session's DynamoDB storage
        let value = (session.get_state)(&self.key).await;

        Ok(json!({
            "found": value.is_some(),
            "session_id": session.session_id,
            "key": self.key,
            "value": value,
            "storage": "DynamoDB (session-scoped)",
            "message": if value.is_some() {
                format!("Retrieved '{}' from session {} (DynamoDB)", self.key, session.session_id)
            } else {
                format!("Key '{}' not found in session {} (DynamoDB)", self.key, session.session_id)
            },
            "timestamp": chrono::Utc::now()
        }))
    }
}

/// Tool that shows session information
#[derive(McpTool, Default)]
#[tool(name = "session_info", description = "Get information about the DynamoDB session")]
struct SessionInfoTool {}

impl SessionInfoTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<Value> {
        let session = session.ok_or_else(|| turul_mcp_protocol::McpError::SessionError("Session required".to_string()))?;

        Ok(json!({
            "session_id": session.session_id,
            "storage_backend": "DynamoDB",
            "features": [
                "Persistent storage",
                "Automatic TTL cleanup",
                "AWS native integration"
            ],
            "message": "Session data isolated per session, backed by AWS DynamoDB",
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

    info!("Starting Simple DynamoDB Session Storage Example");

    // DynamoDB configuration
    let dynamodb_config = DynamoDbConfig {
        table_name: std::env::var("MCP_SESSION_TABLE")
            .unwrap_or_else(|_| "mcp-sessions".to_string()),
        region: std::env::var("AWS_REGION")
            .unwrap_or_else(|_| "us-east-1".to_string()),
        session_ttl_minutes: 24 * 60,  // 24 hours in minutes
        event_ttl_minutes: 24 * 60,   // 24 hours in minutes
        max_events_per_session: 1000,
        enable_backup: true,
        enable_encryption: true,
        create_tables_if_missing: true,
    };

    info!("AWS DynamoDB Configuration:");
    info!("  Table: {}", dynamodb_config.table_name);
    info!("  Region: {}", dynamodb_config.region);

    // Create DynamoDB session storage
    let dynamodb_storage = match DynamoDbSessionStorage::with_config(dynamodb_config).await {
        Ok(storage) => {
            info!("✅ DynamoDB session storage connected successfully");
            Arc::new(storage)
        }
        Err(e) => {
            error!("❌ Failed to connect to DynamoDB: {}", e);
            error!("Make sure AWS credentials are configured:");
            error!("export AWS_ACCESS_KEY_ID=your_access_key");
            error!("export AWS_SECRET_ACCESS_KEY=your_secret_key");
            error!("export AWS_REGION=us-east-1");
            return Err(e.into());
        }
    };

    // Build MCP server with DynamoDB session storage
    let server = McpServer::builder()
        .name("simple-dynamodb-session")
        .version("1.0.0")
        .title("DynamoDB Session Storage Example")
        .instructions("Demonstrates DynamoDB-backed session storage for MCP servers. Use the tools to store and retrieve values that persist in AWS DynamoDB.")
        .with_session_storage(dynamodb_storage)
        .tool(StoreValueTool::default())
        .tool(GetValueTool::default())
        .tool(SessionInfoTool::default())
        .bind_address("127.0.0.1:8062".parse()?)
        .sse(true)
        .build()?;

    info!("🎉 DynamoDB session storage example server ready!");
    info!("🚀 Server running at: http://127.0.0.1:8062/mcp");
    info!("📊 Session Storage: AWS DynamoDB");
    info!("🔄 SSE Notifications: Enabled");
    info!("");
    info!("Available tools:");
    info!("  • store_value    - Store value in DynamoDB");
    info!("  • get_value      - Retrieve value from DynamoDB");
    info!("  • session_info   - View session storage information");
    info!("");
    info!("Example usage:");
    info!("  1. store_value(key='theme', value='dark')");
    info!("  2. Restart the server");
    info!("  3. get_value(key='theme')  // Returns 'dark' - persisted!");
    info!("  4. session_info()  // View DynamoDB backend info");

    server.run().await?;
    Ok(())
}