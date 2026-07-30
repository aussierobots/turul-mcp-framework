//! Middleware error types

use std::fmt;

/// JSON-RPC 2.0 error codes for middleware errors
///
/// These codes are used when converting `MiddlewareError` to `JsonRpcError`.
///
/// MCP 2026-07-28 partitions JSON-RPC's `-32000..-32099` implementation-defined
/// range: `-32000..-32019` is the legacy sub-range — new codes MUST NOT be
/// allocated in it and new implementations SHOULD NOT use it at all — and
/// `-32020..-32099` is reserved for the specification. New codes for purposes
/// the specification does not define SHOULD be allocated outside the JSON-RPC
/// reserved range `-32768..-32000`. See
/// [Error Codes](https://modelcontextprotocol.io/specification/2026-07-28/basic/index#error-codes).
///
/// The three codes below predate that partition and are frozen as legacy
/// allocations; nothing new may join them, except `UNAUTHORIZED` — relocated
/// below to correct a `MUST NOT` violation.
///
/// `UNAUTHORIZED` used to be `-32002`, one of the codes 2026-07-28 names as
/// forbidden for this version to emit — it meant "resource not found" in
/// 2025-11-25 and earlier, so a 2026 permission denial was wire-indistinguishable
/// from a missing resource. It is now `-32005`. This trades that `MUST NOT`
/// violation for a `SHOULD NOT`: `-32005` still sits in the legacy
/// `-32000..-32019` sub-range the spec says new implementations should avoid
/// entirely, rather than in the unreserved space above `-32099` the spec
/// recommends for new codes. The spec's recommended range is unreachable for
/// these three: [`map_middleware_error_to_jsonrpc`] builds them with
/// `JsonRpcErrorObject::server_error`, whose `assert!` requires the code to lie
/// in `-32099..=-32000` (a release decision of the sibling `turul-rpc` crate,
/// not this one) and panics otherwise. `INVALID_REQUEST` and `INTERNAL_ERROR`
/// are standard JSON-RPC codes and use their own constructors, so the assert
/// does not apply to them.
pub mod error_codes {
    /// Authentication required (-32001)
    pub const UNAUTHENTICATED: i64 = -32001;
    /// Permission denied (-32005). Relocated from -32002, which 2026-07-28
    /// forbids implementations of this version from emitting.
    pub const UNAUTHORIZED: i64 = -32005;
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
/// - `Unauthorized` → `-32005` "Permission denied"
/// - `RateLimitExceeded` → `-32003` "Rate limit exceeded"
/// - `InvalidRequest` → `-32600` (standard Invalid Request), with the message in
///   `data.reason`
/// - `Internal` → `-32603` (standard Internal error)
/// - `Custom{code, msg}` → `-32603`; the `code` string is application-level and
///   has no JSON-RPC number, so it does not reach the wire
/// - `HttpChallenge` → no JSON-RPC code; answered as a raw 401/403 before dispatch
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

/// Convert a middleware rejection into the JSON-RPC error response the client sees.
///
/// The sole owner of this mapping. Both transports call it, so the code a client
/// receives cannot differ by which handler served the request.
///
/// The constructor is chosen by code class, not uniformly: `-32600` and `-32603`
/// are standard JSON-RPC codes with their own constructors, while
/// `JsonRpcErrorObject::server_error` asserts the code lies in the
/// implementation-defined `-32099..=-32000`. Routing the standard codes through
/// `server_error` tripped that assert, so `InvalidRequest`, `Internal` and
/// `Custom` aborted the request instead of answering it.
///
/// `Custom` reports `-32603`: its `code` is a free-form application string with
/// no JSON-RPC number, and inventing one would put it in a range the spec governs.
///
/// # Panics
///
/// On `HttpChallenge`, which the transport answers as a raw 401/403 before
/// dispatch and must never reach here.
pub fn map_middleware_error_to_jsonrpc(
    err: MiddlewareError,
    request_id: turul_rpc::RequestId,
) -> turul_rpc::JsonRpcResponse {
    use turul_rpc::error::JsonRpcErrorObject;

    let error_obj = match err {
        MiddlewareError::Unauthenticated(msg) => JsonRpcErrorObject::server_error(
            error_codes::UNAUTHENTICATED,
            &msg,
            None::<serde_json::Value>,
        ),
        MiddlewareError::Unauthorized(msg) => JsonRpcErrorObject::server_error(
            error_codes::UNAUTHORIZED,
            &msg,
            None::<serde_json::Value>,
        ),
        MiddlewareError::RateLimitExceeded {
            message,
            retry_after,
        } => JsonRpcErrorObject::server_error(
            error_codes::RATE_LIMIT_EXCEEDED,
            &message,
            retry_after.map(|s| serde_json::json!({ "retryAfter": s })),
        ),
        MiddlewareError::InvalidRequest(msg) => {
            JsonRpcErrorObject::invalid_request(Some(serde_json::json!({ "reason": msg })))
        }
        MiddlewareError::Internal(msg) => JsonRpcErrorObject::internal_error(Some(msg)),
        MiddlewareError::Custom { message, .. } => {
            JsonRpcErrorObject::internal_error(Some(message))
        }
        MiddlewareError::HttpChallenge { .. } => {
            unreachable!("HttpChallenge must be caught at transport level before JSON-RPC dispatch")
        }
    };

    turul_rpc::JsonRpcResponse::Error(turul_rpc::JsonRpcError::new(Some(request_id), error_obj))
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

    /// Every variant a middleware can return must produce a response. Three of
    /// them used to panic: `-32600`/`-32603` fall outside the
    /// `-32099..=-32000` that `JsonRpcErrorObject::server_error` asserts, and
    /// all six were routed through it, so `InvalidRequest`, `Internal` and
    /// `Custom` aborted the request instead of answering it.
    #[test]
    fn every_returnable_variant_maps_to_a_response_without_panicking() {
        let id = turul_rpc::RequestId::Number(1);
        let cases: Vec<(MiddlewareError, i64)> = vec![
            (MiddlewareError::unauthenticated("no token"), -32001),
            (MiddlewareError::unauthorized("wrong scope"), -32005),
            (MiddlewareError::rate_limit("slow down", Some(60)), -32003),
            (MiddlewareError::invalid_request("malformed"), -32600),
            (MiddlewareError::internal("db down"), -32603),
            (MiddlewareError::custom("APP_CODE", "boom"), -32603),
        ];

        for (err, expected) in cases {
            let label = err.to_string();
            let response = map_middleware_error_to_jsonrpc(err, id.clone());
            let turul_rpc::JsonRpcResponse::Error(e) = response else {
                panic!("{label} must map to an error response");
            };
            assert_eq!(
                e.error.code,
                expected,
                "{label} must answer {expected}"
            );
        }
    }

    /// `retryAfter` is the one piece of data the mapping carries through, and a
    /// client uses it to decide when to retry.
    #[test]
    fn rate_limit_carries_retry_after_but_only_when_given() {
        let id = turul_rpc::RequestId::Number(1);

        let with = map_middleware_error_to_jsonrpc(
            MiddlewareError::rate_limit("slow down", Some(30)),
            id.clone(),
        );
        let turul_rpc::JsonRpcResponse::Error(e) = with else {
            panic!("expected an error response");
        };
        assert_eq!(
            e.error.data.as_ref().and_then(|d| d.get("retryAfter")),
            Some(&serde_json::json!(30))
        );

        let without =
            map_middleware_error_to_jsonrpc(MiddlewareError::rate_limit("slow down", None), id);
        let turul_rpc::JsonRpcResponse::Error(e) = without else {
            panic!("expected an error response");
        };
        assert!(
            e.error.data.is_none(),
            "no retry_after means no data object: {:?}",
            e.error.data
        );
    }

    /// `UNAUTHENTICATED` and `RATE_LIMIT_EXCEEDED` are frozen legacy
    /// allocations. `UNAUTHORIZED` is not frozen at its old value: `-32002` is
    /// a code 2026-07-28 lists among those implementations of this version
    /// MUST NOT emit, so it was relocated to `-32005`. None of the three may
    /// enter the spec-reserved sub-range, and `UNAUTHORIZED` specifically must
    /// never again be `-32002`.
    #[test]
    fn middleware_codes_are_frozen_legacy_allocations() {
        const FROZEN: [(&str, i64); 3] = [
            ("UNAUTHENTICATED", -32001),
            ("UNAUTHORIZED", -32005),
            ("RATE_LIMIT_EXCEEDED", -32003),
        ];
        assert_eq!(error_codes::UNAUTHENTICATED, FROZEN[0].1);
        assert_eq!(error_codes::UNAUTHORIZED, FROZEN[1].1);
        assert_eq!(error_codes::RATE_LIMIT_EXCEEDED, FROZEN[2].1);
        assert_ne!(
            error_codes::UNAUTHORIZED,
            -32002,
            "UNAUTHORIZED must never regress to -32002 — 2026-07-28 forbids \
             implementations of this version from emitting it, and it means \
             resource-not-found to every conformant peer"
        );

        for (name, code) in FROZEN {
            assert!(
                !(-32099..=-32020).contains(&code),
                "{name} emits {code}, inside the spec-reserved -32020..-32099 \
                 sub-range; implementations must not emit codes there that the \
                 specification does not define"
            );
        }
    }

    /// The per-constant guard above did not catch `session_handler.rs`, which
    /// emitted the literal `-32002` directly rather than through `error_codes`.
    /// This scans the crate's own source for the literal, so a new emit site
    /// fails regardless of how it is constructed. Source-level rather than
    /// wire-level on purpose: the invariant is "this code appears in no emit
    /// path", which no single request can demonstrate.
    #[test]
    fn no_source_file_emits_the_forbidden_resource_not_found_code() {
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut offenders = Vec::new();
        let mut stack = vec![src];
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir).expect("read src dir") {
                let path = entry.expect("dir entry").path();
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }
                if path.extension().is_none_or(|e| e != "rs") {
                    continue;
                }
                // This file names the code in its own assertions and in this
                // scan; the constants it defines are pinned by
                // `middleware_codes_are_frozen_legacy_allocations` instead.
                if path.file_name().is_some_and(|f| f == "error.rs")
                    && path.parent().is_some_and(|d| d.ends_with("middleware"))
                {
                    continue;
                }
                let text = std::fs::read_to_string(&path).expect("read source");
                for (n, line) in text.lines().enumerate() {
                    let code = line.trim_start();
                    if code.starts_with("//") {
                        continue;
                    }
                    if code.contains("-32002") {
                        offenders.push(format!("{}:{}: {}", path.display(), n + 1, code.trim()));
                    }
                }
            }
        }
        assert!(
            offenders.is_empty(),
            "2026-07-28 forbids implementations of this version from emitting \
             -32002, which means resource-not-found to every conformant peer:\n{}",
            offenders.join("\n")
        );
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
