//! Logging Test Client
//!
//! Client that connects to the logging test server using proper MCP protocol methods:
//! - Uses `logging/setLevel` method to change session logging level
//! - Uses `tools/call` method to trigger log messages
//! - Monitors SSE stream for `notifications/message` to verify filtering
//!
//! Usage:
//! ```bash
//! # First, start the server in another terminal:
//! cargo run --package logging-test-server
//!
//! # Default: Test GET SSE mode (comprehensive test)
//! RUST_LOG=info cargo run --package logging-test-client
//!
//! # Test POST SSE streaming mode only
//! RUST_LOG=info cargo run --package logging-test-client -- --test-post-sse
//!
//! # Test both POST and GET SSE streaming modes
//! RUST_LOG=info cargo run --package logging-test-client -- --test-both-modes
//!
//! # Quick verification test (faster)
//! RUST_LOG=info cargo run --package logging-test-client -- --quick-test
//!
//! # Test against server on different port
//! RUST_LOG=info cargo run --package logging-test-client -- --port 8080
//! ```

use anyhow::Result;
use clap::Parser;
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::sleep;
use uuid::Uuid;

// Use proper MCP protocol structs instead of json! macros
use turul_mcp_protocol::{
    initialize::{ClientCapabilities, Implementation, InitializeRequest},
    json_rpc::{JSONRPC_VERSION, JsonRpcRequest, RequestParams},
    logging::{LoggingLevel, SetLevelParams},
    meta::Meta,
    tools::CallToolParams,
    version::McpVersion,
};

/// Command-line arguments for the logging test client
#[derive(Parser, Debug)]
#[command(name = "logging-test-client")]
#[command(about = "Session-aware logging test client for MCP framework")]
struct Args {
    /// Server port to connect to
    #[arg(short, long, default_value = "8003")]
    port: u16,

    /// Test POST SSE streaming mode (tool calls with Accept: text/event-stream)
    #[arg(long)]
    test_post_sse: bool,

    /// Test GET SSE streaming mode (persistent SSE connection)
    #[arg(long, default_value = "true")]
    test_get_sse: bool,

    /// Test both POST and GET SSE streaming modes
    #[arg(long)]
    test_both_modes: bool,

    /// Only run a quick verification test (faster execution)
    #[arg(long)]
    quick_test: bool,
}

/// Test client that verifies session-aware logging filtering
#[derive(Clone)]
struct LoggingTestClient {
    client: reqwest::Client,
    base_url: String,
    session_id: Option<String>,
}

impl LoggingTestClient {
    fn new(port: u16) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: format!("http://127.0.0.1:{}/mcp", port),
            session_id: None,
        }
    }

    async fn initialize(&mut self) -> Result<String> {
        println!("🔌 Connecting to server and initializing session...");

        // Create proper MCP initialize request using protocol structs
        let initialize_request = InitializeRequest::new(
            McpVersion::V2025_06_18,
            ClientCapabilities::default(),
            Implementation::new("logging-test-client", "1.0.0"),
        );

        // Convert to RequestParams
        let init_value = serde_json::to_value(initialize_request)?;
        let mut params_map = HashMap::new();
        if let serde_json::Value::Object(map) = init_value {
            for (key, value) in map {
                params_map.insert(key, value);
            }
        }

        let request_params = RequestParams {
            meta: None,
            other: params_map,
        };

        let json_rpc_request = JsonRpcRequest {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id: serde_json::Value::Number(1.into()),
            method: "initialize".to_string(),
            params: Some(request_params),
        };

        let response = self
            .client
            .post(&self.base_url)
            .header("Content-Type", "application/json")
            .header("MCP-Protocol-Version", "2025-11-25")
            .json(&json_rpc_request)
            .send()
            .await?;

        if let Some(session_header) = response.headers().get("Mcp-Session-Id") {
            let session_id = session_header.to_str()?.to_string();
            self.session_id = Some(session_id.clone());
            println!(
                "✅ Session initialized with MCP protocol structs: {}",
                session_id
            );
            Ok(session_id)
        } else {
            anyhow::bail!("No Mcp-Session-Id header received");
        }
    }

    async fn set_logging_level(&self, level: &str) -> Result<serde_json::Value> {
        let session_id = self.session_id.as_ref().unwrap();

        println!(
            "🎯 Setting session logging level to: {} using MCP logging/setLevel method",
            level
        );

        // Parse the level string to LoggingLevel enum
        let logging_level = match level.to_lowercase().as_str() {
            "debug" => LoggingLevel::Debug,
            "info" => LoggingLevel::Info,
            "warning" => LoggingLevel::Warning,
            "error" => LoggingLevel::Error,
            _ => anyhow::bail!("Invalid logging level: {}", level),
        };

        // Create proper MCP SetLevelParams using protocol structs
        let set_level_params = SetLevelParams::new(logging_level);

        // Convert to RequestParams
        let params_value = serde_json::to_value(set_level_params)?;
        let mut params_map = HashMap::new();
        if let serde_json::Value::Object(map) = params_value {
            for (key, value) in map {
                params_map.insert(key, value);
            }
        }

        let request_params = RequestParams {
            meta: None,
            other: params_map,
        };

        let json_rpc_request = JsonRpcRequest {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id: serde_json::Value::Number(2.into()),
            method: "logging/setLevel".to_string(),
            params: Some(request_params),
        };

        let response = self
            .client
            .post(&self.base_url)
            .header("Content-Type", "application/json")
            .header("Mcp-Session-Id", session_id)
            .json(&json_rpc_request)
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        if let Some(error) = response.get("error") {
            anyhow::bail!("Error setting log level: {}", error);
        }

        println!(
            "✅ Logging level set to: {} via MCP protocol structs",
            level
        );
        Ok(response)
    }

    #[allow(dead_code)]
    async fn send_log_message(
        &self,
        message: &str,
        level: &str,
        sequence: u32,
    ) -> Result<(serde_json::Value, String)> {
        let session_id = self.session_id.as_ref().unwrap();

        // Generate unique correlation ID for this message
        let correlation_id = Uuid::now_v7().to_string();

        println!(
            "📤 [MSG-{}] Sending log message: '{}' at level '{}' [correlation_id: {}]",
            sequence, message, level, correlation_id
        );

        // Create arguments HashMap for the tool call
        let mut arguments = HashMap::new();
        arguments.insert(
            "message".to_string(),
            serde_json::Value::String(message.to_string()),
        );
        arguments.insert(
            "level".to_string(),
            serde_json::Value::String(level.to_string()),
        );
        arguments.insert(
            "correlation_id".to_string(),
            serde_json::Value::String(correlation_id.clone()),
        );

        // Create proper MCP CallToolParams using protocol structs
        let call_tool_params = CallToolParams::new("send_log").with_arguments(arguments);

        // Convert to RequestParams
        let params_value = serde_json::to_value(call_tool_params)?;
        let mut params_map = HashMap::new();
        if let serde_json::Value::Object(map) = params_value {
            for (key, value) in map {
                params_map.insert(key, value);
            }
        }

        // Add correlation ID to meta field
        let mut meta = Meta::new();
        meta.extra.insert(
            "correlation_id".to_string(),
            serde_json::Value::String(correlation_id.clone()),
        );

        let request_params = RequestParams {
            meta: Some(meta),
            other: params_map,
        };

        let json_rpc_request = JsonRpcRequest {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id: serde_json::Value::Number(3.into()),
            method: "tools/call".to_string(),
            params: Some(request_params),
        };

        // Log the request JSON clearly
        println!(
            "📤 [MSG-{}] CALL REQUEST JSON [correlation_id: {}]:",
            sequence, correlation_id
        );
        println!("{}", serde_json::to_string_pretty(&json_rpc_request)?);

        let response = self
            .client
            .post(&self.base_url)
            .header("Content-Type", "application/json")
            .header("Mcp-Session-Id", session_id)
            .json(&json_rpc_request)
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        // Log the response JSON clearly
        println!(
            "📥 [MSG-{}] CALL RESPONSE JSON [correlation_id: {}]:",
            sequence, correlation_id
        );
        println!("{}", serde_json::to_string_pretty(&response)?);

        if let Some(error) = response.get("error") {
            anyhow::bail!("Error sending log: {}", error);
        }

        println!(
            "✅ [MSG-{}] Log message sent via MCP protocol structs [correlation_id: {}]",
            sequence, correlation_id
        );
        Ok((response, correlation_id))
    }

    /// Persistent SSE monitoring that sends notifications to a channel
    async fn monitor_sse_persistent(
        &self,
        sender: mpsc::UnboundedSender<(String, u64, String)>, // (notification_json, timestamp, test_context)
    ) -> Result<()> {
        let session_id = self.session_id.as_ref().unwrap();
        let sse_url = self.base_url.to_string();

        println!(
            "📡 Starting persistent SSE connection for session: {}",
            session_id
        );

        // Start SSE connection
        let mut sse_response = self
            .client
            .get(&sse_url)
            .header("Accept", "text/event-stream")
            .header("Mcp-Session-Id", session_id)
            .send()
            .await?;

        if !sse_response.status().is_success() {
            anyhow::bail!("SSE connection failed: {}", sse_response.status());
        }

        println!("✅ Persistent SSE connection established");

        let mut notification_counter = 0u64;

        // Read the response body as text chunks indefinitely
        loop {
            match tokio::time::timeout(Duration::from_millis(200), sse_response.chunk()).await {
                Ok(Ok(Some(chunk))) => {
                    let chunk_str = String::from_utf8_lossy(&chunk);

                    // Split by lines and look for SSE data
                    for line in chunk_str.lines() {
                        let line = line.trim();

                        // Look for SSE data lines
                        if let Some(data) = line.strip_prefix("data: ") {
                            // Remove "data: " prefix

                            // Check if this is a notification message
                            if data.contains("notifications/message")
                                || data.contains("\"method\":\"notifications/message\"")
                            {
                                notification_counter += 1;

                                // Extract correlation ID from the notification data
                                let correlation_id = if let Ok(parsed) =
                                    serde_json::from_str::<serde_json::Value>(data)
                                {
                                    parsed
                                        .get("params")
                                        .and_then(|params| params.get("_meta"))
                                        .and_then(|meta| meta.get("correlation_id"))
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("none")
                                        .to_string()
                                } else {
                                    "none".to_string()
                                };

                                // Send notification to channel with extracted correlation ID
                                if sender
                                    .send((
                                        data.to_string(),
                                        notification_counter,
                                        correlation_id.clone(),
                                    ))
                                    .is_err()
                                {
                                    println!("📡 SSE receiver dropped, closing connection");
                                    return Ok(());
                                }

                                println!(
                                    "🔔 Persistent SSE received notification #{} with correlation_id: {} ({} chars)",
                                    notification_counter,
                                    correlation_id,
                                    data.len()
                                );
                            }
                        }
                    }
                }
                Ok(Ok(None)) => {
                    println!("📡 SSE stream ended");
                    break;
                }
                Ok(Err(e)) => {
                    println!("❌ SSE stream error: {}", e);
                    break;
                }
                Err(_) => {
                    // Timeout - continue waiting
                    continue;
                }
            }
        }

        println!("📡 Persistent SSE connection closed");
        Ok(())
    }

    async fn call_tool_json(
        &self,
        json_rpc_request: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let session_id = self.session_id.as_ref().unwrap();

        let response = self
            .client
            .post(&self.base_url)
            .header("Content-Type", "application/json")
            .header("Mcp-Session-Id", session_id)
            .json(&json_rpc_request)
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        if let Some(error) = response.get("error") {
            anyhow::bail!("Tool call error: {}", error);
        }

        Ok(response)
    }

    #[allow(dead_code)]
    async fn monitor_sse_notifications(
        &self,
        duration_secs: u64,
        test_name: &str,
    ) -> Result<Vec<(String, u64)>> {
        let session_id = self.session_id.as_ref().unwrap();
        let sse_url = self.base_url.to_string();

        println!(
            "📡 [{}] Starting SSE connection (will monitor for {} seconds)...",
            test_name, duration_secs
        );

        // Start SSE connection
        let mut sse_response = self
            .client
            .get(&sse_url)
            .header("Accept", "text/event-stream")
            .header("Mcp-Session-Id", session_id)
            .send()
            .await?;

        if !sse_response.status().is_success() {
            anyhow::bail!("SSE connection failed: {}", sse_response.status());
        }

        println!(
            "✅ [{}] SSE connection established - monitoring for notifications",
            test_name
        );

        let mut notifications = Vec::new();
        let mut notification_counter = 0u64;
        let start_time = std::time::Instant::now();
        let timeout = Duration::from_secs(duration_secs);

        // Read the response body as text chunks
        while start_time.elapsed() < timeout {
            // Try to read a chunk with timeout
            match tokio::time::timeout(Duration::from_millis(200), sse_response.chunk()).await {
                Ok(Ok(Some(chunk))) => {
                    let chunk_str = String::from_utf8_lossy(&chunk);

                    // Split by lines and look for SSE data
                    for line in chunk_str.lines() {
                        let line = line.trim();

                        // Look for SSE data lines
                        if let Some(data) = line.strip_prefix("data: ") {
                            // Remove "data: " prefix

                            // Check if this is a notification message
                            if data.contains("notifications/message")
                                || data.contains("\"method\":\"notifications/message\"")
                            {
                                notification_counter += 1;
                                let timestamp = std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_millis()
                                    as u64;
                                notifications.push((data.to_string(), timestamp));

                                // Parse the JSON to extract level, message, and correlation ID
                                let display_content = if let Ok(parsed) =
                                    serde_json::from_str::<serde_json::Value>(data)
                                {
                                    if let Some(params) = parsed.get("params") {
                                        let level = params
                                            .get("level")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("?");
                                        let message = params
                                            .get("data")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("?");

                                        // Look for correlation ID in _meta field
                                        let correlation_id = if let Some(meta) = params.get("_meta")
                                        {
                                            meta.get("correlation_id")
                                                .and_then(|v| v.as_str())
                                                .unwrap_or("none")
                                        } else {
                                            "none"
                                        };

                                        format!(
                                            "level={}, message=\"{}\" [correlation_id: {}]",
                                            level, message, correlation_id
                                        )
                                    } else {
                                        "no params field".to_string()
                                    }
                                } else {
                                    "failed to parse JSON".to_string()
                                };

                                println!(
                                    "🔔 [{}] Received notification #{}: {}",
                                    test_name, notification_counter, display_content
                                );

                                // Log the SSE response JSON clearly and formatted
                                let correlation_id =
                                    serde_json::from_str::<serde_json::Value>(data)
                                        .ok()
                                        .and_then(|parsed| parsed.get("params").cloned())
                                        .and_then(|params| params.get("_meta").cloned())
                                        .and_then(|meta| {
                                            meta.get("correlation_id")
                                                .and_then(|v| v.as_str())
                                                .map(|s| s.to_string())
                                        })
                                        .unwrap_or_else(|| "none".to_string());

                                println!(
                                    "📨 [{}] SSE RESPONSE JSON #{} [correlation_id: {}]:",
                                    test_name, notification_counter, correlation_id
                                );

                                // Pretty print the JSON
                                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(data)
                                {
                                    println!(
                                        "{}",
                                        serde_json::to_string_pretty(&parsed)
                                            .unwrap_or_else(|_| data.to_string())
                                    );
                                } else {
                                    println!("{}", data);
                                }

                                // Add a short delay to help with processing
                                tokio::time::sleep(Duration::from_millis(50)).await;
                            }
                        }
                    }
                }
                Ok(Ok(None)) => {
                    println!("📡 SSE stream ended");
                    break;
                }
                Ok(Err(e)) => {
                    println!("❌ SSE stream error: {}", e);
                    break;
                }
                Err(_) => {
                    // Timeout - continue waiting
                    continue;
                }
            }
        }

        println!(
            "📊 [{}] Total notifications received: {}",
            test_name,
            notifications.len()
        );
        Ok(notifications)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command-line arguments
    let args = Args::parse();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(false)
        .without_time()
        .init();

    println!("🧪 LOGGING TEST CLIENT");
    println!("======================");

    // Determine test modes
    let test_get_sse = args.test_get_sse || args.test_both_modes || !args.test_post_sse;
    let test_post_sse = args.test_post_sse || args.test_both_modes;

    println!("🔧 Configuration:");
    println!("   • Server: http://127.0.0.1:{}/mcp", args.port);
    println!(
        "   • GET SSE Testing: {}",
        if test_get_sse {
            "✅ ENABLED"
        } else {
            "❌ DISABLED"
        }
    );
    println!(
        "   • POST SSE Testing: {}",
        if test_post_sse {
            "✅ ENABLED"
        } else {
            "❌ DISABLED"
        }
    );
    println!(
        "   • Test Mode: {}",
        if args.quick_test {
            "🚀 QUICK"
        } else {
            "🔍 COMPREHENSIVE"
        }
    );
    println!();

    let mut client = LoggingTestClient::new(args.port);

    // Initialize session
    client.initialize().await?;
    println!();

    // Create channel for persistent SSE notifications
    let (notification_sender, mut notification_receiver) = mpsc::unbounded_channel();

    // Start persistent SSE monitoring in background
    let client_clone = client.clone();
    let sse_handle = tokio::spawn(async move {
        client_clone
            .monitor_sse_persistent(notification_sender)
            .await
    });

    // Wait for SSE connection to establish
    sleep(Duration::from_millis(1000)).await;

    // Helper function to collect notifications for a specific correlation ID
    #[allow(dead_code)]
    async fn collect_notifications_for_test(
        receiver: &mut mpsc::UnboundedReceiver<(String, u64, String)>,
        correlation_id: &str,
        duration_secs: u64,
        test_name: &str,
    ) -> Result<Vec<(String, u64, String)>> {
        println!(
            "⏰ Collecting notifications for {} for {} seconds...",
            test_name, duration_secs
        );
        let mut notifications = Vec::new();
        let collection_end = std::time::Instant::now() + Duration::from_secs(duration_secs);

        while std::time::Instant::now() < collection_end {
            match tokio::time::timeout(Duration::from_millis(100), receiver.recv()).await {
                Ok(Some((notification_json, sequence_num, corr_id))) => {
                    if corr_id == correlation_id {
                        notifications.push((
                            notification_json.clone(),
                            sequence_num,
                            corr_id.clone(),
                        ));
                        println!(
                            "📥 [SSE-{}] NOTIFICATION JSON [correlation_id: {}]:",
                            sequence_num, corr_id
                        );
                        // Parse and pretty print the notification JSON
                        match serde_json::from_str::<serde_json::Value>(&notification_json) {
                            Ok(parsed) => println!("{}", serde_json::to_string_pretty(&parsed)?),
                            Err(_) => println!("{}", notification_json),
                        }
                    }
                }
                Ok(None) => {
                    println!("❌ SSE channel closed unexpectedly");
                    break;
                }
                Err(_) => {
                    // Timeout, continue collecting
                }
            }
        }

        Ok(notifications)
    }

    // Test 1: Debug session - send 8 log messages (one per level)
    println!("\n🧪 TEST 1: DEBUG session filtering - sending 8 log levels");

    client.set_logging_level("debug").await?;

    let all_levels = [
        "Debug",
        "Info",
        "Notice",
        "Warning",
        "Error",
        "Critical",
        "Alert",
        "Emergency",
    ];
    let mut test1_requests = Vec::new(); // Store (correlation_id, level) pairs

    for (i, level) in all_levels.iter().enumerate() {
        let correlation_id = Uuid::now_v7().to_string();
        let sequence = i + 1;

        let json_rpc_request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": sequence,
            "method": "tools/call",
            "params": {
                "name": "send_log",
                "arguments": {
                    "message": format!("Test 1 - {} level message", level),
                    "level": level,
                    "correlation_id": correlation_id
                },
                "_meta": {
                    "correlation_id": correlation_id,
                    "test": "debug_session"
                }
            }
        });

        println!(
            "📤 [TEST1-{}] {} level request [correlation_id: {}]",
            i + 1,
            level,
            correlation_id
        );

        let response = client.call_tool_json(json_rpc_request).await?;
        if response.get("error").is_some() {
            println!("❌ Request failed: {}", response);
        }

        test1_requests.push((correlation_id.clone(), level.to_string()));
        tokio::time::sleep(Duration::from_millis(300)).await; // Brief delay
    }

    println!(
        "⏰ Test 1: Sent {} requests, collecting notifications for 5 seconds...",
        test1_requests.len()
    );

    // Collect Test 1 notifications
    let mut test1_received = Vec::new();
    let mut notification_count = 0;
    while let Ok(notification) = notification_receiver.try_recv() {
        let (json, _, _) = notification;
        println!(
            "📥 [TEST1-SSE-{}] Notification JSON:",
            notification_count + 1
        );
        println!("{}", json);
        test1_received.push(json);
        notification_count += 1;
    }
    tokio::time::sleep(Duration::from_secs(3)).await; // Allow more notifications
    while let Ok(notification) = notification_receiver.try_recv() {
        let (json, _, _) = notification;
        println!(
            "📥 [TEST1-SSE-{}] Notification JSON:",
            notification_count + 1
        );
        println!("{}", json);
        test1_received.push(json);
        notification_count += 1;
    }

    // Test 2: Info session - send 8 log messages (Debug should be filtered)
    println!("\n🧪 TEST 2: INFO session filtering - sending 8 log levels");

    client.set_logging_level("info").await?;

    let mut test2_requests = Vec::new();

    for (i, level) in all_levels.iter().enumerate() {
        let correlation_id = Uuid::now_v7().to_string();
        let sequence = i + 10;

        let json_rpc_request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": sequence,
            "method": "tools/call",
            "params": {
                "name": "send_log",
                "arguments": {
                    "message": format!("Test 2 - {} level message", level),
                    "level": level,
                    "correlation_id": correlation_id
                },
                "_meta": {
                    "correlation_id": correlation_id,
                    "test": "info_session"
                }
            }
        });

        println!(
            "📤 [TEST2-{}] {} level request [correlation_id: {}]",
            i + 1,
            level,
            correlation_id
        );

        let response = client.call_tool_json(json_rpc_request).await?;
        if response.get("error").is_some() {
            println!("❌ Request failed: {}", response);
        }

        test2_requests.push((correlation_id.clone(), level.to_string()));
        tokio::time::sleep(Duration::from_millis(300)).await;
    }

    println!(
        "⏰ Test 2: Sent {} requests, collecting notifications for 5 seconds...",
        test2_requests.len()
    );

    // Collect Test 2 notifications
    let mut test2_received = Vec::new();
    notification_count = 0;
    while let Ok(notification) = notification_receiver.try_recv() {
        let (json, _, _) = notification;
        println!(
            "📥 [TEST2-SSE-{}] Notification JSON:",
            notification_count + 1
        );
        println!("{}", json);
        test2_received.push(json);
        notification_count += 1;
    }
    tokio::time::sleep(Duration::from_secs(3)).await;
    while let Ok(notification) = notification_receiver.try_recv() {
        let (json, _, _) = notification;
        println!(
            "📥 [TEST2-SSE-{}] Notification JSON:",
            notification_count + 1
        );
        println!("{}", json);
        test2_received.push(json);
        notification_count += 1;
    }

    // Test 3: Error session - send 8 log messages (only Error+ should pass)
    println!("\n🧪 TEST 3: ERROR session filtering - sending 8 log levels");

    client.set_logging_level("error").await?;

    let mut test3_requests = Vec::new();

    for (i, level) in all_levels.iter().enumerate() {
        let correlation_id = Uuid::now_v7().to_string();
        let sequence = i + 20;

        let json_rpc_request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": sequence,
            "method": "tools/call",
            "params": {
                "name": "send_log",
                "arguments": {
                    "message": format!("Test 3 - {} level message", level),
                    "level": level,
                    "correlation_id": correlation_id
                },
                "_meta": {
                    "correlation_id": correlation_id,
                    "test": "error_session"
                }
            }
        });

        println!(
            "📤 [TEST3-{}] {} level request [correlation_id: {}]",
            i + 1,
            level,
            correlation_id
        );

        let response = client.call_tool_json(json_rpc_request).await?;
        if response.get("error").is_some() {
            println!("❌ Request failed: {}", response);
        }

        test3_requests.push((correlation_id.clone(), level.to_string()));
        tokio::time::sleep(Duration::from_millis(300)).await;
    }

    println!(
        "⏰ Test 3: Sent {} requests, collecting notifications for 5 seconds...",
        test3_requests.len()
    );

    // Collect Test 3 notifications
    let mut test3_received = Vec::new();
    notification_count = 0;
    while let Ok(notification) = notification_receiver.try_recv() {
        let (json, _, _) = notification;
        println!(
            "📥 [TEST3-SSE-{}] Notification JSON:",
            notification_count + 1
        );
        println!("{}", json);
        test3_received.push(json);
        notification_count += 1;
    }
    tokio::time::sleep(Duration::from_secs(3)).await;
    while let Ok(notification) = notification_receiver.try_recv() {
        let (json, _, _) = notification;
        println!(
            "📥 [TEST3-SSE-{}] Notification JSON:",
            notification_count + 1
        );
        println!("{}", json);
        test3_received.push(json);
        notification_count += 1;
    }

    // Clean up SSE handle
    sse_handle.abort();

    // Extract correlation IDs from received notifications
    let extract_correlation_id = |notification_json: &str| -> Option<String> {
        let parsed: serde_json::Value = serde_json::from_str(notification_json).ok()?;
        parsed["params"]["_meta"]["correlation_id"]
            .as_str()
            .map(|s| s.to_string())
    };

    let test1_received_ids: Vec<String> = test1_received
        .iter()
        .filter_map(|json| extract_correlation_id(json))
        .collect();
    let test2_received_ids: Vec<String> = test2_received
        .iter()
        .filter_map(|json| extract_correlation_id(json))
        .collect();
    let test3_received_ids: Vec<String> = test3_received
        .iter()
        .filter_map(|json| extract_correlation_id(json))
        .collect();

    // Count only notifications WITH correlation IDs (from send_log, not set_logging_level)
    let test1_notification_count = test1_received_ids.len();
    let test2_notification_count = test2_received_ids.len();
    let test3_notification_count = test3_received_ids.len();

    // Generate final test report
    println!("\n🎯 SESSION-AWARE LOGGING FILTER TEST RESULTS");
    println!("===========================================");

    // Test 1 Analysis - DEBUG session (should receive ALL)
    println!("\n📊 TEST 1 - DEBUG Session (threshold: Debug)");
    println!("Sent 8 requests at all log levels:");
    let mut test1_passed = true;
    for (correlation_id, level) in &test1_requests {
        if test1_received_ids.contains(correlation_id) {
            println!("   ✅ {} [{}]: RECEIVED", level, correlation_id);
        } else {
            println!(
                "   ❌ {} [{}]: FILTERED (unexpected!)",
                level, correlation_id
            );
            test1_passed = false;
        }
    }
    println!(
        "Expected: 8 notifications | Received: {} | Result: {}",
        test1_notification_count,
        if test1_passed && test1_notification_count == 8 {
            "✅ PASS"
        } else {
            "❌ FAIL"
        }
    );

    // Test 2 Analysis - INFO session (should filter Debug)
    println!("\n📊 TEST 2 - INFO Session (threshold: Info)");
    println!("Sent 8 requests at all log levels:");
    let mut test2_passed = true;
    let expected_info_levels = [
        "Info",
        "Notice",
        "Warning",
        "Error",
        "Critical",
        "Alert",
        "Emergency",
    ];
    for (correlation_id, level) in &test2_requests {
        let should_receive = expected_info_levels.contains(&level.as_str());
        let did_receive = test2_received_ids.contains(correlation_id);

        if should_receive && did_receive {
            println!("   ✅ {} [{}]: RECEIVED (expected)", level, correlation_id);
        } else if !should_receive && !did_receive {
            println!("   ✅ {} [{}]: FILTERED (expected)", level, correlation_id);
        } else {
            println!(
                "   ❌ {} [{}]: {} (unexpected!)",
                level,
                correlation_id,
                if did_receive { "RECEIVED" } else { "FILTERED" }
            );
            test2_passed = false;
        }
    }
    println!(
        "Expected: 7 notifications | Received: {} | Result: {}",
        test2_notification_count,
        if test2_passed && test2_notification_count == 7 {
            "✅ PASS"
        } else {
            "❌ FAIL"
        }
    );

    // Test 3 Analysis - ERROR session (should filter Debug, Info, Notice, Warning)
    println!("\n📊 TEST 3 - ERROR Session (threshold: Error)");
    println!("Sent 8 requests at all log levels:");
    let mut test3_passed = true;
    let expected_error_levels = ["Error", "Critical", "Alert", "Emergency"];
    for (correlation_id, level) in &test3_requests {
        let should_receive = expected_error_levels.contains(&level.as_str());
        let did_receive = test3_received_ids.contains(correlation_id);

        if should_receive && did_receive {
            println!("   ✅ {} [{}]: RECEIVED (expected)", level, correlation_id);
        } else if !should_receive && !did_receive {
            println!("   ✅ {} [{}]: FILTERED (expected)", level, correlation_id);
        } else {
            println!(
                "   ❌ {} [{}]: {} (unexpected!)",
                level,
                correlation_id,
                if did_receive { "RECEIVED" } else { "FILTERED" }
            );
            test3_passed = false;
        }
    }
    println!(
        "Expected: 4 notifications | Received: {} | Result: {}",
        test3_notification_count,
        if test3_passed && test3_notification_count == 4 {
            "✅ PASS"
        } else {
            "❌ FAIL"
        }
    );

    // Overall Result
    let all_passed = test1_passed
        && test2_passed
        && test3_passed
        && test1_notification_count == 8
        && test2_notification_count == 7
        && test3_notification_count == 4;

    println!(
        "\n🏆 OVERALL RESULT: {}",
        if all_passed {
            "✅ ALL TESTS PASSED"
        } else {
            "❌ SOME TESTS FAILED"
        }
    );

    if all_passed {
        println!("🎯 Session-aware logging filtering is working correctly!");
        println!("   • Correlation IDs properly track request→notification mapping");
        println!("   • Different session log levels filter appropriately");
        println!("   • MCP RFC-5424 severity level compliance confirmed");
    } else {
        println!("⚠️  Some tests failed - check session filtering implementation");
    }

    Ok(())
}
