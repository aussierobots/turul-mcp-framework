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
use tracing::info;
use std::sync::Arc;

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
                let sqlite_storage = SqliteSessionStorage::with_config(config).await
                    .map_err(|e| anyhow::anyhow!("Failed to create SQLite storage: {}", e))?;
                
                McpServer::builder()
                    .name("client-initialise-server")
                    .version("1.0.0")
                    .title("MCP Initialize Test Server")
                    .bind_address(bind_address)
                    .with_session_storage(Arc::new(sqlite_storage))
                    .tool(create_echo_sse_tool()?)
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
                
                McpServer::builder()
                    .name("client-initialise-server")
                    .version("1.0.0")
                    .title("MCP Initialize Test Server")
                    .bind_address(bind_address)
                    .with_session_storage(Arc::new(postgres_storage))
                    .tool(create_echo_sse_tool()?)
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
                let config = DynamoDbConfig::default();
                let dynamodb_storage = DynamoDbSessionStorage::with_config(config).await
                    .map_err(|e| anyhow::anyhow!("Failed to create DynamoDB storage: {}", e))?;
                
                McpServer::builder()
                    .name("client-initialise-server")
                    .version("1.0.0")
                    .title("MCP Initialize Test Server")
                    .bind_address(bind_address)
                    .with_session_storage(Arc::new(dynamodb_storage))
                    .tool(create_echo_sse_tool()?)
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
            
            McpServer::builder()
                .name("client-initialise-server")
                .version("1.0.0")
                .title("MCP Initialize Test Server")
                .bind_address(bind_address)
                .with_session_storage(Arc::new(inmemory_storage))
                .tool(create_echo_sse_tool()?)
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
