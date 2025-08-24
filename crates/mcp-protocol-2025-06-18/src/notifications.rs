//! MCP Notifications Protocol Types
//!
//! This module defines types for notifications in MCP according to the 2025-06-18 specification.
//! MCP notifications are JSON-RPC notifications that inform clients about server state changes.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::logging::LogLevel;
use json_rpc_server::types::RequestId;

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

/// Method: "notifications/resources/listChanged"
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourcesListChangedNotification {
    /// Method name (always "notifications/resources/listChanged")
    pub method: String,
    /// Optional empty params with _meta support
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<NotificationParams>,
}

impl ResourcesListChangedNotification {
    pub fn new() -> Self {
        Self {
            method: "notifications/resources/listChanged".to_string(),
            params: None,
        }
    }

    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.params = Some(NotificationParams::new().with_meta(meta));
        self
    }
}

/// Method: "notifications/tools/listChanged"
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolsListChangedNotification {
    /// Method name (always "notifications/tools/listChanged")
    pub method: String,
    /// Optional empty params with _meta support
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<NotificationParams>,
}

impl ToolsListChangedNotification {
    pub fn new() -> Self {
        Self {
            method: "notifications/tools/listChanged".to_string(),
            params: None,
        }
    }

    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.params = Some(NotificationParams::new().with_meta(meta));
        self
    }
}

/// Method: "notifications/prompts/listChanged"
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptsListChangedNotification {
    /// Method name (always "notifications/prompts/listChanged")
    pub method: String,
    /// Optional empty params with _meta support
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<NotificationParams>,
}

impl PromptsListChangedNotification {
    pub fn new() -> Self {
        Self {
            method: "notifications/prompts/listChanged".to_string(),
            params: None,
        }
    }

    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.params = Some(NotificationParams::new().with_meta(meta));
        self
    }
}

/// Method: "notifications/roots/listChanged"
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RootsListChangedNotification {
    /// Method name (always "notifications/roots/listChanged")
    pub method: String,
    /// Optional empty params with _meta support
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<NotificationParams>,
}

impl RootsListChangedNotification {
    pub fn new() -> Self {
        Self {
            method: "notifications/roots/listChanged".to_string(),
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
    pub level: LogLevel,
    /// Log message
    pub message: String,
    /// Optional logger name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logger: Option<String>,
    /// Optional additional data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    /// Optional MCP meta information
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<HashMap<String, Value>>,
}

impl LoggingMessageNotification {
    pub fn new(level: LogLevel, message: impl Into<String>) -> Self {
        Self {
            method: "notifications/message".to_string(),
            params: LoggingMessageNotificationParams {
                level,
                message: message.into(),
                logger: None,
                data: None,
                meta: None,
            },
        }
    }

    pub fn with_logger(mut self, logger: impl Into<String>) -> Self {
        self.params.logger = Some(logger.into());
        self
    }

    pub fn with_data(mut self, data: Value) -> Self {
        self.params.data = Some(data);
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

// Base notification trait implementations
impl HasMethod for ResourcesListChangedNotification {
    fn method(&self) -> &str {
        &self.method
    }
}

impl HasMethod for ToolsListChangedNotification {
    fn method(&self) -> &str {
        &self.method
    }
}

impl HasMethod for PromptsListChangedNotification {
    fn method(&self) -> &str {
        &self.method
    }
}

impl HasMethod for RootsListChangedNotification {
    fn method(&self) -> &str {
        &self.method
    }
}

impl HasMethod for ProgressNotification {
    fn method(&self) -> &str {
        &self.method
    }
}

impl HasMethod for ResourceUpdatedNotification {
    fn method(&self) -> &str {
        &self.method
    }
}

impl HasMethod for CancelledNotification {
    fn method(&self) -> &str {
        &self.method
    }
}

impl HasMethod for InitializedNotification {
    fn method(&self) -> &str {
        &self.method
    }
}

impl HasMethod for LoggingMessageNotification {
    fn method(&self) -> &str {
        &self.method
    }
}