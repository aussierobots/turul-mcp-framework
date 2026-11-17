//! JSON-RPC 2.0 response builders for the HTTP transport.
//!
//! This module provides functions that always return JSON-RPC 2.0 conformant
//! HTTP responses with proper headers and body content.

use bytes::Bytes;
use http_body_util::Full;
use hyper::{Response, StatusCode, header};
use serde_json::Value;
use tracing::error;

use turul_rpc::{JsonRpcError, JsonRpcResponse, error::JsonRpcErrorObject, types::RequestId};

/// HTTP body type for JSON-RPC responses.
type JsonRpcBody = Full<Bytes>;

/// Builds an HTTP 200 response containing a JSON-RPC 2.0 error object.
///
/// Falls back to an empty `{}` body if serialization fails.
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

    let body_bytes = serde_json::to_vec(&err).unwrap_or_else(|e| {
        error!("Failed to serialize JSON-RPC error: {}", e);
        b"{}".to_vec()
    });

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Full::new(Bytes::from(body_bytes)))
        .unwrap())
}

/// Builds the empty 202 Accepted HTTP response returned for JSON-RPC notifications.
pub fn jsonrpc_notification_response() -> Result<Response<JsonRpcBody>, hyper::Error> {
    Ok(Response::builder()
        .status(StatusCode::ACCEPTED) // MCP 2025-11-25: 202 Accepted for notifications
        .header(header::CONTENT_TYPE, "application/json")
        .body(Full::new(Bytes::new()))
        .unwrap())
}

/// Builds an HTTP 200 response containing a successful JSON-RPC response.
pub fn jsonrpc_success_response(
    id: RequestId,
    result: Value,
) -> Result<Response<JsonRpcBody>, hyper::Error> {
    let response = JsonRpcResponse::success(id, result.into());

    let body_bytes = serde_json::to_vec(&response).unwrap_or_else(|e| {
        error!("Failed to serialize JSON-RPC response: {}", e);
        b"{}".to_vec()
    });

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Full::new(Bytes::from(body_bytes)))
        .unwrap())
}

/// Builds an HTTP 200 response from a JSON-RPC response, adding an `Mcp-Session-Id` header when `session_id` is given.
///
/// # Panics
///
/// Panics if `session_id` is not a valid HTTP header value.
pub fn jsonrpc_response_with_session(
    response: JsonRpcResponse,
    session_id: Option<String>,
) -> Result<Response<JsonRpcBody>, hyper::Error> {
    let body_bytes = serde_json::to_vec(&response).unwrap_or_else(|e| {
        error!("Failed to serialize JSON-RPC response: {}", e);
        b"{}".to_vec()
    });

    let mut builder = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json");

    if let Some(session_id) = session_id {
        builder = builder.header("Mcp-Session-Id", session_id);
    }

    Ok(builder.body(Full::new(Bytes::from(body_bytes))).unwrap())
}

/// Builds a 405 Method Not Allowed response with an `Allow` header.
pub fn method_not_allowed_response() -> Response<JsonRpcBody> {
    Response::builder()
        .status(StatusCode::METHOD_NOT_ALLOWED)
        .header("Allow", "GET, POST, DELETE, OPTIONS")
        .body(Full::new(Bytes::from("Method not allowed")))
        .unwrap()
}

/// Builds a 404 Not Found response.
pub fn not_found_response() -> Response<JsonRpcBody> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Full::new(Bytes::from("Not Found")))
        .unwrap()
}

/// Builds a 400 Bad Request response carrying a JSON-RPC parse error (`-32700`) with the given message.
pub fn bad_request_response(message: &str) -> Response<JsonRpcBody> {
    let error_obj = JsonRpcErrorObject {
        code: -32700, // Parse error
        message: message.to_string(),
        data: None,
    };

    let err = JsonRpcError::new(None, error_obj);

    let body_bytes = serde_json::to_vec(&err).unwrap_or_else(|_| b"{}".to_vec());

    Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Full::new(Bytes::from(body_bytes)))
        .unwrap()
}

/// Builds the response for OPTIONS preflight requests.
///
/// Returns a bare 200 OK with an empty body. CORS headers are added by
/// `CorsLayer::apply_cors_headers` in `server.rs` when `enable_cors` is true.
pub fn options_response() -> Response<JsonRpcBody> {
    Response::builder()
        .status(StatusCode::OK)
        .body(Full::new(Bytes::new()))
        .unwrap()
}
