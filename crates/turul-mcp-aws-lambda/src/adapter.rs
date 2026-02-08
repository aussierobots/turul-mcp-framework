//! HTTP type conversion utilities for Lambda MCP requests
//!
//! This module provides comprehensive conversion between lambda_http and hyper types,
//! enabling seamless integration between Lambda's HTTP model and the SessionMcpHandler.

use std::collections::HashMap;
use std::str::FromStr;

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::Response as HyperResponse;
use lambda_http::{Body as LambdaBody, Request as LambdaRequest, Response as LambdaResponse};
use tracing::{debug, trace};

use crate::error::{LambdaError, Result};

/// Type alias for the unified MCP response body used by SessionMcpHandler
type UnifiedMcpBody = http_body_util::combinators::UnsyncBoxBody<Bytes, hyper::Error>;

/// Error mapping function for Full<Bytes>
fn infallible_to_hyper_error(never: std::convert::Infallible) -> hyper::Error {
    match never {}
}

/// Type alias for Full<Bytes> with mapped error type compatible with SessionMcpHandler
type MappedFullBody =
    http_body_util::combinators::MapErr<Full<Bytes>, fn(std::convert::Infallible) -> hyper::Error>;

/// Convert lambda_http::Request to hyper::Request<MappedFullBody>
///
/// This enables delegation to SessionMcpHandler by converting Lambda's request format
/// to the hyper format expected by the framework. All headers are preserved, and Lambda
/// authorizer context (if present) is extracted and injected as `x-authorizer-*` headers.
///
/// # Authorizer Context
///
/// If the request includes API Gateway authorizer context, fields are extracted and
/// added as headers with the `x-authorizer-` prefix. This makes authorizer data
/// available to middleware via `RequestContext.metadata`.
///
/// Field names are sanitized (lowercase, alphanumeric + dash/underscore only).
/// Invalid header names/values are skipped gracefully.
pub fn lambda_to_hyper_request(
    lambda_req: LambdaRequest,
) -> Result<hyper::Request<MappedFullBody>> {
    // Extract authorizer context BEFORE consuming request
    let authorizer_fields = extract_authorizer_context(&lambda_req);

    // Convert to parts (consumes request)
    let (mut parts, lambda_body) = lambda_req.into_parts();

    // Inject authorizer fields as x-authorizer-* headers (defensive - skip failures)
    for (field_name, field_value) in authorizer_fields {
        let header_name = format!("x-authorizer-{}", field_name);

        // Try to create HeaderName and HeaderValue
        // Skip entry if either fails (defensive - don't break request)
        let Ok(name) = http::HeaderName::from_str(&header_name) else {
            debug!(
                "Skipping authorizer field '{}' - invalid header name",
                field_name
            );
            continue;
        };

        let Ok(value) = http::HeaderValue::from_str(&field_value) else {
            debug!(
                "Skipping authorizer field '{}' - invalid header value",
                field_name
            );
            continue;
        };

        parts.headers.insert(name, value);
        trace!("Injected authorizer header: {} = {}", header_name, field_value);
    }

    // Convert LambdaBody to Bytes
    let body_bytes = match lambda_body {
        LambdaBody::Empty => Bytes::new(),
        LambdaBody::Text(s) => Bytes::from(s),
        LambdaBody::Binary(b) => Bytes::from(b),
        _ => Bytes::new(),
    };

    // Create Full<Bytes> body and map error type to hyper::Error
    let full_body = Full::new(body_bytes)
        .map_err(infallible_to_hyper_error as fn(std::convert::Infallible) -> hyper::Error);

    // Create hyper Request with enhanced headers
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

/// Convert camelCase or PascalCase to snake_case
///
/// # Examples
///
/// ```
/// # use turul_mcp_aws_lambda::adapter::camel_to_snake;
/// assert_eq!(camel_to_snake("userId"), "user_id");
/// assert_eq!(camel_to_snake("deviceId"), "device_id");
/// assert_eq!(camel_to_snake("APIKey"), "api_key");
/// assert_eq!(camel_to_snake("HTTPSEnabled"), "https_enabled");
/// assert_eq!(camel_to_snake("user_id"), "user_id");
/// ```
pub fn camel_to_snake(s: &str) -> String {
    let mut result = String::new();
    let chars: Vec<char> = s.chars().collect();

    for i in 0..chars.len() {
        let ch = chars[i];

        if ch.is_uppercase() {
            let is_first = i == 0;
            let prev_is_lower = i > 0 && chars[i - 1].is_lowercase();
            let next_is_lower = i + 1 < chars.len() && chars[i + 1].is_lowercase();

            // Add underscore before uppercase if:
            // - Not at start AND
            // - (Previous was lowercase OR next is lowercase)
            if !is_first && (prev_is_lower || next_is_lower) {
                result.push('_');
            }

            result.push(ch.to_ascii_lowercase());
        } else {
            result.push(ch);
        }
    }

    result
}

/// Sanitize authorizer field name for use in HTTP headers
///
/// Converts field names to valid HTTP header format:
/// 1. Convert camelCase to snake_case (userId → user_id)
/// 2. ASCII lowercase
/// 3. Replace non-alphanumeric (except _ and -) with dash
///
/// # Examples
///
/// ```
/// # use turul_mcp_aws_lambda::adapter::sanitize_authorizer_field_name;
/// assert_eq!(sanitize_authorizer_field_name("userId"), "user_id");
/// assert_eq!(sanitize_authorizer_field_name("deviceId"), "device_id");
/// assert_eq!(sanitize_authorizer_field_name("device_id"), "device_id");
/// assert_eq!(sanitize_authorizer_field_name("user@email"), "user-email");
/// ```
pub fn sanitize_authorizer_field_name(field: &str) -> String {
    // Step 1: Convert camelCase to snake_case
    let snake_case = camel_to_snake(field);

    // Step 2: Sanitize for HTTP header compatibility
    snake_case
        .to_ascii_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect()
}

/// Extract authorizer context from Lambda request extensions
///
/// Supports both API Gateway V1 (REST API) and V2 (HTTP API) formats.
/// Returns HashMap with snake_case keys ready for header injection.
///
/// # Behavior
///
/// - Returns empty HashMap if no authorizer context present
/// - Converts camelCase to snake_case (userId → user_id)
/// - Skips fields that fail sanitization
/// - Converts non-string values to JSON strings
/// - Handles both `ApiGatewayV2.authorizer.fields` and `ApiGateway.authorizer["lambda"]`
///
/// # Examples
///
/// ```no_run
/// # use lambda_http::Request;
/// # use turul_mcp_aws_lambda::adapter::extract_authorizer_context;
/// # let request: Request = unimplemented!();
/// let fields = extract_authorizer_context(&request);
/// assert_eq!(fields.get("account_id"), Some(&"acc_123".to_string()));
/// ```
pub fn extract_authorizer_context(req: &LambdaRequest) -> HashMap<String, String> {
    use lambda_http::request::RequestContext;

    let mut fields = HashMap::new();

    // Get RequestContext from extensions
    let Some(request_context) = req.extensions().get::<RequestContext>() else {
        return fields; // No context, return empty
    };

    // Extract authorizer fields based on API Gateway version
    // V2 uses HashMap, V1 uses serde_json::Map - convert both to HashMap
    let mut authorizer_fields_map = HashMap::new();

    match request_context {
        RequestContext::ApiGatewayV2(ctx) => {
            // API Gateway V2 (HTTP API) format - already HashMap
            if let Some(ref authorizer) = ctx.authorizer {
                for (key, value) in &authorizer.fields {
                    authorizer_fields_map.insert(key.clone(), value.clone());
                }
            }
        }
        RequestContext::ApiGatewayV1(ctx) => {
            // API Gateway V1 (REST API) format - extract from authorizer.fields["lambda"]
            if let Some(serde_json::Value::Object(auth_map)) = ctx.authorizer.fields.get("lambda") {
                for (key, value) in auth_map {
                    authorizer_fields_map.insert(key.clone(), value.clone());
                }
            }
        }
        _ => {} // Other contexts (ALB, etc.) - no authorizer
    }

    // Convert extracted fields to sanitized headers
    for (key, value) in authorizer_fields_map {
        // Sanitize field name for header compatibility
        let sanitized_key = sanitize_authorizer_field_name(&key);

        // Convert value to string
        let value_str = match value {
            serde_json::Value::String(s) => s,
            other => other.to_string(), // JSON serialize non-strings
        };

        fields.insert(sanitized_key, value_str);
    }

    if !fields.is_empty() {
        debug!(
            "Extracted {} authorizer fields from Lambda context",
            fields.len()
        );
    }

    fields
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
            .body(LambdaBody::Text(
                r#"{"jsonrpc":"2.0","method":"initialize","id":1}"#.to_string(),
            ))
            .unwrap();

        // Add MCP headers
        let headers = lambda_req.headers_mut();
        headers.insert("content-type", HeaderValue::from_static("application/json"));
        headers.insert(
            "mcp-session-id",
            HeaderValue::from_static("test-session-123"),
        );
        headers.insert(
            "mcp-protocol-version",
            HeaderValue::from_static("2025-11-25"),
        );

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
            "2025-11-25"
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
            HeaderValue::from_static("2025-11-25"),
        );
        headers.insert("last-event-id", HeaderValue::from_static("event-456"));

        let mcp_headers = extract_mcp_headers(&request);

        assert_eq!(
            mcp_headers.get("mcp-session-id"),
            Some(&"sess-123".to_string())
        );
        assert_eq!(
            mcp_headers.get("mcp-protocol-version"),
            Some(&"2025-11-25".to_string())
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
        headers.insert("mcp-protocol-version".to_string(), "2025-11-25".to_string());

        inject_mcp_headers(&mut lambda_resp, headers);

        assert_eq!(
            lambda_resp.headers().get("mcp-session-id").unwrap(),
            "sess-789"
        );
        assert_eq!(
            lambda_resp.headers().get("mcp-protocol-version").unwrap(),
            "2025-11-25"
        );
    }

    // Authorizer context tests
    mod authorizer_tests {
        use super::*;

        #[test]
        fn test_sanitize_field_name_camelcase() {
            // camelCase → snake_case conversion
            assert_eq!(sanitize_authorizer_field_name("accountId"), "account_id");
            assert_eq!(sanitize_authorizer_field_name("entityType"), "entity_type");
            assert_eq!(sanitize_authorizer_field_name("deviceId"), "device_id");
            assert_eq!(sanitize_authorizer_field_name("userId"), "user_id");
            assert_eq!(sanitize_authorizer_field_name("tenantId"), "tenant_id");
            assert_eq!(sanitize_authorizer_field_name("customClaim"), "custom_claim");
        }

        #[test]
        fn test_sanitize_field_name_snake_case() {
            // Already snake_case - should remain unchanged
            assert_eq!(sanitize_authorizer_field_name("device_id"), "device_id");
            assert_eq!(sanitize_authorizer_field_name("user_name"), "user_name");
            assert_eq!(sanitize_authorizer_field_name("tenant_id"), "tenant_id");
        }

        #[test]
        fn test_sanitize_field_name_acronyms() {
            // Acronyms: treated as a single unit, underscore before transition to lowercase
            assert_eq!(sanitize_authorizer_field_name("APIKey"), "api_key");
            assert_eq!(sanitize_authorizer_field_name("HTTPSEnabled"), "https_enabled");
            assert_eq!(sanitize_authorizer_field_name("XMLParser"), "xml_parser");
        }

        #[test]
        fn test_sanitize_field_name_with_numbers() {
            // Numbers should be preserved
            assert_eq!(sanitize_authorizer_field_name("userId123"), "user_id123");
            assert_eq!(sanitize_authorizer_field_name("device2Id"), "device2_id");
        }

        #[test]
        fn test_sanitize_field_name_special_chars() {
            assert_eq!(sanitize_authorizer_field_name("user@email"), "user-email");
            assert_eq!(
                sanitize_authorizer_field_name("test.field"),
                "test-field"
            );
            assert_eq!(sanitize_authorizer_field_name("a/b/c"), "a-b-c");
        }

        #[test]
        fn test_sanitize_field_name_unicode() {
            // Unicode characters get replaced with dashes (one dash per character)
            assert_eq!(sanitize_authorizer_field_name("用户"), "--");
        }

        #[test]
        fn test_extract_authorizer_no_context() {
            // Request with no extensions
            let lambda_req = Request::builder()
                .method(Method::POST)
                .uri("/mcp")
                .body(LambdaBody::Empty)
                .unwrap();

            let fields = extract_authorizer_context(&lambda_req);
            assert!(fields.is_empty());
        }

        #[test]
        fn test_lambda_to_hyper_without_authorizer() {
            // Request without authorizer should work normally
            let lambda_req = Request::builder()
                .method(Method::POST)
                .uri("/mcp")
                .header("content-type", "application/json")
                .body(LambdaBody::Empty)
                .unwrap();

            let hyper_req = lambda_to_hyper_request(lambda_req).unwrap();

            // Should succeed, no authorizer headers
            assert!(hyper_req.headers().get("x-authorizer-account_id").is_none());
            assert_eq!(
                hyper_req.headers().get("content-type").unwrap(),
                "application/json"
            );
        }
    }
}
