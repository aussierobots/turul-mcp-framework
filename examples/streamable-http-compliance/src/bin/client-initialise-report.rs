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
//! cargo run --bin client-initialise-report
//! # or with custom server URL:
//! cargo run --bin client-initialise-report -- --url http://127.0.0.1:8001/mcp
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
    #[arg(long, default_value = "http://127.0.0.1:8001/mcp")]
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
    
    // Step 3: Generate Final Report
    info!("\n📊 Final Session Lifecycle Report");
    generate_final_report(&session_report);
    
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
    })
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

    // Final Recommendation
    info!("\n🎯 RECOMMENDATION:");
    match &report.session_source {
        SessionSource::ServerHeader => {
            info!("   ✅ Session architecture is MCP compliant");
            info!("   ✅ Ready to proceed with SSE streaming implementation");
        },
        SessionSource::ServerBody => {
            warn!("   ⚠️ Consider moving session ID to Mcp-Session-Id header");
            info!("   ✅ Can proceed with SSE streaming implementation");
        },
        SessionSource::None => {
            error!("   ❌ CRITICAL: Must implement server session creation first");
            error!("   ❌ DO NOT proceed with SSE streaming until sessions work");
            error!("   🔧 Implement GPS project's new_session() pattern");
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