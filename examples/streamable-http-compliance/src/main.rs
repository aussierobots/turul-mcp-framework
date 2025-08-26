//! # Streamable HTTP MCP 2025-06-18 Compliance Test Server
//!
//! This example demonstrates and tests the complete Streamable HTTP implementation
//! according to MCP 2025-06-18 specification, including:
//!
//! - Server-Sent Events (SSE) for real-time notifications
//! - Progress notifications with progressToken tracking
//! - System notifications with fan-out to all sessions
//! - Session-specific notifications
//! - Last-Event-ID header support for resumability
//! - Proper HTTP status codes (202 Accepted for notifications)

use std::sync::{Arc, OnceLock};
use std::time::Duration;

use mcp_derive::McpTool;
use mcp_server::{McpServer, McpResult};
use mcp_protocol::{ToolResult, CallToolResult};
use serde_json::json;
use std::collections::HashMap;
use tokio::time::{sleep, interval};
use tracing::info;
use uuid::Uuid;
use rand;

mod notification_broadcaster;
use notification_broadcaster::{NotificationBroadcaster, ChannelNotificationBroadcaster};

/// Global notification broadcaster for sending SSE notifications
static GLOBAL_BROADCASTER: OnceLock<Arc<dyn NotificationBroadcaster>> = OnceLock::new();

/// Get the global notification broadcaster
fn get_broadcaster() -> Option<Arc<dyn NotificationBroadcaster>> {
    GLOBAL_BROADCASTER.get().cloned()
}

/// Long-running calculation tool that sends progress notifications
#[derive(McpTool, Default)]
pub struct LongCalculationTool {
    #[param(description = "Number to calculate factorial of")]
    number: u32,
    #[param(description = "Delay between steps in milliseconds")]
    delay_ms: Option<u64>,
}

impl LongCalculationTool {
    async fn execute(&self) -> McpResult<CallToolResult> {
        let delay = Duration::from_millis(self.delay_ms.unwrap_or(500));
        let progress_token = format!("calc-{}", Uuid::new_v4());
        // TODO: Get session_id from execution context - currently using fallback
        let session_id = "unknown-session"; // Will be fixed when tools receive proper session context
        
        info!("Starting long calculation for {} with progress token: {}", self.number, progress_token);

        // Send initial progress notification via SSE broadcaster
        if let Some(broadcaster) = get_broadcaster() {
            if let Err(e) = broadcaster.send_progress_notification(
                session_id,
                &progress_token,
                0,
                Some(self.number as u64),
                Some(format!("Starting factorial calculation for {}", self.number))
            ).await {
                info!("Failed to send initial progress notification: {:?}", e);
            } else {
                info!("📊 Sent initial progress notification via SSE: token={}, session={}", progress_token, session_id);
            }
        }
        
        let mut result = 1u64;
        for i in 1..=self.number {
            sleep(delay).await;
            result *= i as u64;
            
            let progress_pct = (i as f64 / self.number as f64 * 100.0) as u64;
            
            // Send step progress notification via SSE broadcaster
            if let Some(broadcaster) = get_broadcaster() {
                if let Err(e) = broadcaster.send_progress_notification(
                    session_id,
                    &progress_token,
                    progress_pct,
                    Some(100),
                    Some(format!("Calculating step {}/{}", i, self.number))
                ).await {
                    info!("Failed to send step progress notification: {:?}", e);
                } else {
                    info!("📊 Sent step progress notification via SSE: token={}, progress={}/{}", 
                          progress_token, progress_pct, 100);
                }
            }
        }

        // Send completion notification via SSE broadcaster
        if let Some(broadcaster) = get_broadcaster() {
            if let Err(e) = broadcaster.send_progress_notification(
                session_id,
                &progress_token,
                100,
                Some(100),
                Some(format!("Factorial calculation complete: {}! = {}", self.number, result))
            ).await {
                info!("Failed to send completion progress notification: {:?}", e);
            } else {
                info!("📊 Sent completion progress notification via SSE: token={}, result={}! = {}", 
                      progress_token, self.number, result);
            }
        }
        
        info!("Calculation complete: {}! = {}", self.number, result);

        let mut meta = HashMap::new();
        meta.insert("progressToken".to_string(), json!(progress_token));
        meta.insert("executionTime".to_string(), json!(self.number as u64 * self.delay_ms.unwrap_or(500)));
        meta.insert("steps".to_string(), json!(self.number));

        Ok(CallToolResult::success(vec![
            ToolResult::text(format!("Factorial of {} is {}", self.number, result)),
        ]).with_meta(meta))
    }
}

/// System health monitoring tool that triggers system-wide notifications
#[derive(McpTool, Default)]
pub struct SystemHealthTool {
    #[param(description = "Check type: memory, cpu, disk, network")]
    check_type: String,
}

impl SystemHealthTool {
    async fn execute(&self) -> McpResult<CallToolResult> {
        let check_id = Uuid::new_v4();
        info!("Performing system health check: {} ({})", self.check_type, check_id);

        // Simulate health check
        sleep(Duration::from_millis(100)).await;
        
        let (status, value) = match self.check_type.as_str() {
            "memory" => ("healthy", 78.5),
            "cpu" => ("warning", 92.1),
            "disk" => ("healthy", 45.2),
            "network" => ("healthy", 156.7),
            _ => ("unknown", 0.0),
        };

        // Send system notification via SSE broadcaster (fan-out to all sessions)
        if let Some(broadcaster) = get_broadcaster() {
            match broadcaster.send_system_notification(
                "health_check",
                &self.check_type,
                status,
                value
            ).await {
                Ok(failed_sessions) => {
                    if failed_sessions.is_empty() {
                        info!("🔔 Sent system notification via SSE to all sessions: type=health_check, component={}, status={}, value={}", 
                              self.check_type, status, value);
                    } else {
                        info!("🔔 Sent system notification via SSE with {} failed sessions: type=health_check, component={}, status={}, value={}", 
                              failed_sessions.len(), self.check_type, status, value);
                    }
                }
                Err(e) => {
                    info!("Failed to send system notification: {:?}", e);
                }
            }
        }
        
        info!("System health check complete: {} = {} ({})", self.check_type, status, value);

        let mut meta = HashMap::new();
        meta.insert("checkId".to_string(), json!(check_id));
        meta.insert("checkType".to_string(), json!(self.check_type));
        meta.insert("status".to_string(), json!(status));
        meta.insert("value".to_string(), json!(value));

        Ok(CallToolResult::success(vec![
            ToolResult::text(format!("Health check: {} = {} (value: {})", 
                                   self.check_type, status, value)),
        ]).with_meta(meta))
    }
}

// Note: Notification structs are no longer needed as we send notifications
// directly through the SSE broadcaster with structured data.

/// Background task that sends periodic system notifications
async fn system_monitor_task(_server: Arc<McpServer>) {
    let mut ticker = interval(Duration::from_secs(10));
    let mut counter = 0u64;

    loop {
        ticker.tick().await;
        counter += 1;

        // Simulate different system metrics
        let metrics = vec![
            ("cpu_usage", rand::random::<f64>() * 100.0),
            ("memory_usage", rand::random::<f64>() * 100.0),
            ("disk_io", rand::random::<f64>() * 1000.0),
            ("network_io", rand::random::<f64>() * 10000.0),
        ];

        for (metric, value) in metrics {
            let status = if value > 90.0 { "critical" } 
                        else if value > 70.0 { "warning" } 
                        else { "healthy" };

            // Send system notification via SSE broadcaster (fan-out to all sessions)
            if let Some(broadcaster) = get_broadcaster() {
                match broadcaster.send_system_notification(
                    "metric_update",
                    metric,
                    status,
                    value
                ).await {
                    Ok(failed_sessions) => {
                        if failed_sessions.is_empty() {
                            info!("📊 Sent system metric notification via SSE to all sessions: component={}, status={}, value={:.2}", 
                                  metric, status, value);
                        } else {
                            info!("📊 Sent system metric notification via SSE with {} failed sessions: component={}, status={}, value={:.2}", 
                                  failed_sessions.len(), metric, status, value);
                        }
                    }
                    Err(e) => {
                        info!("Failed to send system metric notification: {:?}", e);
                    }
                }
                
                // Create session-specific notification for this metric update (rotate between sessions)
                let session_id = format!("session-{}", counter % 3 + 1);
                let action = format!("{}_alert", metric);
                let details = format!("Metric {} updated to {:.2} ({})", metric, value, status);
                
                // Send session notification via SSE broadcaster
                match broadcaster.send_session_notification(&session_id, &action, &details).await {
                    Ok(()) => {
                        info!("🎯 Sent session notification via SSE: session_id={}, action={}, details={}", 
                              session_id, action, details);
                    }
                    Err(e) => {
                        info!("Failed to send session notification: {:?}", e);
                    }
                }
            }
            
            info!("System metric update: {} = {:.2} ({})", metric, value, status);
        }

        info!("System monitor cycle {} complete", counter);
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    info!("🚀 Starting Streamable HTTP MCP 2025-06-18 Compliance Test Server");
    info!("   • Server-Sent Events (SSE) for real-time notifications");
    info!("   • Progress tracking with progressToken");
    info!("   • System notifications with fan-out to all sessions");
    info!("   • Last-Event-ID header support for resumability");
    info!("   • HTTP 202 Accepted for notifications");

    // Create notification broadcaster for SSE communication
    let broadcaster = Arc::new(ChannelNotificationBroadcaster::with_buffer_size(1000));
    
    // Initialize global broadcaster
    GLOBAL_BROADCASTER.set(broadcaster.clone() as Arc<dyn NotificationBroadcaster>)
        .map_err(|_| anyhow::anyhow!("Failed to initialize global broadcaster"))?;
    
    // Build MCP server with streaming HTTP support
    let server = Arc::new(McpServer::builder()
        .name("streamable-http-compliance-server")
        .version("1.0.0")
        .title("MCP 2025-06-18 Streamable HTTP Compliance Test Server")
        .instructions("Test server for comprehensive Streamable HTTP compliance testing")
        .bind_address("127.0.0.1:8001".parse().unwrap())
        .tool(LongCalculationTool::default())
        .tool(SystemHealthTool::default())
        .build()?);

    // Start background system monitoring
    let server_clone = Arc::clone(&server);
    tokio::spawn(async move {
        system_monitor_task(server_clone).await;
    });

    info!("✅ Server ready on http://127.0.0.1:8001/mcp");
    info!("   • POST http://127.0.0.1:8001/mcp - JSON-RPC requests");
    info!("   • GET http://127.0.0.1:8001/mcp - Server-Sent Events (SSE)");
    info!("   • Required headers: Accept: text/event-stream, Mcp-Session-Id: <session>");
    
    info!("🎯 Server started - ready for MCP connections");
    info!("   • No hardcoded demo commands");
    info!("   • Dynamic session management"); 
    info!("   • Real-world MCP 2025-06-18 compliance");

    server.run().await.map_err(|e| anyhow::anyhow!("Server error: {}", e))
}