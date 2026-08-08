//! # Simple SQLite Storage Backend Example (2026-07-28 lane)
//!
//! Demonstrates wiring a durable SQLite storage backend into an MCP server.
//! On the 2026 stateless core there are NO client-visible sessions — the
//! storage backs the transport's internal per-request contexts and event
//! streams. This example therefore drives the `SessionStorage` backend API
//! DIRECTLY (one demo record per run) so the durability teaching stays
//! true: rows written by previous runs are still in the database file after
//! a restart, observable via the `storage_info` tool's growing counts.
//!
//! ## Features Demonstrated
//!
//! - File-based durable storage using SQLite (config, schema auto-creation)
//! - `with_session_storage()` wiring on the 2026 lane
//! - Driving the backend API directly (`set_session_state`/`get_session_state`)
//! - Restart durability you can observe (counts accumulate across runs)
//!
//! Cross-request APPLICATION state belongs in your own store; the
//! 2025-11-25 stateful session model lives on the opt-in lane (see
//! `stateful-server`).

use serde_json::{Value, json};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{error, info};
use turul_mcp_builders::ToolBuilder;
use turul_mcp_server::McpServer;
use turul_mcp_session_storage::SessionStorage;
use turul_mcp_session_storage::{SqliteConfig, SqliteSessionStorage};

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
        verify_tables: true,
        create_tables: true,
        create_database_if_missing: true,
    };

    info!(
        "Using SQLite database: {}",
        sqlite_config.database_path.display()
    );

    // Create SQLite session storage
    let sqlite_storage = match SqliteSessionStorage::with_config(sqlite_config.clone()).await {
        Ok(storage) => {
            info!("✅ SQLite session storage initialized successfully");
            info!(
                "📁 Database file: {}",
                sqlite_config.database_path.display()
            );
            Arc::new(storage)
        }
        Err(e) => {
            error!("❌ Failed to initialize SQLite session storage: {}", e);
            error!("Check that the database path is writable");
            return Err(e.into());
        }
    };

    // One durable demo record per run — the tools below drive the backend
    // API directly against it.
    let demo = sqlite_storage
        .create_session(Default::default())
        .await
        .map_err(|e| format!("create demo record: {e}"))?;
    let demo_id = Arc::new(demo.session_id);
    info!("📌 This run's demo record id: {demo_id}");

    let store_value = {
        let (storage, id) = (sqlite_storage.clone(), demo_id.clone());
        ToolBuilder::new("store_value")
            .description("Store a value in this run's durable SQLite demo record")
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
                        .map_err(|e| format!("sqlite write: {e}"))?;
                    Ok(json!({ "result": format!("stored {key} in SQLite record {id}") }))
                }
            })
            .build()
            .expect("store_value tool")
    };

    let get_value = {
        let (storage, id) = (sqlite_storage.clone(), demo_id.clone());
        ToolBuilder::new("get_value")
            .description("Read a value back from this run's SQLite demo record")
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
                        .map_err(|e| format!("sqlite read: {e}"))?;
                    Ok(json!({ "result": value }))
                }
            })
            .build()
            .expect("get_value tool")
    };

    let storage_info = {
        let storage = sqlite_storage.clone();
        ToolBuilder::new("storage_info")
            .description(
                "Durable-storage stats — counts accumulate across server restarts, \
                 which is the persistence proof",
            )
            .execute(move |_args| {
                let storage = storage.clone();
                async move {
                    let sessions = storage.session_count().await.unwrap_or(0);
                    let events = storage.event_count().await.unwrap_or(0);
                    Ok(json!({ "result": {
                        "backend": "sqlite",
                        "stored_records": sessions,
                        "stored_events": events,
                        "note": "restart the server and call again — prior runs' rows persist"
                    }}))
                }
            })
            .build()
            .expect("storage_info tool")
    };

    // Build MCP server with the SQLite backend wired in
    let server = McpServer::builder()
        .name("simple-sqlite-session")
        .version(env!("CARGO_PKG_VERSION"))
        .title("SQLite Storage Backend Example")
        .instructions("Demonstrates durable SQLite-backed storage wiring on the 2026 stateless lane. Tools drive the backend API directly against one demo record per run.")
        .with_session_storage(sqlite_storage)
        .tool(store_value)
        .tool(get_value)
        .tool(storage_info)
        .bind_address("127.0.0.1:8061".parse()?)
        .build()?;

    info!("🎉 SQLite session storage example server ready!");
    info!("🚀 Server running at: http://127.0.0.1:8061/mcp");
    info!("📊 Session Storage: SQLite (File-based persistence)");
    info!("🔄 SSE Notifications: Enabled");
    info!(
        "📁 Database File: {}",
        sqlite_config.database_path.display()
    );
    info!("");
    info!("Available tools:");
    info!("  • store_value   - write to this run's durable demo record");
    info!("  • get_value     - read it back (within this run)");
    info!("  • storage_info  - backend stats; counts accumulate across restarts");
    info!("");
    info!("Durability walkthrough:");
    info!("  1. storage_info()              // note stored_records");
    info!("  2. store_value(key='theme', value='dark'); get_value(key='theme')");
    info!("  3. Restart the server (Ctrl+C, then re-run)");
    info!("  4. storage_info()              // stored_records grew — prior rows persist");
    info!("");
    info!(
        "🔧 The database file {} outlives the process — that is the teaching",
        sqlite_config.database_path.display()
    );

    server.run().await?;
    Ok(())
}
