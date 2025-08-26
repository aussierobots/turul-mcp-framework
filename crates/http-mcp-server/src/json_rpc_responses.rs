//! JSON-RPC 2.0 response builders for HTTP transport
//!
//! This module provides functions that always return JSON-RPC 2.0 conformant 
//! HTTP responses with proper headers and body content.

use hyper::{Response, StatusCode, header};
use http_body_util::Full;
use bytes::Bytes;
use serde_json::Value;
use tracing::error;

use mcp_json_rpc_server::{
    JsonRpcResponse, JsonRpcError,
    error::JsonRpcErrorObject,
    types::RequestId
};

/// HTTP body type for JSON-RPC responses
type JsonRpcBody = Full<Bytes>;

/// Build an HTTP response containing a JSON-RPC error object.
/// Always returns a proper JSON-RPC 2.0 conformant response.
pub fn jsonrpc_error_response(
    id: RequestId,
    code: i64,
    message: &str,
    data: Option<Value>,
) -> Result<Response<JsonRpcBody>, hyper::Error> {
    let error_obj = JsonRpcErrorObject {
        code,
        message: message.to_string(), 
        data,
    };
    
    let err = JsonRpcError::new(Some(id), error_obj);
    
    let body_bytes = serde_json::to_vec(&err)
        .unwrap_or_else(|e| {
            error!("Failed to serialize JSON-RPC error: {}", e);
            b"{}".to_vec()
        });

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Full::new(Bytes::from(body_bytes)))
        .unwrap())
}

/// Build an HTTP response for JSON-RPC notifications (202 Accepted per MCP 2025-06-18).
pub fn jsonrpc_notification_response() -> Result<Response<JsonRpcBody>, hyper::Error> {
    Ok(Response::builder()
        .status(StatusCode::ACCEPTED) // MCP 2025-06-18: 202 Accepted for notifications
        .header(header::CONTENT_TYPE, "application/json")
        .body(Full::new(Bytes::new()))
        .unwrap())
}

/// Build an HTTP response containing a successful JSON-RPC response.
pub fn jsonrpc_success_response(
    id: RequestId, 
    result: Value
) -> Result<Response<JsonRpcBody>, hyper::Error> {
    let response = JsonRpcResponse::success(id, result);
    
    let body_bytes = serde_json::to_vec(&response)
        .unwrap_or_else(|e| {
            error!("Failed to serialize JSON-RPC response: {}", e);
            b"{}".to_vec()
        });

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Full::new(Bytes::from(body_bytes)))
        .unwrap())
}

/// Build a generic JSON-RPC response with session header support.
pub fn jsonrpc_response_with_session(
    response: JsonRpcResponse,
    session_id: Option<String>
) -> Result<Response<JsonRpcBody>, hyper::Error> {
    let body_bytes = serde_json::to_vec(&response)
        .unwrap_or_else(|e| {
            error!("Failed to serialize JSON-RPC response: {}", e);
            b"{}".to_vec()
        });

    let mut builder = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json");
        
    if let Some(session_id) = session_id {
        builder = builder.header("Mcp-Session-Id", session_id);
    }

    Ok(builder
        .body(Full::new(Bytes::from(body_bytes)))
        .unwrap())
}

/// Build HTTP response for method not allowed (405).
pub fn method_not_allowed_response() -> Response<JsonRpcBody> {
    Response::builder()
        .status(StatusCode::METHOD_NOT_ALLOWED)
        .header("Allow", "GET, POST, DELETE, OPTIONS")
        .body(Full::new(Bytes::from("Method not allowed")))
        .unwrap()
}

/// Build HTTP response for not found (404).
pub fn not_found_response() -> Response<JsonRpcBody> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Full::new(Bytes::from("Not Found")))
        .unwrap()
}

/// Build HTTP response for bad request (400) with JSON-RPC parse error.
pub fn bad_request_response(message: &str) -> Response<JsonRpcBody> {
    let error_obj = JsonRpcErrorObject {
        code: -32700, // Parse error
        message: message.to_string(),
        data: None,
    };
    
    let err = JsonRpcError::new(None, error_obj);
    
    let body_bytes = serde_json::to_vec(&err)
        .unwrap_or_else(|_| b"{}".to_vec());

    Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Full::new(Bytes::from(body_bytes)))
        .unwrap()
}

/// Build HTTP response for OPTIONS preflight requests.
pub fn options_response() -> Response<JsonRpcBody> {
    Response::builder()
        .status(StatusCode::OK)
        .header("Access-Control-Allow-Methods", "GET, POST, DELETE, OPTIONS")
        .header("Access-Control-Allow-Headers", "Content-Type, Accept, MCP-Protocol-Version, Mcp-Session-Id, Last-Event-ID")
        .header("Access-Control-Max-Age", "86400")
        .body(Full::new(Bytes::new()))
        .unwrap()
}

/// Build HTTP response for SSE stream (proper headers for text/event-stream).
pub fn sse_response_headers() -> http::response::Builder {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/event-stream")
        .header(header::CACHE_CONTROL, "no-cache")
        .header("Connection", "keep-alive")
        .header("Access-Control-Allow-Origin", "*")
}