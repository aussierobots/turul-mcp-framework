//! # Simple DynamoDB Storage Backend Example (2026-07-28 lane)
//!
//! Demonstrates wiring a durable DynamoDB storage backend into an MCP
//! server. On the 2026 stateless core there are NO client-visible sessions —
//! the storage backs the transport's internal per-request contexts and event
//! streams. The demo tools drive the `SessionStorage` backend API DIRECTLY
//! against one durable record per run, so the persistence teaching stays
//! observable and true: `storage_info` counts accumulate across restarts.
//!
//! Cross-request APPLICATION state belongs in your own store; the
//! 2025-11-25 stateful session model lives on the opt-in lane (see
//! `stateful-server`).

use serde_json::{Value, json};
use std::sync::Arc;
use tracing::{error, info};
use turul_mcp_builders::ToolBuilder;
use turul_mcp_server::McpServer;
use turul_mcp_session_storage::SessionStorage;
use turul_mcp_session_storage::{DynamoDbConfig, DynamoDbSessionStorage};

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
        region: std::env::var("AWS_REGION").unwrap_or_else(|_| "us-east-1".to_string()),
        session_ttl_minutes: 24 * 60, // 24 hours in minutes
        event_ttl_minutes: 24 * 60,   // 24 hours in minutes
        max_events_per_session: 1000,
        enable_backup: true,
        enable_encryption: true,
        verify_tables: true,
        create_tables: true,
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

    // One durable demo record per run — the tools below drive the backend
    // API directly against it.
    let demo = dynamodb_storage
        .create_session(Default::default())
        .await
        .map_err(|e| format!("create demo record: {e}"))?;
    let demo_id = Arc::new(demo.session_id);
    info!("📌 This run's demo record id: {demo_id}");

    let store_value = {
        let (storage, id) = (dynamodb_storage.clone(), demo_id.clone());
        ToolBuilder::new("store_value")
            .description("Store a value in this run's durable DynamoDB demo record")
            .string_param("key", "Key to store")
            .string_param("value", "Value to store")
            .execute(move |args| {
                let (storage, id) = (storage.clone(), id.clone());
                async move {
                    let key = args
                        .get("key")
                        .and_then(Value::as_str)
                        .ok_or("missing key")?;
                    let value = args.get("value").cloned().unwrap_or(Value::Null);
                    storage
                        .set_session_state(&id, key, value.clone())
                        .await
                        .map_err(|e| format!("dynamodb write: {e}"))?;
                    Ok(json!({ "result": format!("stored {key} in DynamoDB record {id}") }))
                }
            })
            .build()
            .expect("store_value tool")
    };

    let get_value = {
        let (storage, id) = (dynamodb_storage.clone(), demo_id.clone());
        ToolBuilder::new("get_value")
            .description("Read a value back from this run's DynamoDB demo record")
            .string_param("key", "Key to read")
            .execute(move |args| {
                let (storage, id) = (storage.clone(), id.clone());
                async move {
                    let key = args
                        .get("key")
                        .and_then(Value::as_str)
                        .ok_or("missing key")?;
                    let value = storage
                        .get_session_state(&id, key)
                        .await
                        .map_err(|e| format!("dynamodb read: {e}"))?;
                    Ok(json!({ "result": value }))
                }
            })
            .build()
            .expect("get_value tool")
    };

    let storage_info = {
        let storage = dynamodb_storage.clone();
        ToolBuilder::new("storage_info")
            .description(
                "Durable-storage stats — counts accumulate across server restarts, \\
                 which is the persistence proof",
            )
            .execute(move |_args| {
                let storage = storage.clone();
                async move {
                    let sessions = storage.session_count().await.unwrap_or(0);
                    let events = storage.event_count().await.unwrap_or(0);
                    Ok(json!({ "result": {
                        "backend": "dynamodb",
                        "stored_records": sessions,
                        "stored_events": events,
                        "note": "restart the server and call again — prior runs' rows persist"
                    }}))
                }
            })
            .build()
            .expect("storage_info tool")
    };

    // Build MCP server with DynamoDB session storage
    let server = McpServer::builder()
        .name("simple-dynamodb-session")
        .version("1.0.0")
        .title("DynamoDB Session Storage Example")
        .instructions("Demonstrates DynamoDB-backed session storage for MCP servers. Use the tools to store and retrieve values that persist in AWS DynamoDB.")
        .with_session_storage(dynamodb_storage)
        .tool(store_value)
        .tool(get_value)
        .tool(storage_info)
        .bind_address("127.0.0.1:8062".parse()?)
        .sse(true)
        .build()?;

    info!("🎉 DynamoDB session storage example server ready!");
    info!("🚀 Server running at: http://127.0.0.1:8062/mcp");
    info!("📊 Session Storage: AWS DynamoDB");
    info!("🔄 SSE Notifications: Enabled");
    info!("");
    info!("Available tools:");
    info!("  • store_value   - write to this run's durable demo record");
    info!("  • get_value     - read it back (within this run)");
    info!("  • storage_info  - backend stats; counts accumulate across restarts");
    info!("");
    info!("Durability walkthrough:");
    info!("  1. storage_info()              // note stored_records");
    info!("  2. store_value(key='theme', value='dark'); get_value(key='theme')");
    info!("  3. Restart the server");
    info!("  4. storage_info()              // stored_records grew — prior rows persist");

    server.run().await?;
    Ok(())
}
