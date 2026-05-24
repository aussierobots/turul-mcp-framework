//! Notification payload types for MCP DRAFT-2026-v1.
//!
//! **Important**: `*ListChangedNotification`, [`ProgressNotification`], and the
//! other types in this module carry only the MCP `method` and `params` fields —
//! they are NOT wire-complete JSON-RPC messages. Wrap them in
//! [`crate::JsonRpcNotification`] (which adds `jsonrpc: "2.0"`) before sending
//! over any transport.
//!
//! Method strings use `list_changed` (underscore) per the DRAFT-2026-v1 spec.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::logging::LoggingLevel;
use turul_rpc::RequestId;

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

impl Default for NotificationParams {
    fn default() -> Self {
        Self::new()
    }
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

// `Params` and `HasMetaParam` impls live at the bottom of this file alongside
// the rest of the notifications trait impls. Adding the `HasMeta` and
// `HasDataParam` impls (which the json_rpc layer needs) here for visibility
// with the struct definition.
impl crate::traits::HasMeta for NotificationParams {
    fn meta(&self) -> Option<&crate::meta::MetaObject> {
        self.meta.as_ref()
    }
}

impl crate::traits::HasDataParam for NotificationParams {
    fn data(&self) -> &HashMap<String, Value> {
        &self.other
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

/// MCP notification payload for "notifications/resources/list_changed".
///
/// **WARNING: Not wire-complete.** See [`ToolListChangedNotification`] for details.
/// Use `JsonRpcNotification::new("notifications/resources/list_changed")` for transport.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceListChangedNotification {
    /// Method name (always "notifications/resources/list_changed")
    pub method: String,
    /// Optional empty params with _meta support
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<NotificationParams>,
}

impl Default for ResourceListChangedNotification {
    fn default() -> Self {
        Self::new()
    }
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

/// MCP notification payload for "notifications/tools/list_changed".
///
/// **WARNING: This is NOT a wire-complete JSON-RPC message.** It contains only the
/// MCP-specific fields (`method`, `params`), NOT the `jsonrpc: "2.0"` envelope required
/// for transport. To send on the wire, wrap in `JsonRpcNotification`:
///
/// ```rust,ignore
/// // CORRECT — wire-complete:
/// let wire_msg = JsonRpcNotification::new("notifications/tools/list_changed".to_string());
///
/// // WRONG — missing jsonrpc field, will fail client validation:
/// let payload = ToolListChangedNotification::new(); // NOT wire-complete
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolListChangedNotification {
    /// Method name (always "notifications/tools/list_changed")
    pub method: String,
    /// Optional empty params with _meta support
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<NotificationParams>,
}

impl Default for ToolListChangedNotification {
    fn default() -> Self {
        Self::new()
    }
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

/// MCP notification payload for "notifications/prompts/list_changed".
///
/// **WARNING: Not wire-complete.** See [`ToolListChangedNotification`] for details.
/// Use `JsonRpcNotification::new("notifications/prompts/list_changed")` for transport.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptListChangedNotification {
    /// Method name (always "notifications/prompts/list_changed")
    pub method: String,
    /// Optional empty params with _meta support
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<NotificationParams>,
}

impl Default for PromptListChangedNotification {
    fn default() -> Self {
        Self::new()
    }
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

/// Method: "notifications/progress"
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressNotification {
    /// Method name (always "notifications/progress")
    pub method: String,
    /// Progress parameters
    pub params: ProgressNotificationParams,
}

/// Progress token value — a string or a number. Used by the caller to opt in
/// to out-of-band `notifications/progress` for a request; echoed in every
/// progress notification for that operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ProgressTokenValue {
    String(String),
    Number(i64),
}

impl From<String> for ProgressTokenValue {
    fn from(s: String) -> Self {
        Self::String(s)
    }
}

impl From<&str> for ProgressTokenValue {
    fn from(s: &str) -> Self {
        Self::String(s.to_string())
    }
}

impl From<i64> for ProgressTokenValue {
    fn from(n: i64) -> Self {
        Self::Number(n)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressNotificationParams {
    /// Token to correlate with the original request (string or number)
    pub progress_token: ProgressTokenValue,
    /// Amount of work completed so far (fractional progress)
    pub progress: f64,
    /// Optional total work count
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<f64>,
    /// Optional human-readable message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Optional MCP meta information
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<HashMap<String, Value>>,
}

impl ProgressNotification {
    pub fn new(progress_token: impl Into<ProgressTokenValue>, progress: f64) -> Self {
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

    pub fn with_total(mut self, total: f64) -> Self {
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

/// Method: `"notifications/elicitation/complete"`.
///
/// Sent by the client when an elicitation has been completed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElicitationCompleteNotification {
    /// Method name (always "notifications/elicitation/complete")
    pub method: String,
    /// Elicitation complete parameters
    pub params: ElicitationCompleteNotificationParams,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElicitationCompleteNotificationParams {
    /// The ID of the elicitation that was completed
    pub elicitation_id: String,
    /// Optional MCP meta information
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<HashMap<String, Value>>,
}

impl ElicitationCompleteNotification {
    pub fn new(elicitation_id: impl Into<String>) -> Self {
        Self {
            method: "notifications/elicitation/complete".to_string(),
            params: ElicitationCompleteNotificationParams {
                elicitation_id: elicitation_id.into(),
                meta: None,
            },
        }
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
    fn test_progress_notification() {
        let notification = ProgressNotification::new("token123", 50.0)
            .with_total(100.0)
            .with_message("Processing...");

        assert_eq!(notification.method, "notifications/progress");
        assert_eq!(
            notification.params.progress_token,
            ProgressTokenValue::String("token123".to_string())
        );
        assert_eq!(notification.params.progress, 50.0);
        assert_eq!(notification.params.total, Some(100.0));
        assert_eq!(
            notification.params.message,
            Some("Processing...".to_string())
        );
    }

    #[test]
    fn test_progress_token_number() {
        let notification = ProgressNotification::new(ProgressTokenValue::Number(42), 0.5);
        let json = serde_json::to_value(&notification).unwrap();
        assert_eq!(json["params"]["progressToken"], 42);
        assert_eq!(json["params"]["progress"], 0.5);
    }

    #[test]
    fn test_resource_updated() {
        let notification = ResourceUpdatedNotification::new("file:///test.txt");
        assert_eq!(notification.method, "notifications/resources/updated");
        assert_eq!(notification.params.uri, "file:///test.txt");
    }

    #[test]
    fn test_cancelled_notification() {
        use turul_rpc::RequestId;
        let notification =
            CancelledNotification::new(RequestId::Number(123)).with_reason("User cancelled");

        assert_eq!(notification.method, "notifications/cancelled");
        assert_eq!(notification.params.request_id, RequestId::Number(123));
        assert_eq!(
            notification.params.reason,
            Some("User cancelled".to_string())
        );
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
    fn test_elicitation_complete_notification() {
        let notification = ElicitationCompleteNotification::new("elicit-xyz-789");

        assert_eq!(notification.method, "notifications/elicitation/complete");

        let json = serde_json::to_value(&notification).unwrap();
        assert_eq!(json["method"], "notifications/elicitation/complete");
        assert_eq!(json["params"]["elicitationId"], "elicit-xyz-789");
    }
}
