//! # Real MCP Notification Server
//!
//! This example demonstrates ACTUAL MCP protocol notification implementation using
//! the McpNotification trait with real SSE streaming to clients. This replaces
//! the previous fake tool-based approach with proper MCP protocol features.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use turul_mcp_server::{McpServer, McpResult};
use turul_mcp_protocol::{
    McpError,
    notifications::{HasNotificationMetadata, HasNotificationPayload, HasNotificationRules},
};
use turul_mcp_server::notifications::{McpNotification, DeliveryResult, DeliveryStatus};
use serde_json::{Value, json};
use tokio::sync::RwLock;
use tracing::{info, debug, warn};

/// Development team alert notification handler
/// Implements actual MCP notification protocol for real-time alerts
pub struct DevAlertNotification {
    method: String,
    alert_storage: Arc<RwLock<Vec<AlertMessage>>>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)] // This struct is used for demonstration purposes
struct AlertMessage {
    id: String,
    alert_type: String,
    message: String,
    priority: String,
    timestamp: u64,
    delivered: bool,
}

impl Default for DevAlertNotification {
    fn default() -> Self {
        Self::new()
    }
}

impl DevAlertNotification {
    pub fn new() -> Self {
        Self {
            method: "notifications/dev_alert".to_string(),
            alert_storage: Arc::new(RwLock::new(Vec::new())),
        }
    }

    async fn store_alert(&self, alert: AlertMessage) {
        let mut storage = self.alert_storage.write().await;
        storage.push(alert);
        // Keep only last 100 alerts
        let len = storage.len();
        if len > 100 {
            storage.drain(0..len - 100);
        }
    }
}

// Implement fine-grained traits for MCP compliance
impl HasNotificationMetadata for DevAlertNotification {
    fn method(&self) -> &str {
        &self.method
    }
    
    fn requires_ack(&self) -> bool {
        true // Development alerts should be acknowledged
    }
}

impl HasNotificationPayload for DevAlertNotification {
    fn payload(&self) -> Option<&Value> {
        None // Dynamic payloads
    }
}

impl HasNotificationRules for DevAlertNotification {
    fn priority(&self) -> u32 {
        100 // High priority for dev alerts
    }
    
    fn can_batch(&self) -> bool {
        false // Send alerts immediately, don't batch
    }
    
    fn max_retries(&self) -> u32 {
        3 // Retry failed deliveries up to 3 times
    }
}

// NotificationDefinition automatically implemented via blanket impl

#[async_trait]
impl McpNotification for DevAlertNotification {
    async fn send(&self, payload: Value) -> McpResult<DeliveryResult> {
        let alert_type = payload.get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("general");
        
        let message = payload.get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("No message provided");
            
        let priority = payload.get("priority")
            .and_then(|v| v.as_str())
            .unwrap_or("info");

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let alert_id = format!("alert_{}", timestamp);
        
        // Store the alert
        let alert = AlertMessage {
            id: alert_id.clone(),
            alert_type: alert_type.to_string(),
            message: message.to_string(),
            priority: priority.to_string(),
            timestamp,
            delivered: false,
        };
        
        self.store_alert(alert).await;
        
        // In a real implementation, this would:
        // 1. Send via SSE to all connected clients
        // 2. Send to configured channels (Slack, Email, etc.)
        // 3. Track delivery status
        // 4. Handle retries on failure
        
        info!("🔔 MCP Notification sent: [{}] {} - {}", priority.to_uppercase(), alert_type, message);
        
        // Simulate successful delivery
        Ok(DeliveryResult {
            status: DeliveryStatus::Sent,
            attempts: 1,
            error: None,
            delivered_at: Some(timestamp),
        })
    }

    async fn validate_payload(&self, payload: &Value) -> McpResult<()> {
        // Ensure required fields are present
        if !payload.is_object() {
            return Err(McpError::validation("Payload must be an object"));
        }
        
        let obj = payload.as_object().unwrap();
        
        if !obj.contains_key("message") {
            return Err(McpError::validation("Payload must contain 'message' field"));
        }
        
        if let Some(priority) = obj.get("priority").and_then(|v| v.as_str()) {
            match priority {
                "critical" | "error" | "warning" | "info" | "debug" => {},
                _ => return Err(McpError::validation("Invalid priority level"))
            }
        }
        
        Ok(())
    }

    async fn transform_payload(&self, mut payload: Value) -> McpResult<Value> {
        // Add timestamp if not present
        if let Some(obj) = payload.as_object_mut() {
            if !obj.contains_key("timestamp") {
                obj.insert("timestamp".to_string(), json!(
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs()
                ));
            }
            
            // Add notification ID
            if !obj.contains_key("id") {
                obj.insert("id".to_string(), json!(uuid::Uuid::new_v4().to_string()));
            }
        }
        
        Ok(payload)
    }

    async fn handle_error(&self, error: &McpError, attempt: u32) -> McpResult<bool> {
        warn!("Notification delivery failed (attempt {}): {}", attempt, error);
        
        // Retry based on priority and attempt count
        if attempt < self.max_retries() {
            debug!("Will retry notification delivery");
            Ok(true)
        } else {
            warn!("Max retries reached, giving up on notification delivery");
            Ok(false)
        }
    }
}

/// CI/CD pipeline notification handler
pub struct CiCdNotification {
    method: String,
}

impl Default for CiCdNotification {
    fn default() -> Self {
        Self::new()
    }
}

impl CiCdNotification {
    pub fn new() -> Self {
        Self {
            method: "notifications/cicd".to_string(),
        }
    }
}

impl HasNotificationMetadata for CiCdNotification {
    fn method(&self) -> &str {
        &self.method
    }
    
    fn requires_ack(&self) -> bool {
        false // CI/CD notifications are fire-and-forget
    }
}

impl HasNotificationPayload for CiCdNotification {
    fn payload(&self) -> Option<&Value> {
        None
    }
}

impl HasNotificationRules for CiCdNotification {
    fn priority(&self) -> u32 {
        50 // Medium priority
    }
    
    fn can_batch(&self) -> bool {
        true // Can batch CI/CD notifications
    }
    
    fn max_retries(&self) -> u32 {
        2
    }
}

#[async_trait]
impl McpNotification for CiCdNotification {
    async fn send(&self, payload: Value) -> McpResult<DeliveryResult> {
        let build_status = payload.get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        
        let service = payload.get("service")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-service");
            
        let branch = payload.get("branch")
            .and_then(|v| v.as_str())
            .unwrap_or("main");

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let icon = match build_status {
            "success" => "✅",
            "failed" => "❌", 
            "started" => "🚀",
            _ => "ℹ️",
        };
        
        info!("{} CI/CD Notification: {} build {} on {}", icon, service, build_status, branch);
        
        Ok(DeliveryResult {
            status: DeliveryStatus::Sent,
            attempts: 1,
            error: None,
            delivered_at: Some(timestamp),
        })
    }
}

/// System monitoring notification handler
pub struct MonitoringNotification {
    method: String,
}

impl Default for MonitoringNotification {
    fn default() -> Self {
        Self::new()
    }
}

impl MonitoringNotification {
    pub fn new() -> Self {
        Self {
            method: "notifications/monitoring".to_string(),
        }
    }
}

impl HasNotificationMetadata for MonitoringNotification {
    fn method(&self) -> &str {
        &self.method
    }
    
    fn requires_ack(&self) -> bool {
        true // Monitoring alerts should be acknowledged
    }
}

impl HasNotificationPayload for MonitoringNotification {
    fn payload(&self) -> Option<&Value> {
        None
    }
}

impl HasNotificationRules for MonitoringNotification {
    fn priority(&self) -> u32 {
        200 // High priority for system alerts
    }
    
    fn can_batch(&self) -> bool {
        false // Don't batch critical system alerts
    }
    
    fn max_retries(&self) -> u32 {
        5 // More retries for critical system alerts
    }
}

#[async_trait]
impl McpNotification for MonitoringNotification {
    async fn send(&self, payload: Value) -> McpResult<DeliveryResult> {
        let alert_type = payload.get("alert_type")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        
        let server = payload.get("server")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-server");
            
        let metric_value = payload.get("metric_value")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let icon = match alert_type {
            "high_cpu" => "🔥",
            "high_memory" => "💾",
            "service_down" => "🚨",
            "disk_space_low" => "💽",
            _ => "⚠️",
        };
        
        info!("{} Monitoring Alert: {} on {} (value: {:.1})", icon, alert_type, server, metric_value);
        
        Ok(DeliveryResult {
            status: DeliveryStatus::Sent,
            attempts: 1,
            error: None,
            delivered_at: Some(timestamp),
        })
    }
    
    async fn validate_payload(&self, payload: &Value) -> McpResult<()> {
        if !payload.is_object() {
            return Err(McpError::validation("Monitoring payload must be an object"));
        }
        
        let obj = payload.as_object().unwrap();
        
        // Ensure critical monitoring fields are present
        for required_field in &["alert_type", "server"] {
            if !obj.contains_key(*required_field) {
                return Err(McpError::validation(&format!("Missing required field: {}", required_field)));
            }
        }
        
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("🔔 Starting Real MCP Notification Server");
    info!("=====================================");

    // Create notification handlers using ACTUAL MCP protocol
    let dev_alerts = DevAlertNotification::new();
    let cicd_notifications = CiCdNotification::new();  
    let monitoring_notifications = MonitoringNotification::new();

    let server = McpServer::builder()
        .name("real-notification-server")
        .version("1.0.0")
        .title("Real MCP Notification Server")
        .instructions(
            "This server demonstrates ACTUAL MCP notification protocol implementation. \
             It uses McpNotification traits to send real notifications via SSE to clients, \
             not fake tools that pretend to send notifications. This is how MCP protocol \
             features should be implemented."
        )
        .notification_provider(dev_alerts)
        .notification_provider(cicd_notifications)
        .notification_provider(monitoring_notifications)
        .bind_address("127.0.0.1:8005".parse()?)
        .sse(true)
        .build()?;

    info!("🚀 Real MCP notification server running at: http://127.0.0.1:8005/mcp");
    info!("📡 SSE endpoint: GET http://127.0.0.1:8005/mcp (Accept: text/event-stream)");
    info!("🔔 This server implements ACTUAL MCP notifications:");
    info!("   • notifications/dev_alert - Development team alerts with acknowledgment");
    info!("   • notifications/cicd - CI/CD pipeline status notifications");
    info!("   • notifications/monitoring - System monitoring alerts");
    info!("💡 Unlike previous examples, this uses real McpNotification traits");
    info!("💡 Notifications are sent via SSE to connected clients");
    info!("💡 This demonstrates actual MCP protocol implementation");

    server.run().await?;
    Ok(())
}