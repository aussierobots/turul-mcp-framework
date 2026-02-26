//! # MCP Initialize Test Server
//!
//! A simple MCP server for testing session management and initialize lifecycle.
//! This server implements proper MCP session creation where:
//! - Server generates session IDs (not client)
//! - Session IDs are returned in Mcp-Session-Id headers
//! - Sessions persist for subsequent requests
//!
//! ## Usage
//! ```bash
//! # Start server on default port (8641)
//! cargo run --package client-initialise-server
//! ```
//!
//! ## Test with Client
//! ```bash
//! # In another terminal:
//! cargo run --package client-initialise-report -- --url http://127.0.0.1:8641/mcp
//! ```

use anyhow::Result;
use serde::Deserialize;
use serde_json::json;
use std::sync::{Arc, OnceLock};
use tracing::{debug, info};

use turul_mcp_derive::McpTool;
use turul_mcp_protocol::logging::LoggingLevel;
use turul_mcp_server::prelude::*;
use turul_mcp_session_storage::InMemorySessionStorage;
#[cfg(feature = "dynamodb")]
use turul_mcp_session_storage::{DynamoDbConfig, DynamoDbSessionStorage};
#[cfg(feature = "postgres")]
use turul_mcp_session_storage::{PostgresConfig, PostgresSessionStorage};
#[cfg(feature = "sqlite")]
use turul_mcp_session_storage::{SqliteConfig, SqliteSessionStorage};

/// Shared session storage accessible by tools via OnceLock
static SESSION_STORAGE: OnceLock<Arc<turul_mcp_session_storage::BoxedSessionStorage>> =
    OnceLock::new();

/// Echo SSE tool that demonstrates MCP notifications via SessionContext
#[derive(McpTool, Clone, Default, Deserialize)]
#[tool(
    name = "echo_sse",
    description = "Echoes text back via POST response and streams it via SSE. Server logs all calls."
)]
pub struct EchoSseTool {
    #[param(description = "Text to echo back")]
    pub text: String,
}

impl EchoSseTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<serde_json::Value> {
        // Log the call on the server side
        info!("echo_sse called with text: '{}'", self.text);

        // Send a progress notification if we have a session
        if let Some(session_context) = &session {
            info!("Sending progress notification via SessionContext");
            session_context.notify_progress("echo_processing", 50).await;

            // Also send a log message notification
            session_context
                .notify_log(
                    LoggingLevel::Info,
                    json!(format!("Processing echo for text: '{}'", self.text)),
                    Some("echo-tool".to_string()),
                    None,
                )
                .await;
        } else {
            info!("No session context available for notifications");
        }

        // Create response text
        let response_text = format!("Echo: {}", self.text);
        info!("Echo response created: {}", response_text);

        // Send completion notification
        if let Some(session_context) = &session {
            session_context
                .notify_progress("echo_processing", 100)
                .await;
            session_context
                .notify_log(
                    LoggingLevel::Info,
                    json!(format!("Echo completed successfully: '{}'", response_text)),
                    Some("echo-tool".to_string()),
                    None,
                )
                .await;
        }

        Ok(json!({"result": response_text}))
    }
}

/// Session inspection tool that returns current session data
#[derive(McpTool, Clone, Default, Deserialize)]
#[tool(
    name = "get_session_data",
    description = "Returns current session information from session storage"
)]
pub struct GetSessionDataTool {}

impl GetSessionDataTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<serde_json::Value> {
        info!("get_session_data called");

        let storage = SESSION_STORAGE.get().ok_or_else(|| {
            turul_mcp_protocol::McpError::tool_execution("Session storage not initialized")
        })?;

        if let Some(session_ctx) = &session {
            // Get session info from storage
            match storage.get_session(&session_ctx.session_id).await {
                Ok(Some(session_info)) => {
                    // Determine data source (direct from storage backend)
                    let storage_type = if cfg!(feature = "dynamodb") {
                        "DynamoDB"
                    } else if cfg!(feature = "postgres") {
                        "PostgreSQL"
                    } else if cfg!(feature = "sqlite") {
                        "SQLite"
                    } else {
                        "InMemory"
                    };

                    let session_data = json!({
                        "session_id": session_info.session_id,
                        "client_capabilities": session_info.client_capabilities,
                        "server_capabilities": session_info.server_capabilities,
                        "is_initialized": session_info.is_initialized,
                        "created_at": session_info.created_at,
                        "last_activity": session_info.last_activity,
                        "state": session_info.state,
                        "metadata": session_info.metadata,
                        "data_source": {
                            "source_type": "storage_backend",
                            "backend_type": storage_type,
                            "cache_status": "direct_read",
                            "retrieved_at": chrono::Utc::now().timestamp_millis(),
                            "session_table": match storage_type {
                                "DynamoDB" => "mcp-sessions",
                                "SQLite" => "sessions",
                                "PostgreSQL" => "sessions",
                                _ => "in_memory"
                            }
                        }
                    });

                    info!("Retrieved session data for: {}", session_ctx.session_id);
                    Ok(session_data)
                }
                Ok(None) => {
                    let error_msg =
                        format!("Session {} not found in storage", session_ctx.session_id);
                    info!("{}", error_msg);
                    Ok(json!({"error": error_msg}))
                }
                Err(e) => {
                    let error_msg = format!("Failed to retrieve session data: {}", e);
                    info!("{}", error_msg);
                    Ok(json!({"error": error_msg}))
                }
            }
        } else {
            let error_msg = "No session context available";
            info!("{}", error_msg);
            Ok(json!({"error": error_msg}))
        }
    }
}

/// Session events inspection tool that returns events for the current session
#[derive(McpTool, Clone, Default, Deserialize)]
#[tool(
    name = "get_session_events",
    description = "Returns SSE events for the current session from session storage"
)]
pub struct GetSessionEventsTool {
    #[param(
        description = "Maximum number of events to return (default: 10)",
        optional
    )]
    pub limit: Option<u64>,
}

impl GetSessionEventsTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<serde_json::Value> {
        info!("get_session_events called");

        let storage = SESSION_STORAGE.get().ok_or_else(|| {
            turul_mcp_protocol::McpError::tool_execution("Session storage not initialized")
        })?;

        let limit = self.limit.unwrap_or(10) as usize;

        if let Some(session_ctx) = &session {
            match storage
                .get_recent_events(&session_ctx.session_id, limit)
                .await
            {
                Ok(events) => {
                    let events_data: Vec<serde_json::Value> = events
                        .into_iter()
                        .map(|event| {
                            json!({
                                "id": event.id,
                                "event_type": event.event_type,
                                "data": event.data,
                                "timestamp": event.timestamp,
                                "retry": event.retry
                            })
                        })
                        .collect();

                    // Determine data source (direct from storage backend)
                    let storage_type = if cfg!(feature = "dynamodb") {
                        "DynamoDB"
                    } else if cfg!(feature = "postgres") {
                        "PostgreSQL"
                    } else if cfg!(feature = "sqlite") {
                        "SQLite"
                    } else {
                        "InMemory"
                    };

                    let response = json!({
                        "session_id": session_ctx.session_id,
                        "event_count": events_data.len(),
                        "events": events_data,
                        "data_source": {
                            "source_type": "storage_backend",
                            "backend_type": storage_type,
                            "cache_status": "direct_read",
                            "retrieved_at": chrono::Utc::now().timestamp_millis(),
                            "query_limit": limit,
                            "events_table": match storage_type {
                                "DynamoDB" => "mcp-sessions-events",
                                "SQLite" => "events",
                                "PostgreSQL" => "events",
                                _ => "in_memory"
                            }
                        }
                    });

                    info!(
                        "Retrieved {} events for session: {}",
                        events_data.len(),
                        session_ctx.session_id
                    );
                    Ok(response)
                }
                Err(e) => {
                    let error_msg = format!("Failed to retrieve session events: {}", e);
                    info!("{}", error_msg);
                    Ok(json!({"error": error_msg}))
                }
            }
        } else {
            let error_msg = "No session context available";
            info!("{}", error_msg);
            Ok(json!({"error": error_msg}))
        }
    }
}

/// Session table information inspection tool
#[derive(McpTool, Clone, Default, Deserialize)]
#[tool(
    name = "get_table_info",
    description = "Returns information about session and events tables in the storage backend"
)]
pub struct GetTableInfoTool {}

impl GetTableInfoTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<serde_json::Value> {
        info!("get_table_info called");

        let storage = SESSION_STORAGE.get().ok_or_else(|| {
            turul_mcp_protocol::McpError::tool_execution("Session storage not initialized")
        })?;

        debug!("Using session storage for table info: {:p}", storage);

        // Determine storage backend type and table information
        let storage_type = if cfg!(feature = "dynamodb") {
            "DynamoDB"
        } else if cfg!(feature = "postgres") {
            "PostgreSQL"
        } else if cfg!(feature = "sqlite") {
            "SQLite"
        } else {
            "InMemory"
        };

        let table_info = json!({
            "storage_backend": storage_type,
            "session_table": {
                "name": match storage_type {
                    "DynamoDB" => "mcp-sessions",
                    "SQLite" => "sessions",
                    "PostgreSQL" => "sessions",
                    _ => "in_memory"
                },
                "description": "Stores session metadata, capabilities, and state",
                "primary_key": match storage_type {
                    "DynamoDB" => "session_id (String)",
                    "SQLite" | "PostgreSQL" => "session_id (UUID)",
                    _ => "session_id (String)"
                }
            },
            "events_table": {
                "name": match storage_type {
                    "DynamoDB" => "mcp-sessions-events",
                    "SQLite" => "events",
                    "PostgreSQL" => "events",
                    _ => "in_memory"
                },
                "description": "Stores SSE events and notifications for resumability",
                "primary_key": match storage_type {
                    "DynamoDB" => "session_id (String) + event_id (Number)",
                    "SQLite" | "PostgreSQL" => "id (auto-increment) + session_id (UUID)",
                    _ => "event_id (u64)"
                }
            },
            "features": {
                "ttl_enabled": match storage_type {
                    "DynamoDB" => true,
                    "SQLite" | "PostgreSQL" => false, // Handled by application cleanup
                    _ => false
                },
                "automatic_cleanup": match storage_type {
                    "DynamoDB" => "TTL-based (24 hours default)",
                    "SQLite" | "PostgreSQL" => "Application-based cleanup",
                    _ => "Memory-based (session lifetime)"
                }
            },
            "data_source": {
                "source_type": "storage_metadata",
                "backend_type": storage_type,
                "cache_status": "static_configuration",
                "retrieved_at": chrono::Utc::now().timestamp_millis()
            }
        });

        info!("Retrieved table information for backend: {}", storage_type);
        Ok(table_info)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    info!("Starting MCP Initialize Test Server");
    info!("   Server creates and manages session IDs");
    info!("   Session IDs returned via Mcp-Session-Id header");

    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();
    let mut port = 8641;
    let mut storage_backend = "inmemory".to_string(); // Default to InMemory storage
    let mut create_tables = false; // Default to not creating tables

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--port" => {
                if i + 1 < args.len() {
                    port = args[i + 1].parse().unwrap_or(8641);
                    i += 2;
                } else {
                    i += 1;
                }
            }
            "--storage-backend" => {
                if i + 1 < args.len() {
                    storage_backend = args[i + 1].clone();
                    i += 2;
                } else {
                    i += 1;
                }
            }
            "--create-tables" => {
                create_tables = true;
                i += 1;
            }
            _ => i += 1,
        }
    }

    let bind_address: std::net::SocketAddr = format!("127.0.0.1:{}", port).parse()?;
    info!("   Binding to: http://{}/mcp", bind_address);

    // Build server using builder pattern with appropriate storage backend
    let server = match storage_backend.as_str() {
        "sqlite" => {
            #[cfg(feature = "sqlite")]
            {
                let temp_dir = std::env::temp_dir();
                info!("   System temp directory: {}", temp_dir.display());

                // Create a subdirectory for MCP sessions
                let mcp_temp_dir = temp_dir.join("mcp-sessions");
                std::fs::create_dir_all(&mcp_temp_dir)
                    .map_err(|e| anyhow::anyhow!("Failed to create MCP temp directory: {}", e))?;

                let db_path = mcp_temp_dir.join("mcp_sessions.db");
                info!(
                    "   Using SQLite session storage (database: {})",
                    db_path.display()
                );

                let config = SqliteConfig {
                    database_path: db_path,
                    create_tables_if_missing: create_tables,
                    ..Default::default()
                };
                if create_tables {
                    info!("   Table creation enabled: Will create tables if missing");
                } else {
                    info!("   Table creation disabled: Will fail if tables don't exist");
                }
                let sqlite_storage = SqliteSessionStorage::with_config(config)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to create SQLite storage: {}", e))?;

                let storage_arc = Arc::new(sqlite_storage);
                let boxed_storage: Arc<turul_mcp_session_storage::BoxedSessionStorage> =
                    storage_arc.clone();
                SESSION_STORAGE.set(boxed_storage).ok();

                McpServer::builder()
                    .name("client-initialise-server")
                    .version("1.0.0")
                    .title("MCP Initialize Test Server")
                    .bind_address(bind_address)
                    .with_session_storage(storage_arc)
                    .tool(EchoSseTool::default())
                    .tool(GetSessionDataTool::default())
                    .tool(GetSessionEventsTool::default())
                    .tool(GetTableInfoTool::default())
                    .build()?
            }
            #[cfg(not(feature = "sqlite"))]
            {
                return Err(anyhow::anyhow!(
                    "SQLite support not compiled in. Please rebuild with --features sqlite"
                ));
            }
        }
        "postgres" => {
            #[cfg(feature = "postgres")]
            {
                info!("   Using PostgreSQL session storage");
                let config = PostgresConfig::default();
                let postgres_storage = PostgresSessionStorage::with_config(config)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to create PostgreSQL storage: {}", e))?;

                let storage_arc = Arc::new(postgres_storage);
                let boxed_storage: Arc<turul_mcp_session_storage::BoxedSessionStorage> =
                    storage_arc.clone();
                SESSION_STORAGE.set(boxed_storage).ok();

                McpServer::builder()
                    .name("client-initialise-server")
                    .version("1.0.0")
                    .title("MCP Initialize Test Server")
                    .bind_address(bind_address)
                    .with_session_storage(storage_arc)
                    .tool(EchoSseTool::default())
                    .tool(GetSessionDataTool::default())
                    .tool(GetSessionEventsTool::default())
                    .tool(GetTableInfoTool::default())
                    .build()?
            }
            #[cfg(not(feature = "postgres"))]
            {
                return Err(anyhow::anyhow!(
                    "PostgreSQL support not compiled in. Please rebuild with --features postgres"
                ));
            }
        }
        "dynamodb" => {
            #[cfg(feature = "dynamodb")]
            {
                info!("   Using DynamoDB session storage");
                let config = DynamoDbConfig {
                    create_tables_if_missing: create_tables,
                    ..Default::default()
                };
                if create_tables {
                    info!("   Table creation enabled: Will create tables if missing");
                } else {
                    info!("   Table creation disabled: Will fail if tables don't exist");
                }
                let dynamodb_storage = DynamoDbSessionStorage::with_config(config)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to create DynamoDB storage: {}", e))?;

                let storage_arc = Arc::new(dynamodb_storage);
                let boxed_storage: Arc<turul_mcp_session_storage::BoxedSessionStorage> =
                    storage_arc.clone();
                SESSION_STORAGE.set(boxed_storage).ok();

                McpServer::builder()
                    .name("client-initialise-server")
                    .version("1.0.0")
                    .title("MCP Initialize Test Server")
                    .bind_address(bind_address)
                    .with_session_storage(storage_arc)
                    .tool(EchoSseTool::default())
                    .tool(GetSessionDataTool::default())
                    .tool(GetSessionEventsTool::default())
                    .tool(GetTableInfoTool::default())
                    .build()?
            }
            #[cfg(not(feature = "dynamodb"))]
            {
                return Err(anyhow::anyhow!(
                    "DynamoDB support not compiled in. Please rebuild with --features dynamodb"
                ));
            }
        }
        "inmemory" => {
            info!("   Using InMemory session storage");
            let inmemory_storage = InMemorySessionStorage::new();

            let storage_arc = Arc::new(inmemory_storage);
            let boxed_storage: Arc<turul_mcp_session_storage::BoxedSessionStorage> =
                storage_arc.clone();
            SESSION_STORAGE.set(boxed_storage).ok();

            McpServer::builder()
                .name("client-initialise-server")
                .version("1.0.0")
                .title("MCP Initialize Test Server")
                .bind_address(bind_address)
                .with_session_storage(storage_arc)
                .tool(EchoSseTool::default())
                .tool(GetSessionDataTool::default())
                .tool(GetSessionEventsTool::default())
                .tool(GetTableInfoTool::default())
                .build()?
        }
        _ => {
            return Err(anyhow::anyhow!(
                "Unknown storage backend: {}. Supported backends: inmemory, sqlite, postgres, dynamodb",
                storage_backend
            ));
        }
    };

    info!("Server configured with proper session management");
    info!("Ready to accept initialize requests");
    info!("");
    info!("Test with client:");
    info!(
        "   cargo run --package client-initialise-report -- --url http://127.0.0.1:{}/mcp",
        port
    );
    info!("");
    info!("Storage backends:");
    info!(
        "   InMemory (default): cargo run --package client-initialise-server -- --port {} --storage-backend inmemory",
        port
    );
    info!(
        "   SQLite (persistent): cargo run --package client-initialise-server -- --port {} --storage-backend sqlite",
        port
    );
    info!(
        "   PostgreSQL (enterprise): cargo run --features postgres --example client-initialise-server -- --port {} --storage-backend postgres",
        port
    );
    info!(
        "   DynamoDB (AWS cloud): cargo run --features dynamodb --example client-initialise-server -- --port {} --storage-backend dynamodb",
        port
    );
    info!("");
    info!("Additional flags:");
    info!(
        "   --create-tables: Enable table creation if tables don't exist (required for DynamoDB first run)"
    );
    info!("Manual curl test:");
    info!("   curl -X POST http://127.0.0.1:{}/mcp \\", port);
    info!("     -H \"Content-Type: application/json\" \\");
    info!(
        "     -d '{{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{{\"protocolVersion\":\"2025-11-25\",\"capabilities\":{{}},\"clientInfo\":{{\"name\":\"test\",\"version\":\"1.0\"}}}}}}' \\"
    );
    info!("     -i");
    info!("");

    // Start the server
    server.run().await?;

    Ok(())
}
