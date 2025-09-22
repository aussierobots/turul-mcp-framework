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
//! ```rust,ignore
//! // Handlers return domain errors only
//! #[async_trait]
//! impl JsonRpcHandler for MyHandler {
//!     type Error = MyDomainError;  // Not JsonRpcError!
//!
//!     async fn handle(&self, ...) -> Result<Value, MyDomainError> {
//!         Err(MyDomainError::InvalidInput("bad data".to_string()))
//!     }
//! }
//!
//! // Dispatcher converts domain → protocol automatically
//! let dispatcher: JsonRpcDispatcher<MyDomainError> = JsonRpcDispatcher::new();
//! ```

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