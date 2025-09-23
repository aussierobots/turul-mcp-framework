//! # JSON-RPC Server Prelude
//!
//! This module provides convenient re-exports of the most commonly used types
//! from the JSON-RPC server library.
//!
//! ```rust
//! use turul_mcp_json_rpc_server::prelude::*;
//! ```

// Core JSON-RPC types
pub use crate::error::{JsonRpcError, JsonRpcErrorCode};
pub use crate::notification::JsonRpcNotification;
pub use crate::request::{JsonRpcRequest, RequestParams};
pub use crate::response::{JsonRpcMessage, JsonRpcResponse, ResponseResult};
pub use crate::types::{JsonRpcVersion, RequestId};

#[cfg(feature = "async")]
pub use crate::r#async::{JsonRpcDispatcher, JsonRpcHandler, SessionContext};

// Standard error codes
pub use crate::error_codes::*;