//! Error types for MCP client operations

use serde_json::Value;
use thiserror::Error;

/// Result type for MCP client operations
pub type McpClientResult<T> = Result<T, McpClientError>;

/// Comprehensive error type for MCP client operations
#[derive(Error, Debug)]
pub enum McpClientError {
    /// Transport-level errors
    #[error("Transport error: {0}")]
    Transport(#[from] TransportError),

    /// Protocol-level errors
    #[error("Protocol error: {0}")]
    Protocol(#[from] ProtocolError),

    /// Session management errors
    #[error("Session error: {0}")]
    Session(#[from] SessionError),

    /// Authentication/authorization errors
    #[error("Authentication error: {0}")]
    Auth(String),

    /// Configuration errors
    #[error("Configuration error: {0}")]
    Config(String),

    /// Network/connection errors
    #[error("Connection error: {0}")]
    Connection(#[from] reqwest::Error),

    /// JSON parsing errors
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// MRTR (SEP-2322): the server returned `resultType: "input_required"`.
    /// Gather the requested inputs and retry the original call with
    /// `call_tool_with_input_responses`, echoing `request_state` verbatim.
    #[error("Input required: the server needs client input before completing")]
    InputRequired {
        /// The server's `inputRequests` map (id → elicit/sampling/roots request).
        input_requests: Option<serde_json::Value>,
        /// Opaque state to echo verbatim on the retry.
        request_state: Option<String>,
    },

    /// Timeout errors
    #[error("Operation timed out")]
    Timeout,

    /// Server returned an error
    #[error("Server error (code {code}): {message}")]
    ServerError {
        /// JSON-RPC error code returned by the server.
        code: i32,
        /// Human-readable error message returned by the server.
        message: String,
        /// Optional structured `data` member of the JSON-RPC error, when the server sent one.
        data: Option<Value>,
    },

    /// Generic error with context
    #[error("Error: {message}")]
    Generic {
        /// Description of what went wrong.
        message: String,
    },

    /// The server did not declare the extension this call belongs to, so the
    /// call was not sent (an extension's methods may be issued only after the
    /// server declares it).
    #[error("Server did not declare extension {0}")]
    ExtensionNotDeclared(String),

    /// A skill file failed verification against its entry (size, digest,
    /// frontmatter, or not listed in the manifest). Its content must not be
    /// used; refresh the entry with `skills_get` and retry.
    #[error("Skill verification failed for {uri}: {reason}")]
    SkillVerification {
        /// URI of the skill file that failed verification.
        uri: String,
        /// Why the file was declined, e.g. not in the manifest or a size, digest or frontmatter mismatch.
        reason: String,
    },
}

/// Transport-specific errors
#[derive(Error, Debug)]
pub enum TransportError {
    /// The HTTP transport failed; carries the failure description.
    #[error("HTTP transport error: {0}")]
    Http(String),

    /// The SSE transport failed; carries the failure description.
    #[error("SSE transport error: {0}")]
    Sse(String),

    /// The stdio transport failed; carries the failure description.
    #[error("Stdio transport error: {0}")]
    Stdio(String),

    /// The endpoint names a transport this client does not implement; carries the reason.
    #[error("Unsupported transport: {0}")]
    Unsupported(String),

    /// The transport could not establish a connection; carries the failure description.
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    /// The server answered with a non-success HTTP status.
    #[error("HTTP {status}: {message}")]
    HttpStatus {
        /// HTTP status code of the response.
        status: u16,
        /// Response body or reason text accompanying the status.
        message: String,
    },

    /// The transport was closed while an operation was still in progress.
    #[error("Transport closed unexpectedly")]
    Closed,
}

/// Protocol-specific errors
#[derive(Error, Debug)]
pub enum ProtocolError {
    /// A JSON-RPC request was malformed; carries the reason.
    #[error("Invalid JSON-RPC request: {0}")]
    InvalidRequest(String),

    /// A JSON-RPC response was malformed or did not match the request; carries the reason.
    #[error("Invalid JSON-RPC response: {0}")]
    InvalidResponse(String),

    /// The server's protocol version is not one this client accepts; carries the reason.
    #[error("Unsupported protocol version: {0}")]
    UnsupportedVersion(String),

    /// The peer reported the requested method does not exist; carries the method name.
    #[error("Method not found: {0}")]
    MethodNotFound(String),

    /// The method parameters were rejected as invalid; carries the reason.
    #[error("Invalid parameters: {0}")]
    InvalidParams(String),

    /// Protocol version or capability negotiation failed; carries the reason.
    #[error("Protocol negotiation failed: {0}")]
    NegotiationFailed(String),

    /// The server lacks a capability the operation requires; carries the capability description.
    #[error("Capability mismatch: {0}")]
    CapabilityMismatch(String),
}

/// Session management errors
#[derive(Error, Debug)]
pub enum SessionError {
    /// An operation needing an initialized session was attempted before initialization.
    #[error("Session not initialized")]
    NotInitialized,

    /// Initialization was attempted on a session that is already initialized.
    #[error("Session already initialized")]
    AlreadyInitialized,

    /// The session expired and can no longer be used.
    #[error("Session expired")]
    Expired,

    /// The session was terminated and can no longer be used.
    #[error("Session terminated")]
    Terminated,

    /// The session was in a state that does not permit the operation.
    #[error("Invalid session state: expected {expected}, found {actual}")]
    InvalidState {
        /// The state the operation required.
        expected: String,
        /// The state the session was actually in.
        actual: String,
    },

    /// Attempting to recover the session failed; carries the reason.
    #[error("Session recovery failed: {0}")]
    RecoveryFailed(String),
}

impl McpClientError {
    /// Creates a generic error with a message.
    pub fn generic(message: impl Into<String>) -> Self {
        Self::Generic {
            message: message.into(),
        }
    }

    /// Creates a configuration error.
    pub fn config(message: impl Into<String>) -> Self {
        Self::Config(message.into())
    }

    /// Creates an authentication error.
    pub fn auth(message: impl Into<String>) -> Self {
        Self::Auth(message.into())
    }

    /// Creates a server error from a JSON-RPC error response.
    pub fn server_error(code: i32, message: impl Into<String>, data: Option<Value>) -> Self {
        Self::ServerError {
            code,
            message: message.into(),
            data,
        }
    }

    /// Returns whether this is a resource-not-found error from `resources/read`, on a
    /// connection negotiated at the given wire version.
    ///
    /// Resources §Error Handling: servers answer a nonexistent resource with
    /// `-32602`; "For backwards compatibility, clients SHOULD also accept
    /// -32002 as a resource not found error, as earlier protocol versions
    /// used this code." That backwards-compat acceptance is scoped to peers
    /// still speaking 2025-11-25 or earlier, where `-32002` is legitimately
    /// resource-not-found. On a connection negotiated at 2026-07-28, the
    /// [Error Codes](https://modelcontextprotocol.io/specification/2026-07-28/basic/index#error-codes)
    /// section lists `-32002` among the codes implementations of this
    /// version MUST NOT emit — so on that lane it carries no not-found
    /// meaning and must not be classified as one, regardless of what a
    /// non-compliant peer sends.
    pub fn is_resource_not_found(&self, version: crate::version::McpVersion) -> bool {
        use crate::version::McpVersion;
        matches!(
            (self, version),
            (Self::ServerError { code: -32602, .. }, _)
                | (
                    Self::ServerError { code: -32002, .. },
                    McpVersion::V2025_11_25
                )
        )
    }

    /// Returns whether the error is retryable.
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Transport(TransportError::ConnectionFailed(_)) => true,
            Self::Transport(TransportError::Closed) => true,
            Self::Connection(_) => true,
            Self::Timeout => true,
            Self::ServerError { code, .. } => {
                // Retry on server errors that might be temporary
                matches!(code, -32099..=-32000) // Implementation-defined server errors
            }
            _ => false,
        }
    }

    /// Returns whether this error indicates the session is expired or unknown (HTTP 404).
    ///
    /// Per the MCP spec, a client MUST start a new session on 404.
    pub fn is_session_expired(&self) -> bool {
        matches!(
            self,
            Self::Transport(TransportError::HttpStatus { status: 404, .. })
        )
    }

    /// Returns whether the server refused the request's protocol version.
    ///
    /// The signal is JSON-RPC `-32022` UnsupportedProtocolVersionError. On an established
    /// connection this means the server behind the URL now speaks a different
    /// spec revision than the one locked at `connect()`.
    pub fn is_unsupported_protocol_version(&self) -> bool {
        matches!(
            self,
            Self::ServerError { code, .. } if i64::from(*code) == crate::version::UNSUPPORTED_PROTOCOL_VERSION
        )
    }

    /// Returns whether a stateful (2025-era) server refused the request for lack of an
    /// `Mcp-Session-Id`.
    ///
    /// On a connection locked to 2026-07-28, which never sends
    /// one, this means the server behind the URL is now a 2025-era server.
    pub fn is_missing_session_id(&self) -> bool {
        matches!(
            self,
            Self::ServerError { code, message, .. }
                if crate::version::is_missing_session_id(i64::from(*code), Some(message))
        )
    }

    /// Returns whether the server rejected the request because
    /// `notifications/initialized` has not been processed yet (JSON-RPC -32031
    /// or message containing "Session not initialized").
    ///
    /// This is distinct from `is_session_expired()` (HTTP 404): the session
    /// exists but the server hasn't finished the handshake. The client should
    /// disconnect, re-run `connect()`, and retry the request once.
    pub fn is_session_not_initialized(&self) -> bool {
        match self {
            Self::ServerError { code, message, .. } => {
                *code == -32031 || message.contains("Session not initialized")
            }
            _ => false,
        }
    }

    /// Returns whether the error is a protocol-level issue.
    pub fn is_protocol_error(&self) -> bool {
        matches!(self, Self::Protocol(_))
    }

    /// Returns whether the error is a session-level issue.
    pub fn is_session_error(&self) -> bool {
        matches!(self, Self::Session(_))
    }

    /// Returns the error code if this is a server error.
    pub fn error_code(&self) -> Option<i32> {
        match self {
            Self::ServerError { code, .. } => Some(*code),
            _ => None,
        }
    }
}

// From implementations are handled by #[from] in the enum

/// Convenience macro for creating generic errors
#[macro_export]
macro_rules! client_error {
    ($($arg:tt)*) => {
        $crate::error::McpClientError::generic(format!($($arg)*))
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_404_is_session_expired() {
        let err = McpClientError::Transport(TransportError::HttpStatus {
            status: 404,
            message: "Not Found".to_string(),
        });
        assert!(err.is_session_expired());
    }

    #[test]
    fn test_500_is_not_session_expired() {
        let err = McpClientError::Transport(TransportError::HttpStatus {
            status: 500,
            message: "Internal".to_string(),
        });
        assert!(!err.is_session_expired());
    }

    #[test]
    fn test_http_status_error_display() {
        let err = TransportError::HttpStatus {
            status: 404,
            message: "Not Found".to_string(),
        };
        assert_eq!(err.to_string(), "HTTP 404: Not Found");
    }

    #[test]
    fn test_session_not_initialized_by_code() {
        let err = McpClientError::server_error(-32031, "Session error: something", None);
        assert!(err.is_session_not_initialized());
    }

    #[test]
    fn test_session_not_initialized_by_message() {
        let err = McpClientError::server_error(
            -32000,
            "Session not initialized - client must send notifications/initialized first",
            None,
        );
        assert!(err.is_session_not_initialized());
    }

    #[test]
    fn test_unrelated_error_is_not_session_not_initialized() {
        let err = McpClientError::server_error(-32602, "Invalid params", None);
        assert!(!err.is_session_not_initialized());
    }

    #[test]
    fn test_transport_error_is_not_session_not_initialized() {
        let err = McpClientError::Transport(TransportError::HttpStatus {
            status: 500,
            message: "Internal".to_string(),
        });
        assert!(!err.is_session_not_initialized());
    }
}

#[cfg(test)]
mod resource_not_found_tests {
    use super::*;
    use crate::version::McpVersion;

    /// Resources §Error Handling backwards-compat: on a 2025-11-25 connection,
    /// both -32602 and the pre-2026 -32002 classify as resource-not-found.
    #[test]
    fn resource_not_found_accepts_both_codes_on_2025_11_25() {
        assert!(
            McpClientError::server_error(-32602, "no such resource", None)
                .is_resource_not_found(McpVersion::V2025_11_25)
        );
        assert!(
            McpClientError::server_error(-32002, "no such resource", None)
                .is_resource_not_found(McpVersion::V2025_11_25)
        );
        assert!(
            !McpClientError::server_error(-32601, "no such method", None)
                .is_resource_not_found(McpVersion::V2025_11_25)
        );
        assert!(!McpClientError::Timeout.is_resource_not_found(McpVersion::V2025_11_25));
    }

    /// On a 2026-07-28 connection, `-32602` is the only resource-not-found
    /// code. `-32002` is a code this spec version MUST NOT emit, so a peer
    /// sending it is non-compliant rather than reporting a legitimate miss —
    /// the client must not paper over that by classifying it as not-found.
    #[test]
    fn resource_not_found_rejects_legacy_code_on_2026_07_28() {
        assert!(
            McpClientError::server_error(-32602, "no such resource", None)
                .is_resource_not_found(McpVersion::V2026_07_28)
        );
        assert!(
            !McpClientError::server_error(-32002, "no such resource", None)
                .is_resource_not_found(McpVersion::V2026_07_28)
        );
        assert!(
            !McpClientError::server_error(-32601, "no such method", None)
                .is_resource_not_found(McpVersion::V2026_07_28)
        );
        assert!(!McpClientError::Timeout.is_resource_not_found(McpVersion::V2026_07_28));
    }
}
