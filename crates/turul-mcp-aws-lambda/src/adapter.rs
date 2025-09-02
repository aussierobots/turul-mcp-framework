//! HTTP header utilities for Lambda MCP requests
//!
//! This module provides utilities for handling MCP-specific HTTP headers
//! in the Lambda execution environment.

use std::collections::HashMap;

use lambda_http::{Body as LambdaBody, Request as LambdaRequest, Response as LambdaResponse};
use tracing::{debug, trace};

/// Extract MCP-specific headers from Lambda request context
/// 
/// Lambda requests may have additional context that needs to be preserved
/// for proper MCP protocol handling.
pub fn extract_mcp_headers(req: &LambdaRequest) -> HashMap<String, String> {
    let mut mcp_headers = HashMap::new();
    
    // Extract session ID from headers
    if let Some(session_id) = req.headers().get("mcp-session-id") {
        if let Ok(session_id_str) = session_id.to_str() {
            mcp_headers.insert("mcp-session-id".to_string(), session_id_str.to_string());
        }
    }
    
    // Extract protocol version
    if let Some(protocol_version) = req.headers().get("mcp-protocol-version") {
        if let Ok(version_str) = protocol_version.to_str() {
            mcp_headers.insert("mcp-protocol-version".to_string(), version_str.to_string());
        }
    }
    
    // Extract Last-Event-ID for SSE resumability
    if let Some(last_event_id) = req.headers().get("last-event-id") {
        if let Ok(event_id_str) = last_event_id.to_str() {
            mcp_headers.insert("last-event-id".to_string(), event_id_str.to_string());
        }
    }
    
    trace!("Extracted MCP headers: {:?}", mcp_headers);
    mcp_headers
}

/// Add MCP-specific headers to Lambda response
/// 
/// Ensures proper MCP protocol headers are included in the response.
pub fn inject_mcp_headers(
    resp: &mut LambdaResponse<LambdaBody>, 
    headers: HashMap<String, String>
) {
    for (name, value) in headers {
        if let (Ok(header_name), Ok(header_value)) = (
            http::HeaderName::from_bytes(name.as_bytes()),
            http::HeaderValue::from_str(&value)
        ) {
            resp.headers_mut().insert(header_name, header_value);
            debug!("Injected MCP header: {} = {}", name, value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_mcp_headers_extraction() {
        use http::{Request, HeaderValue};
        
        // Create a test request with MCP headers
        let mut request = Request::builder()
            .method("POST")
            .uri("/mcp")
            .body(LambdaBody::Empty)
            .unwrap();
        
        let headers = request.headers_mut();
        headers.insert("mcp-session-id", HeaderValue::from_static("sess-123"));
        headers.insert("mcp-protocol-version", HeaderValue::from_static("2025-06-18"));
        headers.insert("last-event-id", HeaderValue::from_static("event-456"));
        
        let mcp_headers = extract_mcp_headers(&request);
        
        assert_eq!(mcp_headers.get("mcp-session-id"), Some(&"sess-123".to_string()));
        assert_eq!(mcp_headers.get("mcp-protocol-version"), Some(&"2025-06-18".to_string()));
        assert_eq!(mcp_headers.get("last-event-id"), Some(&"event-456".to_string()));
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