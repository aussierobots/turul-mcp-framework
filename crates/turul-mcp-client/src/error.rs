//! Error types for MCP client operations

use thiserror::Error;
use serde_json::Value;

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
    
    /// Timeout errors
    #[error("Operation timed out")]
    Timeout,
    
    /// Server returned an error
    #[error("Server error (code {code}): {message}")]
    ServerError { code: i32, message: String, data: Option<Value> },
    
    /// Generic error with context
    #[error("Error: {message}")]
    Generic { message: String },
}

/// Transport-specific errors
#[derive(Error, Debug)]
pub enum TransportError {
    #[error("HTTP transport error: {0}")]
    Http(String),
    
    #[error("SSE transport error: {0}")]
    Sse(String),
    
    #[error("WebSocket transport error: {0}")]
    WebSocket(String),
    
    #[error("Stdio transport error: {0}")]
    Stdio(String),
    
    #[error("Unsupported transport: {0}")]
    Unsupported(String),
    
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    
    #[error("Transport closed unexpectedly")]
    Closed,
}

/// Protocol-specific errors
#[derive(Error, Debug)]
pub enum ProtocolError {
    #[error("Invalid JSON-RPC request: {0}")]
    InvalidRequest(String),
    
    #[error("Invalid JSON-RPC response: {0}")]
    InvalidResponse(String),
    
    #[error("Unsupported protocol version: {0}")]
    UnsupportedVersion(String),
    
    #[error("Method not found: {0}")]
    MethodNotFound(String),
    
    #[error("Invalid parameters: {0}")]
    InvalidParams(String),
    
    #[error("Protocol negotiation failed: {0}")]
    NegotiationFailed(String),
    
    #[error("Capability mismatch: {0}")]
    CapabilityMismatch(String),
}

/// Session management errors
#[derive(Error, Debug)]
pub enum SessionError {
    #[error("Session not initialized")]
    NotInitialized,
    
    #[error("Session already initialized")]
    AlreadyInitialized,
    
    #[error("Session expired")]
    Expired,
    
    #[error("Session terminated")]
    Terminated,
    
    #[error("Invalid session state: expected {expected}, found {actual}")]
    InvalidState { expected: String, actual: String },
    
    #[error("Session recovery failed: {0}")]
    RecoveryFailed(String),
}

impl McpClientError {
    /// Create a generic error with a message
    pub fn generic(message: impl Into<String>) -> Self {
        Self::Generic {
            message: message.into(),
        }
    }
    
    /// Create a configuration error
    pub fn config(message: impl Into<String>) -> Self {
        Self::Config(message.into())
    }
    
    /// Create an authentication error
    pub fn auth(message: impl Into<String>) -> Self {
        Self::Auth(message.into())
    }
    
    /// Create a server error from JSON-RPC error response
    pub fn server_error(code: i32, message: impl Into<String>, data: Option<Value>) -> Self {
        Self::ServerError {
            code,
            message: message.into(),
            data,
        }
    }
    
    /// Check if the error is retryable
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
    
    /// Check if the error is a protocol-level issue
    pub fn is_protocol_error(&self) -> bool {
        matches!(self, Self::Protocol(_))
    }
    
    /// Check if the error is a session-level issue
    pub fn is_session_error(&self) -> bool {
        matches!(self, Self::Session(_))
    }
    
    /// Get the error code if this is a server error
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