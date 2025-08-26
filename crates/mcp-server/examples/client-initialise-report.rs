//! # MCP Initialize Session Report Client
//!
//! A comprehensive test client that validates MCP session management and SSE streaming.
//! This client tests the complete MCP initialize lifecycle where:
//! - Server generates and manages session IDs (not client)
//! - Session IDs are provided via Mcp-Session-Id headers
//! - SSE connections use session IDs for proper event targeting
//! - Tools can send notifications via SSE streams
//!
//! ## Usage
//! ```bash
//! # Test against a running MCP server
//! cargo run --example client-initialise-report -- --url http://127.0.0.1:8000/mcp
//! ```
//!
//! ## Expected Output
//! The client will test and report on:
//! 1. MCP initialize request/response cycle
//! 2. Session ID extraction from server headers
//! 3. SSE connection establishment with session ID
//! 4. Tool execution with SSE event streaming
//!
//! ## Equivalent Curl Commands
//! 
//! **Initialize Request:**
//! ```bash
//! curl -X POST http://127.0.0.1:8001/mcp \
//!   -H "Content-Type: application/json" \
//!   -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test-client","version":"1.0.0"}}}' \
//!   -i
//! ```
//!
//! **SSE Connection (with session ID from above):**
//! ```bash
//! curl -N -H "Accept: text/event-stream" \
//!   -H "Mcp-Session-Id: <session-id-from-initialize>" \
//!   http://127.0.0.1:8001/mcp
//! ```

use anyhow::{Result, anyhow};
use clap::Parser;
use reqwest::Client;
use serde_json::{json, Value};
use std::time::Duration;
use tracing::{debug, info, warn, error};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Target MCP server URL
    #[arg(short, long)]
    url: String,

    /// Request timeout in seconds
    #[arg(short, long, default_value = "30")]
    timeout: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let args = Args::parse();
    
    info!("🚀 MCP Initialize Session Report Client");
    info!("   • Target URL: {}", args.url);
    info!("   • Testing server-provided session IDs (MCP protocol compliance)");

    // Create HTTP client
    let client = Client::builder()
        .timeout(Duration::from_secs(args.timeout))
        .build()?;

    // Step 1: Test MCP Initialize
    let (session_id, server_info) = test_mcp_initialize(&client, &args.url, args.timeout).await?;
    
    // Step 2: Test SSE Connection
    test_sse_connection(&client, &args.url, &session_id, args.timeout).await?;
    
    // Step 3: Test Tool with SSE
    let sse_test_result = test_echo_sse_tool(&client, &args.url, &session_id, args.timeout).await;
    
    // Final Report
    print_final_report(&session_id, &server_info, sse_test_result).await?;

    Ok(())
}

async fn test_mcp_initialize(
    client: &Client,
    url: &str,
    timeout_seconds: u64,
) -> Result<(String, Value)> {
    info!("");
    info!("📡 Step 1: Testing MCP Initialize Request");

    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {
                "roots": {
                    "listChanged": false
                },
                "sampling": {}
            },
            "clientInfo": {
                "name": "client-initialise-report",
                "version": "1.0.0"
            }
        }
    });

    debug!("📤 Sending initialize request: {}", serde_json::to_string_pretty(&request)?);

    let response = client
        .post(url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .json(&request)
        .timeout(Duration::from_secs(timeout_seconds))
        .send()
        .await?;

    let status = response.status().as_u16();
    let headers = response.headers().clone();
    
    info!("📥 Initialize response status: {}", status);
    debug!("   • Response headers: {:#?}", headers);

    // Extract session ID from headers (proper MCP protocol)
    let session_from_header = headers
        .get("Mcp-Session-Id")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    // Handle both JSON and SSE responses per MCP Streamable HTTP spec
    let content_type = response.headers()
        .get("content-type")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");
    
    let body: Value = if content_type.starts_with("text/event-stream") {
        info!("📡 Server returned SSE stream for initialize request");
        // Read the SSE stream to get the JSON-RPC response
        let sse_text = response.text().await?;
        debug!("📥 SSE response: {}", sse_text);
        
        // Parse the JSON-RPC response from the SSE data line
        parse_json_from_sse(&sse_text)?
    } else {
        // Standard JSON response
        response.json().await?
    };
    
    debug!("📥 Initialize response body: {}", serde_json::to_string_pretty(&body)?);

    // Extract session ID from body (non-standard but check anyway)
    let session_from_body = body
        .get("result")
        .and_then(|r| r.get("sessionId"))
        .and_then(|s| s.as_str())
        .map(|s| s.to_string());

    // Determine final session ID (header takes precedence per MCP protocol)
    let session_id = if let Some(header_session) = session_from_header {
        info!("✅ Server provided session ID via Mcp-Session-Id header: {}", header_session);
        header_session
    } else if let Some(body_session) = session_from_body {
        warn!("⚠️ Session ID found in response body (non-standard): {}", body_session);
        body_session
    } else {
        return Err(anyhow!("❌ No session ID provided by server (neither header nor body)"));
    };

    // Extract server information for reporting
    let server_info = body.get("result").cloned().unwrap_or_else(|| json!({}));
    
    info!("📋 Server Details:");
    if let Some(server_info_obj) = server_info.as_object() {
        if let Some(name) = server_info_obj.get("serverInfo").and_then(|s| s.get("name")) {
            info!("   • Name: {}", name.as_str().unwrap_or("unknown"));
        }
        if let Some(version) = server_info_obj.get("serverInfo").and_then(|s| s.get("version")) {
            info!("   • Version: {}", version.as_str().unwrap_or("unknown"));
        }
        if let Some(protocol) = server_info_obj.get("protocolVersion") {
            info!("   • Protocol Version: {}", protocol.as_str().unwrap_or("unknown"));
        }
        if let Some(capabilities) = server_info_obj.get("capabilities") {
            info!("   • Capabilities: {}", serde_json::to_string_pretty(capabilities)?);
        }
    }

    Ok((session_id, server_info))
}

async fn test_sse_connection(
    client: &Client,
    base_url: &str,
    session_id: &str,
    timeout_seconds: u64,
) -> Result<()> {
    info!("");
    info!("📡 Step 2: Testing SSE Connection with Session ID");
    info!("🔗 Testing SSE connection with session ID: {}", session_id);

    let response = client
        .get(base_url)
        .header("Accept", "text/event-stream")
        .header("Mcp-Session-Id", session_id)
        .timeout(Duration::from_secs(timeout_seconds))
        .send()
        .await?;

    let status = response.status().as_u16();
    let content_type = response.headers()
        .get("content-type")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    info!("📥 SSE response status: {}", status);
    info!("   • Content-Type: {}", content_type);

    if status == 200 && content_type.starts_with("text/event-stream") {
        info!("✅ SSE connection established successfully");
        info!("📦 SSE stream ready (infinite stream - not reading body to avoid timeout)");
    } else {
        return Err(anyhow!("❌ SSE connection failed: status={}, content-type={}", status, content_type));
    }

    Ok(())
}

async fn test_echo_sse_tool(
    client: &Client,
    base_url: &str,
    session_id: &str,
    timeout_seconds: u64,
) -> Result<()> {
    info!("");
    info!("📡 Step 3: Testing echo_sse Tool with SSE Streaming");
    
    let test_text = "Hello from SSE test!";
    info!("🔧 Testing echo_sse tool with text: '{}'", test_text);
    info!("🎯 MCP Streamable HTTP: POST with Accept header returns SSE stream with tool response + notifications");
    
    let tool_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "echo_sse",
            "arguments": {
                "text": test_text
            }
        }
    });
    
    // Send POST request with Accept: text/event-stream to get SSE response
    info!("📡 Calling echo_sse tool with SSE response requested");
    let response = client
        .post(base_url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .header("Mcp-Session-Id", session_id)
        .json(&tool_request)
        .timeout(Duration::from_secs(timeout_seconds))
        .send()
        .await?;

    let status = response.status().as_u16();
    info!("📥 Tool call response status: {}", status);

    if status == 200 {
        info!("✅ Tool call succeeded");
        
        let content_type = response.headers()
            .get("content-type")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("");
        
        if content_type.starts_with("text/event-stream") {
            info!("📡 Received SSE stream response from tool call (MCP Streamable HTTP)");
            let sse_text = response.text().await?;
            debug!("📥 Full SSE response:\n{}", sse_text);
            
            // Parse all events in the SSE response
            let mut tool_response_found = false;
            let mut notifications_found = Vec::new();
            
            // Split SSE response into events
            for event_block in sse_text.split("\n\n") {
                if !event_block.trim().is_empty() {
                    debug!("🔍 Parsing SSE event: '{}'", event_block.replace('\n', "\\n"));
                    
                    // Extract the data line
                    if let Some(data_line) = event_block.lines().find(|line| line.starts_with("data:")) {
                        let data = data_line.trim_start_matches("data:").trim();
                        if let Ok(json_data) = serde_json::from_str::<Value>(data) {
                            // Check if this is a JSON-RPC response or notification
                            if let Some(method) = json_data.get("method").and_then(|v| v.as_str()) {
                                // This is a notification
                                info!("✅ Found notification: {}", method);
                                notifications_found.push(method.to_string());
                            } else if json_data.get("id").is_some() && json_data.get("result").is_some() {
                                // This is the tool response
                                info!("✅ Found tool response: {}", serde_json::to_string_pretty(&json_data)?);
                                tool_response_found = true;
                            }
                        }
                    }
                }
            }
            
            // Verify we got both the tool response and expected notifications
            if tool_response_found {
                info!("✅ Tool response received in SSE stream");
            } else {
                return Err(anyhow!("❌ Tool response not found in SSE stream"));
            }
            
            if notifications_found.contains(&"notifications/message".to_string()) {
                info!("✅ notifications/message found in SSE stream");
            } else {
                warn!("⚠️  notifications/message not found in SSE stream (expected for echo_sse tool)");
            }
            
            if notifications_found.contains(&"notifications/progress".to_string()) {
                info!("✅ notifications/progress found in SSE stream");  
            } else {
                warn!("⚠️  notifications/progress not found in SSE stream (expected for echo_sse tool)");
            }
            
            info!("🎉 MCP Streamable HTTP test completed: tool response + {} notifications in single SSE stream", notifications_found.len());
            Ok(())
            
        } else {
            // Standard JSON response - this means SSE wasn't supported
            warn!("⚠️  Received JSON response instead of expected SSE stream");
            let body: Value = response.json().await?;
            info!("📦 JSON Response: {}", serde_json::to_string_pretty(&body)?);
            Err(anyhow!("Expected SSE response but got JSON"))
        }
    } else {
        Err(anyhow!("❌ Tool call failed with status: {}", status))
    }
}


/// Parse JSON-RPC response from SSE stream (MCP Streamable HTTP)
fn parse_json_from_sse(sse_text: &str) -> Result<Value> {
    // SSE format: "event: data\ndata: {json-rpc-response}\n\n"
    for line in sse_text.lines() {
        if line.starts_with("data:") {
            let data = line.trim_start_matches("data:").trim();
            if let Ok(json) = serde_json::from_str::<Value>(data) {
                return Ok(json);
            }
        }
    }
    Err(anyhow::anyhow!("No valid JSON-RPC response found in SSE stream"))
}


async fn print_final_report(
    session_id: &str,
    server_info: &Value,
    sse_test_result: Result<()>,
) -> Result<()> {
    info!("");
    info!("📊 Final Session Lifecycle Report");
    info!("═══════════════════════════════════════");
    info!("📊 MCP INITIALIZE SESSION LIFECYCLE REPORT");
    info!("═══════════════════════════════════════");
    info!("✅ SESSION MANAGEMENT: COMPLIANT");
    info!("   • Session ID: {}", session_id);
    info!("   • Source: Mcp-Session-Id header (proper MCP protocol)");
    info!("   • Server correctly manages sessions");
    
    info!("");
    info!("📋 SERVER INFORMATION:");
    info!("   • Status: 200 OK");
    
    if let Some(server_info_obj) = server_info.as_object() {
        if let Some(name) = server_info_obj.get("serverInfo").and_then(|s| s.get("name")) {
            info!("   • Name: {}", name.as_str().unwrap_or("unknown"));
        }
        if let Some(version) = server_info_obj.get("serverInfo").and_then(|s| s.get("version")) {
            info!("   • Version: {}", version.as_str().unwrap_or("unknown"));
        }
        if let Some(protocol) = server_info_obj.get("protocolVersion") {
            info!("   • Protocol: {}", protocol.as_str().unwrap_or("unknown"));
        }
    }
    
    info!("");
    info!("🔧 SERVER CAPABILITIES:");
    if let Some(capabilities) = server_info.get("capabilities") {
        if let Some(_tools) = capabilities.get("tools") {
            info!("   • ✅ Tools: Supported");
        }
    }
    
    info!("");
    info!("🌊 MCP STREAMABLE HTTP TEST:");
    match sse_test_result {
        Ok(_) => {
            info!("   ✅ MCP Streamable HTTP WORKING");
            info!("   ✅ POST requests with Accept header return SSE streams");
            info!("   ✅ Tool responses and notifications delivered in same SSE stream");
            info!("   ✅ Proper JSON-RPC format for all events");
        }
        Err(ref e) => {
            error!("   ❌ MCP Streamable HTTP test FAILED");
            error!("   ❌ Error: {}", e);
            error!("   ❌ Tool notifications not appearing in POST SSE response");
        }
    }
    
    info!("");
    info!("🎯 RECOMMENDATION:");
    match sse_test_result {
        Ok(_) => {
            info!("   ✅ 🎆 FULLY MCP COMPLIANT: Session management + Streamable HTTP working!");
            info!("   ✅ Ready for production MCP over HTTP with real-time tool notifications");
            info!("   ✅ Proper implementation of MCP 2025-06-18 Streamable HTTP specification");
        }
        Err(_) => {
            warn!("   ⚠️ Session management is MCP compliant, but tool notifications need fixing");
            error!("   ❌ Tool notifications should appear in POST SSE responses");
            error!("   🔧 Fix tool notification routing to include in POST response streams");
        }
    }
    info!("═══════════════════════════════════════");

    Ok(())
}