//! Test Helpers Module for MCP Framework Integration Tests
//!
//! This module provides shared utilities and builders for creating SessionContext
//! instances and other test infrastructure needed across integration tests.
//!
//! The primary goal is to enable proper SessionContext testing while maintaining
//! simplicity and avoiding the complexity of full server setup.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use async_trait::async_trait;
use uuid::Uuid;

use turul_mcp_server::SessionContext;
use turul_mcp_session_storage::SessionStorageError;
use turul_mcp_session_storage::{InMemorySessionStorage, SessionStorage};
use turul_mcp_protocol::ServerCapabilities;
use turul_mcp_json_rpc_server::{JsonRpcNotification, JsonRpcVersion, RequestParams};
use turul_http_mcp_server::notification_bridge::{NotificationBroadcaster, BroadcastError};
use turul_mcp_protocol::notifications::{
    ProgressNotification, LoggingMessageNotification, ResourceUpdatedNotification,
    ResourceListChangedNotification, ToolListChangedNotification,
    PromptListChangedNotification, CancelledNotification
};

/// Test notification broadcaster that collects notifications for verification
/// instead of sending them over SSE streams
#[derive(Default)]
pub struct TestNotificationBroadcaster {
    notifications: Arc<Mutex<Vec<(String, JsonRpcNotification)>>>,
}

#[allow(dead_code)]
impl TestNotificationBroadcaster {
    pub fn new() -> Self {
        Self {
            notifications: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Get all notifications for a specific session
    #[allow(dead_code)]
    pub fn get_notifications(&self, session_id: &str) -> Vec<JsonRpcNotification> {
        self.notifications
            .lock()
            .unwrap()
            .iter()
            .filter(|(sid, _)| sid == session_id)
            .map(|(_, notification)| notification.clone())
            .collect()
    }

    /// Get all notifications across all sessions
    pub fn get_all_notifications(&self) -> Vec<(String, JsonRpcNotification)> {
        self.notifications.lock().unwrap().clone()
    }

    /// Clear all collected notifications
    pub fn clear_notifications(&self) {
        self.notifications.lock().unwrap().clear();
    }

    /// Count notifications for a session
    pub fn count_notifications(&self, session_id: &str) -> usize {
        self.notifications
            .lock()
            .unwrap()
            .iter()
            .filter(|(sid, _)| sid == session_id)
            .count()
    }

    /// Check if a specific notification method was sent to a session
    pub fn has_notification(&self, session_id: &str, method: &str) -> bool {
        self.notifications
            .lock()
            .unwrap()
            .iter()
            .any(|(sid, notification)| sid == session_id && notification.method == method)
    }

    /// Helper to store notification
    fn store_notification(&self, session_id: &str, notification: JsonRpcNotification) {
        self.notifications
            .lock()
            .unwrap()
            .push((session_id.to_string(), notification));
    }
}

#[async_trait]
impl NotificationBroadcaster for TestNotificationBroadcaster {
    async fn send_progress_notification(
        &self,
        session_id: &str,
        notification: ProgressNotification,
    ) -> Result<(), BroadcastError> {
        let json_rpc = JsonRpcNotification {
            version: JsonRpcVersion::V2_0,
            method: "notifications/progress".to_string(),
            params: Some(RequestParams::Object({
                let val = serde_json::to_value(notification)?;
                match val {
                    serde_json::Value::Object(map) => map.into_iter().collect(),
                    _ => std::collections::HashMap::new(),
                }
            })),
        };
        self.store_notification(session_id, json_rpc);
        Ok(())
    }

    async fn send_message_notification(
        &self,
        session_id: &str,
        notification: LoggingMessageNotification,
    ) -> Result<(), BroadcastError> {
        let json_rpc = JsonRpcNotification {
            version: JsonRpcVersion::V2_0,
            method: "notifications/message".to_string(),
            params: Some(RequestParams::Object({
                let val = serde_json::to_value(notification)?;
                match val {
                    serde_json::Value::Object(map) => map.into_iter().collect(),
                    _ => std::collections::HashMap::new(),
                }
            })),
        };
        self.store_notification(session_id, json_rpc);
        Ok(())
    }

    async fn send_resource_updated_notification(
        &self,
        session_id: &str,
        notification: ResourceUpdatedNotification,
    ) -> Result<(), BroadcastError> {
        let json_rpc = JsonRpcNotification {
            version: JsonRpcVersion::V2_0,
            method: "notifications/resources/updated".to_string(),
            params: Some(RequestParams::Object({
                let val = serde_json::to_value(notification)?;
                match val {
                    serde_json::Value::Object(map) => map.into_iter().collect(),
                    _ => std::collections::HashMap::new(),
                }
            })),
        };
        self.store_notification(session_id, json_rpc);
        Ok(())
    }

    async fn send_resource_list_changed_notification(
        &self,
        session_id: &str,
        notification: ResourceListChangedNotification,
    ) -> Result<(), BroadcastError> {
        let json_rpc = JsonRpcNotification {
            version: JsonRpcVersion::V2_0,
            method: "notifications/resources/listChanged".to_string(),
            params: Some(RequestParams::Object({
                let val = serde_json::to_value(notification)?;
                match val {
                    serde_json::Value::Object(map) => map.into_iter().collect(),
                    _ => std::collections::HashMap::new(),
                }
            })),
        };
        self.store_notification(session_id, json_rpc);
        Ok(())
    }

    async fn send_tool_list_changed_notification(
        &self,
        session_id: &str,
        notification: ToolListChangedNotification,
    ) -> Result<(), BroadcastError> {
        let json_rpc = JsonRpcNotification {
            version: JsonRpcVersion::V2_0,
            method: "notifications/tools/listChanged".to_string(),
            params: Some(RequestParams::Object({
                let val = serde_json::to_value(notification)?;
                match val {
                    serde_json::Value::Object(map) => map.into_iter().collect(),
                    _ => std::collections::HashMap::new(),
                }
            })),
        };
        self.store_notification(session_id, json_rpc);
        Ok(())
    }

    async fn send_prompt_list_changed_notification(
        &self,
        session_id: &str,
        notification: PromptListChangedNotification,
    ) -> Result<(), BroadcastError> {
        let json_rpc = JsonRpcNotification {
            version: JsonRpcVersion::V2_0,
            method: "notifications/prompts/listChanged".to_string(),
            params: Some(RequestParams::Object({
                let val = serde_json::to_value(notification)?;
                match val {
                    serde_json::Value::Object(map) => map.into_iter().collect(),
                    _ => std::collections::HashMap::new(),
                }
            })),
        };
        self.store_notification(session_id, json_rpc);
        Ok(())
    }

    async fn send_cancelled_notification(
        &self,
        session_id: &str,
        notification: CancelledNotification,
    ) -> Result<(), BroadcastError> {
        let json_rpc = JsonRpcNotification {
            version: JsonRpcVersion::V2_0,
            method: "notifications/cancelled".to_string(),
            params: Some(RequestParams::Object({
                let val = serde_json::to_value(notification)?;
                match val {
                    serde_json::Value::Object(map) => map.into_iter().collect(),
                    _ => std::collections::HashMap::new(),
                }
            })),
        };
        self.store_notification(session_id, json_rpc);
        Ok(())
    }

    /// Broadcast to all sessions - for tests, just collect without specific targeting
    async fn broadcast_to_all_sessions(&self, notification: JsonRpcNotification) -> Result<Vec<String>, BroadcastError> {
        self.notifications.lock().unwrap().push(("*".to_string(), notification));
        Ok(vec!["*".to_string()])
    }

    /// Send generic JSON-RPC notification
    async fn send_notification(
        &self,
        session_id: &str,
        notification: JsonRpcNotification,
    ) -> Result<(), BroadcastError> {
        self.store_notification(session_id, notification);
        Ok(())
    }
}

/// Builder for creating SessionContext instances in tests
pub struct TestSessionBuilder {
    storage: Arc<InMemorySessionStorage>,
    broadcaster: Arc<TestNotificationBroadcaster>,
}

#[allow(dead_code)]
impl TestSessionBuilder {
    /// Create a new TestSessionBuilder with default components
    pub fn new() -> Self {
        Self {
            storage: Arc::new(InMemorySessionStorage::new()),
            broadcaster: Arc::new(TestNotificationBroadcaster::new()),
        }
    }

    /// Build a SessionContext with working state management and notification support
    pub async fn build_session_context(&self) -> SessionContext {
        let session_id = Uuid::now_v7().to_string();
        self.build_session_context_with_id(session_id).await
    }

    /// Build a SessionContext with a specific session ID
    pub async fn build_session_context_with_id(&self, session_id: String) -> SessionContext {
        // Create the session in storage first
        match self.storage.create_session_with_id(
            session_id.clone(),
            ServerCapabilities::default()
        ).await {
            Ok(_) => {},
            Err(e) => panic!("Failed to create session in storage: {}", e),
        }

        // Create JSON-RPC SessionContext
        let json_rpc_ctx = turul_mcp_json_rpc_server::SessionContext {
            session_id: session_id.clone(),
            metadata: HashMap::new(),
            broadcaster: Some(Arc::new(self.broadcaster.clone()) as Arc<dyn std::any::Any + Send + Sync>),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        };

        // Convert to MCP SessionContext with storage integration
        SessionContext::from_json_rpc_with_broadcaster(
            json_rpc_ctx,
            self.storage.clone() as Arc<dyn SessionStorage<Error = SessionStorageError>>,
        )
    }

    /// Build two independent SessionContext instances for testing cross-session scenarios
    pub async fn build_session_pair(&self) -> (SessionContext, SessionContext) {
        let session1 = self.build_session_context().await;
        let session2 = self.build_session_context().await;
        (session1, session2)
    }

    /// Get reference to the notification broadcaster for verification
    pub fn broadcaster(&self) -> &TestNotificationBroadcaster {
        &self.broadcaster
    }

    /// Get reference to the storage backend
    pub fn storage(&self) -> &InMemorySessionStorage {
        &self.storage
    }
}

impl Default for TestSessionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience function to create a SessionContext for simple tests
pub async fn create_test_session() -> SessionContext {
    TestSessionBuilder::new().build_session_context().await
}

/// Convenience function to create two SessionContext instances
#[allow(dead_code)]
pub async fn create_test_session_pair() -> (SessionContext, SessionContext) {
    TestSessionBuilder::new().build_session_pair().await
}

/// Helper function to verify session state contains expected value
#[allow(dead_code)]
pub async fn assert_session_state<T>(session: &SessionContext, key: &str, expected: T) 
where 
    T: serde::de::DeserializeOwned + PartialEq + std::fmt::Debug,
{
    let actual: Option<T> = session.get_typed_state(key).await;
    assert_eq!(actual, Some(expected), "Session state mismatch for key '{}'", key);
}

/// Helper function to verify a notification was sent to a session
#[allow(dead_code)]
pub fn assert_notification_sent(broadcaster: &TestNotificationBroadcaster, session_id: &str, method: &str) {
    assert!(
        broadcaster.has_notification(session_id, method),
        "Expected notification '{}' not found for session '{}'", 
        method, session_id
    );
}
