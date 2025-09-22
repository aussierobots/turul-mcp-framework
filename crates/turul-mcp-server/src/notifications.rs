//! MCP Notifications Trait
//!
//! This module defines the high-level trait for implementing MCP notifications.

use async_trait::async_trait;
use serde_json::Value;
use turul_mcp_protocol::notifications::NotificationDefinition;
use turul_mcp_protocol::{McpResult, notifications::Notification};

/// Notification delivery status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveryStatus {
    Pending,
    Sent,
    Acknowledged,
    Failed,
    Retrying,
}

/// Notification delivery result
#[derive(Debug, Clone)]
pub struct DeliveryResult {
    pub status: DeliveryStatus,
    pub attempts: u32,
    pub error: Option<String>,
    pub delivered_at: Option<u64>, // Unix timestamp
}

/// High-level trait for implementing MCP notifications
///
/// McpNotification extends NotificationDefinition with execution capabilities.
/// All metadata is provided by the NotificationDefinition trait, ensuring
/// consistency between concrete Notification structs and dynamic implementations.
#[async_trait]
pub trait McpNotification: NotificationDefinition + Send + Sync {
    /// Send a notification (per MCP spec)
    ///
    /// This method processes and delivers notifications to clients,
    /// handling serialization, transport, and error recovery.
    async fn send(&self, payload: Value) -> McpResult<DeliveryResult>;

    /// Optional: Check if this notification handler can send the given notification
    ///
    /// This allows for conditional notification handling based on method type,
    /// payload content, or transport availability.
    fn can_send(&self, method: &str) -> bool {
        method == self.method()
    }

    /// Optional: Get notification handler priority
    ///
    /// Higher priority handlers are tried first when multiple handlers
    /// can send the same notification type.
    fn priority(&self) -> u32 {
        0
    }

    /// Optional: Validate the notification payload
    ///
    /// This method can perform validation of notification data before sending.
    async fn validate_payload(&self, payload: &Value) -> McpResult<()> {
        // Basic validation - ensure payload is not null for required fields
        if payload.is_null() && self.requires_ack() {
            return Err(turul_mcp_protocol::McpError::validation(
                "Payload cannot be null for notifications requiring acknowledgment",
            ));
        }
        Ok(())
    }

    /// Optional: Transform payload before sending
    ///
    /// This allows for data enrichment, filtering, or formatting
    /// before the notification is transmitted.
    async fn transform_payload(&self, payload: Value) -> McpResult<Value> {
        Ok(payload)
    }

    /// Optional: Handle notification delivery errors
    ///
    /// This method is called when notification delivery fails, allowing
    /// for retry logic, fallback notifications, or error logging.
    async fn handle_error(
        &self,
        _error: &turul_mcp_protocol::McpError,
        attempt: u32,
    ) -> McpResult<bool> {
        // Default: retry up to max_retries
        Ok(attempt < self.max_retries())
    }

    /// Optional: Batch multiple notifications
    ///
    /// This method can be used to optimize notification delivery by batching
    /// multiple notifications together when supported.
    async fn batch_send(&self, payloads: Vec<Value>) -> McpResult<Vec<DeliveryResult>> {
        // Default: send individually
        let mut results = Vec::new();
        for payload in payloads {
            results.push(self.send(payload).await?);
        }
        Ok(results)
    }

    /// Optional: Subscribe to notification acknowledgments
    ///
    /// This method can be used to track which notifications have been
    /// successfully received and processed by clients.
    async fn on_acknowledged(&self, _delivery_result: &DeliveryResult) -> McpResult<()> {
        // Default: no-op
        Ok(())
    }

    /// Optional: Check delivery status
    ///
    /// This method allows querying the current delivery status of notifications.
    async fn check_status(&self, _notification_id: &str) -> McpResult<DeliveryStatus> {
        // Default: assume sent immediately
        Ok(DeliveryStatus::Sent)
    }
}

/// Convert an McpNotification trait object to a Notification
///
/// This is a convenience function for converting notification definitions
/// to protocol notifications.
pub fn notification_to_protocol(
    notification: &dyn McpNotification,
    payload: Value,
) -> Notification {
    let mut protocol_notification = notification.to_notification();
    // Add payload to params if not already present
    if protocol_notification.params.is_none() {
        use turul_mcp_protocol::notifications::NotificationParams;
        let mut params = NotificationParams::new();
        if let Ok(obj) = serde_json::from_value::<std::collections::HashMap<String, Value>>(payload)
        {
            params.other = obj;
        }
        protocol_notification.params = Some(params);
    }
    protocol_notification
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use turul_mcp_protocol::notifications::{
        HasNotificationMetadata, HasNotificationPayload, HasNotificationRules,
    };

    struct TestNotification {
        method: String,
        payload: Option<Value>,
        priority: u32,
    }

    // Implement fine-grained traits (MCP spec compliant)
    impl HasNotificationMetadata for TestNotification {
        fn method(&self) -> &str {
            &self.method
        }

        fn requires_ack(&self) -> bool {
            self.method.contains("important")
        }
    }

    impl HasNotificationPayload for TestNotification {
        fn payload(&self) -> Option<&Value> {
            self.payload.as_ref()
        }
    }

    impl HasNotificationRules for TestNotification {
        fn priority(&self) -> u32 {
            self.priority
        }

        fn can_batch(&self) -> bool {
            !self.method.contains("urgent")
        }

        fn max_retries(&self) -> u32 {
            if self.method.contains("critical") {
                5
            } else {
                3
            }
        }
    }

    // NotificationDefinition automatically implemented via blanket impl!

    #[async_trait]
    impl McpNotification for TestNotification {
        async fn send(&self, payload: Value) -> McpResult<DeliveryResult> {
            // Simulate notification sending
            println!(
                "Sending notification: {} with payload: {}",
                self.method, payload
            );

            Ok(DeliveryResult {
                status: DeliveryStatus::Sent,
                attempts: 1,
                error: None,
                delivered_at: Some(
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                ),
            })
        }
    }

    #[test]
    fn test_notification_trait() {
        let notification = TestNotification {
            method: "notifications/test".to_string(),
            payload: Some(json!({"data": "test"})),
            priority: 5,
        };

        assert_eq!(notification.method(), "notifications/test");
        assert_eq!(HasNotificationRules::priority(&notification), 5);
        assert!(!notification.requires_ack());
        assert!(notification.can_batch());
        assert_eq!(notification.max_retries(), 3);
    }

    #[tokio::test]
    async fn test_notification_validation() {
        let notification = TestNotification {
            method: "notifications/important/test".to_string(),
            payload: None,
            priority: 0,
        };

        let result = notification.validate_payload(&Value::Null).await;
        assert!(result.is_err()); // Should fail because requires_ack() is true

        let valid_result = notification.validate_payload(&json!({"valid": true})).await;
        assert!(valid_result.is_ok());
    }

    #[tokio::test]
    async fn test_notification_sending() {
        let notification = TestNotification {
            method: "notifications/test".to_string(),
            payload: Some(json!({"test": true})),
            priority: 1,
        };

        let payload = json!({"message": "test notification"});
        let result = notification.send(payload).await.unwrap();

        assert_eq!(result.status, DeliveryStatus::Sent);
        assert_eq!(result.attempts, 1);
        assert!(result.error.is_none());
        assert!(result.delivered_at.is_some());
    }

    #[tokio::test]
    async fn test_batch_sending() {
        let notification = TestNotification {
            method: "notifications/batch_test".to_string(),
            payload: None,
            priority: 2,
        };

        let payloads = vec![
            json!({"id": 1, "data": "first"}),
            json!({"id": 2, "data": "second"}),
            json!({"id": 3, "data": "third"}),
        ];

        let results = notification.batch_send(payloads).await.unwrap();
        assert_eq!(results.len(), 3);

        for result in results {
            assert_eq!(result.status, DeliveryStatus::Sent);
            assert_eq!(result.attempts, 1);
        }
    }

    #[tokio::test]
    async fn test_delivery_status() {
        let notification = TestNotification {
            method: "notifications/status_test".to_string(),
            payload: None,
            priority: 0,
        };

        let status = notification
            .check_status("test-notification-123")
            .await
            .unwrap();
        assert_eq!(status, DeliveryStatus::Sent);
    }
}
