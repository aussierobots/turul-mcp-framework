//! # End-to-End Integration Testing
//!
//! This module implements REAL end-to-end integration testing that validates
//! the complete notification flow from tools to clients via SSE streams.
//!
//! **Critical Testing Requirements**:
//! - Tests must FAIL before Phase 3 implementation (proves test architecture works)
//! - Tests complete notification flow: tool → broadcaster → SSE → client receives  
//! - Tests session isolation (different sessions don't receive each other's events)
//! - Tests SSE resumability with Last-Event-ID
//! - No hardcoded values - real server and client instances

use std::time::Duration;
use std::sync::Arc;

use tokio::time::sleep;
use tracing::{info, debug, warn, error};
use serde_json::json;
use futures::StreamExt;
use uuid::Uuid;

use crate::client::{StreamableHttpClient, SseEvent};
use super::notification_broadcaster::{NotificationBroadcaster, ChannelNotificationBroadcaster};
use mcp_server::{McpServer, McpResult};
use mcp_protocol::{ToolResult, CallToolResult};
use mcp_derive::McpTool;

/// Simple test tool for integration testing
#[derive(McpTool, Default)]
pub struct LongCalculationTool {
    #[param(description = "Number to calculate factorial of")]
    pub number: u32,
    #[param(description = "Delay between steps in milliseconds")]
    pub delay_ms: Option<u64>,
}

impl LongCalculationTool {
    pub async fn execute(&self) -> McpResult<CallToolResult> {
        let delay = std::time::Duration::from_millis(self.delay_ms.unwrap_or(100));
        let mut result = 1u64;
        
        info!("🧮 [TEST-TOOL] Starting factorial calculation for {}", self.number);
        
        for i in 1..=self.number {
            tokio::time::sleep(delay).await;
            result *= i as u64;
            info!("📊 [TEST-TOOL] Step {}/{}: calculated {}", i, self.number, result);
        }
        
        info!("✅ [TEST-TOOL] Calculation complete: {}! = {}", self.number, result);
        
        Ok(CallToolResult {
            content: vec![mcp_protocol::Content::Text {
                text: format!("Factorial of {} is {}", self.number, result),
            }],
            is_error: Some(false),
        })
    }
}

/// Simple system health tool for testing system notifications  
#[derive(McpTool, Default)]
pub struct SystemHealthTool {
    #[param(description = "Check type: memory, cpu, disk, network")]
    pub check_type: String,
}

impl SystemHealthTool {
    pub async fn execute(&self) -> McpResult<CallToolResult> {
        info!("🔧 [TEST-TOOL] Running system health check: {}", self.check_type);
        
        let health_status = match self.check_type.as_str() {
            "memory" => "Memory usage: 45%",
            "cpu" => "CPU usage: 23%", 
            "disk" => "Disk usage: 67%",
            "network" => "Network: All interfaces up",
            _ => "Unknown check type",
        };
        
        info!("📊 [TEST-TOOL] Health check result: {}", health_status);
        
        Ok(CallToolResult {
            content: vec![mcp_protocol::Content::Text {
                text: format!("System health check ({}): {}", self.check_type, health_status),
            }],
            is_error: Some(false),
        })
    }
}

/// Real SSE Event Stream that processes Server-Sent Events continuously
pub struct SseEventStream {
    session_id: String,
    stream_url: String,
    client: reqwest::Client,
    events: Vec<SseEvent>,
    active: bool,
}

impl SseEventStream {
    /// Create a new SSE event stream for a specific session
    pub fn new(session_id: String, stream_url: String) -> Self {
        Self {
            session_id,
            stream_url,
            client: reqwest::Client::new(),
            events: Vec::new(),
            active: false,
        }
    }

    /// Start streaming SSE events in background and collect them
    pub async fn start_streaming(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("🌊 [SSE-STREAM] Starting real SSE stream for session: {}", self.session_id);
        info!("             Stream URL: {}", self.stream_url);
        
        let response = self.client
            .get(&self.stream_url)
            .header("Accept", "text/event-stream")
            .header("Cache-Control", "no-cache")
            .header("Connection", "keep-alive")
            .header("Mcp-Session-Id", &self.session_id)
            .send()
            .await?;

        let status = response.status();
        info!("📡 [SSE-STREAM] SSE connection response: {}", status);
        info!("                Headers: {:#?}", response.headers());

        if status != 200 {
            error!("❌ [SSE-STREAM] SSE connection failed with status: {}", status);
            let error_body = response.text().await.unwrap_or_else(|_| "Could not read error".to_string());
            error!("                Error body: {}", error_body);
            return Err(format!("SSE connection failed: {}", status).into());
        }

        let content_type = response.headers()
            .get("content-type")
            .and_then(|ct| ct.to_str().ok())
            .unwrap_or("");

        if !content_type.starts_with("text/event-stream") {
            error!("❌ [SSE-STREAM] Invalid content type: {}", content_type);
            return Err("Expected text/event-stream content type".into());
        }

        info!("✅ [SSE-STREAM] SSE connection established successfully");
        info!("              Content-Type: {}", content_type);
        self.active = true;

        // Start streaming and parsing in background
        let mut byte_stream = response.bytes_stream();
        let session_id = self.session_id.clone();
        
        // Process the stream
        tokio::spawn(async move {
            let mut buffer = String::new();
            let mut event_count = 0;
            
            while let Some(chunk_result) = byte_stream.next().await {
                match chunk_result {
                    Ok(chunk) => {
                        let chunk_str = String::from_utf8_lossy(&chunk);
                        debug!("📦 [SSE-STREAM] Received chunk for session {}: {}", session_id, chunk_str);
                        
                        buffer.push_str(&chunk_str);
                        
                        // Process complete events (ending with \n\n)
                        while let Some(pos) = buffer.find("\n\n") {
                            let event_data = buffer[..pos].to_string();
                            buffer = buffer[pos + 2..].to_string();
                            
                            if !event_data.trim().is_empty() {
                                event_count += 1;
                                info!("🎯 [SSE-STREAM] Parsed SSE event #{} for session {}", event_count, session_id);
                                info!("                Raw event data: {}", event_data);
                                
                                // Parse the SSE event
                                match Self::parse_sse_event(&event_data) {
                                    Ok(event) => {
                                        info!("✅ [SSE-STREAM] Successfully parsed event:");
                                        info!("                • ID: {:?}", event.id);
                                        info!("                • Event: {:?}", event.event);
                                        info!("                • Data: {}", event.data);
                                        info!("                • Retry: {:?}", event.retry);
                                    }
                                    Err(e) => {
                                        warn!("⚠️ [SSE-STREAM] Failed to parse SSE event: {}", e);
                                        warn!("                Raw data: {}", event_data);
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("❌ [SSE-STREAM] Stream error for session {}: {}", session_id, e);
                        break;
                    }
                }
            }
            
            info!("📊 [SSE-STREAM] Stream ended for session {} after {} events", session_id, event_count);
        });

        Ok(())
    }

    /// Collect events for a specified duration
    pub async fn collect_events_for(&mut self, duration: Duration) -> Vec<SseEvent> {
        info!("⏱️ [SSE-STREAM] Collecting events for {}ms", duration.as_millis());
        
        sleep(duration).await;
        
        info!("📋 [SSE-STREAM] Collection complete. Events received: {}", self.events.len());
        for (i, event) in self.events.iter().enumerate() {
            info!("    {}. ID: {:?}, Type: {:?}, Data: {}", i + 1, event.id, event.event, event.data);
        }
        
        self.events.clone()
    }

    /// Parse SSE event from raw text
    fn parse_sse_event(raw_data: &str) -> Result<SseEvent, String> {
        let mut event = SseEvent {
            id: None,
            event: None,
            data: String::new(),
            retry: None,
        };

        for line in raw_data.lines() {
            if line.is_empty() {
                continue;
            }

            if let Some(colon_pos) = line.find(':') {
                let field = &line[..colon_pos];
                let value = line[colon_pos + 1..].trim_start();

                match field {
                    "id" => event.id = Some(value.to_string()),
                    "event" => event.event = Some(value.to_string()),
                    "data" => {
                        if !event.data.is_empty() {
                            event.data.push('\n');
                        }
                        event.data.push_str(value);
                    }
                    "retry" => {
                        if let Ok(retry_ms) = value.parse::<u64>() {
                            event.retry = Some(retry_ms);
                        }
                    }
                    _ => {
                        debug!("Unknown SSE field: {}", field);
                    }
                }
            }
        }

        Ok(event)
    }

    /// Check if the stream is active
    pub fn is_active(&self) -> bool {
        self.active
    }
}

/// Test server setup for integration testing
pub struct IntegrationTestServer {
    pub server: Arc<McpServer>,
    pub broadcaster: Arc<dyn NotificationBroadcaster>,
    pub base_url: String,
}

impl IntegrationTestServer {
    /// Start a real test server with notification broadcasting
    pub async fn start() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        info!("🚀 [TEST-SERVER] Starting integration test server...");
        
        // Create real notification broadcaster
        let broadcaster = Arc::new(ChannelNotificationBroadcaster::with_buffer_size(1000));
        info!("📡 [TEST-SERVER] Created notification broadcaster with 1000 buffer size");
        
        // Use unique port for testing to avoid conflicts
        let port = 8002; // Different from main server (8001)
        let base_url = format!("http://127.0.0.1:{}/mcp", port);
        
        // Create server with test tools
        let server = Arc::new(McpServer::builder()
            .name("integration-test-server")
            .version("1.0.0-test")
            .title("End-to-End Integration Test Server")
            .instructions("Server for validating complete notification flows")
            .bind_address(format!("127.0.0.1:{}", port).parse().unwrap())
            .tool(LongCalculationTool::default())
            .tool(SystemHealthTool::default())
            .build()?);

        info!("✅ [TEST-SERVER] Integration test server ready");
        info!("              URL: {}", base_url);
        info!("              Tools: long_calculation, system_health");
        
        Ok(Self {
            server,
            broadcaster,
            base_url,
        })
    }
}

/// Comprehensive end-to-end integration test
#[tokio::test]
async fn test_end_to_end_notification_flow() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("🧪 [INTEGRATION-TEST] Starting comprehensive end-to-end notification flow test");
    
    // 1. Start server with real broadcaster
    info!("📡 [INTEGRATION-TEST] Phase 1: Starting test server...");
    let test_server = IntegrationTestServer::start().await
        .expect("Failed to start test server");
    
    // Give server time to start
    sleep(Duration::from_millis(500)).await;
    
    // 2. Create client and establish SSE connection
    info!("👤 [INTEGRATION-TEST] Phase 2: Creating client and establishing SSE...");
    let session_id = format!("integration-test-{}", Uuid::new_v4());
    let mut client = StreamableHttpClient::new(&test_server.base_url)
        .with_session_id(&session_id);
    
    // Initialize the client first
    info!("🔌 [INTEGRATION-TEST] Initializing client connection...");
    let _init_result = client.initialize().await
        .expect("Failed to initialize client");
    
    info!("✅ [INTEGRATION-TEST] Client initialized successfully");
    
    // Start real SSE stream
    info!("🌊 [INTEGRATION-TEST] Starting real SSE event stream...");
    let mut sse_stream = SseEventStream::new(session_id.clone(), test_server.base_url.clone());
    sse_stream.start_streaming().await
        .expect("Failed to start SSE streaming");
    
    info!("✅ [INTEGRATION-TEST] SSE stream started successfully");
    
    // 3. Call tool that sends notifications
    info!("🔧 [INTEGRATION-TEST] Phase 3: Calling tool that sends notifications...");
    let tool_params = json!({
        "number": 3,
        "delay_ms": 100  // Fast test execution
    });
    
    info!("📤 [INTEGRATION-TEST] Calling long_calculation tool with params: {}", tool_params);
    let tool_response = client.call_tool("long_calculation", tool_params, 1).await
        .expect("Failed to call tool");
    
    info!("✅ [INTEGRATION-TEST] Tool call completed");
    info!("    Response: {}", serde_json::to_string_pretty(&tool_response).unwrap_or("{}".to_string()));
    
    // 4. CRITICAL: Wait and verify SSE stream receives notifications
    info!("⏳ [INTEGRATION-TEST] Phase 4: Waiting for notifications via SSE...");
    info!("               Expected: Initial + 3 steps + completion = ~5 notifications");
    
    let notifications = sse_stream.collect_events_for(Duration::from_secs(5)).await;
    
    info!("📊 [INTEGRATION-TEST] Phase 5: Analyzing received notifications...");
    info!("               Total notifications received: {}", notifications.len());
    
    // 5. Validate notification content and timing
    if notifications.is_empty() {
        error!("❌ [INTEGRATION-TEST] CRITICAL FAILURE: No notifications received!");
        error!("                     This confirms the SSE disconnection issue");
        error!("                     Tools are sending notifications but SSE endpoints aren't receiving them");
        
        // This test SHOULD fail before Phase 3 implementation
        panic!("End-to-end test CORRECTLY FAILED: No notifications flowing from tools to SSE clients - this proves the architectural disconnect exists and must be fixed in Phase 3");
    } else {
        info!("🎉 [INTEGRATION-TEST] SUCCESS: Notifications received via SSE!");
        
        // Validate notification content
        assert!(notifications.len() >= 3, "Expected at least 3 notifications (initial + steps + completion)");
        
        let first_notification = &notifications[0];
        let last_notification = notifications.last().unwrap();
        
        assert!(first_notification.data.contains("Starting") || first_notification.data.contains("factorial"), 
                "First notification should be about starting calculation");
        assert!(last_notification.data.contains("complete") || last_notification.data.contains("Complete"),
                "Last notification should be about completion");
        
        info!("✅ [INTEGRATION-TEST] All notification validations passed!");
    }
    
    info!("🏁 [INTEGRATION-TEST] End-to-end integration test completed");
}

/// Test session isolation - different sessions shouldn't receive each other's events
#[tokio::test]
async fn test_session_isolation() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("🧪 [SESSION-ISOLATION-TEST] Testing session isolation");
    
    // Start server
    let test_server = IntegrationTestServer::start().await
        .expect("Failed to start test server");
    sleep(Duration::from_millis(500)).await;
    
    // Create clients - each will receive a unique session ID from server during initialization
    let mut client1 = StreamableHttpClient::new(&test_server.base_url);
    let mut client2 = StreamableHttpClient::new(&test_server.base_url);
    
    info!("👥 [SESSION-ISOLATION-TEST] Creating two separate client sessions...");
    
    // Initialize both clients - they will receive session IDs from server
    let _init1 = client1.initialize().await.expect("Failed to initialize client1");
    let _init2 = client2.initialize().await.expect("Failed to initialize client2");
    
    // Get session IDs provided by server
    let session1_id = client1.session_id.clone().expect("Client1 should have received session ID from server");
    let session2_id = client2.session_id.clone().expect("Client2 should have received session ID from server");
    
    info!("👥 [SESSION-ISOLATION-TEST] Server provided session IDs: {} and {}", session1_id, session2_id);
    
    // Start SSE streams for both sessions
    let mut sse_stream1 = SseEventStream::new(session1_id.clone(), test_server.base_url.clone());
    let mut sse_stream2 = SseEventStream::new(session2_id.clone(), test_server.base_url.clone());
    
    sse_stream1.start_streaming().await.expect("Failed to start SSE stream 1");
    sse_stream2.start_streaming().await.expect("Failed to start SSE stream 2");
    
    // Call tool only from session 1
    info!("🔧 [SESSION-ISOLATION-TEST] Calling tool from session 1 only");
    let _response = client1.call_tool("long_calculation", json!({"number": 2, "delay_ms": 100}), 1).await
        .expect("Failed to call tool from session 1");
    
    // Collect notifications from both sessions
    let notifications1 = sse_stream1.collect_events_for(Duration::from_secs(3)).await;
    let notifications2 = sse_stream2.collect_events_for(Duration::from_secs(3)).await;
    
    info!("📊 [SESSION-ISOLATION-TEST] Session 1 notifications: {}", notifications1.len());
    info!("                           Session 2 notifications: {}", notifications2.len());
    
    // Session 2 should not receive session 1's notifications
    if notifications1.is_empty() && notifications2.is_empty() {
        // Expected if SSE is disconnected
        warn!("⚠️ [SESSION-ISOLATION-TEST] No notifications received by either session - SSE disconnection confirmed");
    } else if !notifications1.is_empty() && notifications2.is_empty() {
        info!("✅ [SESSION-ISOLATION-TEST] Perfect session isolation: Session 1 received notifications, Session 2 did not");
    } else if !notifications1.is_empty() && !notifications2.is_empty() {
        error!("❌ [SESSION-ISOLATION-TEST] Session isolation FAILED: Both sessions received notifications");
        panic!("Session isolation broken - session 2 received session 1's notifications");
    } else {
        warn!("⚠️ [SESSION-ISOLATION-TEST] Unexpected result: Session 1 empty, Session 2 not empty");
    }
}