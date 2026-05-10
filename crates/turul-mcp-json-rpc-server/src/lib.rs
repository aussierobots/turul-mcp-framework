//! # turul-mcp-json-rpc-server (compatibility shim)
//!
//! This crate is a thin re-export of [`turul-rpc`](https://crates.io/crates/turul-rpc),
//! the generic JSON-RPC 2.0 framework extracted from turul-mcp-framework at v0.3.39.
//! Every public type, trait, and module from prior 0.3.x releases continues to
//! resolve at the same path with the same nominal type — no code changes required
//! for existing consumers.
//!
//! ## New code should depend on `turul-rpc` directly
//!
//! ```toml
//! [dependencies]
//! turul-rpc = "0.1"
//! ```
//!
//! ```rust,no_run
//! use turul_rpc::{JsonRpcDispatcher, JsonRpcHandler, RequestParams, SessionContext};
//! ```
//!
//! ## Lifecycle
//!
//! `turul-mcp-json-rpc-server` is maintained on the **0.3.x** line as a re-export
//! shim. There is no 0.4 release of this crate — turul-mcp-framework 0.4.0
//! removes the dependency and imports `turul-rpc` directly. Existing 0.3
//! consumers may continue to use this crate indefinitely.
//!
//! See [ADR-003 in the turul-rpc repository][adr-003] for the technical contract.
//!
//! [adr-003]: https://github.com/aussierobots/turul-rpc/blob/main/docs/adr/003-compatibility-with-turul-mcp-json-rpc-server.md

// Module re-exports — match the original crate's `pub mod` layout.
pub use turul_rpc::{dispatch, error, error_codes, notification, prelude, request, response, types};

#[cfg(feature = "async")]
pub use turul_rpc::r#async;

// Root re-exports — match the original crate's `pub use` lines.
pub use turul_rpc::{
    JsonRpcError, JsonRpcErrorCode, JsonRpcMessage, JsonRpcNotification, JsonRpcRequest,
    JsonRpcResponse, JsonRpcVersion, RequestId, RequestParams, ResponseResult, JSONRPC_VERSION,
};

#[cfg(feature = "async")]
pub use turul_rpc::{JsonRpcDispatcher, JsonRpcHandler, SessionContext};

#[cfg(feature = "streams")]
pub use turul_rpc::{JsonRpcFrame, StreamingJsonRpcDispatcher, StreamingJsonRpcHandler};
