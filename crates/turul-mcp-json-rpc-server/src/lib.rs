//! # JSON-RPC 2.0 Server Implementation
//!
//! A pure, transport-agnostic JSON-RPC 2.0 server implementation with clean domain/protocol separation.
//! This crate provides the core types and dispatch logic for JSON-RPC without any transport-specific code.
//!
//! ## Features
//! - **JSON-RPC 2.0 Compliance**: Full specification support with proper error handling
//! - **Type-Safe Error Handling**: Domain errors with automatic protocol conversion
//! - **Clean Architecture**: Handlers return domain errors, dispatcher owns protocol
//! - **Transport Agnostic**: Works with HTTP, WebSocket, TCP, etc.
//! - **Async/Await Support**: Full async support with session context
//! - **Zero Double-Wrapping**: Clean error handling without intermediate wrappers
//!
//! ## Architecture
//!
//! ```rust,no_run
//! # use async_trait::async_trait;
//! # use turul_mcp_json_rpc_server::{JsonRpcHandler, JsonRpcDispatcher, request::RequestParams};
//! # use serde_json::Value;
//! # use std::fmt;
//! #
//! # #[derive(Debug)]
//! # enum MyDomainError {
//! #     InvalidInput(String),
//! # }
//! #
//! # impl fmt::Display for MyDomainError {
//! #     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//! #         match self {
//! #             MyDomainError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
//! #         }
//! #     }
//! # }
//! #
//! # impl std::error::Error for MyDomainError {}
//! #
//! #
//! # struct MyHandler;
//! // Handlers return domain errors only
//! #[async_trait]
//! impl JsonRpcHandler for MyHandler {
//!     type Error = MyDomainError;  // Not JsonRpcError!
//!
//!     async fn handle(&self, _method: &str, _params: Option<RequestParams>, _session: Option<turul_mcp_json_rpc_server::SessionContext>) -> Result<Value, MyDomainError> {
//!         Err(MyDomainError::InvalidInput("bad data".to_string()))
//!     }
//! }
//!
//! // Dispatcher converts domain → protocol automatically (example concept)
//! // let dispatcher = JsonRpcDispatcher::new(); // Actual usage requires ToJsonRpcError trait
//! ```

pub mod dispatch;
pub mod error;
pub mod notification;
pub mod prelude;
pub mod request;
pub mod response;
pub mod types;

#[cfg(feature = "async")]
pub mod r#async;

// Re-export main types
/// JSON-RPC 2.0 error types and standard error codes
pub use error::{JsonRpcError, JsonRpcErrorCode};
/// JSON-RPC notification message structure for fire-and-forget communications
pub use notification::JsonRpcNotification;
/// JSON-RPC request structure with method and parameters
pub use request::{JsonRpcRequest, RequestParams};
/// JSON-RPC response types including success and error variants
pub use response::{JsonRpcMessage, JsonRpcResponse, ResponseResult};
/// Core JSON-RPC types for version and request identification
pub use types::{JsonRpcVersion, RequestId};

#[cfg(feature = "async")]
pub use r#async::{JsonRpcDispatcher, JsonRpcHandler, SessionContext};

#[cfg(feature = "streams")]
pub use r#async::streaming::{JsonRpcFrame, StreamingJsonRpcDispatcher, StreamingJsonRpcHandler};

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
