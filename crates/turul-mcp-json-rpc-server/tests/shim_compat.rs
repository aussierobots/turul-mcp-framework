//! Backwards-compatibility guard for the `turul-mcp-json-rpc-server` shim.
//!
//! This file names every symbol the original 0.3.x crate published, at every
//! original path. If any item fails to resolve, this file fails to compile —
//! that's the gate.
//!
//! Add to this list when discovering missed symbols. Do NOT remove items
//! without a coordinated breaking-change review.

#![allow(dead_code, unused_imports)]

// Crate-root re-exports
use turul_mcp_json_rpc_server::JSONRPC_VERSION;
use turul_mcp_json_rpc_server::{
    JsonRpcDispatcher, JsonRpcError, JsonRpcErrorCode, JsonRpcHandler, JsonRpcMessage,
    JsonRpcNotification, JsonRpcRequest, JsonRpcResponse, JsonRpcVersion, RequestId, RequestParams,
    ResponseResult, SessionContext,
};

#[cfg(feature = "streams")]
use turul_mcp_json_rpc_server::{JsonRpcFrame, StreamingJsonRpcDispatcher, StreamingJsonRpcHandler};

// Module re-exports
use turul_mcp_json_rpc_server::error::{JsonRpcError as _E, JsonRpcErrorCode as _C, JsonRpcErrorObject, JsonRpcTransportError};
use turul_mcp_json_rpc_server::request::{JsonRpcRequest as _R, RequestParams as _RP};
use turul_mcp_json_rpc_server::response::{JsonRpcMessage as _M, JsonRpcResponse as _Rsp, ResponseResult as _RR};
use turul_mcp_json_rpc_server::notification::JsonRpcNotification as _N;
use turul_mcp_json_rpc_server::types::{JsonRpcVersion as _V, RequestId as _ID};
use turul_mcp_json_rpc_server::dispatch::{
    create_error_response, create_success_response, parse_json_rpc_message,
    parse_json_rpc_messages, JsonRpcMessage as DispatchMessage, JsonRpcMessageResult,
};
#[cfg(feature = "async")]
use turul_mcp_json_rpc_server::r#async::{JsonRpcDispatcher as _D, JsonRpcHandler as _H, SessionContext as _S, ToJsonRpcError};

// error_codes module
use turul_mcp_json_rpc_server::error_codes::{
    INTERNAL_ERROR, INVALID_PARAMS, INVALID_REQUEST, METHOD_NOT_FOUND, PARSE_ERROR,
    SERVER_ERROR_END, SERVER_ERROR_START,
};

// prelude
use turul_mcp_json_rpc_server::prelude::*;

#[test]
fn error_codes_match_spec() {
    assert_eq!(PARSE_ERROR, -32700);
    assert_eq!(INVALID_REQUEST, -32600);
    assert_eq!(METHOD_NOT_FOUND, -32601);
    assert_eq!(INVALID_PARAMS, -32602);
    assert_eq!(INTERNAL_ERROR, -32603);
    assert_eq!(SERVER_ERROR_START, -32099);
    assert_eq!(SERVER_ERROR_END, -32000);
}

#[test]
fn jsonrpc_version_constant() {
    assert_eq!(JSONRPC_VERSION, "2.0");
}

#[test]
fn type_identity_request_id_is_same_across_paths() {
    // RequestId reached via the crate root must be the same nominal type as
    // RequestId reached via the `types` module and via `turul_rpc` directly —
    // `pub use` chains preserve identity. This compiles only if they are the
    // same type.
    let id_a: RequestId = RequestId::Number(1);
    let id_b: turul_mcp_json_rpc_server::types::RequestId = id_a.clone();
    let id_c: turul_rpc::RequestId = id_b.clone();
    let id_d: turul_rpc::types::RequestId = id_c.clone();
    assert_eq!(id_a, id_d);
}

#[test]
fn type_identity_dispatch_message_reaches_jsonrpc_via_facade() {
    // The `dispatch::JsonRpcMessage` (incoming union) reached via the shim
    // must be the same nominal type as the one reached via the turul-rpc
    // facade's `dispatch` module.
    let _: fn(turul_rpc::dispatch::JsonRpcMessage) -> DispatchMessage = |x| x;
}

#[test]
fn dispatch_module_helpers_compile() {
    let _: turul_mcp_json_rpc_server::dispatch::JsonRpcMessageResult =
        create_success_response(RequestId::Number(1), serde_json::json!({}));
    let _: turul_mcp_json_rpc_server::dispatch::JsonRpcMessageResult =
        create_error_response(Some(RequestId::Number(1)), -32601, "missing");
    let _ = parse_json_rpc_message(r#"{"jsonrpc":"2.0","method":"x","id":1}"#);
    let _ = parse_json_rpc_messages(r#"{"jsonrpc":"2.0","method":"x","id":1}"#);
}

#[cfg(feature = "async")]
#[test]
fn dispatcher_constructs() {
    use turul_mcp_json_rpc_server::error::JsonRpcErrorObject;

    #[derive(Debug)]
    struct E;
    impl std::fmt::Display for E {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("test")
        }
    }
    impl std::error::Error for E {}
    impl ToJsonRpcError for E {
        fn to_error_object(&self) -> JsonRpcErrorObject {
            JsonRpcErrorObject::internal_error(None)
        }
    }

    let _: JsonRpcDispatcher<E> = JsonRpcDispatcher::new();
}
