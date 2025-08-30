//! Notification Bridge - Connects NotificationBroadcaster to StreamManager
//!
//! This module provides the critical bridge between the notification system
//! (where tools send events) and the SSE streaming system (where clients receive events).
//!
//! CRITICAL: All notifications MUST use proper MCP JSON-RPC format per specification:
//! {"jsonrpc":"2.0","method":"notifications/{type}","params":{...}}
//!
//! Without this bridge: Tools send notifications → NotificationBroadcaster → VOID
//! With this bridge: Tools send notifications → NotificationBroadcaster → StreamManager → SSE clients ✅

use std::sync::Arc;
use async_trait::async_trait;
use tracing::{info, error};

use turul_mcp_session_storage::SessionStorage;
use turul_mcp_json_rpc_server::JsonRpcNotification;
use turul_mcp_protocol::notifications::{
    ProgressNotification, LoggingMessageNotification, ResourceUpdatedNotification,
    ResourceListChangedNotification, ToolListChangedNotification,
    PromptListChangedNotification, CancelledNotification
};
use crate::StreamManager;

/// MCP-compliant notification broadcaster trait for sending ALL notification types over SSE
///
/// ALL methods send proper JSON-RPC notifications per MCP 2025-06-18 specification
#[async_trait]
pub trait NotificationBroadcaster: Send + Sync {
    // ================== SERVER-TO-CLIENT NOTIFICATIONS ==================

    /// Send a progress notification (notifications/progress)
    /// Used for long-running operations to show progress updates
    async fn send_progress_notification(
        &self,
        session_id: &str,
        notification: ProgressNotification,
    ) -> Result<(), BroadcastError>;

    /// Send a logging message notification (notifications/message)
    /// Used to send log messages with different levels (debug, info, warning, error)
    async fn send_message_notification(
        &self,
        session_id: &str,
        notification: LoggingMessageNotification,
    ) -> Result<(), BroadcastError>;

    /// Send resource updated notification (notifications/resources/updated)
    /// Notifies that a specific resource has been updated
    async fn send_resource_updated_notification(
        &self,
        session_id: &str,
        notification: ResourceUpdatedNotification,
    ) -> Result<(), BroadcastError>;

    /// Send resource list changed notification (notifications/resources/list_changed)
    /// Notifies that the resource list has changed (added/removed resources)
    async fn send_resource_list_changed_notification(
        &self,
        session_id: &str,
        notification: ResourceListChangedNotification,
    ) -> Result<(), BroadcastError>;

    /// Send tool list changed notification (notifications/tools/list_changed)
    /// Notifies that the tool list has changed (added/removed tools)
    async fn send_tool_list_changed_notification(
        &self,
        session_id: &str,
        notification: ToolListChangedNotification,
    ) -> Result<(), BroadcastError>;

    /// Send prompt list changed notification (notifications/prompts/list_changed)
    /// Notifies that the prompt list has changed (added/removed prompts)
    async fn send_prompt_list_changed_notification(
        &self,
        session_id: &str,
        notification: PromptListChangedNotification,
    ) -> Result<(), BroadcastError>;

    // ================== BIDIRECTIONAL NOTIFICATIONS ==================

    /// Send cancelled notification (notifications/cancelled)
    /// Can be sent by either client or server to cancel a request
    async fn send_cancelled_notification(
        &self,
        session_id: &str,
        notification: CancelledNotification,
    ) -> Result<(), BroadcastError>;

    // ================== BROADCAST METHODS ==================

    /// Broadcast any JSON-RPC notification to all active sessions (server-wide notifications)
    async fn broadcast_to_all_sessions(&self, notification: JsonRpcNotification) -> Result<Vec<String>, BroadcastError>;

    /// Send any generic JSON-RPC notification to a specific session
    async fn send_notification(
        &self,
        session_id: &str,
        notification: JsonRpcNotification,
    ) -> Result<(), BroadcastError>;
}

/// Errors that can occur during notification broadcasting
#[derive(Debug, thiserror::Error)]
pub enum BroadcastError {
    #[error("Session not found: {0}")]
    SessionNotFound(String),
    #[error("Broadcasting failed: {0}")]
    BroadcastFailed(String),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

/// StreamManager-backed notification broadcaster that bridges events to SSE
///
/// This implementation converts ALL MCP notification types to proper JSON-RPC format
/// and forwards them to StreamManager for SSE delivery
pub struct StreamManagerNotificationBroadcaster<S: SessionStorage> {
    stream_manager: Arc<StreamManager<S>>,
}

impl<S: SessionStorage + 'static> StreamManagerNotificationBroadcaster<S> {
    /// Create new broadcaster that forwards events to StreamManager
    pub fn new(stream_manager: Arc<StreamManager<S>>) -> Self {
        Self { stream_manager }
    }
}

// ================== CONVERSION HELPERS ==================
// Helper functions to convert MCP notification types to JsonRpcNotification format

/// Convert MCP notifications to proper JSON-RPC notifications
pub mod conversion {
    use super::*;
    use std::collections::HashMap;

    pub fn progress_to_json_rpc(notification: ProgressNotification) -> JsonRpcNotification {
        let mut params = HashMap::new();
        params.insert("progressToken".to_string(), serde_json::json!(notification.params.progress_token));
        params.insert("progress".to_string(), serde_json::json!(notification.params.progress));
        if let Some(total) = notification.params.total {
            params.insert("total".to_string(), serde_json::json!(total));
        }
        if let Some(message) = notification.params.message {
            params.insert("message".to_string(), serde_json::json!(message));
        }
        if let Some(meta) = notification.params.meta {
            params.insert("_meta".to_string(), serde_json::json!(meta));
        }

        JsonRpcNotification::new_with_object_params(notification.method, params)
    }

    pub fn message_to_json_rpc(notification: LoggingMessageNotification) -> JsonRpcNotification {
        let mut params = HashMap::new();
        params.insert("level".to_string(), serde_json::json!(notification.params.level));
        params.insert("data".to_string(), notification.params.data);
        if let Some(logger) = notification.params.logger {
            params.insert("logger".to_string(), serde_json::json!(logger));
        }
        if let Some(meta) = notification.params.meta {
            params.insert("_meta".to_string(), serde_json::json!(meta));
        }

        JsonRpcNotification::new_with_object_params(notification.method, params)
    }

    pub fn resource_updated_to_json_rpc(notification: ResourceUpdatedNotification) -> JsonRpcNotification {
        let mut params = HashMap::new();
        params.insert("uri".to_string(), serde_json::json!(notification.params.uri));
        if let Some(meta) = notification.params.meta {
            params.insert("_meta".to_string(), serde_json::json!(meta));
        }

        JsonRpcNotification::new_with_object_params(notification.method, params)
    }

    pub fn resource_list_changed_to_json_rpc(notification: ResourceListChangedNotification) -> JsonRpcNotification {
        if let Some(params) = notification.params {
            if let Some(meta) = params.meta {
                let mut param_map = HashMap::new();
                param_map.insert("_meta".to_string(), serde_json::json!(meta));
                JsonRpcNotification::new_with_object_params(notification.method, param_map)
            } else {
                JsonRpcNotification::new_no_params(notification.method)
            }
        } else {
            JsonRpcNotification::new_no_params(notification.method)
        }
    }

    pub fn tool_list_changed_to_json_rpc(notification: ToolListChangedNotification) -> JsonRpcNotification {
        if let Some(params) = notification.params {
            if let Some(meta) = params.meta {
                let mut param_map = HashMap::new();
                param_map.insert("_meta".to_string(), serde_json::json!(meta));
                JsonRpcNotification::new_with_object_params(notification.method, param_map)
            } else {
                JsonRpcNotification::new_no_params(notification.method)
            }
        } else {
            JsonRpcNotification::new_no_params(notification.method)
        }
    }

    pub fn prompt_list_changed_to_json_rpc(notification: PromptListChangedNotification) -> JsonRpcNotification {
        if let Some(params) = notification.params {
            if let Some(meta) = params.meta {
                let mut param_map = HashMap::new();
                param_map.insert("_meta".to_string(), serde_json::json!(meta));
                JsonRpcNotification::new_with_object_params(notification.method, param_map)
            } else {
                JsonRpcNotification::new_no_params(notification.method)
            }
        } else {
            JsonRpcNotification::new_no_params(notification.method)
        }
    }

    pub fn cancelled_to_json_rpc(notification: CancelledNotification) -> JsonRpcNotification {
        let mut params = HashMap::new();
        params.insert("requestId".to_string(), serde_json::json!(notification.params.request_id));
        if let Some(reason) = notification.params.reason {
            params.insert("reason".to_string(), serde_json::json!(reason));
        }
        if let Some(meta) = notification.params.meta {
            params.insert("_meta".to_string(), serde_json::json!(meta));
        }

        JsonRpcNotification::new_with_object_params(notification.method, params)
    }
}

#[async_trait]
impl<S: SessionStorage + 'static> NotificationBroadcaster for StreamManagerNotificationBroadcaster<S> {
    // ================== SERVER-TO-CLIENT NOTIFICATIONS ==================

    async fn send_progress_notification(
        &self,
        session_id: &str,
        notification: ProgressNotification,
    ) -> Result<(), BroadcastError> {
        let json_rpc_notification = conversion::progress_to_json_rpc(notification);
        self.send_notification(session_id, json_rpc_notification).await
    }

    async fn send_message_notification(
        &self,
        session_id: &str,
        notification: LoggingMessageNotification,
    ) -> Result<(), BroadcastError> {
        let json_rpc_notification = conversion::message_to_json_rpc(notification);
        self.send_notification(session_id, json_rpc_notification).await
    }

    async fn send_resource_updated_notification(
        &self,
        session_id: &str,
        notification: ResourceUpdatedNotification,
    ) -> Result<(), BroadcastError> {
        let json_rpc_notification = conversion::resource_updated_to_json_rpc(notification);
        self.send_notification(session_id, json_rpc_notification).await
    }

    async fn send_resource_list_changed_notification(
        &self,
        session_id: &str,
        notification: ResourceListChangedNotification,
    ) -> Result<(), BroadcastError> {
        let json_rpc_notification = conversion::resource_list_changed_to_json_rpc(notification);
        self.send_notification(session_id, json_rpc_notification).await
    }

    async fn send_tool_list_changed_notification(
        &self,
        session_id: &str,
        notification: ToolListChangedNotification,
    ) -> Result<(), BroadcastError> {
        let json_rpc_notification = conversion::tool_list_changed_to_json_rpc(notification);
        self.send_notification(session_id, json_rpc_notification).await
    }

    async fn send_prompt_list_changed_notification(
        &self,
        session_id: &str,
        notification: PromptListChangedNotification,
    ) -> Result<(), BroadcastError> {
        let json_rpc_notification = conversion::prompt_list_changed_to_json_rpc(notification);
        self.send_notification(session_id, json_rpc_notification).await
    }

    // ================== BIDIRECTIONAL NOTIFICATIONS ==================

    async fn send_cancelled_notification(
        &self,
        session_id: &str,
        notification: CancelledNotification,
    ) -> Result<(), BroadcastError> {
        let json_rpc_notification = conversion::cancelled_to_json_rpc(notification);
        self.send_notification(session_id, json_rpc_notification).await
    }

    // ================== BROADCAST METHODS ==================

    async fn broadcast_to_all_sessions(&self, notification: JsonRpcNotification) -> Result<Vec<String>, BroadcastError> {
        // Convert JsonRpcNotification to SSE-formatted JSON
        let sse_data = serde_json::to_value(&notification)
            .map_err(|e| BroadcastError::SerializationError(e))?;

        // Use StreamManager's built-in broadcast_to_all_sessions method
        match self.stream_manager.broadcast_to_all_sessions(
            notification.method.clone(), // Use MCP method name as event type
            sse_data
        ).await {
            Ok(failed_sessions) => {
                info!("📡 Broadcast JSON-RPC notification to all sessions: method={}, failed={}",
                      notification.method, failed_sessions.len());
                Ok(failed_sessions)
            }
            Err(e) => {
                error!("❌ Failed to broadcast JSON-RPC notification: method={}, error={}",
                       notification.method, e);
                Err(BroadcastError::BroadcastFailed(e.to_string()))
            }
        }
    }

    async fn send_notification(
        &self,
        session_id: &str,
        notification: JsonRpcNotification,
    ) -> Result<(), BroadcastError> {
        // Convert JsonRpcNotification to SSE-formatted JSON
        let sse_data = serde_json::to_value(&notification)
            .map_err(|e| BroadcastError::SerializationError(e))?;

        // Send via StreamManager with proper JSON-RPC format
        match self.stream_manager.broadcast_to_session(
            session_id,
            notification.method.clone(), // Use actual MCP method name as event type
            sse_data
        ).await {
            Ok(event_id) => {
                info!("✅ Sent JSON-RPC notification: session={}, method={}, event_id={}",
                      session_id, notification.method, event_id);
                Ok(())
            }
            Err(e) => {
                error!("❌ Failed to send JSON-RPC notification: session={}, method={}, error={}",
                       session_id, notification.method, e);
                Err(BroadcastError::BroadcastFailed(e.to_string()))
            }
        }
    }
}

/// Shared NotificationBroadcaster type alias for use across the turul-http-mcp-server crate
pub type SharedNotificationBroadcaster = Arc<dyn NotificationBroadcaster + Send + Sync>;
