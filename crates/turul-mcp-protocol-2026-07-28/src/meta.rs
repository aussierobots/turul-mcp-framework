//! `_meta` Field Support for MCP
//!
//! Two distinct meta carriers live here:
//! - [`Meta`] — the legacy 2025-11-25 carrier used by existing request/result
//!   types in this crate. Carries `progressToken`, pagination state, etc.
//!   Will be progressively replaced by the strictly-typed
//!   [`RequestMetaObject`] / loose [`MetaObject`] split.
//! - [`RequestMetaObject`] / [`MetaObject`] — DRAFT-2026-v1 meta carriers.
//!   `RequestMetaObject` carries the per-request capability negotiation that
//!   replaces the deleted `initialize` handshake (stateless core, SEP-2567/2575).

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Annotations for resources, prompts, and tools (matches TypeScript Annotations per MCP 2025-11-25).
/// See [MCP spec](https://modelcontextprotocol.io/specification/2025-11-25)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Annotations {
    /// Target audience for this item: "user", "assistant", or both
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audience: Option<Vec<String>>,
    /// Priority hint (0.0 = lowest, 1.0 = highest)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<f64>,
    /// ISO 8601 datetime when this item was last modified
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_modified: Option<String>,
}

impl Annotations {
    pub fn new() -> Self {
        Self {
            audience: None,
            priority: None,
            last_modified: None,
        }
    }

    pub fn with_audience(mut self, audience: Vec<String>) -> Self {
        self.audience = Some(audience);
        self
    }

    pub fn with_priority(mut self, priority: f64) -> Self {
        self.priority = Some(priority.clamp(0.0, 1.0));
        self
    }

    pub fn with_last_modified(mut self, last_modified: impl Into<String>) -> Self {
        self.last_modified = Some(last_modified.into());
        self
    }
}

impl Default for Annotations {
    fn default() -> Self {
        Self::new()
    }
}

/// Progress token for tracking long-running operations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct ProgressToken(pub String);

impl ProgressToken {
    pub fn new(token: impl Into<String>) -> Self {
        Self(token.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for ProgressToken {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for ProgressToken {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Cursor for pagination support
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct Cursor(pub String);

impl Cursor {
    pub fn new(cursor: impl Into<String>) -> Self {
        Self(cursor.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for Cursor {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for Cursor {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

// The misshapen 2025-11-25 `Meta` struct (pagination/progress fields stuffed
// into `_meta`) is removed. DRAFT-2026-v1 defines two distinct types:
//   - `MetaObject` (loose `HashMap<String, Value>`) — for Notification.params._meta
//     and Result._meta.
//   - `RequestMetaObject` (typed) — for Request.params._meta, with required
//     namespaced fields (`io.modelcontextprotocol/protocolVersion`, etc.).
//
// The `Meta::with_cursor` / `with_pagination` / `with_progress` builders were
// spec-incorrect: they encoded `cursor`, `total`, `progress`, etc. INSIDE
// `_meta`, but per schema:
//   - `cursor` belongs at `PaginatedRequestParams.cursor` (top-level)
//   - `total`, `hasMore` belong at `PaginatedResult.{total, hasMore}` (top-level)
//   - `progress`, `currentStep`, `totalSteps`, `estimatedRemainingSeconds`
//     belong at `ProgressNotificationParams.{...}` (top-level)
//
// Callers should set those fields on their schema-correct location, NOT
// wrap them in a `_meta` payload. `PaginatedResponse<T>` / `ProgressResponse<T>`
// helpers are removed for the same reason.

// ---------------------------------------------------------------------------
// Convention `_meta` keys (changelog Minor #2: OpenTelemetry trace context;
// changelog Major #4: subscription tagging; SEP-2575: per-request log level).
// These keys are not declared as named fields in the schema's RequestMetaObject
// — they ride the catch-all `[key: string]: unknown` from the MetaObject parent
// and the per-request `extra: HashMap<String, Value>` on `RequestMetaObject` /
// `NotificationParams._meta`. Typed constants here give consumers a single
// source of truth for the spelling.
// ---------------------------------------------------------------------------

/// `_meta` key carrying the W3C Trace Context `traceparent` header value per
/// SEP-414. Conventional, not schema-declared.
pub const META_KEY_TRACEPARENT: &str = "traceparent";

/// `_meta` key carrying the W3C Trace Context `tracestate` header value per
/// SEP-414. Conventional, not schema-declared.
pub const META_KEY_TRACESTATE: &str = "tracestate";

/// `_meta` key carrying the W3C Trace Context `baggage` header value per
/// SEP-414. Conventional, not schema-declared.
pub const META_KEY_BAGGAGE: &str = "baggage";

/// `_meta` key carrying the subscription id used by the server to tag
/// notifications delivered on a `subscriptions/listen` stream (changelog
/// Major #4 / SEP-2575). Conventional, not schema-declared.
pub const META_KEY_SUBSCRIPTION_ID: &str = "io.modelcontextprotocol/subscriptionId";

/// Schema-declared `_meta` key for the per-request log level opt-in.
/// Declared as the named field `io.modelcontextprotocol/logLevel` on
/// `RequestMetaObject`. The constant is provided for use when callers index
/// into a generic `MetaObject` and want the spelling pinned.
pub const META_KEY_LOG_LEVEL: &str = "io.modelcontextprotocol/logLevel";

/// Schema-declared `_meta` key for the request protocol version.
pub const META_KEY_PROTOCOL_VERSION: &str = "io.modelcontextprotocol/protocolVersion";

/// Schema-declared `_meta` key for the request client info.
pub const META_KEY_CLIENT_INFO: &str = "io.modelcontextprotocol/clientInfo";

/// Schema-declared `_meta` key for the request client capabilities.
pub const META_KEY_CLIENT_CAPABILITIES: &str = "io.modelcontextprotocol/clientCapabilities";

// ---------------------------------------------------------------------------
// DRAFT-2026-v1 _meta types.
// ---------------------------------------------------------------------------

/// Loose `_meta` carrier — `Record<string, unknown>` in the schema.
///
/// Used on `NotificationParams._meta?` and `Result._meta?`. Keys must follow
/// the reverse-DNS prefix rules; `io.modelcontextprotocol/` and `dev.mcp/`
/// are reserved for MCP use.
///
/// See [General fields: _meta](https://modelcontextprotocol.io/specification/draft/basic/index#meta).
pub type MetaObject = HashMap<String, Value>;

/// Strictly-typed request `_meta` carrying the per-request capability
/// negotiation. See also [`MetaObject`] for key naming rules and reserved
/// prefixes, and [General fields: _meta](https://modelcontextprotocol.io/specification/draft/basic/index#meta)
/// for the full spec section.
///
/// Replaces the 2025-11-25 `initialize` handshake: every DRAFT-2026-v1 request's
/// `params._meta` is REQUIRED and carries the per-request protocol version,
/// client info, and client capabilities. The server cannot infer these from
/// prior requests in the stateless model (SEP-2567, SEP-2575).
///
/// Required fields (schema: not marked `?`):
/// - `io.modelcontextprotocol/protocolVersion: string`
/// - `io.modelcontextprotocol/clientInfo: Implementation`
/// - `io.modelcontextprotocol/clientCapabilities: ClientCapabilities`
///
/// Optional fields:
/// - `progressToken?: ProgressToken` — caller opts in to `notifications/progress`
/// - `io.modelcontextprotocol/logLevel?: LoggingLevel` — replaces the removed
///   `logging/setLevel` RPC; client opts in to log notifications per-request
///
/// Extra keys per `MetaObject` rules are preserved in [`Self::extra`].
///
/// **Note on type references**: `Implementation` and `ClientCapabilities` are
/// imported from `crate::initialize`, which still holds the 2025-11-25 shapes.
/// If/when these migrate to `crate::discover`, this struct will pick up the
/// new shapes automatically via the import path.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestMetaObject {
    /// Caller-supplied progress token. Optional.
    #[serde(rename = "progressToken", skip_serializing_if = "Option::is_none")]
    pub progress_token: Option<ProgressToken>,

    /// Protocol version this request is encoded against. Required.
    ///
    /// For HTTP transport, MUST match the `MCP-Protocol-Version` header.
    /// If unsupported by the server, the server returns
    /// [`McpError::UnsupportedProtocolVersion`](crate::McpError::UnsupportedProtocolVersion).
    #[serde(rename = "io.modelcontextprotocol/protocolVersion")]
    pub protocol_version: String,

    /// Identifies the client. Required.
    #[serde(rename = "io.modelcontextprotocol/clientInfo")]
    pub client_info: crate::initialize::Implementation,

    /// Client capabilities for this specific request. Required.
    ///
    /// Per spec: "Capabilities are declared per-request rather than once at
    /// initialization; an empty object means the client supports no optional
    /// capabilities. Servers MUST NOT infer capabilities from prior requests."
    #[serde(rename = "io.modelcontextprotocol/clientCapabilities")]
    pub client_capabilities: crate::initialize::ClientCapabilities,

    /// Per-request log level opt-in. Optional.
    ///
    /// Replaces the former `logging/setLevel` RPC. If absent, the server MUST
    /// NOT send `notifications/message` for this request.
    #[serde(
        rename = "io.modelcontextprotocol/logLevel",
        skip_serializing_if = "Option::is_none"
    )]
    pub log_level: Option<crate::logging::LoggingLevel>,

    /// Additional caller-supplied meta keys per the `MetaObject` extension rules.
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

impl RequestMetaObject {
    /// Construct with the three required fields. All optionals start `None`,
    /// `extra` empty.
    pub fn new(
        protocol_version: impl Into<String>,
        client_info: crate::initialize::Implementation,
        client_capabilities: crate::initialize::ClientCapabilities,
    ) -> Self {
        Self {
            progress_token: None,
            protocol_version: protocol_version.into(),
            client_info,
            client_capabilities,
            log_level: None,
            extra: HashMap::new(),
        }
    }

    /// Attach a progress token.
    pub fn with_progress_token(mut self, token: impl Into<ProgressToken>) -> Self {
        self.progress_token = Some(token.into());
        self
    }

    /// Opt in to log notifications at this level for this request.
    pub fn with_log_level(mut self, level: crate::logging::LoggingLevel) -> Self {
        self.log_level = Some(level);
        self
    }

    /// Add an extra meta key. Key must follow [`MetaObject`] naming rules
    /// (no validation performed here; keep keys spec-conformant).
    pub fn with_extra(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.extra.insert(key.into(), value.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_progress_token() {
        let token = ProgressToken::new("task-123");
        assert_eq!(token.as_str(), "task-123");

        let from_string: ProgressToken = "task-456".into();
        assert_eq!(from_string.as_str(), "task-456");
    }

    #[test]
    fn test_cursor() {
        let cursor = Cursor::new("page-2");
        assert_eq!(cursor.as_str(), "page-2");

        let from_string: Cursor = "page-3".into();
        assert_eq!(from_string.as_str(), "page-3");
    }

    // `Meta` / `WithMeta` / `PaginatedResponse` / `ProgressResponse` tests
    // removed alongside their misshapen types. New schema-aligned tests live
    // alongside `RequestMetaObject` and `MetaObject`.
}
