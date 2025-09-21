//! # JSON-RPC 2.0 Server Implementation
//! 
//! A pure, transport-agnostic JSON-RPC 2.0 server implementation that follows the specification.
//! This crate provides the core types and dispatch logic for JSON-RPC without any transport-specific code.
//!
//! ## Features
//! - Full JSON-RPC 2.0 specification compliance
//! - Transport agnostic (works with HTTP, WebSocket, TCP, etc.)
//! - Async/await support with `async` feature
//! - Comprehensive error handling
//! - Support for notifications and requests

pub mod error;
pub mod request;
pub mod response; 
pub mod notification;
pub mod dispatch;
pub mod types;

#[cfg(feature = "async")]
pub mod r#async;

// Re-export main types
pub use error::{JsonRpcError, JsonRpcErrorCode};
pub use request::{JsonRpcRequest, RequestParams};
pub use response::{JsonRpcResponse, ResponseResult, JsonRpcMessage};
pub use notification::JsonRpcNotification;
pub use types::{RequestId, JsonRpcVersion};

#[cfg(feature = "async")]
pub use r#async::{JsonRpcHandler, JsonRpcDispatcher, SessionContext};

/// JSON-RPC 2.0 version constant
pub const JSONRPC_VERSION: &str = "2.0";

/// Standard JSON-RPC 2.0 error codes
pub mod error_codes {
    pub const PARSE_ERROR: i64 = -32700;
    pub const INVALID_REQUEST: i64 = -32600;
    pub const METHOD_NOT_FOUND: i64 = -32601;
    pub const INVALID_PARAMS: i64 = -32602;
    pub const INTERNAL_ERROR: i64 = -32603;
    
    // Server error range: -32099 to -32000
    pub const SERVER_ERROR_START: i64 = -32099;
    pub const SERVER_ERROR_END: i64 = -32000;
}