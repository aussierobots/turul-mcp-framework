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
//! ```ignore
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
pub use turul_rpc::{error, error_codes, notification, prelude, request, response, types};

/// JSON-RPC dispatch helpers and the inbound message union.
///
/// **Curated** to the v0.3.38 surface. New `turul-rpc 0.1` APIs (notably
/// `parse_json_rpc_batch` and `BatchOrSingle`) live in `turul_rpc::batch`
/// and are intentionally NOT re-exported here so the shim keeps its
/// preservation-only posture per [ADR-003][adr-003]. Code that wants
/// batch processing should depend on `turul-rpc` directly.
///
/// [adr-003]: https://github.com/aussierobots/turul-rpc/blob/main/docs/adr/003-compatibility-with-turul-mcp-json-rpc-server.md
pub mod dispatch {
    pub use turul_rpc::dispatch::{
        create_error_response, create_success_response, parse_json_rpc_message,
        parse_json_rpc_messages, JsonRpcMessage, JsonRpcMessageResult,
    };
}

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
