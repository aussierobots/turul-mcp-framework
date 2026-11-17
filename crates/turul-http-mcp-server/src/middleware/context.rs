//! Request context and session injection types.

use serde_json::{Map, Value};
use std::collections::HashMap;

/// Holds the normalized request context across all transports (HTTP, Lambda, etc.).
///
/// Provides uniform access to request data regardless of transport mechanism.
///
/// # Examples
///
/// ```rust,no_run
/// use turul_http_mcp_server::middleware::RequestContext;
/// use serde_json::json;
///
/// let mut ctx = RequestContext::new(
///     "tools/call",
///     Some(json!({"name": "calculator"})),
/// );
///
/// ctx.add_metadata("user-agent", json!("Claude-Code/1.0"));
///
/// assert_eq!(ctx.method(), "tools/call");
/// assert!(ctx.params().is_some());
/// assert_eq!(ctx.metadata().get("user-agent").unwrap(), "Claude-Code/1.0");
/// ```
#[derive(Debug, Clone)]
pub struct RequestContext<'a> {
    /// MCP method name (e.g., "tools/call", "resources/read")
    method: &'a str,

    /// Request parameters (JSON-RPC params field)
    params: Option<Value>,

    /// Transport-specific metadata (HTTP headers, Lambda event fields, etc.)
    metadata: Map<String, Value>,

    /// Bearer token extracted from Authorization header (never stored in `metadata`)
    bearer_token: Option<String>,
    authorization_malformed: bool,

    /// Request-scoped extensions for passing data between middleware and tools
    ///
    /// Written by pre-session middleware (e.g., auth claims), threaded through
    /// to `SessionContext.extensions` for tool access. Never persisted to storage.
    extensions: HashMap<String, Value>,
}

impl<'a> RequestContext<'a> {
    /// Creates a new request context.
    ///
    /// # Parameters
    ///
    /// - `method`: MCP method name (e.g., "tools/call")
    /// - `params`: Optional request parameters
    pub fn new(method: &'a str, params: Option<Value>) -> Self {
        Self {
            method,
            params,
            metadata: Map::new(),
            bearer_token: None,
            authorization_malformed: false,
            extensions: HashMap::new(),
        }
    }

    /// Returns the MCP method name.
    pub fn method(&self) -> &str {
        self.method
    }

    /// Returns the request parameters, if any.
    pub fn params(&self) -> Option<&Value> {
        self.params.as_ref()
    }

    /// Returns the request parameters mutably, if any.
    pub fn params_mut(&mut self) -> Option<&mut Value> {
        self.params.as_mut()
    }

    /// Returns the transport metadata (read-only).
    pub fn metadata(&self) -> &Map<String, Value> {
        &self.metadata
    }

    /// Adds a metadata entry.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use turul_http_mcp_server::middleware::RequestContext;
    /// use serde_json::json;
    ///
    /// let mut ctx = RequestContext::new("tools/call", None);
    /// ctx.add_metadata("client-ip", json!("127.0.0.1"));
    /// ```
    pub fn add_metadata(&mut self, key: impl Into<String>, value: Value) {
        self.metadata.insert(key.into(), value);
    }

    /// Returns the Bearer token, if one was extracted from the `Authorization` header.
    pub fn bearer_token(&self) -> Option<&str> {
        self.bearer_token.as_deref()
    }

    /// Sets the Bearer token (called by the transport during extraction).
    pub fn set_bearer_token(&mut self, token: String) {
        self.bearer_token = Some(token);
    }

    /// Returns whether an `Authorization` header was present but not a parseable
    /// Bearer credential. Auth middleware uses this to answer 400 invalid_request
    /// (RFC 6750 §3.1, "Malformed authorization request") instead of the
    /// missing-credentials 401.
    pub fn authorization_malformed(&self) -> bool {
        self.authorization_malformed
    }

    /// Marks the request's `Authorization` header as present-but-malformed
    /// (called by the transport during extraction).
    pub fn set_authorization_malformed(&mut self, malformed: bool) {
        self.authorization_malformed = malformed;
    }

    /// Returns the request-scoped extensions (read-only).
    pub fn extensions(&self) -> &HashMap<String, Value> {
        &self.extensions
    }

    /// Sets an extension value (used by middleware to pass data to tools).
    pub fn set_extension(&mut self, key: impl Into<String>, value: Value) {
        self.extensions.insert(key.into(), value);
    }

    /// Returns the extension value for `key`, if present.
    pub fn get_extension(&self, key: &str) -> Option<&Value> {
        self.extensions.get(key)
    }

    /// Takes ownership of the extensions, leaving them empty (used by the transport to thread them into `SessionContext`).
    pub fn take_extensions(&mut self) -> HashMap<String, Value> {
        std::mem::take(&mut self.extensions)
    }
}

/// Provides a write-only mechanism for middleware to populate session state.
///
/// Prevents middleware from interfering with core session management while
/// allowing controlled injection of custom state and metadata.
///
/// # Design
///
/// - **Write-only**: Middleware cannot read existing session state
/// - **Deferred application**: Changes applied after all middleware succeed
/// - **Isolation**: Each middleware's changes are independent
///
/// # Examples
///
/// ```rust,no_run
/// use turul_http_mcp_server::middleware::SessionInjection;
/// use serde_json::json;
///
/// let mut injection = SessionInjection::new();
///
/// // Set typed state
/// injection.set_state("user_id", json!(12345));
/// injection.set_state("role", json!("admin"));
///
/// // Set metadata
/// injection.set_metadata("authenticated_at", json!("2025-10-04T12:00:00Z"));
///
/// // State and metadata are applied to session after middleware succeeds
/// assert!(!injection.is_empty());
/// ```
#[derive(Debug, Default, Clone)]
pub struct SessionInjection {
    /// State entries to inject into session
    state: HashMap<String, Value>,

    /// Metadata entries to inject into session
    metadata: HashMap<String, Value>,
}

impl SessionInjection {
    /// Creates a new empty session injection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets a state entry.
    ///
    /// # Parameters
    ///
    /// - `key`: State key (used with `SessionContext::get_typed_state()`)
    /// - `value`: JSON value to store
    pub fn set_state(&mut self, key: impl Into<String>, value: Value) {
        self.state.insert(key.into(), value);
    }

    /// Sets a metadata entry.
    ///
    /// # Parameters
    ///
    /// - `key`: Metadata key
    /// - `value`: JSON value to store
    pub fn set_metadata(&mut self, key: impl Into<String>, value: Value) {
        self.metadata.insert(key.into(), value);
    }

    /// Returns all state entries (for internal use).
    pub(crate) fn state(&self) -> &HashMap<String, Value> {
        &self.state
    }

    /// Returns all metadata entries (for internal use).
    pub(crate) fn metadata(&self) -> &HashMap<String, Value> {
        &self.metadata
    }

    /// Returns whether the injection holds no state or metadata entries.
    pub fn is_empty(&self) -> bool {
        self.state.is_empty() && self.metadata.is_empty()
    }
}

/// Holds the result from the MCP dispatcher (success or error).
///
/// Middleware can inspect and modify this result in `after_dispatch()`.
///
/// # Examples
///
/// ```rust,no_run
/// use turul_http_mcp_server::middleware::DispatcherResult;
/// use serde_json::json;
///
/// let mut result = DispatcherResult::Success(json!({"output": "Hello"}));
///
/// // Middleware can transform successful responses
/// if let DispatcherResult::Success(ref mut value) = result {
///     if let Some(obj) = value.as_object_mut() {
///         obj.insert("timestamp".to_string(), json!("2025-10-04T12:00:00Z"));
///     }
/// }
/// ```
#[derive(Debug, Clone)]
pub enum DispatcherResult {
    /// Successful response (JSON-RPC result field)
    Success(Value),

    /// Error response (will be converted to JSON-RPC error)
    Error(String),
}

impl DispatcherResult {
    /// Returns whether the result is a success.
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success(_))
    }

    /// Returns whether the result is an error.
    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error(_))
    }

    /// Returns the success value, if any.
    pub fn success(&self) -> Option<&Value> {
        match self {
            Self::Success(v) => Some(v),
            Self::Error(_) => None,
        }
    }

    /// Returns the success value mutably, if any.
    pub fn success_mut(&mut self) -> Option<&mut Value> {
        match self {
            Self::Success(v) => Some(v),
            Self::Error(_) => None,
        }
    }

    /// Returns the error message, if any.
    pub fn error(&self) -> Option<&str> {
        match self {
            Self::Success(_) => None,
            Self::Error(e) => Some(e),
        }
    }
}
