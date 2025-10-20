//! Framework traits for MCP notification construction
//!
//! **IMPORTANT**: These are framework features, NOT part of the MCP specification.

use turul_mcp_protocol::notifications::{Notification, NotificationParams};
use serde_json::Value;
use std::collections::HashMap;

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
    /// Get the notification payload data (owned Value for computed serialization)
    fn payload(&self) -> Option<Value> {
        None
    }

    /// Serialize notification to JSON
    fn serialize_payload(&self) -> Result<String, String> {
        match self.payload() {
            Some(data) => {
                serde_json::to_string(&data).map_err(|e| format!("Serialization error: {}", e))
            }
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

/// **Complete MCP Notification Creation** - Build real-time event broadcasting systems.
///
/// This trait represents a **complete, working MCP notification** that can broadcast
/// real-time events to connected clients with structured payloads and routing rules.
/// When you implement the required metadata traits, you automatically get
/// `NotificationDefinition` for free via blanket implementation.
///
/// # What You're Building
///
/// A notification is a real-time event system that:
/// - Broadcasts events to connected clients instantly
/// - Carries structured JSON payloads with event data
/// - Supports routing rules for targeted delivery
/// - Provides reliable event ordering and delivery
///
/// # How to Create a Notification
///
/// Implement these three traits on your struct:
///
/// ```rust
/// # use turul_mcp_protocol::notifications::*;
/// # use turul_mcp_builders::prelude::*;
/// # use serde_json::{Value, json};
///
/// // This struct will automatically implement NotificationDefinition!
/// struct FileChangeNotification {
///     file_path: String,
///     change_type: String,
///     payload_data: Value,
/// }
///
/// impl FileChangeNotification {
///     fn new(file_path: String, change_type: String) -> Self {
///         let payload_data = json!({
///             "path": file_path,
///             "type": change_type,
///             "timestamp": "2024-01-01T00:00:00Z"
///         });
///         Self { file_path, change_type, payload_data }
///     }
/// }
///
/// impl HasNotificationMetadata for FileChangeNotification {
///     fn method(&self) -> &str {
///         "file/changed"
///     }
/// }
///
/// impl HasNotificationPayload for FileChangeNotification {
///     fn payload(&self) -> Option<Value> {
///         Some(self.payload_data.clone())
///     }
/// }
///
/// impl HasNotificationRules for FileChangeNotification {
///     fn priority(&self) -> u32 {
///         1
///     }
/// }
///
/// // Now you can use it with the server:
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let notification = FileChangeNotification::new(
///     "/workspace/src/main.rs".to_string(),
///     "modified".to_string(),
/// );
///
/// // The notification automatically implements NotificationDefinition
/// let base_notification = notification.to_notification();
/// # Ok(())
/// # }
/// ```
///
/// # Key Benefits
///
/// - **Real-Time**: Instant event delivery to connected clients
/// - **Structured Data**: JSON payloads for rich event information
/// - **Targeted Delivery**: Client-specific routing rules
/// - **MCP Compliant**: Fully compatible with MCP 2025-06-18 specification
///
/// # Common Use Cases
///
/// - File system watch notifications
/// - Database change events
/// - User activity broadcasts
/// - System status updates
/// - Real-time collaboration events
pub trait NotificationDefinition:
    HasNotificationMetadata + HasNotificationPayload + HasNotificationRules
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
impl<T> NotificationDefinition for T where
    T: HasNotificationMetadata + HasNotificationPayload + HasNotificationRules
{
}
