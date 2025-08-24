//! Streaming Response Infrastructure
//!
//! SSE (Server-Sent Events) streaming for real-time MCP notifications in Lambda environment

use crate::global_events::EventFilter;
use lambda_http::{Body, Response};
use lambda_runtime::Error as LambdaError;
use serde_json::{Value, json};
use std::time::Duration;
use tracing::{debug, info};

/// SSE streaming response configuration
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SseConfig {
    /// Heartbeat interval to keep connection alive
    pub heartbeat_interval: Duration,
    /// Stream timeout for inactive connections
    pub stream_timeout: Duration,
    /// Maximum message buffer size
    pub max_buffer_size: usize,
}

impl Default for SseConfig {
    fn default() -> Self {
        Self {
            heartbeat_interval: Duration::from_secs(30),
            stream_timeout: Duration::from_secs(900), // 15 minutes
            max_buffer_size: 1000,
        }
    }
}

/// Create an SSE streaming response for MCP events (simplified for Lambda)
pub async fn _create_sse_stream(
    _filter: Option<EventFilter>,
    _config: Option<SseConfig>,
) -> Result<Response<Body>, LambdaError> {
    let _config = _config.unwrap_or_default();
    let _filter = _filter.unwrap_or_default();
    
    info!("Creating SSE stream for Lambda environment (simplified)");
    
    // In Lambda environment, true streaming is limited
    // Return a simple SSE response with connection info
    let sse_response = format!(
        "data: {}\n\n",
        serde_json::to_string(&json!({
            "type": "connection",
            "status": "connected",
            "message": "SSE streaming in Lambda environment (limited)",
            "timestamp": chrono::Utc::now().to_rfc3339()
        })).unwrap_or_default()
    );
    
    // Create the HTTP response with SSE headers
    let response = Response::builder()
        .status(200)
        .header("Content-Type", "text/event-stream")
        .header("Cache-Control", "no-cache")
        .header("Connection", "keep-alive")
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Allow-Headers", "Content-Type, mcp-session-id")
        .body(Body::from(sse_response))?;
    
    Ok(response)
}

/// Create a streaming response for tool execution with real-time updates (simplified for Lambda)
pub async fn _create_tool_streaming_response(
    tool_name: String,
    session_id: String,
    initial_result: Value,
) -> Result<Response<Body>, LambdaError> {
    info!("Creating streaming response for tool: {} in session: {}", tool_name, session_id);
    
    // In Lambda environment, return the initial result as SSE
    let initial_message = json!({
        "type": "tool_result",
        "tool": tool_name,
        "session_id": session_id,
        "result": initial_result,
        "timestamp": chrono::Utc::now().to_rfc3339()
    });
    
    let sse_response = format!("data: {}\n\n", serde_json::to_string(&initial_message).unwrap_or_default());
    
    // Create HTTP response
    let response = Response::builder()
        .status(200)
        .header("Content-Type", "text/event-stream")
        .header("Cache-Control", "no-cache")
        .header("Connection", "keep-alive")
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Allow-Headers", "Content-Type, mcp-session-id")
        .body(Body::from(sse_response))?;
    
    Ok(response)
}

/// Handle SSE connection requests with proper MCP session handling
pub async fn _handle_sse_request(
    session_id: Option<String>,
    event_types: Option<Vec<String>>,
) -> Result<Response<Body>, LambdaError> {
    debug!("Handling SSE request for session: {:?}", session_id);
    
    // Create event filter based on request parameters
    let mut filter = EventFilter::new();
    
    if let Some(types) = event_types {
        filter = filter.with_event_types(types);
    }
    
    if let Some(session) = session_id {
        filter = filter.with_session_id(session);
    }
    
    // Create the SSE stream
    _create_sse_stream(Some(filter), None).await
}

/// Parse SSE request parameters from query string or headers
pub fn _parse_sse_parameters(
    _query_string: Option<&str>,
    _headers: &http::HeaderMap,
) -> (Option<String>, Option<Vec<String>>) {
    let mut session_id = None;
    let mut event_types = None;
    
    // Try to get session ID from header first
    if let Some(header_value) = _headers.get("mcp-session-id") {
        if let Ok(id) = header_value.to_str() {
            session_id = Some(id.to_string());
        }
    }
    
    // Parse query parameters if provided
    if let Some(query) = _query_string {
        for param in query.split('&') {
            if let Some((key, value)) = param.split_once('=') {
                match key {
                    "session_id" => {
                        if session_id.is_none() {
                            session_id = Some(urlencoding::decode(value).unwrap_or_default().to_string());
                        }
                    }
                    "event_types" => {
                        let types: Vec<String> = value
                            .split(',')
                            .map(|t| urlencoding::decode(t).unwrap_or_default().to_string())
                            .collect();
                        if !types.is_empty() {
                            event_types = Some(types);
                        }
                    }
                    _ => {} // Ignore unknown parameters
                }
            }
        }
    }
    
    (session_id, event_types)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sse_config_default() {
        let config = SseConfig::default();
        assert_eq!(config.heartbeat_interval, Duration::from_secs(30));
        assert_eq!(config.stream_timeout, Duration::from_secs(900));
        assert_eq!(config.max_buffer_size, 1000);
    }

    #[test]
    fn test_parse_sse_parameters() {
        let mut headers = http::HeaderMap::new();
        headers.insert("mcp-session-id", "session-123".parse().unwrap());
        
        let query = "event_types=tool_execution,system_health&session_id=query-session";
        let (session_id, event_types) = parse_sse_parameters(Some(query), &headers);
        
        // Header should take precedence over query
        assert_eq!(session_id, Some("session-123".to_string()));
        assert_eq!(event_types, Some(vec!["tool_execution".to_string(), "system_health".to_string()]));
    }

    #[test]
    fn test_parse_sse_parameters_no_header() {
        let headers = http::HeaderMap::new();
        let query = "session_id=query-session&event_types=monitoring_update";
        
        let (session_id, event_types) = parse_sse_parameters(Some(query), &headers);
        
        assert_eq!(session_id, Some("query-session".to_string()));
        assert_eq!(event_types, Some(vec!["monitoring_update".to_string()]));
    }
}