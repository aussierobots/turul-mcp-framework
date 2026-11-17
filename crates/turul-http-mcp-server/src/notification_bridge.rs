//! Bridges `NotificationBroadcaster` to `StreamManager`.
//!
//! This module connects the notification system (where tools send events) to the
//! SSE streaming system (where clients receive events).
//!
//! All notifications use the MCP JSON-RPC notification format:
//! `{"jsonrpc":"2.0","method":"notifications/{type}","params":{...}}`
//!
//! Without this bridge, notifications sent through `NotificationBroadcaster` are not delivered to SSE clients.

// `LoggingMessageNotification` is deprecated-but-present in 2026-07-28 (SEP-2577); this bridge
// still carries logging notifications over SSE, which remains a valid feature.
#![allow(deprecated)]

use async_trait::async_trait;
use std::sync::Arc;
use tracing::{debug, error, info};

use crate::StreamManager;
use turul_mcp_protocol::notifications::{
    CancelledNotification, LoggingMessageNotification, ProgressNotification,
    PromptListChangedNotification, ResourceListChangedNotification, ResourceUpdatedNotification,
    ToolListChangedNotification,
};
use turul_rpc::JsonRpcNotification;

/// Sends every supported notification type over SSE.
///
/// Each method sends a JSON-RPC notification in MCP wire format.
#[async_trait]
pub trait NotificationBroadcaster: Send + Sync {
    /// Sends a progress notification (`notifications/progress`).
    ///
    /// Used for long-running operations to show progress updates.
    async fn send_progress_notification(
        &self,
        session_id: &str,
        notification: ProgressNotification,
    ) -> Result<(), BroadcastError>;

    /// Sends a logging message notification (`notifications/message`).
    ///
    /// Used to send log messages with different levels (debug, info, warning, error).
    async fn send_message_notification(
        &self,
        session_id: &str,
        notification: LoggingMessageNotification,
    ) -> Result<(), BroadcastError>;

    /// Sends a resource updated notification (`notifications/resources/updated`).
    ///
    /// Notifies that a specific resource has been updated.
    async fn send_resource_updated_notification(
        &self,
        session_id: &str,
        notification: ResourceUpdatedNotification,
    ) -> Result<(), BroadcastError>;

    /// Sends a resource list changed notification (`notifications/resources/list_changed`).
    ///
    /// Notifies that the resource list has changed (added/removed resources).
    async fn send_resource_list_changed_notification(
        &self,
        session_id: &str,
        notification: ResourceListChangedNotification,
    ) -> Result<(), BroadcastError>;

    /// Sends a tool list changed notification (`notifications/tools/list_changed`).
    ///
    /// Notifies that the tool list has changed (added/removed tools).
    async fn send_tool_list_changed_notification(
        &self,
        session_id: &str,
        notification: ToolListChangedNotification,
    ) -> Result<(), BroadcastError>;

    /// Sends a prompt list changed notification (`notifications/prompts/list_changed`).
    ///
    /// Notifies that the prompt list has changed (added/removed prompts).
    async fn send_prompt_list_changed_notification(
        &self,
        session_id: &str,
        notification: PromptListChangedNotification,
    ) -> Result<(), BroadcastError>;

    /// Sends a cancelled notification (`notifications/cancelled`).
    ///
    /// Either client or server can send it to cancel a request.
    async fn send_cancelled_notification(
        &self,
        session_id: &str,
        notification: CancelledNotification,
    ) -> Result<(), BroadcastError>;

    /// Broadcasts a JSON-RPC notification to all active sessions.
    async fn broadcast_to_all_sessions(
        &self,
        notification: JsonRpcNotification,
    ) -> Result<Vec<String>, BroadcastError>;

    /// Sends a JSON-RPC notification to a specific session.
    async fn send_notification(
        &self,
        session_id: &str,
        notification: JsonRpcNotification,
    ) -> Result<(), BroadcastError>;
}

/// Errors returned by notification broadcasting.
#[derive(Debug, thiserror::Error)]
pub enum BroadcastError {
    #[error("Session not found: {0}")]
    /// No session exists for the given identifier.
    SessionNotFound(String),
    #[error("Broadcasting failed: {0}")]
    /// The notification could not be delivered; the message gives the reason.
    BroadcastFailed(String),
    #[error("Serialization error: {0}")]
    /// The notification could not be serialized to JSON.
    SerializationError(#[from] serde_json::Error),
}

/// `StreamManager`-backed notification broadcaster that delivers events over SSE.
///
/// Converts each MCP notification type to JSON-RPC format and forwards it to
/// `StreamManager`.
pub struct StreamManagerNotificationBroadcaster {
    stream_manager: Arc<StreamManager>,
}

impl StreamManagerNotificationBroadcaster {
    /// Creates a broadcaster that forwards events to `stream_manager`.
    pub fn new(stream_manager: Arc<StreamManager>) -> Self {
        Self { stream_manager }
    }
}

/// Converts MCP notifications to JSON-RPC notifications.
pub mod conversion {
    use super::*;
    use std::collections::HashMap;

    /// Converts a progress notification into a JSON-RPC notification carrying its method and parameters.
    pub fn progress_to_json_rpc(notification: ProgressNotification) -> JsonRpcNotification {
        let mut params = HashMap::new();
        params.insert(
            "progressToken".to_string(),
            serde_json::json!(notification.params.progress_token),
        );
        params.insert(
            "progress".to_string(),
            serde_json::json!(notification.params.progress),
        );
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

    /// Converts a logging message notification into a JSON-RPC notification carrying its method and parameters.
    pub fn message_to_json_rpc(notification: LoggingMessageNotification) -> JsonRpcNotification {
        let mut params = HashMap::new();
        params.insert(
            "level".to_string(),
            serde_json::json!(notification.params.level),
        );
        params.insert("data".to_string(), notification.params.data);
        if let Some(logger) = notification.params.logger {
            params.insert("logger".to_string(), serde_json::json!(logger));
        }
        if let Some(meta) = notification.params.meta {
            params.insert("_meta".to_string(), serde_json::json!(meta));
        }

        JsonRpcNotification::new_with_object_params(notification.method, params)
    }

    /// Converts a resource-updated notification into a JSON-RPC notification carrying its method and parameters.
    pub fn resource_updated_to_json_rpc(
        notification: ResourceUpdatedNotification,
    ) -> JsonRpcNotification {
        let mut params = HashMap::new();
        params.insert(
            "uri".to_string(),
            serde_json::json!(notification.params.uri),
        );
        if let Some(meta) = notification.params.meta {
            params.insert("_meta".to_string(), serde_json::json!(meta));
        }

        JsonRpcNotification::new_with_object_params(notification.method, params)
    }

    /// Converts a resource-list-changed notification into a JSON-RPC notification carrying its method and parameters.
    pub fn resource_list_changed_to_json_rpc(
        notification: ResourceListChangedNotification,
    ) -> JsonRpcNotification {
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

    /// Converts a tool-list-changed notification into a JSON-RPC notification carrying its method and parameters.
    pub fn tool_list_changed_to_json_rpc(
        notification: ToolListChangedNotification,
    ) -> JsonRpcNotification {
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

    /// Converts a prompt-list-changed notification into a JSON-RPC notification carrying its method and parameters.
    pub fn prompt_list_changed_to_json_rpc(
        notification: PromptListChangedNotification,
    ) -> JsonRpcNotification {
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

    /// Converts a cancelled notification into a JSON-RPC notification carrying its method and parameters.
    pub fn cancelled_to_json_rpc(notification: CancelledNotification) -> JsonRpcNotification {
        let mut params = HashMap::new();
        params.insert(
            "requestId".to_string(),
            serde_json::json!(notification.params.request_id),
        );
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
impl NotificationBroadcaster for StreamManagerNotificationBroadcaster {
    async fn send_progress_notification(
        &self,
        session_id: &str,
        notification: ProgressNotification,
    ) -> Result<(), BroadcastError> {
        let json_rpc_notification = conversion::progress_to_json_rpc(notification);
        self.send_notification(session_id, json_rpc_notification)
            .await
    }

    async fn send_message_notification(
        &self,
        session_id: &str,
        notification: LoggingMessageNotification,
    ) -> Result<(), BroadcastError> {
        let json_rpc_notification = conversion::message_to_json_rpc(notification);
        self.send_notification(session_id, json_rpc_notification)
            .await
    }

    async fn send_resource_updated_notification(
        &self,
        session_id: &str,
        notification: ResourceUpdatedNotification,
    ) -> Result<(), BroadcastError> {
        let json_rpc_notification = conversion::resource_updated_to_json_rpc(notification);
        self.send_notification(session_id, json_rpc_notification)
            .await
    }

    async fn send_resource_list_changed_notification(
        &self,
        session_id: &str,
        notification: ResourceListChangedNotification,
    ) -> Result<(), BroadcastError> {
        let json_rpc_notification = conversion::resource_list_changed_to_json_rpc(notification);
        self.send_notification(session_id, json_rpc_notification)
            .await
    }

    async fn send_tool_list_changed_notification(
        &self,
        session_id: &str,
        notification: ToolListChangedNotification,
    ) -> Result<(), BroadcastError> {
        let json_rpc_notification = conversion::tool_list_changed_to_json_rpc(notification);
        self.send_notification(session_id, json_rpc_notification)
            .await
    }

    async fn send_prompt_list_changed_notification(
        &self,
        session_id: &str,
        notification: PromptListChangedNotification,
    ) -> Result<(), BroadcastError> {
        let json_rpc_notification = conversion::prompt_list_changed_to_json_rpc(notification);
        self.send_notification(session_id, json_rpc_notification)
            .await
    }

    async fn send_cancelled_notification(
        &self,
        session_id: &str,
        notification: CancelledNotification,
    ) -> Result<(), BroadcastError> {
        let json_rpc_notification = conversion::cancelled_to_json_rpc(notification);
        self.send_notification(session_id, json_rpc_notification)
            .await
    }

    async fn broadcast_to_all_sessions(
        &self,
        notification: JsonRpcNotification,
    ) -> Result<Vec<String>, BroadcastError> {
        let sse_data =
            serde_json::to_value(&notification).map_err(BroadcastError::SerializationError)?;

        match self
            .stream_manager
            .broadcast_to_all_sessions(
                notification.method.clone(), // Use MCP method name as event type
                sse_data,
            )
            .await
        {
            Ok(failed_sessions) => {
                info!(
                    "📡 Broadcast JSON-RPC notification to all sessions: method={}, failed={}",
                    notification.method,
                    failed_sessions.len()
                );
                Ok(failed_sessions)
            }
            Err(e) => {
                error!(
                    "❌ Failed to broadcast JSON-RPC notification: method={}, error={}",
                    notification.method, e
                );
                Err(BroadcastError::BroadcastFailed(e.to_string()))
            }
        }
    }

    async fn send_notification(
        &self,
        session_id: &str,
        notification: JsonRpcNotification,
    ) -> Result<(), BroadcastError> {
        let sse_data =
            serde_json::to_value(&notification).map_err(BroadcastError::SerializationError)?;

        match self
            .stream_manager
            .broadcast_to_session(
                session_id,
                notification.method.clone(), // Use actual MCP method name as event type
                sse_data,
            )
            .await
        {
            Ok(event_id) => {
                debug!(
                    "✅ Sent JSON-RPC notification: session={}, method={}, event_id={}",
                    session_id, notification.method, event_id
                );
                Ok(())
            }
            Err(e) => {
                error!(
                    "❌ Failed to send JSON-RPC notification: session={}, method={}, error={}",
                    session_id, notification.method, e
                );
                Err(BroadcastError::BroadcastFailed(e.to_string()))
            }
        }
    }
}

/// Shared, thread-safe handle to a `NotificationBroadcaster`.
pub type SharedNotificationBroadcaster = Arc<dyn NotificationBroadcaster + Send + Sync>;
