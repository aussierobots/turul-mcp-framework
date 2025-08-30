//! Notification Builder for Runtime Notification Creation
//!
//! This module provides builder patterns for creating MCP notifications at runtime.
//! Supports all standard MCP notification types and custom notifications.

use serde_json::Value;
use std::collections::HashMap;

// Import from protocol via alias
use turul_mcp_json_rpc_server::types::RequestId;
use turul_mcp_protocol::logging::LoggingLevel;
use turul_mcp_protocol::notifications::{
    CancelledNotification, HasNotificationMetadata, HasNotificationPayload, HasNotificationRules,
    InitializedNotification, LoggingMessageNotification, Notification, NotificationParams,
    ProgressNotification, PromptListChangedNotification, ResourceListChangedNotification,
    ResourceUpdatedNotification, RootsListChangedNotification, ToolListChangedNotification,
};

/// Builder for creating notifications at runtime
pub struct NotificationBuilder {
    method: String,
    params: Option<NotificationParams>,
    priority: u32,
    can_batch: bool,
    max_retries: u32,
}

impl NotificationBuilder {
    /// Create a new notification builder with the given method
    pub fn new(method: impl Into<String>) -> Self {
        Self {
            method: method.into(),
            params: None,
            priority: 0,
            can_batch: true,
            max_retries: 3,
        }
    }

    /// Set notification parameters
    pub fn params(mut self, params: NotificationParams) -> Self {
        self.params = Some(params);
        self
    }

    /// Add a parameter to the notification
    pub fn param(mut self, key: impl Into<String>, value: Value) -> Self {
        if self.params.is_none() {
            self.params = Some(NotificationParams::new());
        }
        self.params
            .as_mut()
            .unwrap()
            .other
            .insert(key.into(), value);
        self
    }

    /// Set meta information
    pub fn meta(mut self, meta: HashMap<String, Value>) -> Self {
        if self.params.is_none() {
            self.params = Some(NotificationParams::new());
        }
        self.params.as_mut().unwrap().meta = Some(meta);
        self
    }

    /// Add a meta key-value pair
    pub fn meta_value(mut self, key: impl Into<String>, value: Value) -> Self {
        if self.params.is_none() {
            self.params = Some(NotificationParams::new());
        }
        let params = self.params.as_mut().unwrap();
        if params.meta.is_none() {
            params.meta = Some(HashMap::new());
        }
        params.meta.as_mut().unwrap().insert(key.into(), value);
        self
    }

    /// Set notification priority (higher = more important)
    pub fn priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    /// Set whether this notification can be batched with others
    pub fn can_batch(mut self, can_batch: bool) -> Self {
        self.can_batch = can_batch;
        self
    }

    /// Set maximum retry attempts
    pub fn max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// Build the notification
    pub fn build(self) -> Notification {
        let mut notification = Notification::new(self.method);
        if let Some(params) = self.params {
            notification = notification.with_params(params);
        }
        notification
    }

    /// Build a dynamic notification that implements the definition traits
    pub fn build_dynamic(self) -> DynamicNotification {
        DynamicNotification {
            method: self.method,
            params: self.params,
            priority: self.priority,
            can_batch: self.can_batch,
            max_retries: self.max_retries,
        }
    }
}

/// Dynamic notification created by NotificationBuilder
#[derive(Debug)]
pub struct DynamicNotification {
    method: String,
    #[allow(dead_code)]
    params: Option<NotificationParams>,
    priority: u32,
    can_batch: bool,
    max_retries: u32,
}

// Implement all fine-grained traits for DynamicNotification
impl HasNotificationMetadata for DynamicNotification {
    fn method(&self) -> &str {
        &self.method
    }

    fn requires_ack(&self) -> bool {
        // High priority notifications might require acknowledgment
        self.priority >= 5
    }
}

impl HasNotificationPayload for DynamicNotification {
    fn payload(&self) -> Option<&Value> {
        // Convert params to a single Value if needed
        None // For simplicity, custom payloads can be added via trait extension
    }
}

impl HasNotificationRules for DynamicNotification {
    fn priority(&self) -> u32 {
        self.priority
    }

    fn can_batch(&self) -> bool {
        self.can_batch
    }

    fn max_retries(&self) -> u32 {
        self.max_retries
    }
}

// NotificationDefinition is automatically implemented via blanket impl!

/// Builder for progress notifications
pub struct ProgressNotificationBuilder {
    progress_token: String,
    progress: u64,
    total: Option<u64>,
    message: Option<String>,
    meta: Option<HashMap<String, Value>>,
}

impl ProgressNotificationBuilder {
    pub fn new(progress_token: impl Into<String>, progress: u64) -> Self {
        Self {
            progress_token: progress_token.into(),
            progress,
            total: None,
            message: None,
            meta: None,
        }
    }

    /// Set total work amount
    pub fn total(mut self, total: u64) -> Self {
        self.total = Some(total);
        self
    }

    /// Set progress message
    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Set meta information
    pub fn meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.meta = Some(meta);
        self
    }

    /// Add a meta key-value pair
    pub fn meta_value(mut self, key: impl Into<String>, value: Value) -> Self {
        if self.meta.is_none() {
            self.meta = Some(HashMap::new());
        }
        self.meta.as_mut().unwrap().insert(key.into(), value);
        self
    }

    /// Build the progress notification
    pub fn build(self) -> ProgressNotification {
        let mut notification = ProgressNotification::new(self.progress_token, self.progress);
        if let Some(total) = self.total {
            notification = notification.with_total(total);
        }
        if let Some(message) = self.message {
            notification = notification.with_message(message);
        }
        if let Some(meta) = self.meta {
            notification = notification.with_meta(meta);
        }
        notification
    }
}

/// Builder for resource updated notifications
pub struct ResourceUpdatedNotificationBuilder {
    uri: String,
    meta: Option<HashMap<String, Value>>,
}

impl ResourceUpdatedNotificationBuilder {
    pub fn new(uri: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
            meta: None,
        }
    }

    /// Set meta information
    pub fn meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.meta = Some(meta);
        self
    }

    /// Add a meta key-value pair
    pub fn meta_value(mut self, key: impl Into<String>, value: Value) -> Self {
        if self.meta.is_none() {
            self.meta = Some(HashMap::new());
        }
        self.meta.as_mut().unwrap().insert(key.into(), value);
        self
    }

    /// Build the resource updated notification
    pub fn build(self) -> ResourceUpdatedNotification {
        let mut notification = ResourceUpdatedNotification::new(self.uri);
        if let Some(meta) = self.meta {
            notification = notification.with_meta(meta);
        }
        notification
    }
}

/// Builder for cancelled notifications
pub struct CancelledNotificationBuilder {
    request_id: RequestId,
    reason: Option<String>,
    meta: Option<HashMap<String, Value>>,
}

impl CancelledNotificationBuilder {
    pub fn new(request_id: RequestId) -> Self {
        Self {
            request_id,
            reason: None,
            meta: None,
        }
    }

    /// Set cancellation reason
    pub fn reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }

    /// Set meta information
    pub fn meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.meta = Some(meta);
        self
    }

    /// Add a meta key-value pair
    pub fn meta_value(mut self, key: impl Into<String>, value: Value) -> Self {
        if self.meta.is_none() {
            self.meta = Some(HashMap::new());
        }
        self.meta.as_mut().unwrap().insert(key.into(), value);
        self
    }

    /// Build the cancelled notification
    pub fn build(self) -> CancelledNotification {
        let mut notification = CancelledNotification::new(self.request_id);
        if let Some(reason) = self.reason {
            notification = notification.with_reason(reason);
        }
        if let Some(meta) = self.meta {
            notification = notification.with_meta(meta);
        }
        notification
    }
}

/// Convenience methods for common notification patterns
impl NotificationBuilder {
    /// Create a resource list changed notification
    pub fn resource_list_changed() -> ResourceListChangedNotification {
        ResourceListChangedNotification::new()
    }

    /// Create a tool list changed notification
    pub fn tool_list_changed() -> ToolListChangedNotification {
        ToolListChangedNotification::new()
    }

    /// Create a prompt list changed notification
    pub fn prompt_list_changed() -> PromptListChangedNotification {
        PromptListChangedNotification::new()
    }

    /// Create a roots list changed notification
    pub fn roots_list_changed() -> RootsListChangedNotification {
        RootsListChangedNotification::new()
    }

    /// Create an initialized notification
    pub fn initialized() -> InitializedNotification {
        InitializedNotification::new()
    }

    /// Create a progress notification builder
    pub fn progress(
        progress_token: impl Into<String>,
        progress: u64,
    ) -> ProgressNotificationBuilder {
        ProgressNotificationBuilder::new(progress_token, progress)
    }

    /// Create a resource updated notification builder
    pub fn resource_updated(uri: impl Into<String>) -> ResourceUpdatedNotificationBuilder {
        ResourceUpdatedNotificationBuilder::new(uri)
    }

    /// Create a cancelled notification builder
    pub fn cancelled(request_id: RequestId) -> CancelledNotificationBuilder {
        CancelledNotificationBuilder::new(request_id)
    }

    /// Create a logging message notification builder
    pub fn logging_message(level: LoggingLevel, data: Value) -> LoggingMessageNotification {
        LoggingMessageNotification::new(level, data)
    }

    /// Create a custom notification
    pub fn custom(method: impl Into<String>) -> Self {
        Self::new(method)
    }

    /// Create a server-to-client notification
    pub fn server_notification(method: impl Into<String>) -> Self {
        let method = method.into();
        // Ensure it follows MCP notification method pattern
        if !method.starts_with("notifications/") {
            Self::new(format!("notifications/{}", method))
        } else {
            Self::new(method)
        }
    }
}

/// Collection of common notification methods as constants
pub mod methods {
    pub const RESOURCE_LIST_CHANGED: &str = "notifications/resources/list_changed";
    pub const TOOL_LIST_CHANGED: &str = "notifications/tools/list_changed";
    pub const PROMPT_LIST_CHANGED: &str = "notifications/prompts/list_changed";
    pub const ROOTS_LIST_CHANGED: &str = "notifications/roots/list_changed";
    pub const PROGRESS: &str = "notifications/progress";
    pub const RESOURCE_UPDATED: &str = "notifications/resources/updated";
    pub const CANCELLED: &str = "notifications/cancelled";
    pub const INITIALIZED: &str = "notifications/initialized";
    pub const MESSAGE: &str = "notifications/message";
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use turul_mcp_protocol::notifications::NotificationDefinition;

    #[test]
    fn test_notification_builder_basic() {
        let notification = NotificationBuilder::new("notifications/test")
            .param("key1", json!("value1"))
            .param("key2", json!(42))
            .priority(3)
            .can_batch(false)
            .build();

        assert_eq!(notification.method, "notifications/test");
        assert!(notification.params.is_some());

        let params = notification.params.unwrap();
        assert_eq!(params.other.get("key1"), Some(&json!("value1")));
        assert_eq!(params.other.get("key2"), Some(&json!(42)));
    }

    #[test]
    fn test_notification_builder_meta() {
        let mut meta = HashMap::new();
        meta.insert("source".to_string(), json!("test"));
        meta.insert("timestamp".to_string(), json!("2025-01-01T00:00:00Z"));

        let notification = NotificationBuilder::new("notifications/test")
            .meta(meta.clone())
            .build();

        let params = notification.params.expect("Expected params");
        assert_eq!(params.meta, Some(meta));
    }

    #[test]
    fn test_notification_builder_fluent_meta() {
        let notification = NotificationBuilder::new("notifications/test")
            .meta_value("request_id", json!("req-123"))
            .meta_value("user_id", json!("user-456"))
            .build();

        let params = notification.params.expect("Expected params");
        let meta = params.meta.expect("Expected meta");
        assert_eq!(meta.get("request_id"), Some(&json!("req-123")));
        assert_eq!(meta.get("user_id"), Some(&json!("user-456")));
    }

    #[test]
    fn test_progress_notification_builder() {
        let notification = ProgressNotificationBuilder::new("token-123", 75)
            .total(100)
            .message("Processing files...")
            .meta_value("stage", json!("validation"))
            .build();

        assert_eq!(notification.method, "notifications/progress");
        assert_eq!(notification.params.progress_token, "token-123");
        assert_eq!(notification.params.progress, 75);
        assert_eq!(notification.params.total, Some(100));
        assert_eq!(
            notification.params.message,
            Some("Processing files...".to_string())
        );

        let meta = notification.params.meta.expect("Expected meta");
        assert_eq!(meta.get("stage"), Some(&json!("validation")));
    }

    #[test]
    fn test_resource_updated_notification_builder() {
        let notification = ResourceUpdatedNotificationBuilder::new("file:///test.txt")
            .meta_value("change_type", json!("modified"))
            .build();

        assert_eq!(notification.method, "notifications/resources/updated");
        assert_eq!(notification.params.uri, "file:///test.txt");

        let meta = notification.params.meta.expect("Expected meta");
        assert_eq!(meta.get("change_type"), Some(&json!("modified")));
    }

    #[test]
    fn test_cancelled_notification_builder() {
        let notification = CancelledNotificationBuilder::new(RequestId::Number(123))
            .reason("User cancelled operation")
            .meta_value("cancellation_time", json!("2025-01-01T00:00:00Z"))
            .build();

        assert_eq!(notification.method, "notifications/cancelled");
        assert_eq!(notification.params.request_id, RequestId::Number(123));
        assert_eq!(
            notification.params.reason,
            Some("User cancelled operation".to_string())
        );

        let meta = notification.params.meta.expect("Expected meta");
        assert_eq!(
            meta.get("cancellation_time"),
            Some(&json!("2025-01-01T00:00:00Z"))
        );
    }

    #[test]
    fn test_convenience_methods() {
        // Test standard list changed notifications
        let resource_list = NotificationBuilder::resource_list_changed();
        assert_eq!(resource_list.method, "notifications/resources/list_changed");

        let tool_list = NotificationBuilder::tool_list_changed();
        assert_eq!(tool_list.method, "notifications/tools/list_changed");

        let prompt_list = NotificationBuilder::prompt_list_changed();
        assert_eq!(prompt_list.method, "notifications/prompts/list_changed");

        let roots_list = NotificationBuilder::roots_list_changed();
        assert_eq!(roots_list.method, "notifications/roots/list_changed");

        let initialized = NotificationBuilder::initialized();
        assert_eq!(initialized.method, "notifications/initialized");

        // Test logging message
        let logging = NotificationBuilder::logging_message(
            LoggingLevel::Info,
            json!({"message": "Test log"}),
        );
        assert_eq!(logging.method, "notifications/message");
    }

    #[test]
    fn test_custom_notifications() {
        // Custom notification
        let custom = NotificationBuilder::custom("custom/event")
            .param("event_type", json!("user_action"))
            .build();
        assert_eq!(custom.method, "custom/event");

        // Server notification (auto-prefixes)
        let server = NotificationBuilder::server_notification("server/status")
            .param("status", json!("ready"))
            .build();
        assert_eq!(server.method, "notifications/server/status");

        // Already prefixed - should not double-prefix
        let already_prefixed =
            NotificationBuilder::server_notification("notifications/already/prefixed").build();
        assert_eq!(already_prefixed.method, "notifications/already/prefixed");
    }

    #[test]
    fn test_dynamic_notification_traits() {
        let notification = NotificationBuilder::new("notifications/test")
            .priority(7)
            .can_batch(false)
            .max_retries(5)
            .build_dynamic();

        // Test HasNotificationMetadata
        assert_eq!(notification.method(), "notifications/test");
        assert!(notification.requires_ack()); // Priority >= 5

        // Test HasNotificationRules
        assert_eq!(notification.priority(), 7);
        assert!(!notification.can_batch());
        assert_eq!(notification.max_retries(), 5);

        // Test NotificationDefinition (auto-implemented)
        assert!(notification.validate().is_ok());
        let base_notification = notification.to_notification();
        assert_eq!(base_notification.method, "notifications/test");
    }

    #[test]
    fn test_notification_validation() {
        let valid = NotificationBuilder::new("notifications/valid").build_dynamic();
        assert!(valid.validate().is_ok());

        let invalid_empty = NotificationBuilder::new("").build_dynamic();
        assert!(invalid_empty.validate().is_err());

        let invalid_prefix = NotificationBuilder::new("invalid/method").build_dynamic();
        assert!(invalid_prefix.validate().is_err());
    }

    #[test]
    fn test_method_constants() {
        use super::methods::*;

        assert_eq!(
            RESOURCE_LIST_CHANGED,
            "notifications/resources/list_changed"
        );
        assert_eq!(TOOL_LIST_CHANGED, "notifications/tools/list_changed");
        assert_eq!(PROMPT_LIST_CHANGED, "notifications/prompts/list_changed");
        assert_eq!(ROOTS_LIST_CHANGED, "notifications/roots/list_changed");
        assert_eq!(PROGRESS, "notifications/progress");
        assert_eq!(RESOURCE_UPDATED, "notifications/resources/updated");
        assert_eq!(CANCELLED, "notifications/cancelled");
        assert_eq!(INITIALIZED, "notifications/initialized");
        assert_eq!(MESSAGE, "notifications/message");
    }
}
