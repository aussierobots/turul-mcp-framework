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

#[allow(deprecated)] // SEP-2577 migration window
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

/// Progress token value. Deprecated alias for [`crate::meta::ProgressToken`] —
/// the schema has one `ProgressToken = string | number` type used at both
/// request-meta and progress-notification carriers; both sites now reference
/// the unified type. Kept as a re-export for any caller pinning the old name.
pub use crate::meta::ProgressToken as ProgressTokenValue;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressNotificationParams {
    /// Token to correlate with the original request (string or number).
    pub progress_token: crate::meta::ProgressToken,
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
    pub fn new(progress_token: impl Into<crate::meta::ProgressToken>, progress: f64) -> Self {
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
    /// The ID of the request to cancel. Optional per schema (`requestId?`) —
    /// a cancellation MAY arrive after the originating request has already
    /// finished, in which case the id is unknown to the sender.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<RequestId>,
    /// An optional reason for cancelling
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Optional MCP meta information
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<HashMap<String, Value>>,
}

impl CancelledNotification {
    /// Cancel a specific in-flight request.
    pub fn new(request_id: RequestId) -> Self {
        Self::new_optional(Some(request_id))
    }

    /// Cancellation without a specific request id (spec-valid late-arrival case).
    pub fn without_id() -> Self {
        Self::new_optional(None)
    }

    fn new_optional(request_id: Option<RequestId>) -> Self {
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

/// Method: "notifications/message".
///
/// **Deprecated** per SEP-2577 — the whole Logging surface (this
/// notification, the per-request `_meta` logLevel opt-in, and the
/// `LoggingLevel` enum) is deprecated. Migrate to stderr (stdio) or
/// OpenTelemetry. Functional through the migration window.
#[deprecated(
    since = "0.4.0",
    note = "Deprecated per SEP-2577 (DRAFT-2026-v1). \
            Replacement: log to stderr for stdio transports or use OpenTelemetry. \
            Per-request log level opt-in lives on RequestMetaObject.log_level. \
            Earliest removal: first release on/after 2027-07-28."
)]
#[allow(deprecated)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoggingMessageNotification {
    /// Method name (always "notifications/message")
    pub method: String,
    /// Logging parameters
    pub params: LoggingMessageNotificationParams,
}

/// **Deprecated** per SEP-2577 — see [`LoggingMessageNotification`].
#[deprecated(
    since = "0.4.0",
    note = "Deprecated per SEP-2577 (DRAFT-2026-v1). \
            Replacement: log to stderr for stdio transports or use OpenTelemetry. \
            Earliest removal: first release on/after 2027-07-28."
)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoggingMessageNotificationParams {
    /// Log level
    #[allow(deprecated)]
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

#[allow(deprecated)]
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

// ResourceListChangedNotification — empty params, just method
impl HasMethod for ResourceListChangedNotification {
    fn method(&self) -> &str {
        &self.method
    }
}
impl HasParams for ResourceListChangedNotification {
    fn params(&self) -> Option<&dyn Params> {
        self.params.as_ref().map(|p| p as &dyn Params)
    }
}
impl RpcNotification for ResourceListChangedNotification {}
impl ResourcesListChangedNotificationTrait for ResourceListChangedNotification {}

// ToolListChangedNotification — empty params, just method
impl HasMethod for ToolListChangedNotification {
    fn method(&self) -> &str {
        &self.method
    }
}
impl HasParams for ToolListChangedNotification {
    fn params(&self) -> Option<&dyn Params> {
        self.params.as_ref().map(|p| p as &dyn Params)
    }
}
impl RpcNotification for ToolListChangedNotification {}
impl ToolListChangedNotificationTrait for ToolListChangedNotification {}

// PromptListChangedNotification — empty params, just method
impl HasMethod for PromptListChangedNotification {
    fn method(&self) -> &str {
        &self.method
    }
}
impl HasParams for PromptListChangedNotification {
    fn params(&self) -> Option<&dyn Params> {
        self.params.as_ref().map(|p| p as &dyn Params)
    }
}
impl RpcNotification for PromptListChangedNotification {}
impl PromptListChangedNotificationTrait for PromptListChangedNotification {}

// ProgressNotificationParams — field-getter coverage on the params struct.
impl Params for ProgressNotificationParams {}
impl HasProgressParams for ProgressNotificationParams {
    fn progress_token(&self) -> &crate::meta::ProgressToken {
        &self.progress_token
    }
    fn progress(&self) -> f64 {
        self.progress
    }
    fn total(&self) -> Option<f64> {
        self.total
    }
    fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }
}
impl HasMethod for ProgressNotification {
    fn method(&self) -> &str {
        &self.method
    }
}
impl HasParams for ProgressNotification {
    fn params(&self) -> Option<&dyn Params> {
        Some(&self.params as &dyn Params)
    }
}
impl RpcNotification for ProgressNotification {}
impl ProgressNotificationTrait for ProgressNotification {}

// ResourceUpdatedNotificationParams + ResourceUpdatedNotification
impl Params for ResourceUpdatedNotificationParams {}
impl HasResourceUpdatedParams for ResourceUpdatedNotificationParams {
    fn uri(&self) -> &str {
        &self.uri
    }
}
impl HasMethod for ResourceUpdatedNotification {
    fn method(&self) -> &str {
        &self.method
    }
}
impl HasParams for ResourceUpdatedNotification {
    fn params(&self) -> Option<&dyn Params> {
        Some(&self.params as &dyn Params)
    }
}
impl RpcNotification for ResourceUpdatedNotification {}
impl ResourceUpdatedNotificationTrait for ResourceUpdatedNotification {}

// CancelledNotificationParams + CancelledNotification
impl Params for CancelledNotificationParams {}
impl HasCancelledParams for CancelledNotificationParams {
    fn request_id(&self) -> Option<&turul_rpc::RequestId> {
        self.request_id.as_ref()
    }
    fn reason(&self) -> Option<&str> {
        self.reason.as_deref()
    }
}
impl HasMethod for CancelledNotification {
    fn method(&self) -> &str {
        &self.method
    }
}
impl HasParams for CancelledNotification {
    fn params(&self) -> Option<&dyn Params> {
        Some(&self.params as &dyn Params)
    }
}
impl RpcNotification for CancelledNotification {}
impl CancelledNotificationTrait for CancelledNotification {}

// LoggingMessageNotification — `notifications/message` wire payload.
//
// **Deprecated** per SEP-2577 in DRAFT-2026-v1; concrete `#[deprecated]`
// attributes live on the struct definitions below. The trait impls here are
// gated with `#[allow(deprecated)]` so the unimplementable-from-outside trait
// surface still compiles internally without forcing every reader to chase a
// warning through framework-internal code.
#[allow(deprecated)]
impl HasMethod for LoggingMessageNotification {
    fn method(&self) -> &str {
        &self.method
    }
}
#[allow(deprecated)]
impl HasParams for LoggingMessageNotification {
    fn params(&self) -> Option<&dyn Params> {
        Some(&self.params as &dyn Params)
    }
}
#[allow(deprecated)]
impl RpcNotification for LoggingMessageNotification {}
#[allow(deprecated)]
impl LoggingMessageNotificationTrait for LoggingMessageNotification {}

#[allow(deprecated)]
impl Params for LoggingMessageNotificationParams {}

#[allow(deprecated)]
impl HasLevelParam for LoggingMessageNotificationParams {
    fn level(&self) -> &crate::logging::LoggingLevel {
        &self.level
    }
}

#[allow(deprecated)]
impl HasLoggerParam for LoggingMessageNotificationParams {
    fn logger(&self) -> Option<&String> {
        self.logger.as_ref()
    }
}

#[allow(deprecated)]
impl HasMetaParam for LoggingMessageNotificationParams {
    fn meta(&self) -> Option<&HashMap<String, Value>> {
        self.meta.as_ref()
    }
}

// ===========================================
// === Fine-Grained Notification Traits ===
// ===========================================

/// Trait for notification metadata (method, type info)
#[cfg(test)]
#[allow(deprecated)]
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
            crate::meta::ProgressToken::String("token123".to_string())
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
        let notification = ProgressNotification::new(
            crate::meta::ProgressToken::Number(serde_json::Number::from(42i64)),
            0.5,
        );
        let json = serde_json::to_value(&notification).unwrap();
        assert_eq!(json["params"]["progressToken"], 42);
        assert_eq!(json["params"]["progress"], 0.5);
    }

    #[test]
    fn test_request_meta_accepts_numeric_progress_token() {
        // Schema-anchor: `RequestMetaObject.progressToken?: ProgressToken` where
        // `ProgressToken = string | number`. A numeric token must round-trip
        // through the unified type at both carriers.
        use crate::meta::RequestMetaObject;
        let meta = RequestMetaObject::new(
            "DRAFT-2026-v1",
            crate::initialize::Implementation::new("c", "1"),
            crate::initialize::ClientCapabilities::default(),
        )
        .with_progress_token(7i64);
        let json = serde_json::to_value(&meta).unwrap();
        assert_eq!(json["progressToken"], 7);
        let back: RequestMetaObject = serde_json::from_value(json).unwrap();
        assert_eq!(
            back.progress_token,
            Some(crate::meta::ProgressToken::Number(
                serde_json::Number::from(7i64)
            ))
        );
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
        assert_eq!(notification.params.request_id, Some(RequestId::Number(123)));
        assert_eq!(
            notification.params.reason,
            Some("User cancelled".to_string())
        );
    }

    #[test]
    fn test_cancelled_notification_without_id() {
        let notification =
            CancelledNotification::without_id().with_reason("Request finished before cancel");

        let json = serde_json::to_value(&notification).unwrap();
        assert_eq!(json["method"], "notifications/cancelled");
        // requestId must be omitted entirely when None per schema.
        assert!(
            !json["params"]
                .as_object()
                .unwrap()
                .contains_key("requestId")
        );
        assert_eq!(json["params"]["reason"], "Request finished before cancel");
    }

    #[test]
    fn test_cancelled_notification_deserializes_without_request_id() {
        // Spec-valid late-arrival shape — `notifications/cancelled` MAY arrive
        // after the request finished, with no `requestId`.
        let wire = serde_json::json!({
            "reason": "late arrival"
        });
        let params: CancelledNotificationParams = serde_json::from_value(wire).unwrap();
        assert!(params.request_id.is_none());
        assert_eq!(params.reason.as_deref(), Some("late arrival"));
    }

    #[test]
    fn test_logging_message_notification() {
        #[allow(deprecated)] // SEP-2577 migration window
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

    // ---- Notification trait coverage ----

    #[test]
    fn tool_list_changed_satisfies_rpc_notification_and_trait() {
        // Generic function over the trait abstraction.
        fn check_method<N: ToolListChangedNotificationTrait>(n: &N) -> &str {
            n.method_string()
        }
        let n = ToolListChangedNotification::new();
        assert_eq!(HasMethod::method(&n), "notifications/tools/list_changed");
        assert_eq!(check_method(&n), "notifications/tools/list_changed");
    }

    #[test]
    fn cancelled_params_field_getters_via_trait() {
        use turul_rpc::RequestId;
        let params = CancelledNotificationParams {
            request_id: Some(RequestId::Number(42)),
            reason: Some("user clicked stop".to_string()),
            meta: None,
        };
        // Drive the field-getters through the `HasCancelledParams` trait.
        let request_id: Option<&RequestId> = HasCancelledParams::request_id(&params);
        let reason: Option<&str> = HasCancelledParams::reason(&params);
        assert_eq!(request_id, Some(&RequestId::Number(42)));
        assert_eq!(reason, Some("user clicked stop"));
    }

    #[test]
    fn progress_params_field_getters_via_trait() {
        let params = ProgressNotificationParams {
            progress_token: crate::meta::ProgressToken::Number(serde_json::Number::from(7i64)),
            progress: 0.42,
            total: Some(1.0),
            message: Some("loading...".to_string()),
            meta: None,
        };
        assert_eq!(
            HasProgressParams::progress_token(&params),
            &crate::meta::ProgressToken::Number(serde_json::Number::from(7i64))
        );
        assert_eq!(HasProgressParams::progress(&params), 0.42);
        assert_eq!(HasProgressParams::total(&params), Some(1.0));
        assert_eq!(HasProgressParams::message(&params), Some("loading..."));
    }

    #[test]
    fn resource_updated_uri_via_trait() {
        let p = ResourceUpdatedNotificationParams {
            uri: "file:///cfg.toml".to_string(),
            meta: None,
        };
        assert_eq!(HasResourceUpdatedParams::uri(&p), "file:///cfg.toml");
    }
}
