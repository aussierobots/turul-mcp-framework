//! Exhaustive symbol coverage test.
//!
//! Generated from `cargo +nightly public-api -p turul-mcp-json-rpc-server` on
//! the original v0.3.38 source. Every public path the original crate exposed
//! is listed below as a `use` statement (where applicable). If any path fails
//! to resolve through the shim, this file fails to compile — which is the
//! gate that proves the shim preserves the public surface from the rename's
//! perspective.

#![allow(unused_imports, dead_code)]

use turul_mcp_json_rpc_server::r#async;
use turul_mcp_json_rpc_server::r#async::FunctionHandler as _use_2_FunctionHandler;
use turul_mcp_json_rpc_server::r#async::JsonRpcDispatcher as _use_3_JsonRpcDispatcher;
use turul_mcp_json_rpc_server::r#async::JsonRpcHandler as _use_4_JsonRpcHandler;
use turul_mcp_json_rpc_server::r#async::SessionContext as _use_5_SessionContext;
use turul_mcp_json_rpc_server::r#async::ToJsonRpcError as _use_6_ToJsonRpcError;
use turul_mcp_json_rpc_server::dispatch;
use turul_mcp_json_rpc_server::dispatch::create_error_response as _use_8_create_error_response;
use turul_mcp_json_rpc_server::dispatch::create_success_response as _use_9_create_success_response;
use turul_mcp_json_rpc_server::dispatch::JsonRpcMessage as _use_10_JsonRpcMessage;
use turul_mcp_json_rpc_server::dispatch::JsonRpcMessageResult as _use_11_JsonRpcMessageResult;
use turul_mcp_json_rpc_server::dispatch::parse_json_rpc_message as _use_13_parse_json_rpc_message;
use turul_mcp_json_rpc_server::dispatch::parse_json_rpc_messages as _use_14_parse_json_rpc_messages;
use turul_mcp_json_rpc_server::error_codes;
use turul_mcp_json_rpc_server::error_codes::INTERNAL_ERROR as _use_16_INTERNAL_ERROR;
use turul_mcp_json_rpc_server::error_codes::INVALID_PARAMS as _use_17_INVALID_PARAMS;
use turul_mcp_json_rpc_server::error_codes::INVALID_REQUEST as _use_18_INVALID_REQUEST;
use turul_mcp_json_rpc_server::error_codes::METHOD_NOT_FOUND as _use_19_METHOD_NOT_FOUND;
use turul_mcp_json_rpc_server::error_codes::PARSE_ERROR as _use_20_PARSE_ERROR;
use turul_mcp_json_rpc_server::error_codes::SERVER_ERROR_END as _use_21_SERVER_ERROR_END;
use turul_mcp_json_rpc_server::error_codes::SERVER_ERROR_START as _use_22_SERVER_ERROR_START;
use turul_mcp_json_rpc_server::error::JsonRpcError as _use_23_JsonRpcError;
use turul_mcp_json_rpc_server::error::JsonRpcErrorCode as _use_24_JsonRpcErrorCode;
use turul_mcp_json_rpc_server::error::JsonRpcErrorObject as _use_25_JsonRpcErrorObject;
use turul_mcp_json_rpc_server::error::JsonRpcTransportError as _use_26_JsonRpcTransportError;
use turul_mcp_json_rpc_server::JSONRPC_VERSION as _use_28_JSONRPC_VERSION;
use turul_mcp_json_rpc_server::JsonRpcDispatcher as _use_29_JsonRpcDispatcher;
use turul_mcp_json_rpc_server::JsonRpcError as _use_30_JsonRpcError;
use turul_mcp_json_rpc_server::JsonRpcErrorCode as _use_31_JsonRpcErrorCode;
use turul_mcp_json_rpc_server::JsonRpcHandler as _use_32_JsonRpcHandler;
use turul_mcp_json_rpc_server::JsonRpcMessage as _use_33_JsonRpcMessage;
use turul_mcp_json_rpc_server::JsonRpcNotification as _use_34_JsonRpcNotification;
use turul_mcp_json_rpc_server::JsonRpcRequest as _use_35_JsonRpcRequest;
use turul_mcp_json_rpc_server::JsonRpcResponse as _use_36_JsonRpcResponse;
use turul_mcp_json_rpc_server::JsonRpcVersion as _use_37_JsonRpcVersion;
use turul_mcp_json_rpc_server::notification;
use turul_mcp_json_rpc_server::notification::JsonRpcNotification as _use_39_JsonRpcNotification;
use turul_mcp_json_rpc_server::prelude;
use turul_mcp_json_rpc_server::prelude::INTERNAL_ERROR as _use_41_INTERNAL_ERROR;
use turul_mcp_json_rpc_server::prelude::INVALID_PARAMS as _use_42_INVALID_PARAMS;
use turul_mcp_json_rpc_server::prelude::INVALID_REQUEST as _use_43_INVALID_REQUEST;
use turul_mcp_json_rpc_server::prelude::JsonRpcDispatcher as _use_44_JsonRpcDispatcher;
use turul_mcp_json_rpc_server::prelude::JsonRpcError as _use_45_JsonRpcError;
use turul_mcp_json_rpc_server::prelude::JsonRpcErrorCode as _use_46_JsonRpcErrorCode;
use turul_mcp_json_rpc_server::prelude::JsonRpcHandler as _use_47_JsonRpcHandler;
use turul_mcp_json_rpc_server::prelude::JsonRpcMessage as _use_48_JsonRpcMessage;
use turul_mcp_json_rpc_server::prelude::JsonRpcNotification as _use_49_JsonRpcNotification;
use turul_mcp_json_rpc_server::prelude::JsonRpcRequest as _use_50_JsonRpcRequest;
use turul_mcp_json_rpc_server::prelude::JsonRpcResponse as _use_51_JsonRpcResponse;
use turul_mcp_json_rpc_server::prelude::JsonRpcVersion as _use_52_JsonRpcVersion;
use turul_mcp_json_rpc_server::prelude::METHOD_NOT_FOUND as _use_53_METHOD_NOT_FOUND;
use turul_mcp_json_rpc_server::prelude::PARSE_ERROR as _use_54_PARSE_ERROR;
use turul_mcp_json_rpc_server::prelude::RequestId as _use_55_RequestId;
use turul_mcp_json_rpc_server::prelude::RequestParams as _use_56_RequestParams;
use turul_mcp_json_rpc_server::prelude::ResponseResult as _use_57_ResponseResult;
use turul_mcp_json_rpc_server::prelude::SERVER_ERROR_END as _use_58_SERVER_ERROR_END;
use turul_mcp_json_rpc_server::prelude::SERVER_ERROR_START as _use_59_SERVER_ERROR_START;
use turul_mcp_json_rpc_server::prelude::SessionContext as _use_60_SessionContext;
use turul_mcp_json_rpc_server::request;
use turul_mcp_json_rpc_server::request::JsonRpcRequest as _use_62_JsonRpcRequest;
use turul_mcp_json_rpc_server::request::RequestParams as _use_63_RequestParams;
use turul_mcp_json_rpc_server::RequestId as _use_64_RequestId;
use turul_mcp_json_rpc_server::RequestParams as _use_65_RequestParams;
use turul_mcp_json_rpc_server::response;
use turul_mcp_json_rpc_server::response::JsonRpcMessage as _use_67_JsonRpcMessage;
use turul_mcp_json_rpc_server::response::JsonRpcResponse as _use_68_JsonRpcResponse;
use turul_mcp_json_rpc_server::response::ResponseResult as _use_69_ResponseResult;
use turul_mcp_json_rpc_server::ResponseResult as _use_70_ResponseResult;
use turul_mcp_json_rpc_server::SessionContext as _use_71_SessionContext;
use turul_mcp_json_rpc_server::types;
use turul_mcp_json_rpc_server::types::JsonRpcVersion as _use_73_JsonRpcVersion;
use turul_mcp_json_rpc_server::types::RequestId as _use_74_RequestId;

#[test] fn symbol_coverage_compiles() {}
