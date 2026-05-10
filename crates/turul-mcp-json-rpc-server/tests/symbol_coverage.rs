//! Exhaustive symbol coverage test.
//!
//! Generated from `cargo +nightly public-api -p turul-mcp-json-rpc-server` on
//! the original v0.3.38 source. Every public path the original crate exposed
//! is named below as a `use` statement (where applicable). If any path fails
//! to resolve through the shim, this file fails to compile — that is the
//! gate that proves the shim preserves the public surface.
//!
//! Async-gated paths are scoped under a `#[cfg(feature = "async")]` module so
//! `cargo test --no-default-features` succeeds.

#![allow(unused_imports, dead_code)]

// -----------------------------------------------------------------------------
// Non-async paths — always present
// -----------------------------------------------------------------------------

use turul_mcp_json_rpc_server::dispatch;
use turul_mcp_json_rpc_server::dispatch::create_error_response as _use_dispatch_create_error_response;
use turul_mcp_json_rpc_server::dispatch::create_success_response as _use_dispatch_create_success_response;
use turul_mcp_json_rpc_server::dispatch::JsonRpcMessage as _use_dispatch_JsonRpcMessage;
use turul_mcp_json_rpc_server::dispatch::JsonRpcMessageResult as _use_dispatch_JsonRpcMessageResult;
use turul_mcp_json_rpc_server::dispatch::parse_json_rpc_message as _use_dispatch_parse_json_rpc_message;
use turul_mcp_json_rpc_server::dispatch::parse_json_rpc_messages as _use_dispatch_parse_json_rpc_messages;

use turul_mcp_json_rpc_server::error_codes;
use turul_mcp_json_rpc_server::error_codes::INTERNAL_ERROR as _use_INTERNAL_ERROR;
use turul_mcp_json_rpc_server::error_codes::INVALID_PARAMS as _use_INVALID_PARAMS;
use turul_mcp_json_rpc_server::error_codes::INVALID_REQUEST as _use_INVALID_REQUEST;
use turul_mcp_json_rpc_server::error_codes::METHOD_NOT_FOUND as _use_METHOD_NOT_FOUND;
use turul_mcp_json_rpc_server::error_codes::PARSE_ERROR as _use_PARSE_ERROR;
use turul_mcp_json_rpc_server::error_codes::SERVER_ERROR_END as _use_SERVER_ERROR_END;
use turul_mcp_json_rpc_server::error_codes::SERVER_ERROR_START as _use_SERVER_ERROR_START;

use turul_mcp_json_rpc_server::error::JsonRpcError as _use_error_JsonRpcError;
use turul_mcp_json_rpc_server::error::JsonRpcErrorCode as _use_error_JsonRpcErrorCode;
use turul_mcp_json_rpc_server::error::JsonRpcErrorObject as _use_error_JsonRpcErrorObject;
use turul_mcp_json_rpc_server::error::JsonRpcTransportError as _use_error_JsonRpcTransportError;

use turul_mcp_json_rpc_server::JSONRPC_VERSION as _use_JSONRPC_VERSION;
use turul_mcp_json_rpc_server::JsonRpcError as _use_JsonRpcError;
use turul_mcp_json_rpc_server::JsonRpcErrorCode as _use_JsonRpcErrorCode;
use turul_mcp_json_rpc_server::JsonRpcMessage as _use_JsonRpcMessage;
use turul_mcp_json_rpc_server::JsonRpcNotification as _use_JsonRpcNotification;
use turul_mcp_json_rpc_server::JsonRpcRequest as _use_JsonRpcRequest;
use turul_mcp_json_rpc_server::JsonRpcResponse as _use_JsonRpcResponse;
use turul_mcp_json_rpc_server::JsonRpcVersion as _use_JsonRpcVersion;
use turul_mcp_json_rpc_server::RequestId as _use_RequestId;
use turul_mcp_json_rpc_server::RequestParams as _use_RequestParams;
use turul_mcp_json_rpc_server::ResponseResult as _use_ResponseResult;

use turul_mcp_json_rpc_server::notification;
use turul_mcp_json_rpc_server::notification::JsonRpcNotification as _use_notification_JsonRpcNotification;

use turul_mcp_json_rpc_server::prelude;
use turul_mcp_json_rpc_server::prelude::INTERNAL_ERROR as _use_prelude_INTERNAL_ERROR;
use turul_mcp_json_rpc_server::prelude::INVALID_PARAMS as _use_prelude_INVALID_PARAMS;
use turul_mcp_json_rpc_server::prelude::INVALID_REQUEST as _use_prelude_INVALID_REQUEST;
use turul_mcp_json_rpc_server::prelude::JsonRpcError as _use_prelude_JsonRpcError;
use turul_mcp_json_rpc_server::prelude::JsonRpcErrorCode as _use_prelude_JsonRpcErrorCode;
use turul_mcp_json_rpc_server::prelude::JsonRpcMessage as _use_prelude_JsonRpcMessage;
use turul_mcp_json_rpc_server::prelude::JsonRpcNotification as _use_prelude_JsonRpcNotification;
use turul_mcp_json_rpc_server::prelude::JsonRpcRequest as _use_prelude_JsonRpcRequest;
use turul_mcp_json_rpc_server::prelude::JsonRpcResponse as _use_prelude_JsonRpcResponse;
use turul_mcp_json_rpc_server::prelude::JsonRpcVersion as _use_prelude_JsonRpcVersion;
use turul_mcp_json_rpc_server::prelude::METHOD_NOT_FOUND as _use_prelude_METHOD_NOT_FOUND;
use turul_mcp_json_rpc_server::prelude::PARSE_ERROR as _use_prelude_PARSE_ERROR;
use turul_mcp_json_rpc_server::prelude::RequestId as _use_prelude_RequestId;
use turul_mcp_json_rpc_server::prelude::RequestParams as _use_prelude_RequestParams;
use turul_mcp_json_rpc_server::prelude::ResponseResult as _use_prelude_ResponseResult;
use turul_mcp_json_rpc_server::prelude::SERVER_ERROR_END as _use_prelude_SERVER_ERROR_END;
use turul_mcp_json_rpc_server::prelude::SERVER_ERROR_START as _use_prelude_SERVER_ERROR_START;

use turul_mcp_json_rpc_server::request;
use turul_mcp_json_rpc_server::request::JsonRpcRequest as _use_request_JsonRpcRequest;
use turul_mcp_json_rpc_server::request::RequestParams as _use_request_RequestParams;

use turul_mcp_json_rpc_server::response;
use turul_mcp_json_rpc_server::response::JsonRpcMessage as _use_response_JsonRpcMessage;
use turul_mcp_json_rpc_server::response::JsonRpcResponse as _use_response_JsonRpcResponse;
use turul_mcp_json_rpc_server::response::ResponseResult as _use_response_ResponseResult;

use turul_mcp_json_rpc_server::types;
use turul_mcp_json_rpc_server::types::JsonRpcVersion as _use_types_JsonRpcVersion;
use turul_mcp_json_rpc_server::types::RequestId as _use_types_RequestId;

// -----------------------------------------------------------------------------
// Async-gated paths
// -----------------------------------------------------------------------------

#[cfg(feature = "async")]
mod async_gated {
    use turul_mcp_json_rpc_server::r#async;
    use turul_mcp_json_rpc_server::r#async::FunctionHandler as _use_async_FunctionHandler;
    use turul_mcp_json_rpc_server::r#async::JsonRpcDispatcher as _use_async_JsonRpcDispatcher;
    use turul_mcp_json_rpc_server::r#async::JsonRpcHandler as _use_async_JsonRpcHandler;
    use turul_mcp_json_rpc_server::r#async::SessionContext as _use_async_SessionContext;
    use turul_mcp_json_rpc_server::r#async::ToJsonRpcError as _use_async_ToJsonRpcError;

    use turul_mcp_json_rpc_server::JsonRpcDispatcher as _use_JsonRpcDispatcher;
    use turul_mcp_json_rpc_server::JsonRpcHandler as _use_JsonRpcHandler;
    use turul_mcp_json_rpc_server::SessionContext as _use_SessionContext;

    use turul_mcp_json_rpc_server::prelude::JsonRpcDispatcher as _use_prelude_JsonRpcDispatcher;
    use turul_mcp_json_rpc_server::prelude::JsonRpcHandler as _use_prelude_JsonRpcHandler;
    use turul_mcp_json_rpc_server::prelude::SessionContext as _use_prelude_SessionContext;
}

#[test]
fn symbol_coverage_compiles() {}
