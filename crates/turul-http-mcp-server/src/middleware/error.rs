//! Middleware error types

use std::fmt;

/// JSON-RPC 2.0 error codes for middleware errors
///
/// These codes are used when converting `MiddlewareError` to `JsonRpcError`.
/// Codes `-32000` to `-32099` are reserved for application-defined server errors.
pub mod error_codes {
    /// Authentication required (-32001)
    pub const UNAUTHENTICATED: i64 = -32001;
    /// Permission denied (-32002)
    pub const UNAUTHORIZED: i64 = -32002;
    /// Rate limit exceeded (-32003)
    pub const RATE_LIMIT_EXCEEDED: i64 = -32003;
    /// Invalid request (standard JSON-RPC error)
    pub const INVALID_REQUEST: i64 = -32600;
    /// Internal error (standard JSON-RPC error)
    pub const INTERNAL_ERROR: i64 = -32603;
}

/// Errors that can occur during middleware execution
///
/// These errors are converted to `McpError` by the framework and then to
/// JSON-RPC error responses. Middleware should use semantic error types
/// rather than creating JSON-RPC errors directly.
///
/// # Conversion Chain
///
/// ```text
/// MiddlewareError → McpError → JsonRpcError → HTTP/Lambda response
/// ```
///
/// # JSON-RPC Error Codes
///
/// Each error variant maps to a specific JSON-RPC error code (see [`error_codes`]):
///
/// - `Unauthenticated` → `-32001` "Authentication required"
/// - `Unauthorized` → `-32002` "Permission denied"
/// - `RateLimitExceeded` → `-32003` "Rate limit exceeded"
/// - `InvalidRequest` → `-32600` (standard Invalid Request)
/// - `Internal` → `-32603` (standard Internal error)
/// - `Custom{code, msg}` → custom code from variant
///
/// # Examples
///
/// ```rust,no_run
/// use turul_http_mcp_server::middleware::{MiddlewareError, McpMiddleware, RequestContext, SessionInjection};
/// use turul_mcp_session_storage::SessionView;
/// use async_trait::async_trait;
///
/// struct ApiKeyAuth {
///     valid_key: String,
/// }
///
/// #[async_trait]
/// impl McpMiddleware for ApiKeyAuth {
///     async fn before_dispatch(
///         &self,
///         ctx: &mut RequestContext<'_>,
///         _session: Option<&dyn SessionView>,
///         _injection: &mut SessionInjection,
///     ) -> Result<(), MiddlewareError> {
///         let key = ctx.metadata()
///             .get("api-key")
///             .and_then(|v| v.as_str())
///             .ok_or_else(|| MiddlewareError::Unauthorized("Missing API key".into()))?;
///
///         if key != self.valid_key {
///             return Err(MiddlewareError::Unauthorized("Invalid API key".into()));
///         }
///
///         Ok(())
///     }
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum MiddlewareError {
    /// Authentication required but not provided
    Unauthenticated(String),

    /// Authentication provided but insufficient permissions
    Unauthorized(String),

    /// Rate limit exceeded
    RateLimitExceeded {
        /// Human-readable message
        message: String,
        /// Seconds until limit resets
        retry_after: Option<u64>,
    },

    /// Request validation failed
    InvalidRequest(String),

    /// Internal middleware error (should not expose to client)
    Internal(String),

    /// Custom error with code and message
    Custom {
        /// Error code (for structured error handling)
        code: String,
        /// Human-readable message
        message: String,
    },

    /// HTTP-level challenge response (401/403 with WWW-Authenticate header)
    ///
    /// Used for OAuth 2.1 Bearer token challenges. This variant is handled
    /// exclusively at the transport level (pre-session phase) and produces
    /// a raw HTTP response — it NEVER reaches `map_middleware_error_to_jsonrpc()`.
    ///
    /// An `unreachable!()` guard in that function catches programming errors.
    HttpChallenge {
        /// HTTP status code (401 or 403)
        status: u16,
        /// WWW-Authenticate header value (e.g., `Bearer realm="mcp", resource_metadata="..."`)
        www_authenticate: String,
        /// Optional JSON error body
        body: Option<String>,
    },
}

impl fmt::Display for MiddlewareError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unauthenticated(msg) => write!(f, "Authentication required: {}", msg),
            Self::Unauthorized(msg) => write!(f, "Unauthorized: {}", msg),
            Self::RateLimitExceeded {
                message,
                retry_after,
            } => {
                if let Some(seconds) = retry_after {
                    write!(f, "{} (retry after {} seconds)", message, seconds)
                } else {
                    write!(f, "{}", message)
                }
            }
            Self::InvalidRequest(msg) => write!(f, "Invalid request: {}", msg),
            Self::Internal(msg) => write!(f, "Internal middleware error: {}", msg),
            Self::Custom { code, message } => write!(f, "{}: {}", code, message),
            Self::HttpChallenge {
                status,
                www_authenticate,
                ..
            } => write!(f, "HTTP {} WWW-Authenticate: {}", status, www_authenticate),
        }
    }
}

impl std::error::Error for MiddlewareError {}

impl MiddlewareError {
    /// Create an unauthenticated error
    pub fn unauthenticated(msg: impl Into<String>) -> Self {
        Self::Unauthenticated(msg.into())
    }

    /// Create an unauthorized error
    pub fn unauthorized(msg: impl Into<String>) -> Self {
        Self::Unauthorized(msg.into())
    }

    /// Create a rate limit error
    pub fn rate_limit(msg: impl Into<String>, retry_after: Option<u64>) -> Self {
        Self::RateLimitExceeded {
            message: msg.into(),
            retry_after,
        }
    }

    /// Create an invalid request error
    pub fn invalid_request(msg: impl Into<String>) -> Self {
        Self::InvalidRequest(msg.into())
    }

    /// Create an internal error
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal(msg.into())
    }

    /// Create a custom error
    pub fn custom(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Custom {
            code: code.into(),
            message: message.into(),
        }
    }

    /// Create an HTTP challenge error (401/403 with WWW-Authenticate header)
    ///
    /// Used for OAuth 2.1 Bearer token challenges. Handled at transport level only.
    pub fn http_challenge(status: u16, www_authenticate: impl Into<String>) -> Self {
        Self::HttpChallenge {
            status,
            www_authenticate: www_authenticate.into(),
            body: None,
        }
    }

    /// Create an HTTP challenge error with a response body
    pub fn http_challenge_with_body(
        status: u16,
        www_authenticate: impl Into<String>,
        body: impl Into<String>,
    ) -> Self {
        Self::HttpChallenge {
            status,
            www_authenticate: www_authenticate.into(),
            body: Some(body.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = MiddlewareError::unauthenticated("Missing token");
        assert_eq!(err.to_string(), "Authentication required: Missing token");

        let err = MiddlewareError::unauthorized("Insufficient permissions");
        assert_eq!(err.to_string(), "Unauthorized: Insufficient permissions");

        let err = MiddlewareError::rate_limit("Too many requests", Some(60));
        assert_eq!(
            err.to_string(),
            "Too many requests (retry after 60 seconds)"
        );

        let err = MiddlewareError::rate_limit("Too many requests", None);
        assert_eq!(err.to_string(), "Too many requests");

        let err = MiddlewareError::invalid_request("Malformed params");
        assert_eq!(err.to_string(), "Invalid request: Malformed params");

        let err = MiddlewareError::internal("Database connection failed");
        assert_eq!(
            err.to_string(),
            "Internal middleware error: Database connection failed"
        );

        let err = MiddlewareError::custom("CUSTOM_ERROR", "Something went wrong");
        assert_eq!(err.to_string(), "CUSTOM_ERROR: Something went wrong");
    }

    #[test]
    fn test_error_equality() {
        let err1 = MiddlewareError::unauthenticated("test");
        let err2 = MiddlewareError::unauthenticated("test");
        assert_eq!(err1, err2);

        let err3 = MiddlewareError::rate_limit("test", Some(60));
        let err4 = MiddlewareError::rate_limit("test", Some(60));
        assert_eq!(err3, err4);
    }

    #[test]
    fn test_http_challenge_variant_display() {
        let err = MiddlewareError::http_challenge(401, "Bearer realm=\"mcp\"");
        assert_eq!(
            err.to_string(),
            "HTTP 401 WWW-Authenticate: Bearer realm=\"mcp\""
        );

        let err = MiddlewareError::http_challenge(403, "Bearer error=\"insufficient_scope\"");
        assert_eq!(
            err.to_string(),
            "HTTP 403 WWW-Authenticate: Bearer error=\"insufficient_scope\""
        );
    }

    #[test]
    fn test_http_challenge_constructor() {
        let err = MiddlewareError::http_challenge(401, "Bearer realm=\"mcp\"");
        match &err {
            MiddlewareError::HttpChallenge {
                status,
                www_authenticate,
                body,
            } => {
                assert_eq!(*status, 401);
                assert_eq!(www_authenticate, "Bearer realm=\"mcp\"");
                assert!(body.is_none());
            }
            _ => panic!("Expected HttpChallenge variant"),
        }

        let err_with_body = MiddlewareError::http_challenge_with_body(
            401,
            "Bearer realm=\"mcp\"",
            r#"{"error":"unauthorized"}"#,
        );
        match &err_with_body {
            MiddlewareError::HttpChallenge {
                status,
                www_authenticate,
                body,
            } => {
                assert_eq!(*status, 401);
                assert_eq!(www_authenticate, "Bearer realm=\"mcp\"");
                assert_eq!(body.as_deref(), Some(r#"{"error":"unauthorized"}"#));
            }
            _ => panic!("Expected HttpChallenge variant"),
        }
    }

    #[test]
    fn test_http_challenge_roundtrip_equality() {
        let err1 = MiddlewareError::http_challenge(401, "Bearer realm=\"mcp\"");
        let err2 = MiddlewareError::http_challenge(401, "Bearer realm=\"mcp\"");
        assert_eq!(err1, err2);

        let err3 = MiddlewareError::http_challenge(401, "Bearer realm=\"mcp\"");
        let err4 = MiddlewareError::http_challenge(403, "Bearer realm=\"mcp\"");
        assert_ne!(err3, err4);
    }
}
