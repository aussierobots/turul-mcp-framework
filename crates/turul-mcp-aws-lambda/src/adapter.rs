//! HTTP type conversion utilities for Lambda MCP requests
//!
//! This module provides comprehensive conversion between lambda_http and hyper types,
//! enabling seamless integration between Lambda's HTTP model and the SessionMcpHandler.

use std::collections::HashMap;

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use lambda_http::{Body as LambdaBody, Request as LambdaRequest, Response as LambdaResponse};
use hyper::Response as HyperResponse;
use tracing::{debug, trace};

use crate::error::{LambdaError, Result};

/// Type alias for the unified MCP response body used by SessionMcpHandler
type UnifiedMcpBody = http_body_util::combinators::UnsyncBoxBody<Bytes, hyper::Error>;

/// Error mapping function for Full<Bytes>
fn infallible_to_hyper_error(never: std::convert::Infallible) -> hyper::Error {
    match never {}
}

/// Type alias for Full<Bytes> with mapped error type compatible with SessionMcpHandler
type MappedFullBody = http_body_util::combinators::MapErr<Full<Bytes>, fn(std::convert::Infallible) -> hyper::Error>;

/// Convert lambda_http::Request to hyper::Request<MappedFullBody>
///
/// This enables delegation to SessionMcpHandler by converting Lambda's request format
/// to the hyper format expected by the framework. All headers are preserved.
pub fn lambda_to_hyper_request(
    lambda_req: LambdaRequest,
) -> Result<hyper::Request<MappedFullBody>> {
    let (parts, lambda_body) = lambda_req.into_parts();

    // Convert LambdaBody to Bytes
    let body_bytes = match lambda_body {
        LambdaBody::Empty => Bytes::new(),
        LambdaBody::Text(s) => Bytes::from(s),
        LambdaBody::Binary(b) => Bytes::from(b),
    };

    // Create Full<Bytes> body and map error type to hyper::Error
    let full_body = Full::new(body_bytes).map_err(infallible_to_hyper_error as fn(std::convert::Infallible) -> hyper::Error);

    // Create hyper Request with preserved headers and new body type
    let hyper_req = hyper::Request::from_parts(parts, full_body);

    debug!(
        "Converted Lambda request: {} {} -> hyper::Request<Full<Bytes>>",
        hyper_req.method(),
        hyper_req.uri()
    );

    Ok(hyper_req)
}

/// Convert hyper::Response<UnifiedMcpBody> to lambda_http::Response<LambdaBody>
///
/// This collects the streaming body into a LambdaBody for non-streaming responses.
/// Used by the handle() method which returns snapshot responses.
pub async fn hyper_to_lambda_response(
    hyper_resp: HyperResponse<UnifiedMcpBody>,
) -> Result<LambdaResponse<LambdaBody>> {
    let (parts, body) = hyper_resp.into_parts();

    // Collect the body into bytes
    let body_bytes = match body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(err) => {
            return Err(LambdaError::Body(format!(
                "Failed to collect response body: {}",
                err
            )));
        }
    };

    // Convert to LambdaBody
    let lambda_body = if body_bytes.is_empty() {
        LambdaBody::Empty
    } else {
        // Try to convert to text if it's valid UTF-8, otherwise use binary
        match String::from_utf8(body_bytes.to_vec()) {
            Ok(text) => LambdaBody::Text(text),
            Err(_) => LambdaBody::Binary(body_bytes.to_vec()),
        }
    };

    // Create Lambda response with preserved headers
    let lambda_resp = LambdaResponse::from_parts(parts, lambda_body);

    debug!(
        "Converted hyper response -> Lambda response (status: {})",
        lambda_resp.status()
    );

    Ok(lambda_resp)
}

/// Convert hyper::Response<UnifiedMcpBody> to lambda_http streaming response
///
/// This preserves the streaming body for real-time SSE responses.
/// Used by the handle_streaming() method for true streaming.
pub fn hyper_to_lambda_streaming(
    hyper_resp: HyperResponse<UnifiedMcpBody>,
) -> lambda_http::Response<UnifiedMcpBody> {
    let (parts, body) = hyper_resp.into_parts();

    // Direct passthrough - no body collection, preserves streaming
    let lambda_resp = lambda_http::Response::from_parts(parts, body);

    debug!(
        "Converted hyper response -> Lambda streaming response (status: {})",
        lambda_resp.status()
    );

    lambda_resp
}

/// Extract MCP-specific headers from Lambda request context
///
/// Lambda requests may have additional context that needs to be preserved
/// for proper MCP protocol handling.
pub fn extract_mcp_headers(req: &LambdaRequest) -> HashMap<String, String> {
    let mut mcp_headers = HashMap::new();

    // Extract session ID from headers
    if let Some(session_id) = req.headers().get("mcp-session-id")
        && let Ok(session_id_str) = session_id.to_str()
    {
        mcp_headers.insert("mcp-session-id".to_string(), session_id_str.to_string());
    }

    // Extract protocol version
    if let Some(protocol_version) = req.headers().get("mcp-protocol-version")
        && let Ok(version_str) = protocol_version.to_str()
    {
        mcp_headers.insert("mcp-protocol-version".to_string(), version_str.to_string());
    }

    // Extract Last-Event-ID for SSE resumability
    if let Some(last_event_id) = req.headers().get("last-event-id")
        && let Ok(event_id_str) = last_event_id.to_str()
    {
        mcp_headers.insert("last-event-id".to_string(), event_id_str.to_string());
    }

    trace!("Extracted MCP headers: {:?}", mcp_headers);
    mcp_headers
}

/// Add MCP-specific headers to Lambda response
///
/// Ensures proper MCP protocol headers are included in the response.
pub fn inject_mcp_headers(resp: &mut LambdaResponse<LambdaBody>, headers: HashMap<String, String>) {
    for (name, value) in headers {
        if let (Ok(header_name), Ok(header_value)) = (
            http::HeaderName::from_bytes(name.as_bytes()),
            http::HeaderValue::from_str(&value),
        ) {
            resp.headers_mut().insert(header_name, header_value);
            debug!("Injected MCP header: {} = {}", name, value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::{HeaderValue, Method, Request, StatusCode};
    use http_body_util::Full;

    #[test]
    fn test_lambda_to_hyper_request_conversion() {
        // Create a test Lambda request with headers and body
        let mut lambda_req = Request::builder()
            .method(Method::POST)
            .uri("/mcp")
            .body(LambdaBody::Text(r#"{"jsonrpc":"2.0","method":"initialize","id":1}"#.to_string()))
            .unwrap();

        // Add MCP headers
        let headers = lambda_req.headers_mut();
        headers.insert("content-type", HeaderValue::from_static("application/json"));
        headers.insert("mcp-session-id", HeaderValue::from_static("test-session-123"));
        headers.insert("mcp-protocol-version", HeaderValue::from_static("2025-06-18"));

        // Test the conversion
        let hyper_req = lambda_to_hyper_request(lambda_req).unwrap();

        // Verify method and URI are preserved
        assert_eq!(hyper_req.method(), &Method::POST);
        assert_eq!(hyper_req.uri().path(), "/mcp");

        // Verify headers are preserved
        assert_eq!(
            hyper_req.headers().get("content-type").unwrap(),
            "application/json"
        );
        assert_eq!(
            hyper_req.headers().get("mcp-session-id").unwrap(),
            "test-session-123"
        );
        assert_eq!(
            hyper_req.headers().get("mcp-protocol-version").unwrap(),
            "2025-06-18"
        );
    }

    #[test]
    fn test_lambda_to_hyper_empty_body() {
        let lambda_req = Request::builder()
            .method(Method::GET)
            .uri("/sse")
            .body(LambdaBody::Empty)
            .unwrap();

        let hyper_req = lambda_to_hyper_request(lambda_req).unwrap();
        assert_eq!(hyper_req.method(), &Method::GET);
        assert_eq!(hyper_req.uri().path(), "/sse");
    }

    #[test]
    fn test_lambda_to_hyper_binary_body() {
        let test_data = vec![0x48, 0x65, 0x6c, 0x6c, 0x6f]; // "Hello" in bytes
        let lambda_req = Request::builder()
            .method(Method::POST)
            .uri("/binary")
            .body(LambdaBody::Binary(test_data.clone()))
            .unwrap();

        let hyper_req = lambda_to_hyper_request(lambda_req).unwrap();
        assert_eq!(hyper_req.method(), &Method::POST);
        assert_eq!(hyper_req.uri().path(), "/binary");
    }

    #[tokio::test]
    async fn test_hyper_to_lambda_response_conversion() {
        // Create a test hyper response
        let json_body = r#"{"jsonrpc":"2.0","id":1,"result":{"capabilities":{}}}"#;
        let full_body = Full::new(Bytes::from(json_body));
        let boxed_body = full_body.map_err(|never| match never {}).boxed_unsync();

        let hyper_resp = hyper::Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "application/json")
            .header("mcp-session-id", "resp-session-456")
            .body(boxed_body)
            .unwrap();

        // Test the conversion
        let lambda_resp = hyper_to_lambda_response(hyper_resp).await.unwrap();

        // Verify status and headers are preserved
        assert_eq!(lambda_resp.status(), StatusCode::OK);
        assert_eq!(
            lambda_resp.headers().get("content-type").unwrap(),
            "application/json"
        );
        assert_eq!(
            lambda_resp.headers().get("mcp-session-id").unwrap(),
            "resp-session-456"
        );

        // Verify body is converted to text
        match lambda_resp.body() {
            LambdaBody::Text(text) => assert_eq!(text, json_body),
            _ => panic!("Expected text body"),
        }
    }

    #[tokio::test]
    async fn test_hyper_to_lambda_empty_response() {
        let empty_body = Full::new(Bytes::new());
        let boxed_body = empty_body.map_err(|never| match never {}).boxed_unsync();

        let hyper_resp = hyper::Response::builder()
            .status(StatusCode::NO_CONTENT)
            .body(boxed_body)
            .unwrap();

        let lambda_resp = hyper_to_lambda_response(hyper_resp).await.unwrap();

        assert_eq!(lambda_resp.status(), StatusCode::NO_CONTENT);
        match lambda_resp.body() {
            LambdaBody::Empty => {} // Expected
            _ => panic!("Expected empty body"),
        }
    }

    #[test]
    fn test_hyper_to_lambda_streaming() {
        // Create a streaming response
        let stream_body = Full::new(Bytes::from("data: test\n\n"));
        let boxed_body = stream_body.map_err(|never| match never {}).boxed_unsync();

        let hyper_resp = hyper::Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "text/event-stream")
            .header("cache-control", "no-cache")
            .body(boxed_body)
            .unwrap();

        // Test streaming conversion (should preserve body as-is)
        let lambda_resp = hyper_to_lambda_streaming(hyper_resp);

        assert_eq!(lambda_resp.status(), StatusCode::OK);
        assert_eq!(
            lambda_resp.headers().get("content-type").unwrap(),
            "text/event-stream"
        );
        assert_eq!(
            lambda_resp.headers().get("cache-control").unwrap(),
            "no-cache"
        );
        // Body should be preserved as UnifiedMcpBody for streaming
    }

    #[tokio::test]
    async fn test_mcp_headers_extraction() {
        use http::{HeaderValue, Request};

        // Create a test request with MCP headers
        let mut request = Request::builder()
            .method("POST")
            .uri("/mcp")
            .body(LambdaBody::Empty)
            .unwrap();

        let headers = request.headers_mut();
        headers.insert("mcp-session-id", HeaderValue::from_static("sess-123"));
        headers.insert(
            "mcp-protocol-version",
            HeaderValue::from_static("2025-06-18"),
        );
        headers.insert("last-event-id", HeaderValue::from_static("event-456"));

        let mcp_headers = extract_mcp_headers(&request);

        assert_eq!(
            mcp_headers.get("mcp-session-id"),
            Some(&"sess-123".to_string())
        );
        assert_eq!(
            mcp_headers.get("mcp-protocol-version"),
            Some(&"2025-06-18".to_string())
        );
        assert_eq!(
            mcp_headers.get("last-event-id"),
            Some(&"event-456".to_string())
        );
    }

    #[tokio::test]
    async fn test_mcp_headers_injection() {
        use lambda_http::Body;

        let mut lambda_resp = LambdaResponse::builder()
            .status(200)
            .body(Body::Empty)
            .unwrap();

        let mut headers = HashMap::new();
        headers.insert("mcp-session-id".to_string(), "sess-789".to_string());
        headers.insert("mcp-protocol-version".to_string(), "2025-06-18".to_string());

        inject_mcp_headers(&mut lambda_resp, headers);

        assert_eq!(
            lambda_resp.headers().get("mcp-session-id").unwrap(),
            "sess-789"
        );
        assert_eq!(
            lambda_resp.headers().get("mcp-protocol-version").unwrap(),
            "2025-06-18"
        );
    }
}
