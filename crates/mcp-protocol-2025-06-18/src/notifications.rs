//! MCP Notifications Protocol Types
//!
//! This module defines types for notifications in MCP according to the 2025-06-18 specification.
//! MCP notifications are JSON-RPC notifications that inform clients about server state changes.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::logging::LogLevel;
use json_rpc_server::types::RequestId;

/// Base notification structure
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Notification {
    /// Notification method
    pub method: String,
    /// Notification parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

/// Resource list changed notification
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceListChangedNotification {
    /// Empty params object
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

/// Tool list changed notification
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolListChangedNotification {
    /// Empty params object
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

/// Prompt list changed notification
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptListChangedNotification {
    /// Empty params object
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

/// Root list changed notification
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RootListChangedNotification {
    /// Empty params object
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

/// Progress notification
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressNotification {
    /// Progress token
    pub progress_token: String,
    /// Progress value (0.0 to 1.0)
    pub progress: f64,
    /// Optional progress total
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<f64>,
}

/// Resource updated notification
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceUpdatedNotification {
    /// URI of the updated resource
    pub uri: String,
}

/// Subscription cancelled notification
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionCancelledNotification {
    /// URI of the cancelled subscription
    pub uri: String,
}

// ==== MCP 2025-06-18 Core Notifications ====

/// Method: "notifications/cancelled"
/// Notification sent when a request is cancelled.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelledNotification {
    /// The ID of the request to cancel
    pub request_id: RequestId,
    /// An optional reason for cancelling
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Method: "notifications/initialized"
/// Notification sent after the client has completed initialization.
/// No parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializedNotification {
    // Intentionally empty: no params for InitializedNotification
}

/// Method: "notifications/progress"  
/// Notification used to report progress on a long-running request.
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
}

/// Method: "notifications/message"
/// Notification used to send log messages to the client.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoggingMessageNotification {
    /// Log level
    pub level: LogLevel,
    /// Log message
    pub message: String,
    /// Optional logger name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logger: Option<String>,
    /// Optional additional data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// Method: "notifications/resources/listChanged"
/// Notification indicating the list of resources has changed.
/// No parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceListChangedNotificationParams {
    // Intentionally empty
}

/// Method: "notifications/resources/updated"  
/// Notification indicating a specific resource has been updated.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceUpdatedNotificationParams {
    /// The URI of the resource that was updated
    pub uri: String,
}

/// Method: "notifications/prompts/listChanged"
/// Notification indicating the list of prompts has changed.
/// No parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptListChangedNotificationParams {
    // Intentionally empty
}

/// Method: "notifications/tools/listChanged"
/// Notification indicating the list of tools has changed.
/// No parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolListChangedNotificationParams {
    // Intentionally empty
}

/// Method: "notifications/roots/listChanged"
/// Notification indicating the list of roots has changed.
/// No parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RootListChangedNotificationParams {
    // Intentionally empty
}

// ==== Convenience Functions for Creating Notifications ====

impl CancelledNotification {
    pub fn new(request_id: RequestId) -> Self {
        Self {
            request_id,
            reason: None,
        }
    }

    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }
}

impl ProgressNotificationParams {
    pub fn new(progress_token: impl Into<String>, progress: u64) -> Self {
        Self {
            progress_token: progress_token.into(),
            progress,
            total: None,
            message: None,
        }
    }

    pub fn with_total(mut self, total: u64) -> Self {
        self.total = Some(total);
        self
    }

    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }
}

impl LoggingMessageNotification {
    pub fn new(level: LogLevel, message: impl Into<String>) -> Self {
        Self {
            level,
            message: message.into(),
            logger: None,
            data: None,
        }
    }

    pub fn with_logger(mut self, logger: impl Into<String>) -> Self {
        self.logger = Some(logger.into());
        self
    }

    pub fn with_data(mut self, data: Value) -> Self {
        self.data = Some(data);
        self
    }
}

impl ResourceUpdatedNotificationParams {
    pub fn new(uri: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
        }
    }
}