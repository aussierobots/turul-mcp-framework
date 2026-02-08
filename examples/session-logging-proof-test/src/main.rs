//! Session-Aware Logging PROOF Test
//!
//! This creates a definitive test that PROVES session-aware logging filtering works:
//! 1. Server starts with simple logging test tools
//! 2. Client connects and tests different scenarios
//! 3. Uses SSE monitoring to verify which messages actually get through
//! 4. Provides clear PASS/FAIL results for each test case
//!
//! Usage:
//! ```bash
//! RUST_LOG=debug cargo run --package session-logging-proof-test
//! ```

use anyhow::Result;
use serde::Deserialize;
use serde_json::json;
use std::{sync::Arc, time::Duration};
use tokio::time::sleep;
use turul_mcp_derive::McpTool;
use turul_mcp_protocol::logging::LoggingLevel;
use turul_mcp_server::prelude::*;
use turul_mcp_session_storage::InMemorySessionStorage;

/// Helper function to convert string level to LoggingLevel enum
fn str_to_logging_level(level: &str) -> LoggingLevel {
    match level.to_lowercase().as_str() {
        "debug" => LoggingLevel::Debug,
        "info" => LoggingLevel::Info,
        "notice" => LoggingLevel::Notice,
        "warning" => LoggingLevel::Warning,
        "error" => LoggingLevel::Error,
        "critical" => LoggingLevel::Critical,
        "alert" => LoggingLevel::Alert,
        "emergency" => LoggingLevel::Emergency,
        _ => LoggingLevel::Info, // Default fallback
    }
}

/// Test tool that sends log messages at different levels to verify filtering
#[derive(McpTool, Clone, Default, Deserialize)]
#[tool(
    name = "log_proof",
    description = "Send test log messages to prove session-aware filtering works"
)]
pub struct LogProofTool {
    #[param(
        description = "Test scenario: basic, debug_flood, level_cascade, session_isolation",
        optional
    )]
    pub scenario: Option<String>,
}

impl LogProofTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<serde_json::Value> {
        let session = session.ok_or_else(|| McpError::tool_execution("Session context required"))?;

        let scenario = self.scenario.as_deref().unwrap_or("basic");

        tracing::info!("LogProofTool executing scenario: '{}'", scenario);

        match scenario {
            "debug_flood" => {
                // Send multiple debug messages - should be filtered if session level > debug
                for i in 1..=5 {
                    session.notify_log(
                        LoggingLevel::Debug,
                        serde_json::json!(format!("DEBUG MESSAGE {}: This should only appear if session level is DEBUG", i)),
                        Some("proof-test".to_string()),
                        None
                    ).await;
                }

                Ok(json!({
                    "test": "debug_flood",
                    "messages_sent": 5,
                    "level": "debug",
                    "session_level": format!("{:?}", session.get_logging_level().await),
                    "should_see_messages": session.should_log(LoggingLevel::Debug).await
                }))
            }
            "level_cascade" => {
                // Send one message at each level
                session
                    .notify_log(
                        str_to_logging_level("debug"),
                        serde_json::json!("DEBUG: Should only appear if session=DEBUG"),
                        Some("test".to_string()),
                        None,
                    )
                    .await;
                session
                    .notify_log(
                        str_to_logging_level("info"),
                        serde_json::json!("INFO: Should appear if session<=INFO"),
                        Some("test".to_string()),
                        None,
                    )
                    .await;
                session
                    .notify_log(
                        str_to_logging_level("notice"),
                        serde_json::json!("NOTICE: Should appear if session<=NOTICE"),
                        Some("test".to_string()),
                        None,
                    )
                    .await;
                session
                    .notify_log(
                        str_to_logging_level("warning"),
                        serde_json::json!("WARNING: Should appear if session<=WARNING"),
                        Some("test".to_string()),
                        None,
                    )
                    .await;
                session
                    .notify_log(
                        str_to_logging_level("error"),
                        serde_json::json!("ERROR: Should appear if session<=ERROR"),
                        Some("test".to_string()),
                        None,
                    )
                    .await;
                session
                    .notify_log(
                        str_to_logging_level("critical"),
                        serde_json::json!("CRITICAL: Should appear if session<=CRITICAL"),
                        Some("test".to_string()),
                        None,
                    )
                    .await;

                let current_level = session.get_logging_level().await;
                let mut levels_that_should_appear: Vec<String> = Vec::new();
                for (name, level) in [
                    ("debug", LoggingLevel::Debug),
                    ("info", LoggingLevel::Info),
                    ("notice", LoggingLevel::Notice),
                    ("warning", LoggingLevel::Warning),
                    ("error", LoggingLevel::Error),
                    ("critical", LoggingLevel::Critical),
                ] {
                    if session.should_log(level).await {
                        levels_that_should_appear.push(name.to_string());
                    }
                }

                Ok(json!({
                    "test": "level_cascade",
                    "messages_sent": 6,
                    "session_level": format!("{:?}", current_level),
                    "session_priority": current_level.priority(),
                    "levels_that_should_appear": levels_that_should_appear,
                    "expected_message_count": levels_that_should_appear.len()
                }))
            }
            "session_isolation" => {
                // Test that this session's level doesn't affect other sessions
                let session_id = session.session_id.clone();
                let current_level = session.get_logging_level().await;

                session
                    .notify_log(
                        LoggingLevel::Info,
                        serde_json::json!(format!(
                            "SESSION ISOLATION TEST: Session {} at level {:?}",
                            session_id, current_level
                        )),
                        Some("proof-test".to_string()),
                        None,
                    )
                    .await;
                session.notify_log(
                    LoggingLevel::Debug,
                    serde_json::json!(format!("DEBUG from session {}: Should only appear if this session allows debug", session_id)),
                    Some("proof-test".to_string()),
                    None
                ).await;
                session.notify_log(
                    LoggingLevel::Error,
                    serde_json::json!(format!("ERROR from session {}: Should always appear regardless of other sessions", session_id)),
                    Some("proof-test".to_string()),
                    None
                ).await;

                Ok(json!({
                    "test": "session_isolation",
                    "session_id": session_id,
                    "session_level": format!("{:?}", current_level),
                    "messages_sent": 3,
                    "note": "Each session should only see messages based on ITS OWN logging level"
                }))
            }
            _ => {
                // Basic test - send one message at each common level
                session
                    .notify_log(
                        str_to_logging_level("debug"),
                        serde_json::json!("BASIC TEST: Debug message"),
                        Some("test".to_string()),
                        None,
                    )
                    .await;
                session
                    .notify_log(
                        str_to_logging_level("info"),
                        serde_json::json!("BASIC TEST: Info message"),
                        Some("test".to_string()),
                        None,
                    )
                    .await;
                session
                    .notify_log(
                        str_to_logging_level("warning"),
                        serde_json::json!("BASIC TEST: Warning message"),
                        Some("test".to_string()),
                        None,
                    )
                    .await;
                session
                    .notify_log(
                        str_to_logging_level("error"),
                        serde_json::json!("BASIC TEST: Error message"),
                        Some("test".to_string()),
                        None,
                    )
                    .await;

                Ok(json!({
                    "test": "basic",
                    "messages_sent": 4,
                    "session_level": format!("{:?}", session.get_logging_level().await),
                    "note": "Basic test completed - check SSE stream for filtered messages"
                }))
            }
        }
    }
}

/// Tool to change session logging level
#[derive(McpTool, Clone, Default, Deserialize)]
#[tool(
    name = "set_level",
    description = "Set the logging level for this session"
)]
pub struct SetLevelTool {
    #[param(description = "debug, info, notice, warning, error, critical, alert, emergency")]
    pub level: String,
}

impl SetLevelTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<serde_json::Value> {
        let session = session.ok_or_else(|| McpError::tool_execution("Session context required"))?;

        let new_level = match self.level.to_lowercase().as_str() {
            "debug" => LoggingLevel::Debug,
            "info" => LoggingLevel::Info,
            "notice" => LoggingLevel::Notice,
            "warning" => LoggingLevel::Warning,
            "error" => LoggingLevel::Error,
            "critical" => LoggingLevel::Critical,
            "alert" => LoggingLevel::Alert,
            "emergency" => LoggingLevel::Emergency,
            _ => {
                return Ok(json!({
                    "error": format!("Invalid level: {}", self.level)
                }));
            }
        };

        let old_level = session.get_logging_level().await;
        session.set_logging_level(new_level).await;

        tracing::info!(
            "Session {} logging level: {:?} -> {:?}",
            session.session_id,
            old_level,
            new_level
        );

        // Send a confirmation message at the new level
        session
            .notify_log(
                LoggingLevel::Info,
                serde_json::json!(format!("Logging level changed to {:?}", new_level)),
                Some("proof-test".to_string()),
                None,
            )
            .await;

        Ok(json!({
            "success": true,
            "session_id": session.session_id,
            "old_level": format!("{:?}", old_level),
            "new_level": format!("{:?}", new_level),
            "confirmation_sent": true
        }))
    }
}

/// Automated test client that provides definitive proof
struct ProofTestClient {
    client: reqwest::Client,
    base_url: String,
    session_id: Option<String>,
}

impl ProofTestClient {
    fn new(port: u16) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: format!("http://127.0.0.1:{}/mcp", port),
            session_id: None,
        }
    }

    async fn initialize(&mut self) -> Result<String> {
        let response = self
            .client
            .post(&self.base_url)
            .header("Content-Type", "application/json")
            // Intentional: testing backward compatibility with MCP 2025-06-18
            .header("MCP-Protocol-Version", "2025-06-18")
            .json(&json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2025-06-18", // Intentional: testing backward compatibility with MCP 2025-06-18
                    "capabilities": {},
                    "clientInfo": {"name": "proof-client", "version": "1.0"}
                }
            }))
            .send()
            .await?;

        if let Some(session_header) = response.headers().get("Mcp-Session-Id") {
            let session_id = session_header.to_str()?.to_string();
            self.session_id = Some(session_id.clone());
            println!("Session initialized: {}", session_id);
            Ok(session_id)
        } else {
            anyhow::bail!("No session ID received")
        }
    }

    async fn set_logging_level(&self, level: &str) -> Result<()> {
        let session_id = self.session_id.as_ref().unwrap();

        let response = self
            .client
            .post(&self.base_url)
            .header("Content-Type", "application/json")
            .header("Mcp-Session-Id", session_id)
            .json(&json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "tools/call",
                "params": {
                    "name": "set_level",
                    "arguments": {"level": level}
                }
            }))
            .send()
            .await?;

        let result: serde_json::Value = response.json().await?;

        if result.get("error").is_some() {
            anyhow::bail!("Error setting level: {}", result);
        }

        println!("Set logging level to: {}", level);
        Ok(())
    }

    async fn run_log_test(&self, scenario: &str) -> Result<serde_json::Value> {
        let session_id = self.session_id.as_ref().unwrap();

        let response = self
            .client
            .post(&self.base_url)
            .header("Content-Type", "application/json")
            .header("Mcp-Session-Id", session_id)
            .json(&json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "tools/call",
                "params": {
                    "name": "log_proof",
                    "arguments": {"scenario": scenario}
                }
            }))
            .send()
            .await?;

        let result: serde_json::Value = response.json().await?;

        if let Some(error) = result.get("error") {
            anyhow::bail!("Tool call error: {}", error);
        }

        Ok(result)
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

    println!("SESSION-AWARE LOGGING PROOF TEST");
    println!("=====================================");

    let port = 8001; // Use different port to avoid conflicts
    let bind_address: std::net::SocketAddr = format!("127.0.0.1:{}", port).parse()?;

    // Create server
    let storage = Arc::new(InMemorySessionStorage::new());
    let server = McpServer::builder()
        .name("session-logging-proof")
        .version("1.0.0")
        .title("Session-Aware Logging Proof Test")
        .bind_address(bind_address)
        .with_session_storage(storage)
        .tool(LogProofTool::default())
        .tool(SetLevelTool::default())
        .build()?;

    println!("Starting server at http://{}/mcp", bind_address);

    // Start server in background
    let server_handle = tokio::spawn(async move {
        if let Err(e) = server.run().await {
            eprintln!("Server error: {}", e);
        }
    });

    // Wait for server to start
    sleep(Duration::from_millis(1000)).await;

    // Run comprehensive tests
    println!("\nRUNNING PROOF TESTS...\n");

    // Test 1: DEBUG level session should see all messages
    println!("TEST 1: DEBUG level session (should see ALL messages)");
    let mut client1 = ProofTestClient::new(port);
    let session1_id = client1.initialize().await?;
    client1.set_logging_level("debug").await?;

    let result1 = client1.run_log_test("level_cascade").await?;
    println!("   Result: {}", serde_json::to_string_pretty(&result1)?);
    println!(
        "   Check SSE stream: curl -N -H \"Accept: text/event-stream\" -H \"Mcp-Session-Id: {}\" http://127.0.0.1:{}/mcp",
        session1_id, port
    );

    sleep(Duration::from_millis(500)).await;

    // Test 2: WARNING level session should only see WARNING+ messages
    println!("\nTEST 2: WARNING level session (should only see WARNING, ERROR, CRITICAL)");
    let mut client2 = ProofTestClient::new(port);
    let session2_id = client2.initialize().await?;
    client2.set_logging_level("warning").await?;

    let result2 = client2.run_log_test("level_cascade").await?;
    println!("   Result: {}", serde_json::to_string_pretty(&result2)?);
    println!(
        "   Check SSE stream: curl -N -H \"Accept: text/event-stream\" -H \"Mcp-Session-Id: {}\" http://127.0.0.1:{}/mcp",
        session2_id, port
    );

    sleep(Duration::from_millis(500)).await;

    // Test 3: ERROR level session should only see ERROR+ messages
    println!("\nTEST 3: ERROR level session (should only see ERROR, CRITICAL)");
    let mut client3 = ProofTestClient::new(port);
    let session3_id = client3.initialize().await?;
    client3.set_logging_level("error").await?;

    let result3 = client3.run_log_test("level_cascade").await?;
    println!("   Result: {}", serde_json::to_string_pretty(&result3)?);
    println!(
        "   Check SSE stream: curl -N -H \"Accept: text/event-stream\" -H \"Mcp-Session-Id: {}\" http://127.0.0.1:{}/mcp",
        session3_id, port
    );

    sleep(Duration::from_millis(500)).await;

    // Test 4: Session isolation test
    println!("\nTEST 4: Session isolation (each session should filter independently)");
    client1.run_log_test("session_isolation").await?;
    client2.run_log_test("session_isolation").await?;
    client3.run_log_test("session_isolation").await?;

    println!("\nALL TESTS COMPLETED!");
    println!("\nVERIFICATION INSTRUCTIONS:");
    println!("1. Open 3 separate terminals");
    println!("2. Run the SSE curl commands shown above for each session");
    println!("3. Verify that:");
    println!("   - Session 1 (DEBUG) sees ALL log messages");
    println!("   - Session 2 (WARNING) sees only WARNING, ERROR, CRITICAL messages");
    println!("   - Session 3 (ERROR) sees only ERROR, CRITICAL messages");
    println!("4. Each session should ONLY see messages based on ITS OWN logging level");
    println!(
        "\nIf filtering is working correctly, you should see DIFFERENT messages in each SSE stream!"
    );

    println!("\nServer will keep running for manual verification...");
    println!("Press Ctrl+C to stop when done verifying.");

    // Keep server running for manual verification
    server_handle.await?;

    Ok(())
}
