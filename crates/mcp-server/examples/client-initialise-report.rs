//! # MCP Initialize Session Report Client
//!
//! This client tests and reports on the MCP initialize session lifecycle:
//! - Connects to MCP server
//! - Tests initialize request/response
//! - Reports server-provided session ID (Mcp-Session-Id header)  
//! - Shows server capabilities and info
//! - Tests SSE GET connection with session ID
//! 
//! This is critical for verifying the server follows proper MCP protocol
//! where sessions are SERVER-MANAGED, not client-generated.
//!
//! ## Usage
//! ```bash
//! cargo run --example client-initialise-report
//! # or with custom server URL:
//! cargo run --example client-initialise-report -- --url http://127.0.0.1:8001/mcp
//! ```
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
//! curl -X GET http://127.0.0.1:8001/mcp \
//!   -H "Accept: text/event-stream" \
//!   -H "Mcp-Session-Id: YOUR_SESSION_ID_HERE" \
//!   -N
//! ```

use std::time::Duration;
use anyhow::{Result, anyhow};
use clap::Parser;
use reqwest::Client;
use serde_json::{json, Value};
use tracing::{info, warn, error, debug};

#[derive(Parser)]
#[command(name = "client-initialise-report")]
#[command(about = "Test and report on MCP initialize session lifecycle")]
struct Args {
    #[arg(long, default_value = "http://127.0.0.1:8000/mcp")]
    url: String,
    
    #[arg(long, default_value = "5")]
    timeout_seconds: u64,
    
    #[arg(long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    // Initialize tracing
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(if args.verbose { tracing::Level::DEBUG } else { tracing::Level::INFO })
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;
    
    info!("🚀 MCP Initialize Session Report Client");
    info!("   • Target URL: {}", args.url);
    info!("   • Testing server-provided session IDs (MCP protocol compliance)");
    
    let client = Client::new();
    
    // Step 1: Test Initialize Request
    info!("\n📡 Step 1: Testing MCP Initialize Request");
    let session_report = test_initialize_request(&client, &args.url, args.timeout_seconds).await?;
    
    // Step 2: Test SSE Connection (if we got a session ID)
    if let Some(ref session_id) = session_report.session_id {
        info!("\n📡 Step 2: Testing SSE Connection with Session ID");
        test_sse_connection(&client, &args.url, session_id, args.timeout_seconds).await?;
    } else {
        warn!("\n⚠️ Step 2 SKIPPED: No session ID provided by server - cannot test SSE");
    }
    
    // Step 3: Test echo_sse tool (after SSE is established)
    let mut sse_test_result = SseTestResult {
        success: false,
        error_message: Some("SSE test not performed".to_string()),
    };
    
    if let Some(ref session_id) = session_report.session_id {
        info!("\n📡 Step 3: Testing echo_sse Tool with SSE Streaming");
        sse_test_result = test_echo_sse_tool(&client, &args.url, session_id, args.timeout_seconds).await?;
    } else {
        warn!("\n⚠️ Step 3 SKIPPED: No session ID provided by server - cannot test tool");
        sse_test_result = SseTestResult {
            success: false,
            error_message: Some("No session ID available for SSE test".to_string()),
        };
    }
    
    // Update the report with SSE test results
    let mut final_report = session_report;
    final_report.sse_streaming_success = sse_test_result.success;
    final_report.sse_streaming_error = sse_test_result.error_message;
    
    // Step 4: Generate Final Report
    info!("\n📊 Final Session Lifecycle Report");
    generate_final_report(&final_report);
    
    Ok(())
}

#[derive(Debug)]
struct SessionReport {
    pub session_id: Option<String>,
    pub session_source: SessionSource,
    pub protocol_version: Option<String>,
    pub server_name: Option<String>,
    pub server_version: Option<String>,
    pub server_capabilities: Option<Value>,
    pub initialize_status: u16,
    pub sse_connection_success: bool,
    pub sse_content_type: Option<String>,
    pub sse_streaming_success: bool,
    pub sse_streaming_error: Option<String>,
}

#[derive(Debug)]
enum SessionSource {
    ServerHeader,    // ✅ Proper MCP protocol
    ServerBody,      // ⚠️ Non-standard but acceptable
    ClientGenerated, // ❌ Protocol violation
    None,           // ❌ No session management
}

async fn test_initialize_request(
    client: &Client, 
    url: &str, 
    timeout_seconds: u64
) -> Result<SessionReport> {
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

    let body: Value = response.json().await?;
    debug!("📥 Initialize response body: {}", serde_json::to_string_pretty(&body)?);

    // Extract session ID from body (non-standard but check anyway)
    let session_from_body = body
        .get("result")
        .and_then(|r| r.get("sessionId"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // Determine session source and ID
    let (session_id, session_source) = if let Some(header_session) = session_from_header {
        info!("✅ Server provided session ID via Mcp-Session-Id header: {}", header_session);
        (Some(header_session), SessionSource::ServerHeader)
    } else if let Some(body_session) = session_from_body {
        warn!("⚠️ Server provided session ID via response body (non-standard): {}", body_session);
        (Some(body_session), SessionSource::ServerBody)
    } else {
        error!("❌ Server did not provide session ID - this violates MCP protocol!");
        error!("   • Servers MUST provide session IDs during initialize");
        error!("   • This forces clients to generate session IDs (protocol violation)");
        (None, SessionSource::None)
    };

    // Extract server info
    let result = body.get("result");
    let protocol_version = result
        .and_then(|r| r.get("protocolVersion"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    
    let server_info = result.and_then(|r| r.get("serverInfo"));
    let server_name = server_info
        .and_then(|s| s.get("name"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    
    let server_version = server_info
        .and_then(|s| s.get("version"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    
    let server_capabilities = result
        .and_then(|r| r.get("capabilities"))
        .cloned();

    info!("📋 Server Details:");
    if let Some(ref name) = server_name {
        info!("   • Name: {}", name);
    }
    if let Some(ref version) = server_version {
        info!("   • Version: {}", version);
    }
    if let Some(ref protocol) = protocol_version {
        info!("   • Protocol Version: {}", protocol);
    }
    if let Some(ref caps) = server_capabilities {
        info!("   • Capabilities: {}", serde_json::to_string_pretty(caps)?);
    }

    Ok(SessionReport {
        session_id,
        session_source,
        protocol_version,
        server_name,
        server_version,
        server_capabilities,
        initialize_status: status,
        sse_connection_success: false,
        sse_content_type: None,
        sse_streaming_success: false,
        sse_streaming_error: None,
    })
}

// New streaming SSE listener that doesn't consume entire response
async fn listen_for_sse_event_streaming(
    client: &Client,
    base_url: &str,
    session_id: &str,
    event_type: &str,
    timeout_seconds: u64,
) -> Result<Value> {
    use tokio::time::{timeout, Duration as TokioDuration};
    use futures::StreamExt;

    debug!("🌊 Starting streaming SSE listener for event '{}'", event_type);

    // Create SSE connection with session ID
    let response = client
        .get(base_url)
        .header("Accept", "text/event-stream")
        .header("Cache-Control", "no-cache")
        .header("Mcp-Session-Id", session_id)
        .timeout(Duration::from_secs(timeout_seconds))
        .send()
        .await?;

    if response.status() != 200 {
        return Err(anyhow!("SSE connection failed with status: {}", response.status()));
    }

    debug!("🔗 SSE connection established, streaming events...");

    // Convert response to byte stream
    let mut stream = response.bytes_stream();
    let mut buffer = String::new();
    
    let listen_timeout = TokioDuration::from_secs(timeout_seconds);
    
    match timeout(listen_timeout, async {
        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(chunk) => {
                    let chunk_str = String::from_utf8_lossy(&chunk);
                    buffer.push_str(&chunk_str);
                    debug!("📈 Received chunk: '{}' (buffer now {} chars)", chunk_str.trim(), buffer.len());
                    
                    // Process complete events (separated by double newlines)
                    while let Some(event_end) = buffer.find("\n\n") {
                        let event_block = buffer[..event_end].to_string();
                        buffer.drain(..event_end + 2);
                        
                        debug!("🎆 Processing complete event block: '{}'", event_block.trim());
                        
                        if !event_block.trim().is_empty() {
                            if let Some(event_data) = parse_sse_event(&event_block, event_type)? {
                                debug!("✅ Found matching event of type '{}'", event_type);
                                return Ok(event_data);
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("❌ SSE stream error: {}", e);
                    return Err(anyhow!("SSE stream error: {}", e));
                }
            }
        }
        
        Err(anyhow!("SSE stream ended without finding expected event type '{}'", event_type))
    }).await {
        Ok(result) => result,
        Err(_) => Err(anyhow!("Timeout waiting for SSE event '{}' after {}s", event_type, listen_timeout.as_secs()))
    }
}

async fn test_sse_connection(
    client: &Client, 
    url: &str, 
    session_id: &str,
    timeout_seconds: u64
) -> Result<()> {
    info!("🔗 Testing SSE connection with session ID: {}", session_id);
    
    let response = client
        .get(url)
        .header("Accept", "text/event-stream")
        .header("Cache-Control", "no-cache")
        .header("Connection", "keep-alive")
        .header("Mcp-Session-Id", session_id)
        .timeout(Duration::from_secs(timeout_seconds))
        .send()
        .await?;

    let status = response.status().as_u16();
    let headers = response.headers().clone();
    let content_type = headers
        .get("content-type")
        .and_then(|ct| ct.to_str().ok())
        .unwrap_or("unknown");

    info!("📥 SSE response status: {}", status);
    info!("   • Content-Type: {}", content_type);
    debug!("   • Response headers: {:#?}", headers);

    if status == 200 && content_type.contains("text/event-stream") {
        info!("✅ SSE connection established successfully");
        
        // Read a small sample of the SSE stream
        let body = response.bytes().await?;
        let sample = String::from_utf8_lossy(&body[..std::cmp::min(200, body.len())]);
        info!("📦 SSE stream sample:");
        for line in sample.lines().take(5) {
            info!("   {}", line);
        }
    } else {
        error!("❌ SSE connection failed");
        error!("   • Expected: 200 OK with Content-Type: text/event-stream");
        error!("   • Actual: {} with Content-Type: {}", status, content_type);
    }

    Ok(())
}

#[derive(Debug)]
struct SseTestResult {
    pub success: bool,
    pub error_message: Option<String>,
}

async fn test_echo_sse_tool(
    client: &Client,
    base_url: &str,
    session_id: &str,
    timeout_seconds: u64,
) -> Result<SseTestResult> {
    info!("🔧 Testing echo_sse tool with text: 'Hello from SSE test!'");
    info!("🎯 Strategy: Start SSE listener FIRST, then call tool while it's listening");
    
    use tokio::time::{timeout, Duration as TokioDuration};
    
    // Start SSE listener in background FIRST
    info!("🌊 Step 1: Starting background SSE listener for echo_response events");
    let client_clone = client.clone();
    let base_url_clone = base_url.to_string();
    let session_id_clone = session_id.to_string();
    
    let sse_listener = tokio::spawn(async move {
        listen_for_sse_event_streaming(&client_clone, &base_url_clone, &session_id_clone, "notifications/message", 15).await
    });
    
    // Small delay to ensure SSE connection is established
    tokio::time::sleep(TokioDuration::from_millis(500)).await;
    info!("✅ SSE listener active, now making tool call...");
    
    // Prepare tool call request
    info!("📡 Step 2: Calling echo_sse tool while SSE connection is listening");
    let tool_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "echo_sse",
            "arguments": {
                "text": "Hello from SSE test!"
            }
        }
    });
    
    // Send POST request with session ID header
    let response = client
        .post(base_url)
        .header("Content-Type", "application/json")
        .header("Mcp-Session-Id", session_id)
        .json(&tool_request)
        .timeout(Duration::from_secs(timeout_seconds))
        .send()
        .await?;
    
    let status = response.status().as_u16();
    info!("📥 Tool call response status: {}", status);
    
    if status == 200 {
        let response_body: Value = response.json().await?;
        info!("✅ Tool call succeeded");
        info!("📦 POST Response:");
        if let Some(result) = response_body.get("result") {
            info!("   {}", serde_json::to_string_pretty(result)?);
        } else {
            info!("   {}", serde_json::to_string_pretty(&response_body)?);
        }
        
        info!("📡 Step 3: Tool call succeeded, now waiting for SSE event from background listener...");
        
        // Wait for SSE event from background task
        match timeout(TokioDuration::from_secs(10), sse_listener).await {
            Ok(Ok(Ok(event_data))) => {
                info!("✅ Received SSE event for echo_sse from persistent connection!");
                info!("📦 SSE Event Data:");
                info!("   {}", serde_json::to_string_pretty(&event_data)?);
                
                // Validate the event contains expected data
                if let Some(tool) = event_data.get("tool").and_then(|v| v.as_str()) {
                    if tool == "echo_sse" {
                        info!("🎯 SSE event correctly identifies tool as 'echo_sse'");
                    }
                }
                if let Some(original) = event_data.get("original_text").and_then(|v| v.as_str()) {
                    if original == "Hello from SSE test!" {
                        info!("🎯 SSE event contains correct original text");
                    }
                }
                return Ok(SseTestResult {
                    success: true,
                    error_message: None,
                });
            }
            Ok(Ok(Err(e))) => {
                error!("❌ Background SSE listener failed: {}", e);
                error!("   This indicates the SSE streaming is not working properly");
                return Ok(SseTestResult {
                    success: false,
                    error_message: Some(format!("SSE listener failed: {}", e)),
                });
            }
            Ok(Err(e)) => {
                error!("❌ Background SSE listener panicked: {}", e);
                return Ok(SseTestResult {
                    success: false,
                    error_message: Some(format!("SSE listener panicked: {}", e)),
                });
            }
            Err(_) => {
                error!("❌ Timeout waiting for SSE event from background listener (10s)");
                error!("   Event may not have been sent or connection may have failed");
                return Ok(SseTestResult {
                    success: false,
                    error_message: Some("Timeout waiting for SSE event (10s)".to_string()),
                });
            }
        }
        
    } else {
        error!("❌ Tool call failed with status: {}", status);
        let error_body = response.text().await?;
        error!("   Error: {}", error_body);
        return Ok(SseTestResult {
            success: false,
            error_message: Some(format!("Tool call failed with status {}: {}", status, error_body)),
        });
    }
    
    // Fallback - should not reach here
    Ok(SseTestResult {
        success: false,
        error_message: Some("Unknown error in SSE test".to_string()),
    })
}

async fn listen_for_sse_event(
    client: &Client,
    base_url: &str,
    session_id: &str,
    event_type: &str,
    timeout_seconds: u64,
) -> Result<Value> {
    use tokio::time::{timeout, Duration as TokioDuration};

    // Create SSE connection with session ID
    let response = client
        .get(base_url)
        .header("Accept", "text/event-stream")
        .header("Cache-Control", "no-cache")
        .header("Mcp-Session-Id", session_id)
        .timeout(Duration::from_secs(timeout_seconds))
        .send()
        .await?;

    if response.status() != 200 {
        return Err(anyhow!("SSE connection failed with status: {}", response.status()));
    }

    // Read the response body as text with timeout
    let listen_timeout = TokioDuration::from_secs(10); // Wait up to 10 seconds for the event
    
    match timeout(listen_timeout, async {
        // Get the full response body
        let body = response.text().await?;
        debug!("SSE response body: {}", body);
        
        // Process SSE events (separated by double newlines)
        for event_block in body.split("\n\n") {
            if event_block.trim().is_empty() {
                continue;
            }
            
            debug!("Processing SSE event block: {}", event_block);
            
            // Parse SSE event
            if let Some(event_data) = parse_sse_event(event_block, event_type)? {
                return Ok(event_data);
            }
        }
        
        Err(anyhow!("SSE stream did not contain expected event type '{}'", event_type))
    }).await {
        Ok(result) => result,
        Err(_) => Err(anyhow!("Timeout waiting for SSE event '{}' after {}s", event_type, listen_timeout.as_secs()))
    }
}

fn parse_sse_event(event_text: &str, expected_event_type: &str) -> Result<Option<Value>> {
    let mut event_type_line = None;
    let mut data_line = None;
    
    for line in event_text.lines() {
        if line.starts_with("event:") {
            event_type_line = Some(line.trim_start_matches("event:").trim());
        } else if line.starts_with("data:") {
            data_line = Some(line.trim_start_matches("data:").trim());
        }
    }
    
    // If this is a custom event, check the event type
    if let Some(event_type) = event_type_line {
        if event_type != expected_event_type {
            return Ok(None); // Not the event we're looking for
        }
    } else if let Some(data) = data_line {
        // For events without explicit event type, check the data content
        if let Ok(parsed_data) = serde_json::from_str::<Value>(data) {
            // Check if this is a custom event with the expected type
            if let Some(custom_type) = parsed_data.get("type").and_then(|v| v.as_str()) {
                if custom_type == "custom" {
                    if let Some(data_event_type) = parsed_data.get("event_type").and_then(|v| v.as_str()) {
                        if data_event_type == expected_event_type {
                            // Return the nested data
                            return Ok(parsed_data.get("data").cloned());
                        }
                    }
                }
            }
        }
        return Ok(None); // Not the event we're looking for
    }
    
    // Parse the data if we found the right event type
    if let Some(data) = data_line {
        let parsed_data: Value = serde_json::from_str(data)?;
        Ok(Some(parsed_data))
    } else {
        Ok(None)
    }
}

fn generate_final_report(report: &SessionReport) {
    info!("═══════════════════════════════════════");
    info!("📊 MCP INITIALIZE SESSION LIFECYCLE REPORT");
    info!("═══════════════════════════════════════");
    
    // Session ID Analysis
    match (&report.session_id, &report.session_source) {
        (Some(id), SessionSource::ServerHeader) => {
            info!("✅ SESSION MANAGEMENT: COMPLIANT");
            info!("   • Session ID: {}", id);
            info!("   • Source: Mcp-Session-Id header (proper MCP protocol)");
            info!("   • Server correctly manages sessions");
        },
        (Some(id), SessionSource::ServerBody) => {
            warn!("⚠️ SESSION MANAGEMENT: NON-STANDARD");
            warn!("   • Session ID: {}", id);
            warn!("   • Source: Response body (non-standard but acceptable)");
            warn!("   • Consider moving to Mcp-Session-Id header");
        },
        (None, SessionSource::None) => {
            error!("❌ SESSION MANAGEMENT: PROTOCOL VIOLATION");
            error!("   • Session ID: None provided by server");
            error!("   • This violates MCP protocol requirements");
            error!("   • Clients will be forced to generate session IDs");
            error!("   • CRITICAL: Must implement server session creation");
        },
        _ => {
            error!("❌ SESSION MANAGEMENT: INCONSISTENT STATE");
        }
    }

    // Server Info
    info!("\n📋 SERVER INFORMATION:");
    info!("   • Status: {} {}", report.initialize_status, 
          if report.initialize_status == 200 { "OK" } else { "ERROR" });
    
    if let Some(ref name) = report.server_name {
        info!("   • Name: {}", name);
    }
    if let Some(ref version) = report.server_version {
        info!("   • Version: {}", version);
    }
    if let Some(ref protocol) = report.protocol_version {
        info!("   • Protocol: {}", protocol);
    } else {
        warn!("   • Protocol: Not specified");
    }

    // Capability Analysis
    if let Some(ref caps) = report.server_capabilities {
        info!("\n🔧 SERVER CAPABILITIES:");
        
        // Check for standard capabilities
        if caps.get("tools").is_some() {
            info!("   • ✅ Tools: Supported");
        }
        if caps.get("resources").is_some() {
            info!("   • ✅ Resources: Supported");
        }
        if caps.get("prompts").is_some() {
            info!("   • ✅ Prompts: Supported");
        }
        if caps.get("logging").is_some() {
            info!("   • ✅ Logging: Supported");
        }
        if caps.get("sampling").is_some() {
            info!("   • ✅ Sampling: Supported");
        }
        
        // Check for non-standard capabilities
        if let Some(experimental) = caps.get("experimental") {
            warn!("   • ⚠️ Experimental capabilities detected:");
            warn!("      {}", serde_json::to_string_pretty(experimental).unwrap_or_else(|_| "Invalid JSON".to_string()));
        }
    } else {
        warn!("\n🔧 SERVER CAPABILITIES: None specified");
    }

    // SSE Streaming Test Results  
    info!("\n🌊 SSE STREAMING TEST:");
    if report.sse_streaming_success {
        info!("   ✅ SSE streaming is working correctly");
        info!("   ✅ Real-time events flow from tools to clients via SSE");
    } else {
        error!("   ❌ SSE streaming test FAILED");
        if let Some(ref error_msg) = report.sse_streaming_error {
            error!("   ❌ Error: {}", error_msg);
        }
        error!("   ❌ Events are not reaching clients via SSE streams");
    }
    
    // Combined Recommendation (Session + SSE)
    info!("\n🎯 RECOMMENDATION:");
    match (&report.session_source, report.sse_streaming_success) {
        (SessionSource::ServerHeader, true) => {
            info!("   ✅ 🎆 FULLY COMPLIANT: Session architecture + SSE streaming both working!");
            info!("   ✅ Ready for production MCP over HTTP with real-time events");
        },
        (SessionSource::ServerHeader, false) => {
            warn!("   ⚠️ Session architecture is MCP compliant, but SSE streaming is broken");
            error!("   ❌ DO NOT proceed to production until SSE streaming is fixed");
            error!("   🔧 Debug the SSE event bridge between session manager and stream manager");
        },
        (SessionSource::ServerBody, true) => {
            warn!("   ⚠️ Consider moving session ID to Mcp-Session-Id header");
            info!("   ✅ SSE streaming works - can proceed with implementation");
        },
        (SessionSource::ServerBody, false) => {
            warn!("   ⚠️ Session management works but not ideal (use headers)");
            error!("   ❌ SSE streaming is broken - fix before proceeding");
        },
        (SessionSource::None, _) => {
            error!("   ❌ CRITICAL: Must implement server session creation first");
            error!("   ❌ DO NOT proceed until sessions work");
            error!("   🔧 Implement GPS project's new_session() pattern");
        },
        (SessionSource::ClientGenerated, _) => {
            error!("   ❌ CRITICAL: Server incorrectly using client-generated sessions");
            error!("   ❌ This violates MCP protocol - server must create session IDs");
            error!("   🔧 Implement proper server-side session creation");
        },
        _ => {
            error!("   ❌ Unknown session state - investigate");
        }
    }
    
    info!("═══════════════════════════════════════");
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::{HeaderMap, HeaderValue};
    
    #[test]
    fn test_session_source_classification() {
        // Test header detection
        let mut headers = HeaderMap::new();
        headers.insert("Mcp-Session-Id", HeaderValue::from_str("test-uuid").unwrap());
        
        let session_from_header = headers
            .get("Mcp-Session-Id")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());
            
        assert_eq!(session_from_header, Some("test-uuid".to_string()));
    }
    
    #[test]
    fn test_server_info_extraction() {
        let body = json!({
            "result": {
                "protocolVersion": "2025-06-18",
                "serverInfo": {
                    "name": "test-server",
                    "version": "1.0.0"
                },
                "capabilities": {
                    "tools": {}
                }
            }
        });
        
        let protocol = body.get("result")
            .and_then(|r| r.get("protocolVersion"))
            .and_then(|v| v.as_str());
            
        assert_eq!(protocol, Some("2025-06-18"));
    }
}