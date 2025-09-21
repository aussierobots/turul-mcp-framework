//! Session-Aware Logging Test for Regular MCP Server
//!
//! This creates a simple test setup:
//! 1. Server with a "test_log" tool that accepts a message and logging level
//! 2. Tool sends the message via session.notify_log() at the specified level
//! 3. Client can test whether it receives messages based on session logging level
//!
//! Usage:
//! ```bash
//! RUST_LOG=debug cargo run --package session-aware-logging-demo
//! ```

use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::{collections::HashMap, sync::Arc};
use tokio::time::{sleep, Duration};
use turul_mcp_protocol::{
    logging::LoggingLevel,
    schema::JsonSchema,
    tools::{CallToolResult, HasBaseMetadata, HasDescription, HasInputSchema, HasOutputSchema, HasAnnotations, HasToolMeta, ToolAnnotations, ToolSchema},
    ToolResult,
};
use turul_mcp_server::{McpResult, McpServer, McpTool, SessionContext};
use turul_mcp_session_storage::InMemorySessionStorage;

/// Simple logging test tool that sends a message at a specified logging level
#[derive(Clone)]
struct LoggingTestTool;

#[async_trait]
impl McpTool for LoggingTestTool {
    async fn call(&self, args: serde_json::Value, session: Option<SessionContext>) -> McpResult<CallToolResult> {
        let session = session.ok_or("Session context required")?;
        
        // Extract message and level parameters
        let message = args.get("message")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'message' parameter")?;
            
        let level_str = args.get("level")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'level' parameter")?;
        
        // Parse logging level
        let logging_level = match level_str.to_lowercase().as_str() {
            "debug" => LoggingLevel::Debug,
            "info" => LoggingLevel::Info,
            "notice" => LoggingLevel::Notice,
            "warning" => LoggingLevel::Warning,
            "error" => LoggingLevel::Error,
            "critical" => LoggingLevel::Critical,
            "alert" => LoggingLevel::Alert,
            "emergency" => LoggingLevel::Emergency,
            _ => return Ok(CallToolResult::success(vec![
                ToolResult::text(format!("Invalid level '{}'. Valid: debug, info, notice, warning, error, critical, alert, emergency", level_str))
            ])),
        };
        
        let current_session_level = session.get_logging_level().await;
        let will_be_filtered = !(session.should_log(logging_level).await);
        
        tracing::info!("🧪 LoggingTestTool called:");
        tracing::info!("   Message: '{}'", message);
        tracing::info!("   Level: {:?} (priority {})", logging_level, logging_level.priority());
        tracing::info!("   Session level: {:?} (priority {})", current_session_level, current_session_level.priority());
        tracing::info!("   Will be filtered: {}", will_be_filtered);
        
        // Send the log message via session notification (will be filtered automatically)
        session.notify_log(logging_level, serde_json::json!(message.to_string()), Some("demo".to_string()), None).await;
        
        // Return test result information
        Ok(CallToolResult::success(vec![
            ToolResult::text(json!({
                "test_message": message,
                "test_level": format!("{:?}", logging_level),
                "test_level_priority": logging_level.priority(),
                "session_level": format!("{:?}", current_session_level),
                "session_level_priority": current_session_level.priority(),
                "should_receive_message": !will_be_filtered,
                "note": if will_be_filtered { 
                    "Message was filtered - you should NOT see it in SSE stream" 
                } else { 
                    "Message was sent - you SHOULD see it in SSE stream" 
                }
            }).to_string())
        ]))
    }
}

// Manual trait implementations for LoggingTestTool
impl HasBaseMetadata for LoggingTestTool {
    fn name(&self) -> &str { "test_log" }
}

impl HasDescription for LoggingTestTool {
    fn description(&self) -> Option<&str> {
        Some("Send a test log message at the specified level to verify session-aware filtering")
    }
}

impl HasInputSchema for LoggingTestTool {
    fn input_schema(&self) -> &ToolSchema {
        static INPUT_SCHEMA: std::sync::OnceLock<ToolSchema> = std::sync::OnceLock::new();
        INPUT_SCHEMA.get_or_init(|| {
            ToolSchema::object()
                .with_properties(HashMap::from([
                    ("message".to_string(), JsonSchema::string().with_description("Test message to send")),
                    ("level".to_string(), JsonSchema::string().with_description("Logging level (debug, info, notice, warning, error, critical, alert, emergency)")),
                ]))
                .with_required(vec!["message".to_string(), "level".to_string()])
        })
    }
}

impl HasOutputSchema for LoggingTestTool {
    fn output_schema(&self) -> Option<&ToolSchema> { None }
}

impl HasAnnotations for LoggingTestTool {
    fn annotations(&self) -> Option<&ToolAnnotations> { None }
}

impl HasToolMeta for LoggingTestTool {
    fn tool_meta(&self) -> Option<&HashMap<String, serde_json::Value>> { None }
}

/// Tool to set the session's logging level
#[derive(Clone)]
struct SetLogLevelTool;

#[async_trait]
impl McpTool for SetLogLevelTool {
    async fn call(&self, args: serde_json::Value, session: Option<SessionContext>) -> McpResult<CallToolResult> {
        let session = session.ok_or("Session context required")?;
        
        let level_str = args.get("level")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'level' parameter")?;
            
        let new_level = match level_str.to_lowercase().as_str() {
            "debug" => LoggingLevel::Debug,
            "info" => LoggingLevel::Info,
            "notice" => LoggingLevel::Notice,
            "warning" => LoggingLevel::Warning,
            "error" => LoggingLevel::Error,
            "critical" => LoggingLevel::Critical,
            "alert" => LoggingLevel::Alert,
            "emergency" => LoggingLevel::Emergency,
            _ => return Ok(CallToolResult::success(vec![
                ToolResult::text(format!("Invalid level '{}'. Valid: debug, info, notice, warning, error, critical, alert, emergency", level_str))
            ])),
        };
        
        let old_level = session.get_logging_level().await;
        session.set_logging_level(new_level).await;
        
        tracing::info!("🎯 Session {} logging level changed: {:?} -> {:?}", 
            session.session_id, old_level, new_level);
        
        Ok(CallToolResult::success(vec![
            ToolResult::text(json!({
                "success": true,
                "old_level": format!("{:?}", old_level),
                "old_priority": old_level.priority(),
                "new_level": format!("{:?}", new_level), 
                "new_priority": new_level.priority()
            }).to_string())
        ]))
    }
}

// Manual trait implementations for SetLogLevelTool
impl HasBaseMetadata for SetLogLevelTool {
    fn name(&self) -> &str { "set_log_level" }
}

impl HasDescription for SetLogLevelTool {
    fn description(&self) -> Option<&str> {
        Some("Set the logging level for this session")
    }
}

impl HasInputSchema for SetLogLevelTool {
    fn input_schema(&self) -> &ToolSchema {
        static INPUT_SCHEMA: std::sync::OnceLock<ToolSchema> = std::sync::OnceLock::new();
        INPUT_SCHEMA.get_or_init(|| {
            ToolSchema::object()
                .with_properties(HashMap::from([
                    ("level".to_string(), JsonSchema::string().with_description("Logging level (debug, info, notice, warning, error, critical, alert, emergency)")),
                ]))
                .with_required(vec!["level".to_string()])
        })
    }
}

impl HasOutputSchema for SetLogLevelTool {
    fn output_schema(&self) -> Option<&ToolSchema> { None }
}

impl HasAnnotations for SetLogLevelTool {
    fn annotations(&self) -> Option<&ToolAnnotations> { None }
}

impl HasToolMeta for SetLogLevelTool {
    fn tool_meta(&self) -> Option<&HashMap<String, serde_json::Value>> { None }
}

/// Test client to verify session-aware logging
struct TestClient {
    client: reqwest::Client,
    base_url: String,
    session_id: Option<String>,
}

impl TestClient {
    fn new(port: u16) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: format!("http://127.0.0.1:{}/mcp", port),
            session_id: None,
        }
    }
    
    async fn initialize(&mut self) -> Result<()> {
        tracing::info!("🔌 Initializing client session...");
        
        let response = self.client
            .post(&self.base_url)
            .header("Content-Type", "application/json")
            .header("MCP-Protocol-Version", "2025-06-18")
            .json(&json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2025-06-18",
                    "capabilities": {},
                    "clientInfo": {"name": "test-client", "version": "1.0"}
                }
            }))
            .send()
            .await?;
            
        // Extract session ID from headers
        if let Some(session_header) = response.headers().get("Mcp-Session-Id") {
            self.session_id = Some(session_header.to_str()?.to_string());
            tracing::info!("✅ Session initialized: {}", self.session_id.as_ref().unwrap());
        } else {
            return Err(anyhow::anyhow!("No Mcp-Session-Id header received"));
        }
        
        Ok(())
    }
    
    async fn set_log_level(&self, level: &str) -> Result<serde_json::Value> {
        let session_id = self.session_id.as_ref().ok_or_else(|| anyhow::anyhow!("No session initialized"))?;
        
        tracing::info!("🎯 Setting session log level to: {}", level);
        
        let response = self.client
            .post(&self.base_url)
            .header("Content-Type", "application/json")
            .header("Mcp-Session-Id", session_id)
            .json(&json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "tools/call",
                "params": {
                    "name": "set_log_level",
                    "arguments": {"level": level}
                }
            }))
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;
            
        Ok(response)
    }
    
    async fn test_log_message(&self, message: &str, level: &str) -> Result<serde_json::Value> {
        let session_id = self.session_id.as_ref().ok_or_else(|| anyhow::anyhow!("No session initialized"))?;
        
        tracing::info!("🧪 Testing log message: '{}' at level '{}'", message, level);
        
        let response = self.client
            .post(&self.base_url)
            .header("Content-Type", "application/json")
            .header("Mcp-Session-Id", session_id)
            .json(&json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "tools/call",
                "params": {
                    "name": "test_log",
                    "arguments": {"message": message, "level": level}
                }
            }))
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;
            
        Ok(response)
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

    tracing::info!("🚀 Starting Session-Aware Logging Test");

    let port = 8000;
    let bind_address: std::net::SocketAddr = format!("127.0.0.1:{}", port).parse()?;

    // Create server with session-aware logging test tools
    let storage = Arc::new(InMemorySessionStorage::new());
    let server = McpServer::builder()
        .name("session-aware-logging-test")
        .version("1.0.0")
        .title("Session-Aware Logging Test Server")
        .bind_address(bind_address)
        .with_session_storage(storage)
        .tool(LoggingTestTool)
        .tool(SetLogLevelTool)
        .build()?;

    tracing::info!("📡 Server ready at http://{}/mcp", bind_address);
    tracing::info!("🔧 Available tools: test_log, set_log_level");

    // Start server in background
    let server_handle = tokio::spawn(async move {
        if let Err(e) = server.run().await {
            tracing::error!("❌ Server error: {}", e);
        }
    });

    // Wait for server to start
    sleep(Duration::from_millis(500)).await;

    // Run tests
    tracing::info!("🧪 Starting automated tests...");
    
    // Test 1: Initialize client and set to DEBUG level
    let mut client = TestClient::new(port);
    client.initialize().await?;
    
    client.set_log_level("debug").await?;
    tracing::info!("✅ Test 1: Set session to DEBUG level");
    
    // Test 2: Send messages at different levels - all should pass through
    tracing::info!("🧪 Test 2: Send messages at different levels (DEBUG session should receive all)");
    
    let test_cases = [
        ("Debug message test", "debug", true),
        ("Info message test", "info", true), 
        ("Warning message test", "warning", true),
        ("Error message test", "error", true),
    ];
    
    for (message, level, should_receive) in &test_cases {
        tracing::debug!("Testing message '{}' at level '{}', should_receive: {}", message, level, should_receive);
        let response = client.test_log_message(message, level).await?;
        tracing::info!("📋 Response: {}", serde_json::to_string_pretty(&response)?);
        sleep(Duration::from_millis(100)).await;
    }
    
    // Test 3: Change to WARNING level and test filtering
    tracing::info!("🧪 Test 3: Change to WARNING level and test filtering");
    client.set_log_level("warning").await?;
    
    let filtered_test_cases = [
        ("Debug should be filtered", "debug", false),
        ("Info should be filtered", "info", false),
        ("Warning should pass", "warning", true),
        ("Error should pass", "error", true),
    ];
    
    for (message, level, should_receive) in &filtered_test_cases {
        tracing::debug!("Testing filtered message '{}' at level '{}', should_receive: {}", message, level, should_receive);
        let response = client.test_log_message(message, level).await?;
        tracing::info!("📋 Response: {}", serde_json::to_string_pretty(&response)?);
        sleep(Duration::from_millis(100)).await;
    }
    
    tracing::info!("✅ Session-aware logging test completed!");
    tracing::info!("💡 To monitor SSE messages, run in another terminal:");
    tracing::info!("   curl -N -H \"Accept: text/event-stream\" -H \"Mcp-Session-Id: {}\" http://127.0.0.1:{}/mcp", 
        client.session_id.unwrap_or_else(|| "SESSION_ID".to_string()), port);

    // Keep server running for manual testing
    tracing::info!("🔄 Server will keep running for manual testing. Press Ctrl+C to stop.");
    let _ = server_handle.await?;

    Ok(())
}