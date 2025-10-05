//! Request context and session injection types

use serde_json::{Value, Map};
use std::collections::HashMap;

/// Normalized request context across all transports (HTTP, Lambda, etc.)
///
/// Provides uniform access to request data regardless of transport mechanism.
///
/// # Examples
///
/// ```rust
/// use turul_mcp_middleware::RequestContext;
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
}

impl<'a> RequestContext<'a> {
    /// Create a new request context
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
        }
    }

    /// Get the MCP method name
    pub fn method(&self) -> &str {
        self.method
    }

    /// Get request parameters (if any)
    pub fn params(&self) -> Option<&Value> {
        self.params.as_ref()
    }

    /// Get mutable request parameters
    pub fn params_mut(&mut self) -> Option<&mut Value> {
        self.params.as_mut()
    }

    /// Get transport metadata (read-only)
    pub fn metadata(&self) -> &Map<String, Value> {
        &self.metadata
    }

    /// Add metadata entry
    ///
    /// # Examples
    ///
    /// ```rust
    /// use turul_mcp_middleware::RequestContext;
    /// use serde_json::json;
    ///
    /// let mut ctx = RequestContext::new("tools/call", None);
    /// ctx.add_metadata("client-ip", json!("127.0.0.1"));
    /// ```
    pub fn add_metadata(&mut self, key: impl Into<String>, value: Value) {
        self.metadata.insert(key.into(), value);
    }
}

/// Write-only mechanism for middleware to populate session state
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
/// ```rust
/// use turul_mcp_middleware::SessionInjection;
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
    /// Create a new empty session injection
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a state entry
    ///
    /// # Parameters
    ///
    /// - `key`: State key (used with `SessionContext::get_typed_state()`)
    /// - `value`: JSON value to store
    pub fn set_state(&mut self, key: impl Into<String>, value: Value) {
        self.state.insert(key.into(), value);
    }

    /// Set a metadata entry
    ///
    /// # Parameters
    ///
    /// - `key`: Metadata key
    /// - `value`: JSON value to store
    pub fn set_metadata(&mut self, key: impl Into<String>, value: Value) {
        self.metadata.insert(key.into(), value);
    }

    /// Get all state entries (for internal use)
    pub(crate) fn state(&self) -> &HashMap<String, Value> {
        &self.state
    }

    /// Get all metadata entries (for internal use)
    pub(crate) fn metadata(&self) -> &HashMap<String, Value> {
        &self.metadata
    }

    /// Check if injection is empty
    pub fn is_empty(&self) -> bool {
        self.state.is_empty() && self.metadata.is_empty()
    }
}

/// Result from the MCP dispatcher (success or error)
///
/// Middleware can inspect and modify this result in `after_dispatch()`.
///
/// # Examples
///
/// ```rust
/// use turul_mcp_middleware::DispatcherResult;
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
    /// Check if result is successful
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success(_))
    }

    /// Check if result is an error
    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error(_))
    }

    /// Get success value (if any)
    pub fn success(&self) -> Option<&Value> {
        match self {
            Self::Success(v) => Some(v),
            Self::Error(_) => None,
        }
    }

    /// Get mutable success value (if any)
    pub fn success_mut(&mut self) -> Option<&mut Value> {
        match self {
            Self::Success(v) => Some(v),
            Self::Error(_) => None,
        }
    }

    /// Get error message (if any)
    pub fn error(&self) -> Option<&str> {
        match self {
            Self::Success(_) => None,
            Self::Error(e) => Some(e),
        }
    }
}
