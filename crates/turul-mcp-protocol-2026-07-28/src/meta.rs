//! `_meta` Field Support for MCP 2026-07-28.
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
    pub audience: Option<Vec<crate::prompts::Role>>,
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

    pub fn with_audience(mut self, audience: Vec<crate::prompts::Role>) -> Self {
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
/// notifications delivered on a `subscriptions/listen` stream. Schema-declared:
/// `NotificationMetaObject.io.modelcontextprotocol/subscriptionId` (optional;
/// value = the `RequestId` of the `subscriptions/listen` request that opened
/// the stream) and `SubscriptionsListenResultMeta.io.modelcontextprotocol/subscriptionId`
/// (required, on the result that closes the stream).
pub const META_KEY_SUBSCRIPTION_ID: &str = "io.modelcontextprotocol/subscriptionId";

/// Schema-declared `_meta` key for the per-request log level opt-in.
/// Declared as the named field `io.modelcontextprotocol/logLevel` on
/// `RequestMetaObject`. The constant is provided for use when callers index
/// into a generic `MetaObject` and want the spelling pinned.
#[deprecated(
    since = "0.4.0",
    note = "Deprecated per SEP-2577 (2026-07-28) along with the whole Logging surface. \
            Replacement: stderr (stdio) or OpenTelemetry. \
            Earliest removal: first release on/after 2027-07-28."
)]
pub const META_KEY_LOG_LEVEL: &str = "io.modelcontextprotocol/logLevel";

/// Schema-declared `_meta` key for the request protocol version.
pub const META_KEY_PROTOCOL_VERSION: &str = "io.modelcontextprotocol/protocolVersion";

/// Schema-declared `_meta` key for the request client info.
pub const META_KEY_CLIENT_INFO: &str = "io.modelcontextprotocol/clientInfo";

/// Schema-declared `_meta` key for the request client capabilities.
pub const META_KEY_CLIENT_CAPABILITIES: &str = "io.modelcontextprotocol/clientCapabilities";

/// Schema-declared `_meta` key for the responding server's implementation info.
pub const META_KEY_SERVER_INFO: &str = "io.modelcontextprotocol/serverInfo";

// ---------------------------------------------------------------------------
// 2026-07-28 _meta types.
// ---------------------------------------------------------------------------

/// Loose `_meta` carrier — `Record<string, unknown>` in the schema.
///
/// Used on `NotificationParams._meta?` and `Result._meta?`. Keys must follow
/// the reverse-DNS prefix rules; `io.modelcontextprotocol/` and `dev.mcp/`
/// are reserved for MCP use.
///
/// See [General fields: _meta](https://modelcontextprotocol.io/specification/draft/basic/index#meta).
pub type MetaObject = HashMap<String, Value>;

/// `_meta` for results — [`MetaObject`] plus the responding server's identity.
///
/// `Result._meta?` is typed as this across every result in the schema, so any
/// result may carry `io.modelcontextprotocol/serverInfo`. Servers SHOULD send
/// it unless configured otherwise. Like `clientInfo` on the request side the
/// value is self-reported and unverified: clients MUST NOT key behavior on it
/// and MUST NOT treat it as a security identity.
///
/// See [General fields: _meta](https://modelcontextprotocol.io/specification/draft/basic/index#meta).
///
/// `Serialize` is hand-written for the same reason as [`RequestMetaObject`]:
/// `extra` is public and caller-writable, so a caller could otherwise insert
/// the reserved `io.modelcontextprotocol/serverInfo` key into it and emit the
/// same key twice. The typed field wins; a colliding `extra` entry is dropped.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ResultMetaObject {
    /// Identifies the responding server. Optional.
    #[serde(rename = "io.modelcontextprotocol/serverInfo")]
    pub server_info: Option<crate::initialize::Implementation>,

    /// Additional keys per the [`MetaObject`] extension rules.
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

impl ResultMetaObject {
    /// Carry the responding server's identity.
    pub fn with_server_info(mut self, info: crate::initialize::Implementation) -> Self {
        self.server_info = Some(info);
        self
    }

    /// Add an extra meta key. Key must follow [`MetaObject`] naming rules
    /// (no validation performed here; keep keys spec-conformant).
    pub fn with_extra(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.extra.insert(key.into(), value.into());
        self
    }

    /// True when nothing would be emitted — lets callers skip an empty `_meta`.
    pub fn is_empty(&self) -> bool {
        self.server_info.is_none()
            && self
                .extra
                .keys()
                .all(|k| k.as_str() == META_KEY_SERVER_INFO)
    }
}

impl From<MetaObject> for ResultMetaObject {
    /// Lift a loose `_meta` map into the typed carrier.
    ///
    /// `io.modelcontextprotocol/serverInfo` is reserved: the typed field owns
    /// it, and `Serialize` never emits that key from `extra`. So a value that
    /// does not parse as an `Implementation` is **dropped, not preserved** —
    /// re-homing it in `extra` would only look preserved in memory while
    /// vanishing on the wire, and emitting it as-is would put a value on the
    /// wire under a reserved key whose declared shape it does not satisfy.
    /// Every other key is carried through untouched.
    ///
    /// The drop is silent: this crate binds the schema and takes no logging
    /// dependency, so there is nowhere here to warn from. Callers that need to
    /// detect a malformed reserved entry must inspect the map before
    /// converting.
    fn from(mut map: MetaObject) -> Self {
        let server_info = map
            .remove(META_KEY_SERVER_INFO)
            .and_then(|v| serde_json::from_value(v).ok());
        Self {
            server_info,
            extra: map,
        }
    }
}

impl Serialize for ResultMetaObject {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;

        let extra_len = self
            .extra
            .keys()
            .filter(|k| k.as_str() != META_KEY_SERVER_INFO)
            .count();
        let len = extra_len + usize::from(self.server_info.is_some());

        let mut map = serializer.serialize_map(Some(len))?;
        if let Some(info) = &self.server_info {
            map.serialize_entry(META_KEY_SERVER_INFO, info)?;
        }
        for (k, v) in &self.extra {
            if k.as_str() != META_KEY_SERVER_INFO {
                map.serialize_entry(k, v)?;
            }
        }
        map.end()
    }
}

/// Strictly-typed request `_meta` carrying the per-request capability
/// negotiation. See also [`MetaObject`] for key naming rules and reserved
/// prefixes, and [General fields: _meta](https://modelcontextprotocol.io/specification/draft/basic/index#meta)
/// for the full spec section.
///
/// Replaces the 2025-11-25 `initialize` handshake: every 2026-07-28 request's
/// `params._meta` is REQUIRED and carries the per-request protocol version,
/// client info, and client capabilities. The server cannot infer these from
/// prior requests in the stateless model (SEP-2567, SEP-2575).
///
/// Required fields (schema: not marked `?`):
/// - `io.modelcontextprotocol/protocolVersion: string`
/// - `io.modelcontextprotocol/clientCapabilities: ClientCapabilities`
///
/// Optional fields:
/// - `progressToken?: ProgressToken` — caller opts in to `notifications/progress`
/// - `io.modelcontextprotocol/clientInfo?: Implementation` — clients SHOULD send
///   it unless configured otherwise. It is self-reported and unverified: servers
///   MUST NOT key behavior on it and MUST NOT treat it as a security identity.
///   A request that omits it is well-formed and MUST be served.
/// - `io.modelcontextprotocol/logLevel?: LoggingLevel` — replaces the removed
///   `logging/setLevel` RPC; client opts in to log notifications per-request
///
/// Extra keys per `MetaObject` rules are preserved in [`Self::extra`].
///
/// [`Implementation`](crate::initialize::Implementation) and
/// [`ClientCapabilities`](crate::initialize::ClientCapabilities) are imported
/// from [`crate::initialize`].
///
/// `Serialize` is hand-written rather than `#[derive]` + `#[serde(flatten)]`:
/// `extra` is public and caller-writable, so a caller could otherwise insert
/// one of the reserved typed keys (`progressToken` or any
/// `io.modelcontextprotocol/*` field) into it and produce the same key twice
/// on the wire. The typed field always wins; a colliding `extra` entry is
/// dropped rather than emitted. Mirrors the same guard on
/// [`crate::subscriptions::SubscriptionsListenResultMeta`].
#[allow(deprecated)] // carries the SEP-2577-deprecated log_level through the migration window
#[derive(Debug, Clone, Deserialize)]
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

    /// Identifies the client. Optional — absent is well-formed. Present but not
    /// a valid `Implementation` is still a parse failure.
    #[serde(rename = "io.modelcontextprotocol/clientInfo")]
    pub client_info: Option<crate::initialize::Implementation>,

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
    /// NOT send `notifications/message` for this request. Deprecated per
    /// SEP-2577 along with the whole Logging surface; functional through the
    /// migration window.
    #[deprecated(
        since = "0.4.0",
        note = "Deprecated per SEP-2577 (2026-07-28) along with the whole Logging surface. \
                Earliest removal: first release on/after 2027-07-28."
    )]
    #[serde(
        rename = "io.modelcontextprotocol/logLevel",
        skip_serializing_if = "Option::is_none"
    )]
    #[allow(deprecated)]
    pub log_level: Option<crate::logging::LoggingLevel>,

    /// Additional caller-supplied meta keys per the `MetaObject` extension rules.
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

impl RequestMetaObject {
    /// Construct with the two required fields plus `client_info`, which clients
    /// SHOULD send. Use [`Self::without_client_info`] for the configured-off
    /// case. All other optionals start `None`, `extra` empty.
    pub fn new(
        protocol_version: impl Into<String>,
        client_info: crate::initialize::Implementation,
        client_capabilities: crate::initialize::ClientCapabilities,
    ) -> Self {
        Self {
            progress_token: None,
            protocol_version: protocol_version.into(),
            client_info: Some(client_info),
            client_capabilities,
            #[allow(deprecated)]
            log_level: None,
            extra: HashMap::new(),
        }
    }

    /// Drop `clientInfo`, modelling a client configured not to report itself.
    pub fn without_client_info(mut self) -> Self {
        self.client_info = None;
        self
    }

    /// Attach a progress token.
    pub fn with_progress_token(mut self, token: impl Into<ProgressToken>) -> Self {
        self.progress_token = Some(token.into());
        self
    }

    /// Opt in to log notifications at this level for this request.
    /// Deprecated per SEP-2577 along with the whole Logging surface.
    #[allow(deprecated)]
    #[deprecated(
        since = "0.4.0",
        note = "Deprecated per SEP-2577 (2026-07-28) along with the whole Logging surface."
    )]
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

#[allow(deprecated)] // log_level field access carries the SEP-2577 deprecation
impl Serialize for RequestMetaObject {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;

        const RESERVED_EXTRA_KEYS: [&str; 5] = [
            "progressToken",
            META_KEY_PROTOCOL_VERSION,
            META_KEY_CLIENT_INFO,
            META_KEY_CLIENT_CAPABILITIES,
            META_KEY_LOG_LEVEL,
        ];

        let extra_len = self
            .extra
            .keys()
            .filter(|k| !RESERVED_EXTRA_KEYS.contains(&k.as_str()))
            .count();
        let len = extra_len
            + usize::from(self.progress_token.is_some())
            + 2 // protocol_version, client_capabilities
            + usize::from(self.client_info.is_some())
            + usize::from(self.log_level.is_some());

        let mut map = serializer.serialize_map(Some(len))?;
        if let Some(token) = &self.progress_token {
            map.serialize_entry("progressToken", token)?;
        }
        map.serialize_entry(META_KEY_PROTOCOL_VERSION, &self.protocol_version)?;
        if let Some(info) = &self.client_info {
            map.serialize_entry(META_KEY_CLIENT_INFO, info)?;
        }
        map.serialize_entry(META_KEY_CLIENT_CAPABILITIES, &self.client_capabilities)?;
        if let Some(level) = &self.log_level {
            map.serialize_entry(META_KEY_LOG_LEVEL, level)?;
        }
        for (k, v) in &self.extra {
            if !RESERVED_EXTRA_KEYS.contains(&k.as_str()) {
                map.serialize_entry(k, v)?;
            }
        }
        map.end()
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
    #[allow(clippy::approx_constant)] // 2.71828 is an arbitrary test float, not E
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
    #[allow(clippy::approx_constant)] // 3.14 is an arbitrary test float, not PI
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
            assert_eq!(
                t.as_f64(),
                want,
                "wire shape {wire} did not parse to {:?}",
                want
            );
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
        assert!(
            res.is_err(),
            "spec-invalid (overflowing) number must not deserialize"
        );
    }

    #[test]
    fn test_cursor() {
        let cursor = Cursor::new("page-2");
        assert_eq!(cursor.as_str(), "page-2");

        let from_string: Cursor = "page-3".into();
        assert_eq!(from_string.as_str(), "page-3");
    }

    // Schema-aligned tests for `RequestMetaObject` and `MetaObject` live
    // alongside those type definitions.

    #[test]
    fn request_meta_object_extra_cannot_shadow_protocol_version() {
        // `RequestMetaObject.extra` is a public, caller-writable
        // `#[serde(flatten)]` map. If a caller populates it with a reserved
        // typed key, the typed field and the flattened map must not both
        // emit it on the wire. Checked against raw serialized text:
        // `to_value()` cannot observe a duplicate key (a `Map` silently
        // overwrites on the second insert).
        let mut meta = RequestMetaObject::new(
            "2026-07-28",
            crate::initialize::Implementation::new("test-client", "1.0.0"),
            crate::initialize::ClientCapabilities::default(),
        );
        meta.extra.insert(
            META_KEY_PROTOCOL_VERSION.to_string(),
            Value::String("attacker-controlled".to_string()),
        );

        let json_str = serde_json::to_string(&meta).unwrap();
        assert_eq!(
            json_str.matches(META_KEY_PROTOCOL_VERSION).count(),
            1,
            "must emit protocolVersion exactly once on the wire: {json_str}"
        );
        let v: Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(v[META_KEY_PROTOCOL_VERSION], "2026-07-28");
    }

    #[test]
    fn request_meta_object_extra_cannot_shadow_progress_token() {
        // Same collision, but against the non-namespaced `progressToken` key.
        let mut meta = RequestMetaObject::new(
            "2026-07-28",
            crate::initialize::Implementation::new("test-client", "1.0.0"),
            crate::initialize::ClientCapabilities::default(),
        )
        .with_progress_token("real-token");
        meta.extra.insert(
            "progressToken".to_string(),
            Value::String("attacker-controlled".to_string()),
        );

        let json_str = serde_json::to_string(&meta).unwrap();
        assert_eq!(
            json_str.matches("progressToken").count(),
            1,
            "must emit progressToken exactly once on the wire: {json_str}"
        );
        let v: Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(v["progressToken"], "real-token");
    }

    /// A loose `_meta` map carrying the reserved `serverInfo` key must land on
    /// the TYPED field, not stay in `extra` — otherwise the hand-written
    /// `Serialize` would emit the key from both places.
    #[test]
    fn loose_map_lifts_server_info_into_the_typed_field() {
        let mut map = MetaObject::new();
        map.insert(
            META_KEY_SERVER_INFO.to_string(),
            serde_json::json!({ "name": "srv", "version": "0.4.0" }),
        );
        map.insert("vendor.example/trace".to_string(), serde_json::json!("abc"));

        let carried: ResultMetaObject = map.into();
        assert_eq!(carried.server_info.as_ref().map(|i| i.name.as_str()), Some("srv"));
        assert!(!carried.extra.contains_key(META_KEY_SERVER_INFO));

        // Raw text: a `Value` map would collapse a duplicate and hide it.
        let raw = serde_json::to_string(&carried).unwrap();
        assert_eq!(
            raw.matches(META_KEY_SERVER_INFO).count(),
            1,
            "serverInfo must be emitted exactly once: {raw}"
        );
        assert!(raw.contains("vendor.example/trace"), "{raw}");
    }

    /// A reserved `serverInfo` whose value is not an `Implementation` is
    /// dropped. Asserted on the SERIALIZED form: an earlier version re-homed it
    /// in `extra`, which looked preserved in memory while `Serialize` filtered
    /// it out, so an in-memory assertion could not see the loss.
    #[test]
    fn malformed_server_info_is_dropped_on_the_wire() {
        let mut map = MetaObject::new();
        map.insert(
            META_KEY_SERVER_INFO.to_string(),
            serde_json::json!("not-an-object"),
        );
        map.insert("vendor.example/keep".to_string(), serde_json::json!("kept"));

        let carried: ResultMetaObject = map.into();
        assert!(carried.server_info.is_none());
        assert!(!carried.extra.contains_key(META_KEY_SERVER_INFO));

        let raw = serde_json::to_string(&carried).unwrap();
        assert!(
            !raw.contains("not-an-object"),
            "a malformed reserved entry must not reach the wire: {raw}"
        );
        assert!(
            !raw.contains(META_KEY_SERVER_INFO),
            "the reserved key must not be emitted with no valid value: {raw}"
        );
        assert!(raw.contains("vendor.example/keep"), "other keys survive: {raw}");
    }

    /// A well-formed `serverInfo` survives the full loose-map -> typed ->
    /// wire -> typed round trip.
    #[test]
    fn valid_server_info_round_trips_through_the_wire() {
        let mut map = MetaObject::new();
        map.insert(
            META_KEY_SERVER_INFO.to_string(),
            serde_json::json!({ "name": "srv", "version": "0.4.0" }),
        );

        let carried: ResultMetaObject = map.into();
        let raw = serde_json::to_string(&carried).unwrap();
        let back: ResultMetaObject = serde_json::from_str(&raw).unwrap();

        let info = back.server_info.expect("serverInfo survives the round trip");
        assert_eq!(info.name, "srv");
        assert_eq!(info.version, "0.4.0");
        assert!(!back.extra.contains_key(META_KEY_SERVER_INFO));
    }
}
