//! Logging Test Server
//!
//! Simple MCP server with tools to test session-aware logging filtering.
//!
//! Usage:
//! ```bash
//! RUST_LOG=info cargo run --package turul-mcp-server --example logging-test-server
//! ```

use anyhow::Result;
use std::sync::Arc;
use turul_mcp_protocol::logging::LoggingLevel;
use turul_mcp_server::{McpResult, McpServer, SessionContext};
use turul_mcp_session_storage::InMemorySessionStorage;
use turul_mcp_derive::McpTool;

/// Tool that sends a log message at the specified level using derive macro
#[derive(McpTool, Clone, serde::Serialize, serde::Deserialize)]
#[tool(
    name = "send_log", 
    description = "Send a log message at the specified level (will be filtered by session logging level)"
)]
struct SendLogTool {
    #[param(description = "The log message to send")]
    message: String,
    
    #[param(description = "Logging level: debug, info, notice, warning, error, critical, alert, emergency")]
    level: String,
    
    #[param(description = "Correlation ID for tracking this request", optional)]
    correlation_id: Option<String>,
}

impl SendLogTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<String> {
        let session = session.ok_or("Session context required")?;
        
        // Validate the input level parameter first (all MCP logging levels)
        let _input_level = match self.level.as_str() {
            "Debug" | "debug" => LoggingLevel::Debug,
            "Info" | "info" => LoggingLevel::Info,
            "Notice" | "notice" => LoggingLevel::Notice,
            "Warning" | "warning" => LoggingLevel::Warning,
            "Error" | "error" => LoggingLevel::Error,
            "Critical" | "critical" => LoggingLevel::Critical,
            "Alert" | "alert" => LoggingLevel::Alert,
            "Emergency" | "emergency" => LoggingLevel::Emergency,
            _ => {
                return Err(format!("Invalid logging level '{}'. Valid levels: Debug, Info, Notice, Warning, Error, Critical, Alert, Emergency", self.level).into());
            }
        };

        // Send ONE notification at the requested level
        let request_level = _input_level; // Use the level from the request
        let level_str = match request_level {
            LoggingLevel::Debug => "DEBUG",
            LoggingLevel::Info => "INFO", 
            LoggingLevel::Notice => "NOTICE",
            LoggingLevel::Warning => "WARNING",
            LoggingLevel::Error => "ERROR",
            LoggingLevel::Critical => "CRITICAL",
            LoggingLevel::Alert => "ALERT", 
            LoggingLevel::Emergency => "EMERGENCY"
        };
        
        tracing::info!("📤 SendLogTool: Generating 1 MCP logging notification at {} level for session {}", 
                      level_str, session.session_id);
        tracing::info!("   Session current level: {:?}", session.get_logging_level());
        
        // Create meta map with correlation_id for tracking (if provided)
        let meta_map = if let Some(ref correlation_id) = self.correlation_id {
            let mut map = std::collections::HashMap::new();
            map.insert("correlation_id".to_string(), serde_json::json!(correlation_id));
            Some(map)
        } else {
            None
        };
        
        let message = format!("{}: {}", level_str, self.message);
        
        if let Some(ref correlation_id) = self.correlation_id {
            tracing::info!("🔧 Sending {} level notification with correlation_id={}", level_str, correlation_id);
        } else {
            tracing::info!("🔧 Sending {} level notification (no correlation_id)", level_str);
        }
        
        session.notify_log(
            request_level,
            serde_json::json!(message),
            Some("test-server".to_string()),
            meta_map.clone()
        );
        
        let notifications_sent = 1;
        
        if let Some(ref correlation_id) = self.correlation_id {
            tracing::info!("✅ SendLogTool sent {} notifications, session filtering will determine delivery [correlation_id: {}]", 
                          notifications_sent, correlation_id);
        } else {
            tracing::info!("✅ SendLogTool sent {} notifications, session filtering will determine delivery", 
                          notifications_sent);
        }
        
        let correlation_info = if let Some(ref correlation_id) = self.correlation_id {
            format!(" [correlation_id: {}]", correlation_id)
        } else {
            String::new()
        };
        
        Ok(format!("Sent {} log notifications at {:?} level for session {}{}", 
                  notifications_sent, 
                  session.get_logging_level(), 
                  session.session_id,
                  correlation_info))
    }
}

/// Tool to change the session's logging level using derive macro
#[derive(McpTool, Clone, Default)]
#[tool(
    name = "set_log_level",
    description = "Set the logging level for this session (debug, info, notice, warning, error, critical, alert, emergency)"
)]
struct SetLogLevelTool {
    #[param(description = "Logging level: debug, info, notice, warning, error, critical, alert, emergency")]
    level: String,
}

impl SetLogLevelTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<String> {
        let session = session.ok_or("Session context required")?;
            
        let new_level = match self.level.to_lowercase().as_str() {
            "debug" => LoggingLevel::Debug,
            "info" => LoggingLevel::Info,
            "notice" => LoggingLevel::Notice,
            "warning" => LoggingLevel::Warning,
            "error" => LoggingLevel::Error,
            "critical" => LoggingLevel::Critical,
            "alert" => LoggingLevel::Alert,
            "emergency" => LoggingLevel::Emergency,
            _ => return Err(format!("Invalid level '{}'. Valid: debug, info, notice, warning, error, critical, alert, emergency", self.level).into()),
        };
        
        let old_level = session.get_logging_level();
        session.set_logging_level(new_level);
        
        tracing::info!("🎯 Session {} logging level changed: {:?} -> {:?}", 
            session.session_id, old_level, new_level);
        
        // Send notification about level change
        session.notify_log(
            LoggingLevel::Info,
            serde_json::json!(format!("Logging level changed to: {:?}", new_level)),
            Some("system".to_string()),
            None
        );
        
        Ok(format!("Successfully changed logging level from {:?} to {:?}", old_level, new_level))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(false)
        .without_time()
        .init();

    println!("🚀 Starting Logging Test Server");
    
    let port = 8003;
    let bind_address: std::net::SocketAddr = format!("127.0.0.1:{}", port).parse()?;

    // Create server with logging test tools
    let storage = Arc::new(InMemorySessionStorage::new());
    let server = McpServer::builder()
        .name("logging-test-server")
        .version("1.0.0")
        .title("Session-Aware Logging Test Server")
        .bind_address(bind_address)
        .with_session_storage(storage)
        .with_logging() // Enable logging capability
        .tool(SendLogTool {
            message: String::new(),
            level: String::new(), 
            correlation_id: None,
        })
        .tool(SetLogLevelTool::default())
        .build()?;

    println!("📡 Server listening at http://{}/mcp", bind_address);
    println!("🔧 Available tools:");
    println!("   • send_log(message, level) - Sends log message at specified level");
    println!("   • set_log_level(level) - Changes session logging level");
    println!("");
    println!("💡 Use the client to test session-aware logging filtering!");
    
    // Start server
    server.run().await?;
    
    Ok(())
}