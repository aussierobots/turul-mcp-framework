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
//! # Start server on default port (8000)
//! cargo run --example client-initialise-server
//! ```
//!
//! ## Test with Client
//! ```bash
//! # In another terminal:
//! cargo run --example client-initialise-report -- --url http://127.0.0.1:8000/mcp
//! ```

use anyhow::Result;
use serde_json::json;
use tracing::{info, debug};
use std::sync::Arc;
use chrono;

use turul_mcp_server::{McpServer, McpTool, SessionContext, McpResult};
use turul_mcp_protocol::tools::CallToolResult;
use turul_mcp_session_storage::InMemorySessionStorage;
#[cfg(feature = "sqlite")]
use turul_mcp_session_storage::{SqliteSessionStorage, SqliteConfig};
#[cfg(feature = "postgres")]
use turul_mcp_session_storage::{PostgresSessionStorage, PostgresConfig};
#[cfg(feature = "dynamodb")]
use turul_mcp_session_storage::{DynamoDbSessionStorage, DynamoDbConfig};
use async_trait::async_trait;

/// Echo SSE tool that demonstrates MCP notifications via SessionContext
#[derive(Clone)]
struct EchoSseTool;

#[async_trait]
impl McpTool for EchoSseTool {
    async fn call(&self, args: serde_json::Value, session: Option<SessionContext>) -> McpResult<CallToolResult> {
        // Extract text parameter
        let text = args.get("text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| turul_mcp_protocol::McpError::invalid_param_type("text", "string", "missing"))?;

        // Log the call on the server side
        info!("🔊 echo_sse called with text: '{}'", text);

        // Send a progress notification if we have a session
        if let Some(session_context) = &session {
            info!("📡 Sending progress notification via SessionContext");
            session_context.notify_progress("echo_processing", 50);
            
            // Also send a log message notification
            session_context.notify_log("info", format!("Processing echo for text: '{}'", text));
        } else {
            info!("⚠️  No session context available for notifications");
        }

        // Create response text
        let response_text = format!("Echo: {}", text);
        info!("📤 Echo response created: {}", response_text);

        // Send completion notification
        if let Some(session_context) = &session {
            session_context.notify_progress("echo_processing", 100);
            session_context.notify_log("info", format!("Echo completed successfully: '{}'", response_text));
        }

        Ok(CallToolResult::success(vec![
            turul_mcp_protocol::ToolResult::text(json!({"result": response_text}).to_string())
        ]))
    }
}

// Implement the required traits for the tool
impl turul_mcp_protocol::tools::HasBaseMetadata for EchoSseTool {
    fn name(&self) -> &str {
        "echo_sse"
    }
}

impl turul_mcp_protocol::tools::HasDescription for EchoSseTool {
    fn description(&self) -> Option<&str> {
        Some("Echoes text back via POST response and streams it via SSE. Server logs all calls.")
    }
}

impl turul_mcp_protocol::tools::HasInputSchema for EchoSseTool {
    fn input_schema(&self) -> &turul_mcp_protocol::tools::ToolSchema {
        static INPUT_SCHEMA: std::sync::OnceLock<turul_mcp_protocol::tools::ToolSchema> = std::sync::OnceLock::new();
        INPUT_SCHEMA.get_or_init(|| {
            use turul_mcp_protocol::schema::JsonSchema;
            use std::collections::HashMap;
            
            turul_mcp_protocol::tools::ToolSchema::object()
                .with_properties(HashMap::from([
                    ("text".to_string(), JsonSchema::string().with_description("Text to echo back")),
                ]))
                .with_required(vec!["text".to_string()])
        })
    }
}

impl turul_mcp_protocol::tools::HasOutputSchema for EchoSseTool {
    fn output_schema(&self) -> Option<&turul_mcp_protocol::tools::ToolSchema> {
        None
    }
}

impl turul_mcp_protocol::tools::HasAnnotations for EchoSseTool {
    fn annotations(&self) -> Option<&turul_mcp_protocol::tools::ToolAnnotations> {
        None
    }
}

impl turul_mcp_protocol::tools::HasToolMeta for EchoSseTool {
    fn tool_meta(&self) -> Option<&std::collections::HashMap<String, serde_json::Value>> {
        None
    }
}

fn create_echo_sse_tool() -> Result<EchoSseTool> {
    Ok(EchoSseTool)
}

/// Session inspection tool that returns current session data
#[derive(Clone)]
struct GetSessionDataTool {
    session_storage: Arc<turul_mcp_session_storage::BoxedSessionStorage>,
}

#[async_trait]
impl McpTool for GetSessionDataTool {
    async fn call(&self, _args: serde_json::Value, session: Option<SessionContext>) -> McpResult<CallToolResult> {
        info!("🔍 get_session_data called");
        
        if let Some(session_ctx) = &session {
            // Get session info from storage
            match self.session_storage.get_session(&session_ctx.session_id).await {
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
                    
                    info!("📋 Retrieved session data for: {}", session_ctx.session_id);
                    Ok(CallToolResult::success(vec![
                        turul_mcp_protocol::ToolResult::text(session_data.to_string())
                    ]))
                },
                Ok(None) => {
                    let error_msg = format!("Session {} not found in storage", session_ctx.session_id);
                    info!("⚠️ {}", error_msg);
                    Ok(CallToolResult::success(vec![
                        turul_mcp_protocol::ToolResult::text(json!({"error": error_msg}).to_string())
                    ]))
                },
                Err(e) => {
                    let error_msg = format!("Failed to retrieve session data: {}", e);
                    info!("❌ {}", error_msg);
                    Ok(CallToolResult::success(vec![
                        turul_mcp_protocol::ToolResult::text(json!({"error": error_msg}).to_string())
                    ]))
                }
            }
        } else {
            let error_msg = "No session context available";
            info!("⚠️ {}", error_msg);
            Ok(CallToolResult::success(vec![
                turul_mcp_protocol::ToolResult::text(json!({"error": error_msg}).to_string())
            ]))
        }
    }
}

// Implement required traits for GetSessionDataTool
impl turul_mcp_protocol::tools::HasBaseMetadata for GetSessionDataTool {
    fn name(&self) -> &str {
        "get_session_data"
    }
}

impl turul_mcp_protocol::tools::HasDescription for GetSessionDataTool {
    fn description(&self) -> Option<&str> {
        Some("Returns current session information from session storage")
    }
}

impl turul_mcp_protocol::tools::HasInputSchema for GetSessionDataTool {
    fn input_schema(&self) -> &turul_mcp_protocol::tools::ToolSchema {
        static INPUT_SCHEMA: std::sync::OnceLock<turul_mcp_protocol::tools::ToolSchema> = std::sync::OnceLock::new();
        INPUT_SCHEMA.get_or_init(|| {
            use std::collections::HashMap;
            turul_mcp_protocol::tools::ToolSchema::object()
                .with_properties(HashMap::new()) // No parameters needed
        })
    }
}

impl turul_mcp_protocol::tools::HasOutputSchema for GetSessionDataTool {
    fn output_schema(&self) -> Option<&turul_mcp_protocol::tools::ToolSchema> {
        None
    }
}

impl turul_mcp_protocol::tools::HasAnnotations for GetSessionDataTool {
    fn annotations(&self) -> Option<&turul_mcp_protocol::tools::ToolAnnotations> {
        None
    }
}

impl turul_mcp_protocol::tools::HasToolMeta for GetSessionDataTool {
    fn tool_meta(&self) -> Option<&std::collections::HashMap<String, serde_json::Value>> {
        None
    }
}

/// Session events inspection tool that returns events for the current session
#[derive(Clone)]
struct GetSessionEventsTool {
    session_storage: Arc<turul_mcp_session_storage::BoxedSessionStorage>,
}

#[async_trait]
impl McpTool for GetSessionEventsTool {
    async fn call(&self, args: serde_json::Value, session: Option<SessionContext>) -> McpResult<CallToolResult> {
        info!("📡 get_session_events called");
        
        let limit = args.get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(10) as usize;
            
        if let Some(session_ctx) = &session {
            match self.session_storage.get_recent_events(&session_ctx.session_id, limit).await {
                Ok(events) => {
                    let events_data: Vec<serde_json::Value> = events.into_iter().map(|event| {
                        json!({
                            "id": event.id,
                            "event_type": event.event_type,
                            "data": event.data,
                            "timestamp": event.timestamp,
                            "retry": event.retry
                        })
                    }).collect();
                    
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
                    
                    info!("📊 Retrieved {} events for session: {}", events_data.len(), session_ctx.session_id);
                    Ok(CallToolResult::success(vec![
                        turul_mcp_protocol::ToolResult::text(response.to_string())
                    ]))
                },
                Err(e) => {
                    let error_msg = format!("Failed to retrieve session events: {}", e);
                    info!("❌ {}", error_msg);
                    Ok(CallToolResult::success(vec![
                        turul_mcp_protocol::ToolResult::text(json!({"error": error_msg}).to_string())
                    ]))
                }
            }
        } else {
            let error_msg = "No session context available";
            info!("⚠️ {}", error_msg);
            Ok(CallToolResult::success(vec![
                turul_mcp_protocol::ToolResult::text(json!({"error": error_msg}).to_string())
            ]))
        }
    }
}

// Implement required traits for GetSessionEventsTool
impl turul_mcp_protocol::tools::HasBaseMetadata for GetSessionEventsTool {
    fn name(&self) -> &str {
        "get_session_events"
    }
}

impl turul_mcp_protocol::tools::HasDescription for GetSessionEventsTool {
    fn description(&self) -> Option<&str> {
        Some("Returns SSE events for the current session from session storage")
    }
}

impl turul_mcp_protocol::tools::HasInputSchema for GetSessionEventsTool {
    fn input_schema(&self) -> &turul_mcp_protocol::tools::ToolSchema {
        static INPUT_SCHEMA: std::sync::OnceLock<turul_mcp_protocol::tools::ToolSchema> = std::sync::OnceLock::new();
        INPUT_SCHEMA.get_or_init(|| {
            use turul_mcp_protocol::schema::JsonSchema;
            use std::collections::HashMap;
            
            turul_mcp_protocol::tools::ToolSchema::object()
                .with_properties(HashMap::from([
                    ("limit".to_string(), JsonSchema::integer().with_description("Maximum number of events to return (default: 10)")),
                ]))
        })
    }
}

impl turul_mcp_protocol::tools::HasOutputSchema for GetSessionEventsTool {
    fn output_schema(&self) -> Option<&turul_mcp_protocol::tools::ToolSchema> {
        None
    }
}

impl turul_mcp_protocol::tools::HasAnnotations for GetSessionEventsTool {
    fn annotations(&self) -> Option<&turul_mcp_protocol::tools::ToolAnnotations> {
        None
    }
}

impl turul_mcp_protocol::tools::HasToolMeta for GetSessionEventsTool {
    fn tool_meta(&self) -> Option<&std::collections::HashMap<String, serde_json::Value>> {
        None
    }
}

/// Session table information inspection tool
#[derive(Clone)]
struct GetTableInfoTool {
    session_storage: Arc<turul_mcp_session_storage::BoxedSessionStorage>,
}

// Implement McpTool for GetTableInfoTool
#[async_trait::async_trait]
impl McpTool for GetTableInfoTool {
    async fn call(&self, _args: serde_json::Value, _session: Option<SessionContext>) -> McpResult<CallToolResult> {
        info!("📊 get_table_info called");
        debug!("Using session storage for table info: {:p}", &self.session_storage);
        
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
        
        info!("📋 Retrieved table information for backend: {}", storage_type);
        Ok(CallToolResult::success(vec![
            turul_mcp_protocol::ToolResult::text(table_info.to_string())
        ]))
    }
}

// Implement required traits for GetTableInfoTool
impl turul_mcp_protocol::tools::HasBaseMetadata for GetTableInfoTool {
    fn name(&self) -> &str {
        "get_table_info"
    }
}

impl turul_mcp_protocol::tools::HasDescription for GetTableInfoTool {
    fn description(&self) -> Option<&str> {
        Some("Returns information about session and events tables in the storage backend")
    }
}

impl turul_mcp_protocol::tools::HasInputSchema for GetTableInfoTool {
    fn input_schema(&self) -> &turul_mcp_protocol::tools::ToolSchema {
        static INPUT_SCHEMA: std::sync::OnceLock<turul_mcp_protocol::tools::ToolSchema> = std::sync::OnceLock::new();
        INPUT_SCHEMA.get_or_init(|| {
            use std::collections::HashMap;
            turul_mcp_protocol::tools::ToolSchema::object()
                .with_properties(HashMap::new()) // No parameters needed
        })
    }
}

impl turul_mcp_protocol::tools::HasOutputSchema for GetTableInfoTool {
    fn output_schema(&self) -> Option<&turul_mcp_protocol::tools::ToolSchema> {
        None
    }
}

impl turul_mcp_protocol::tools::HasAnnotations for GetTableInfoTool {
    fn annotations(&self) -> Option<&turul_mcp_protocol::tools::ToolAnnotations> {
        None
    }
}

impl turul_mcp_protocol::tools::HasToolMeta for GetTableInfoTool {
    fn tool_meta(&self) -> Option<&std::collections::HashMap<String, serde_json::Value>> {
        None
    }
}

fn create_session_inspection_tools(storage: Arc<turul_mcp_session_storage::BoxedSessionStorage>) -> Result<(GetSessionDataTool, GetSessionEventsTool, GetTableInfoTool)> {
    let session_data_tool = GetSessionDataTool { 
        session_storage: Arc::clone(&storage) 
    };
    let session_events_tool = GetSessionEventsTool { 
        session_storage: Arc::clone(&storage) 
    };
    let table_info_tool = GetTableInfoTool {
        session_storage: Arc::clone(&storage)
    };
    Ok((session_data_tool, session_events_tool, table_info_tool))
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    info!("🚀 Starting MCP Initialize Test Server");
    info!("   • Server creates and manages session IDs");
    info!("   • Session IDs returned via Mcp-Session-Id header");

    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();
    let mut port = 8000;
    let mut storage_backend = "inmemory".to_string(); // Default to InMemory storage
    let mut create_tables = false; // Default to not creating tables
    
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--port" => {
                if i + 1 < args.len() {
                    port = args[i + 1].parse().unwrap_or(8000);
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
    info!("   • Binding to: http://{}/mcp", bind_address);

    // Build server using builder pattern with appropriate storage backend
    let server = match storage_backend.as_str() {
        "sqlite" => {
            #[cfg(feature = "sqlite")]
            {
                let temp_dir = std::env::temp_dir();
                info!("   • System temp directory: {}", temp_dir.display());
                
                // Create a subdirectory for MCP sessions
                let mcp_temp_dir = temp_dir.join("mcp-sessions");
                std::fs::create_dir_all(&mcp_temp_dir)
                    .map_err(|e| anyhow::anyhow!("Failed to create MCP temp directory: {}", e))?;
                
                let db_path = mcp_temp_dir.join("mcp_sessions.db");
                info!("   • Using SQLite session storage (database: {})", db_path.display());
                
                let mut config = SqliteConfig::default();
                config.database_path = db_path;
                config.create_tables_if_missing = create_tables;
                if create_tables {
                    info!("   • Table creation enabled: Will create tables if missing");
                } else {
                    info!("   • Table creation disabled: Will fail if tables don't exist");
                }
                let sqlite_storage = SqliteSessionStorage::with_config(config).await
                    .map_err(|e| anyhow::anyhow!("Failed to create SQLite storage: {}", e))?;
                
                let storage_arc = Arc::new(sqlite_storage);
                let boxed_storage: Arc<turul_mcp_session_storage::BoxedSessionStorage> = storage_arc.clone();
                let (session_data_tool, session_events_tool, table_info_tool) = create_session_inspection_tools(boxed_storage)?;
                
                McpServer::builder()
                    .name("client-initialise-server")
                    .version("1.0.0")
                    .title("MCP Initialize Test Server")
                    .bind_address(bind_address)
                    .with_session_storage(storage_arc)
                    .tool(create_echo_sse_tool()?)
                    .tool(session_data_tool)
                    .tool(session_events_tool)
                    .tool(table_info_tool)
                    .build()?
            }
            #[cfg(not(feature = "sqlite"))]
            {
                return Err(anyhow::anyhow!("SQLite support not compiled in. Please rebuild with --features sqlite"));
            }
        }
        "postgres" => {
            #[cfg(feature = "postgres")]
            {
                info!("   • Using PostgreSQL session storage");
                let config = PostgresConfig::default();
                let postgres_storage = PostgresSessionStorage::with_config(config).await
                    .map_err(|e| anyhow::anyhow!("Failed to create PostgreSQL storage: {}", e))?;
                
                let storage_arc = Arc::new(postgres_storage);
                let boxed_storage: Arc<turul_mcp_session_storage::BoxedSessionStorage> = storage_arc.clone();
                let (session_data_tool, session_events_tool, table_info_tool) = create_session_inspection_tools(boxed_storage)?;
                
                McpServer::builder()
                    .name("client-initialise-server")
                    .version("1.0.0")
                    .title("MCP Initialize Test Server")
                    .bind_address(bind_address)
                    .with_session_storage(storage_arc)
                    .tool(create_echo_sse_tool()?)
                    .tool(session_data_tool)
                    .tool(session_events_tool)
                    .tool(table_info_tool)
                    .build()?
            }
            #[cfg(not(feature = "postgres"))]
            {
                return Err(anyhow::anyhow!("PostgreSQL support not compiled in. Please rebuild with --features postgres"));
            }
        }
        "dynamodb" => {
            #[cfg(feature = "dynamodb")]
            {
                info!("   • Using DynamoDB session storage");
                let mut config = DynamoDbConfig::default();
                config.create_tables_if_missing = create_tables;
                if create_tables {
                    info!("   • Table creation enabled: Will create tables if missing");
                } else {
                    info!("   • Table creation disabled: Will fail if tables don't exist");
                }
                let dynamodb_storage = DynamoDbSessionStorage::with_config(config).await
                    .map_err(|e| anyhow::anyhow!("Failed to create DynamoDB storage: {}", e))?;
                
                let storage_arc = Arc::new(dynamodb_storage);
                let boxed_storage: Arc<turul_mcp_session_storage::BoxedSessionStorage> = storage_arc.clone();
                let (session_data_tool, session_events_tool, table_info_tool) = create_session_inspection_tools(boxed_storage)?;
                
                McpServer::builder()
                    .name("client-initialise-server")
                    .version("1.0.0")
                    .title("MCP Initialize Test Server")
                    .bind_address(bind_address)
                    .with_session_storage(storage_arc)
                    .tool(create_echo_sse_tool()?)
                    .tool(session_data_tool)
                    .tool(session_events_tool)
                    .tool(table_info_tool)
                    .build()?
            }
            #[cfg(not(feature = "dynamodb"))]
            {
                return Err(anyhow::anyhow!("DynamoDB support not compiled in. Please rebuild with --features dynamodb"));
            }
        }
        "inmemory" => {
            info!("   • Using InMemory session storage");
            let inmemory_storage = InMemorySessionStorage::new();
            
            let storage_arc = Arc::new(inmemory_storage);
            let boxed_storage: Arc<turul_mcp_session_storage::BoxedSessionStorage> = storage_arc.clone();
            let (session_data_tool, session_events_tool, table_info_tool) = create_session_inspection_tools(boxed_storage)?;
            
            McpServer::builder()
                .name("client-initialise-server")
                .version("1.0.0")
                .title("MCP Initialize Test Server")
                .bind_address(bind_address)
                .with_session_storage(storage_arc)
                .tool(create_echo_sse_tool()?)
                .tool(session_data_tool)
                .tool(session_events_tool)
                .tool(table_info_tool)
                .build()?
        }
        _ => {
            return Err(anyhow::anyhow!("Unknown storage backend: {}. Supported backends: inmemory, sqlite, postgres, dynamodb", storage_backend));
        }
    };

    info!("✅ Server configured with proper session management");
    info!("📡 Ready to accept initialize requests");
    info!("");
    info!("🧪 Test with client:");
    info!(
        "   cargo run --example client-initialise-report -- --url http://127.0.0.1:{}/mcp",
        port
    );
    info!("");
    info!("🗄️  Storage backends:");
    info!("   • InMemory (default): cargo run --example client-initialise-server -- --port {} --storage-backend inmemory", port);
    info!("   • SQLite (persistent): cargo run --example client-initialise-server -- --port {} --storage-backend sqlite", port);
    info!("   • PostgreSQL (enterprise): cargo run --features postgres --example client-initialise-server -- --port {} --storage-backend postgres", port);
    info!("   • DynamoDB (AWS cloud): cargo run --features dynamodb --example client-initialise-server -- --port {} --storage-backend dynamodb", port);
    info!("");
    info!("🔧 Additional flags:");
    info!("   • --create-tables: Enable table creation if tables don't exist (required for DynamoDB first run)");
    info!("📋 Manual curl test:");
    info!("   curl -X POST http://127.0.0.1:{}/mcp \\", port);
    info!("     -H \"Content-Type: application/json\" \\");
    info!(
        "     -d '{{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{{\"protocolVersion\":\"2025-06-18\",\"capabilities\":{{}},\"clientInfo\":{{\"name\":\"test\",\"version\":\"1.0\"}}}}}}' \\"
    );
    info!("     -i");
    info!("");

    // Start the server
    server.run().await?;

    Ok(())
}
