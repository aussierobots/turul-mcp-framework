//! # Simple PostgreSQL Storage Backend Example (2026-07-28 lane)
//!
//! Demonstrates wiring a durable PostgreSQL storage backend into an MCP
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
use turul_mcp_session_storage::{PostgresConfig, PostgresSessionStorage};

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
        verify_tables: true,
        create_tables: true,
    };

    info!(
        "Connecting to PostgreSQL at {}",
        mask_db_url(&postgres_config.database_url)
    );

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

    // One durable demo record per run — the tools below drive the backend
    // API directly against it.
    let demo = postgres_storage
        .create_session(Default::default())
        .await
        .map_err(|e| format!("create demo record: {e}"))?;
    let demo_id = Arc::new(demo.session_id);
    info!("📌 This run's demo record id: {demo_id}");

    let store_value = {
        let (storage, id) = (postgres_storage.clone(), demo_id.clone());
        ToolBuilder::new("store_value")
            .description("Store a value in this run's durable PostgreSQL demo record")
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
                        .map_err(|e| format!("postgres write: {e}"))?;
                    Ok(json!({ "result": format!("stored {key} in PostgreSQL record {id}") }))
                }
            })
            .build()
            .expect("store_value tool")
    };

    let get_value = {
        let (storage, id) = (postgres_storage.clone(), demo_id.clone());
        ToolBuilder::new("get_value")
            .description("Read a value back from this run's PostgreSQL demo record")
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
                        .map_err(|e| format!("postgres read: {e}"))?;
                    Ok(json!({ "result": value }))
                }
            })
            .build()
            .expect("get_value tool")
    };

    let storage_info = {
        let storage = postgres_storage.clone();
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
                        "backend": "postgres",
                        "stored_records": sessions,
                        "stored_events": events,
                        "note": "restart the server and call again — prior runs' rows persist"
                    }}))
                }
            })
            .build()
            .expect("storage_info tool")
    };

    // Build MCP server with PostgreSQL session storage
    let server = McpServer::builder()
        .name("simple-postgres-session")
        .version(env!("CARGO_PKG_VERSION"))
        .title("PostgreSQL Session Storage Example")
        .instructions("Demonstrates PostgreSQL-backed session storage for MCP servers. Use the tools to store and retrieve values that persist across server restarts.")
        .with_session_storage(postgres_storage)
        .tool(store_value)
        .tool(get_value)
        .tool(storage_info)
        .bind_address("127.0.0.1:8060".parse()?)
        .sse(true)
        .build()?;

    info!("🎉 PostgreSQL session storage example server ready!");
    info!("🚀 Server running at: http://127.0.0.1:8060/mcp");
    info!("📊 Session Storage: PostgreSQL (Multi-instance support)");
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
