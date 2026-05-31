//! `_meta` Field Support for MCP DRAFT-2026-v1.
//!
//! Two distinct meta carriers per the schema:
//! - [`RequestMetaObject`] — strictly-typed shape for every `Request.params._meta`.
//!   Carries the required `io.modelcontextprotocol/protocolVersion`, `clientInfo`,
//!   and `clientCapabilities` for per-request capability negotiation, plus
//!   optional `progressToken` and `logLevel`. Arbitrary namespaced keys ride
//!   along on the flattened `extra` map.
//! - [`MetaObject`] — loose `HashMap<String, Value>` for `Notification.params._meta`
//!   and `Result._meta`. Open key-value per the schema's reverse-DNS prefix rules.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Annotations for resources, prompts, and tools — `audience`, `priority`,
/// `last_modified`. Carried on `Resource`, `ResourceTemplate`, `Prompt`,
/// `Tool` (via `ToolAnnotations`), and `ContentBlock`.
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

/// Progress token for tracking long-running operations.
///
/// Schema: `export type ProgressToken = string | number;` — used in both
/// `RequestMetaObject.progressToken?` and `ProgressNotificationParams.progressToken`.
/// JSON `number` is any IEEE-754 double, so the numeric variant is modeled as
/// [`serde_json::Number`] to losslessly preserve the wire representation
/// (integer vs. float, large integers beyond `i64::MAX`, etc.). An `i64`
/// variant would reject spec-valid tokens like `1.5`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum ProgressToken {
    String(String),
    Number(serde_json::Number),
}

impl ProgressToken {
    /// Construct a string-form token. For numeric tokens use the `From<i64>` /
    /// `From<u64>` / `From<f64>` impls, or `ProgressToken::Number(num)` directly.
    pub fn new(token: impl Into<String>) -> Self {
        Self::String(token.into())
    }

    /// Return the string body if this is a string-form token; `None` for numeric.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s.as_str()),
            Self::Number(_) => None,
        }
    }

    /// Return the numeric body if this is an integer-form token; `None` for
    /// string or for a JSON float that has no exact `i64` representation.
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Number(n) => n.as_i64(),
            Self::String(_) => None,
        }
    }

    /// Return the numeric body as `f64` if this is a number-form token. Lossy
    /// for integers beyond `2^53`; `None` for string-form tokens.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Number(n) => n.as_f64(),
            Self::String(_) => None,
        }
    }

    /// Return the raw `serde_json::Number` for a number-form token. Use this
    /// when you need to preserve the exact wire representation (int vs float)
    /// without lossy conversion.
    pub fn as_number(&self) -> Option<&serde_json::Number> {
        match self {
            Self::Number(n) => Some(n),
            Self::String(_) => None,
        }
    }
}

impl From<String> for ProgressToken {
    fn from(s: String) -> Self {
        Self::String(s)
    }
}

impl From<&str> for ProgressToken {
    fn from(s: &str) -> Self {
        Self::String(s.to_string())
    }
}

impl From<i64> for ProgressToken {
    fn from(n: i64) -> Self {
        Self::Number(serde_json::Number::from(n))
    }
}

impl From<u64> for ProgressToken {
    fn from(n: u64) -> Self {
        Self::Number(serde_json::Number::from(n))
    }
}

impl From<i32> for ProgressToken {
    fn from(n: i32) -> Self {
        Self::Number(serde_json::Number::from(n))
    }
}

/// Construct from an `f64`. Panics if the value is NaN or ±infinity — JSON
/// has no representation for those. Use [`serde_json::Number::from_f64`]
/// directly + `ProgressToken::Number(...)` if you need explicit error handling.
impl From<f64> for ProgressToken {
    fn from(n: f64) -> Self {
        let num = serde_json::Number::from_f64(n)
            .expect("ProgressToken numeric value must be a finite JSON number (not NaN or ±Inf)");
        Self::Number(num)
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
/// [`Implementation`](crate::initialize::Implementation) and
/// [`ClientCapabilities`](crate::initialize::ClientCapabilities) are imported
/// from [`crate::initialize`].
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

    #[test]
    fn test_progress_token_string() {
        let token = ProgressToken::new("task-123");
        assert_eq!(token.as_str(), Some("task-123"));
        assert_eq!(token.as_i64(), None);
        assert_eq!(token.as_f64(), None);
        assert!(token.as_number().is_none());

        let from_string: ProgressToken = "task-456".into();
        assert_eq!(from_string.as_str(), Some("task-456"));
    }

    #[test]
    fn test_progress_token_integer_round_trips() {
        let token: ProgressToken = 42i64.into();
        assert_eq!(token.as_i64(), Some(42));
        assert_eq!(token.as_f64(), Some(42.0));
        assert_eq!(token.as_str(), None);

        // Round-trip a numeric token through serde.
        let json = serde_json::to_value(&token).unwrap();
        assert_eq!(json, serde_json::json!(42));
        let back: ProgressToken = serde_json::from_value(json).unwrap();
        assert_eq!(back, ProgressToken::Number(serde_json::Number::from(42)));
    }

    #[test]
    fn test_progress_token_float_round_trips() {
        // Schema says `ProgressToken = string | number`; `number` is IEEE-754.
        // 1.5 is a spec-valid token. Pre-fix `Number(i64)` rejected this shape.
        let json = serde_json::json!(1.5);
        let token: ProgressToken = serde_json::from_value(json.clone()).unwrap();
        assert_eq!(token.as_f64(), Some(1.5));
        assert_eq!(token.as_i64(), None); // not an integer
        let back = serde_json::to_value(&token).unwrap();
        assert_eq!(back, json);

        // Constructing from an f64 literal works via the From<f64> impl.
        let from_float: ProgressToken = 2.71828_f64.into();
        assert_eq!(from_float.as_f64(), Some(2.71828));
    }

    #[test]
    fn test_progress_token_negative_and_large_round_trip() {
        let neg: ProgressToken = (-7_i64).into();
        let big: ProgressToken = (u64::MAX).into(); // i64 would have rejected this too
        assert_eq!(neg.as_i64(), Some(-7));
        assert_eq!(big.as_i64(), None); // doesn't fit in i64
        assert_eq!(big.as_number().unwrap().as_u64(), Some(u64::MAX));
        // Round-trip through JSON.
        let big_json = serde_json::to_value(&big).unwrap();
        let big_back: ProgressToken = serde_json::from_value(big_json).unwrap();
        assert_eq!(big_back, big);
    }

    #[test]
    fn test_progress_token_deserializes_from_both_shapes() {
        let s: ProgressToken = serde_json::from_value(serde_json::json!("op-7")).unwrap();
        assert_eq!(s, ProgressToken::String("op-7".to_string()));
        let n: ProgressToken = serde_json::from_value(serde_json::json!(7)).unwrap();
        assert_eq!(n, ProgressToken::Number(serde_json::Number::from(7)));
        let f: ProgressToken = serde_json::from_value(serde_json::json!(3.14)).unwrap();
        assert_eq!(f.as_f64(), Some(3.14));
    }

    #[test]
    #[should_panic(expected = "finite JSON number")]
    fn test_progress_token_from_nan_panics() {
        // JSON has no representation for NaN; `From<f64>` rejects it loudly.
        let _: ProgressToken = f64::NAN.into();
    }

    #[test]
    fn test_progress_token_accepts_scientific_notation() {
        // JSON `number` includes scientific notation per RFC 8259 §6.
        // Verify each form deserializes into a numeric token (structural
        // coverage; serde_json::Number handles the parsing).
        let cases = [
            (serde_json::json!(1e10), Some(1e10)),
            (serde_json::json!(1.5e-3), Some(1.5e-3)),
            (serde_json::json!(1.5e3), Some(1500.0)),
        ];
        for (wire, want) in cases {
            let t: ProgressToken = serde_json::from_value(wire.clone()).unwrap();
            assert_eq!(t.as_f64(), want, "wire shape {wire} did not parse to {:?}", want);
            // Round-trip preserves the numeric body.
            let back = serde_json::to_value(&t).unwrap();
            let again: ProgressToken = serde_json::from_value(back).unwrap();
            assert_eq!(again, t);
        }
    }

    #[test]
    fn test_progress_token_rejects_wire_inf_overflow() {
        // `1e400` overflows IEEE-754 double; serde_json::Number refuses it on the wire.
        let res: Result<ProgressToken, _> = serde_json::from_str("1e400");
        assert!(res.is_err(), "spec-invalid (overflowing) number must not deserialize");
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
