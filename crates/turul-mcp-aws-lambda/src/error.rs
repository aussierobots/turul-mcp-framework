//! Error handling for Lambda MCP integration

use thiserror::Error;

/// Result type for Lambda MCP operations
pub type Result<T> = std::result::Result<T, LambdaError>;

/// Errors that can occur during Lambda MCP server operations
#[derive(Error, Debug)]
pub enum LambdaError {
    /// Type conversion error between lambda_http and hyper types
    #[error("Type conversion failed: {0}")]
    TypeConversion(String),

    /// HTTP error during request processing
    #[error("HTTP error: {0}")]
    Http(#[from] http::Error),

    /// Hyper error during request processing
    #[error("Hyper error: {0}")]
    Hyper(#[from] hyper::Error),

    /// Lambda HTTP error
    #[error("Lambda HTTP error: {0}")]
    LambdaHttp(#[from] lambda_http::Error),

    /// Session storage error
    #[error("Session storage error: {0}")]
    SessionStorage(String),

    /// MCP framework error
    #[error("MCP framework error: {0}")]
    McpFramework(String),

    /// JSON serialization/deserialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Body processing error
    #[error("Body processing error: {0}")]
    Body(String),

    /// CORS configuration error
    #[cfg(feature = "cors")]
    #[error("CORS error: {0}")]
    Cors(String),

    /// SSE streaming error
    #[cfg(feature = "sse")]
    #[error("SSE streaming error: {0}")]
    Sse(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),
    
    /// Session management error
    #[error("Session error: {0}")]
    Session(String),
}

impl From<turul_http_mcp_server::HttpMcpError> for LambdaError {
    fn from(err: turul_http_mcp_server::HttpMcpError) -> Self {
        LambdaError::McpFramework(err.to_string())
    }
}

impl From<turul_mcp_session_storage::SessionStorageError> for LambdaError {
    fn from(err: turul_mcp_session_storage::SessionStorageError) -> Self {
        LambdaError::SessionStorage(err.to_string())
    }
}