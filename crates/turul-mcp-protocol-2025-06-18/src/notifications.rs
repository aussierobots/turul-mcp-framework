//! MCP Notifications Protocol Types
//!
//! This module defines types for notifications in MCP according to the 2025-06-18 specification.
//! MCP notifications are JSON-RPC notifications that inform clients about server state changes.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::logging::LoggingLevel;
use turul_mcp_json_rpc_server::types::RequestId;

/// Base notification parameters that can include _meta
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationParams {
    /// Optional MCP meta information
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<HashMap<String, Value>>,
    /// All other notification-specific parameters
    #[serde(flatten)]
    pub other: HashMap<String, Value>,
}

impl NotificationParams {
    pub fn new() -> Self {
        Self {
            meta: None,
            other: HashMap::new(),
        }
    }

    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.meta = Some(meta);
        self
    }

    pub fn with_param(mut self, key: impl Into<String>, value: Value) -> Self {
        self.other.insert(key.into(), value);
        self
    }
}

/// Base notification structure following MCP TypeScript specification
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Notification {
    /// Notification method
    pub method: String,
    /// Optional notification parameters with _meta support
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<NotificationParams>,
}

impl Notification {
    pub fn new(method: impl Into<String>) -> Self {
        Self {
            method: method.into(),
            params: None,
        }
    }

    pub fn with_params(mut self, params: NotificationParams) -> Self {
        self.params = Some(params);
        self
    }
}

// ==== Specific Notification Types Following MCP Specification ====

/// Method: "notifications/resources/list_changed" (per MCP spec)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceListChangedNotification {
    /// Method name (always "notifications/resources/list_changed")
    pub method: String,
    /// Optional empty params with _meta support
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<NotificationParams>,
}

impl ResourceListChangedNotification {
    pub fn new() -> Self {
        Self {
            method: "notifications/resources/list_changed".to_string(),
            params: None,
        }
    }

    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.params = Some(NotificationParams::new().with_meta(meta));
        self
    }
}

/// Method: "notifications/tools/list_changed" (per MCP spec)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolListChangedNotification {
    /// Method name (always "notifications/tools/list_changed")
    pub method: String,
    /// Optional empty params with _meta support
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<NotificationParams>,
}

impl ToolListChangedNotification {
    pub fn new() -> Self {
        Self {
            method: "notifications/tools/list_changed".to_string(),
            params: None,
        }
    }

    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.params = Some(NotificationParams::new().with_meta(meta));
        self
    }
}

/// Method: "notifications/prompts/list_changed" (per MCP spec)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptListChangedNotification {
    /// Method name (always "notifications/prompts/list_changed")
    pub method: String,
    /// Optional empty params with _meta support
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<NotificationParams>,
}

impl PromptListChangedNotification {
    pub fn new() -> Self {
        Self {
            method: "notifications/prompts/list_changed".to_string(),
            params: None,
        }
    }

    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.params = Some(NotificationParams::new().with_meta(meta));
        self
    }
}

/// Method: "notifications/roots/list_changed" (per MCP spec)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RootsListChangedNotification {
    /// Method name (always "notifications/roots/list_changed")
    pub method: String,
    /// Optional empty params with _meta support
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<NotificationParams>,
}

impl RootsListChangedNotification {
    pub fn new() -> Self {
        Self {
            method: "notifications/roots/list_changed".to_string(),
            params: None,
        }
    }

    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.params = Some(NotificationParams::new().with_meta(meta));
        self
    }
}

/// Method: "notifications/progress"
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressNotification {
    /// Method name (always "notifications/progress")
    pub method: String,
    /// Progress parameters
    pub params: ProgressNotificationParams,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressNotificationParams {
    /// Token to correlate with the original request
    pub progress_token: String,
    /// Amount of work completed so far
    pub progress: u64,
    /// Optional total work count
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,
    /// Optional human-readable message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Optional MCP meta information
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<HashMap<String, Value>>,
}

impl ProgressNotification {
    pub fn new(progress_token: impl Into<String>, progress: u64) -> Self {
        Self {
            method: "notifications/progress".to_string(),
            params: ProgressNotificationParams {
                progress_token: progress_token.into(),
                progress,
                total: None,
                message: None,
                meta: None,
            },
        }
    }

    pub fn with_total(mut self, total: u64) -> Self {
        self.params.total = Some(total);
        self
    }

    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.params.message = Some(message.into());
        self
    }

    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.params.meta = Some(meta);
        self
    }
}

/// Method: "notifications/resources/updated"
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceUpdatedNotification {
    /// Method name (always "notifications/resources/updated")
    pub method: String,
    /// Parameters with URI and optional _meta
    pub params: ResourceUpdatedNotificationParams,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceUpdatedNotificationParams {
    /// The URI of the resource that was updated
    pub uri: String,
    /// Optional MCP meta information
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<HashMap<String, Value>>,
}

impl ResourceUpdatedNotification {
    pub fn new(uri: impl Into<String>) -> Self {
        Self {
            method: "notifications/resources/updated".to_string(),
            params: ResourceUpdatedNotificationParams {
                uri: uri.into(),
                meta: None,
            },
        }
    }

    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.params.meta = Some(meta);
        self
    }
}

/// Method: "notifications/cancelled"
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelledNotification {
    /// Method name (always "notifications/cancelled")
    pub method: String,
    /// Cancellation parameters
    pub params: CancelledNotificationParams,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelledNotificationParams {
    /// The ID of the request to cancel
    pub request_id: RequestId,
    /// An optional reason for cancelling
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Optional MCP meta information
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<HashMap<String, Value>>,
}

impl CancelledNotification {
    pub fn new(request_id: RequestId) -> Self {
        Self {
            method: "notifications/cancelled".to_string(),
            params: CancelledNotificationParams {
                request_id,
                reason: None,
                meta: None,
            },
        }
    }

    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.params.reason = Some(reason.into());
        self
    }

    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.params.meta = Some(meta);
        self
    }
}

/// Method: "notifications/initialized"
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializedNotification {
    /// Method name (always "notifications/initialized")
    pub method: String,
    /// Optional empty params with _meta support
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<NotificationParams>,
}

impl InitializedNotification {
    pub fn new() -> Self {
        Self {
            method: "notifications/initialized".to_string(),
            params: None,
        }
    }

    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.params = Some(NotificationParams::new().with_meta(meta));
        self
    }
}


/// Method: "notifications/message"
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoggingMessageNotification {
    /// Method name (always "notifications/message")
    pub method: String,
    /// Logging parameters
    pub params: LoggingMessageNotificationParams,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoggingMessageNotificationParams {
    /// Log level
    pub level: LoggingLevel,
    /// Optional logger name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logger: Option<String>,
    /// Log data (per MCP spec - any serializable type)
    pub data: Value,
    /// Optional MCP meta information
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<HashMap<String, Value>>,
}

impl LoggingMessageNotification {
    pub fn new(level: LoggingLevel, data: Value) -> Self {
        Self {
            method: "notifications/message".to_string(),
            params: LoggingMessageNotificationParams {
                level,
                logger: None,
                data,
                meta: None,
            },
        }
    }

    pub fn with_logger(mut self, logger: impl Into<String>) -> Self {
        self.params.logger = Some(logger.into());
        self
    }

    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.params.meta = Some(meta);
        self
    }
}

// ==== Notification Trait Implementations ====

use crate::traits::*;

// Trait implementations for NotificationParams
impl Params for NotificationParams {}

impl HasMetaParam for NotificationParams {
    fn meta(&self) -> Option<&HashMap<String, Value>> {
        self.meta.as_ref()
    }
}

// ===========================================
// === Fine-Grained Notification Traits ===
// ===========================================

/// Trait for notification metadata (method, type info)
pub trait HasNotificationMetadata {
    /// The notification method name
    fn method(&self) -> &str;
    
    /// Optional notification type or category
    fn notification_type(&self) -> Option<&str> {
        None
    }
    
    /// Whether this notification requires acknowledgment
    fn requires_ack(&self) -> bool {
        false
    }
}

/// Trait for notification payload and data structure
pub trait HasNotificationPayload {
    /// Get the notification payload data
    fn payload(&self) -> Option<&Value> {
        None
    }
    
    /// Serialize notification to JSON
    fn serialize_payload(&self) -> Result<String, String> {
        match self.payload() {
            Some(data) => serde_json::to_string(data)
                .map_err(|e| format!("Serialization error: {}", e)),
            None => Ok("{}".to_string()),
        }
    }
}

/// Trait for notification delivery rules and filtering
pub trait HasNotificationRules {
    /// Optional delivery priority (higher = more important)
    fn priority(&self) -> u32 {
        0
    }
    
    /// Whether this notification can be batched with others
    fn can_batch(&self) -> bool {
        true
    }
    
    /// Maximum retry attempts for delivery
    fn max_retries(&self) -> u32 {
        3
    }
    
    /// Check if notification should be delivered
    fn should_deliver(&self) -> bool {
        true
    }
}

/// Composed notification definition trait (automatically implemented via blanket impl)
pub trait NotificationDefinition: 
    HasNotificationMetadata + 
    HasNotificationPayload + 
    HasNotificationRules 
{
    /// Convert this notification definition to a base Notification
    fn to_notification(&self) -> Notification {
        let mut notification = Notification::new(self.method());
        if let Some(payload) = self.payload() {
            let mut params = NotificationParams::new();
            // Add payload data to params.other
            if let Ok(obj) = serde_json::from_value::<HashMap<String, Value>>(payload.clone()) {
                params.other = obj;
            }
            notification = notification.with_params(params);
        }
        notification
    }
    
    /// Validate this notification
    fn validate(&self) -> Result<(), String> {
        if self.method().is_empty() {
            return Err("Notification method cannot be empty".to_string());
        }
        if !self.method().starts_with("notifications/") {
            return Err("Notification method must start with 'notifications/'".to_string());
        }
        Ok(())
    }
}

// Blanket implementation: any type implementing the fine-grained traits automatically gets NotificationDefinition
impl<T> NotificationDefinition for T 
where 
    T: HasNotificationMetadata + HasNotificationPayload + HasNotificationRules 
{}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_resource_list_changed() {
        let notification = ResourceListChangedNotification::new();
        assert_eq!(notification.method, "notifications/resources/list_changed");
    }

    #[test]
    fn test_tool_list_changed() {
        let notification = ToolListChangedNotification::new();
        assert_eq!(notification.method, "notifications/tools/list_changed");
    }

    #[test]
    fn test_prompt_list_changed() {
        let notification = PromptListChangedNotification::new();
        assert_eq!(notification.method, "notifications/prompts/list_changed");
    }

    #[test]
    fn test_roots_list_changed() {
        let notification = RootsListChangedNotification::new();
        assert_eq!(notification.method, "notifications/roots/list_changed");
    }

    #[test]
    fn test_progress_notification() {
        let notification = ProgressNotification::new("token123", 50)
            .with_total(100)
            .with_message("Processing...");
        
        assert_eq!(notification.method, "notifications/progress");
        assert_eq!(notification.params.progress_token, "token123");
        assert_eq!(notification.params.progress, 50);
        assert_eq!(notification.params.total, Some(100));
        assert_eq!(notification.params.message, Some("Processing...".to_string()));
    }

    #[test]
    fn test_resource_updated() {
        let notification = ResourceUpdatedNotification::new("file:///test.txt");
        assert_eq!(notification.method, "notifications/resources/updated");
        assert_eq!(notification.params.uri, "file:///test.txt");
    }

    #[test]
    fn test_cancelled_notification() {
        use turul_mcp_json_rpc_server::types::RequestId;
        let notification = CancelledNotification::new(RequestId::Number(123))
            .with_reason("User cancelled");
        
        assert_eq!(notification.method, "notifications/cancelled");
        assert_eq!(notification.params.request_id, RequestId::Number(123));
        assert_eq!(notification.params.reason, Some("User cancelled".to_string()));
    }

    #[test]
    fn test_initialized_notification() {
        let notification = InitializedNotification::new();
        assert_eq!(notification.method, "notifications/initialized");
    }

    #[test]
    fn test_logging_message_notification() {
        use crate::logging::LoggingLevel;
        let data = json!({"message": "Test log message", "context": "test"});
        let notification = LoggingMessageNotification::new(LoggingLevel::Info, data.clone())
            .with_logger("test-logger");
        
        assert_eq!(notification.method, "notifications/message");
        assert_eq!(notification.params.level, LoggingLevel::Info);
        assert_eq!(notification.params.logger, Some("test-logger".to_string()));
        assert_eq!(notification.params.data, data);
    }

    #[test]
    fn test_serialization() {
        let notification = InitializedNotification::new();
        let json = serde_json::to_string(&notification).unwrap();
        assert!(json.contains("notifications/initialized"));
        
        let parsed: InitializedNotification = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.method, "notifications/initialized");
    }
}