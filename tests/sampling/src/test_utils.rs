//! Test utilities specific to sampling protocol testing

use serde_json::{Value, json};
use std::collections::HashMap;

/// Helper to create sampling capabilities for initialize
pub fn sampling_capabilities() -> Value {
    json!({
        "sampling": {}
    })
}

/// Helper to extract sampling message content from response
///
/// MCP 2025-11-25 flattened CreateMessageResult: role and content are at the top level
/// (no nested "message" wrapper).
pub fn extract_sampling_message(response: &HashMap<String, Value>) -> Option<String> {
    response
        .get("result")
        .and_then(|r| r.as_object())
        .and_then(|obj| obj.get("content"))
        .and_then(|content| content.as_object())
        .and_then(|content_obj| content_obj.get("text"))
        .and_then(|text| text.as_str())
        .map(|s| s.to_string())
}

/// Helper to validate CreateMessageRequest structure
pub fn validate_create_message_request(params: &Value) -> bool {
    params.is_object()
        && params
            .get("messages")
            .map(|m| m.is_array())
            .unwrap_or(false)
        && params
            .get("maxTokens")
            .map(|t| t.is_number())
            .unwrap_or(false)
}

/// Helper to create a basic create message request
pub fn create_message_request(content: &str, max_tokens: u32) -> Value {
    json!({
        "messages": [
            {
                "role": "user",
                "content": {
                    "type": "text",
                    "text": content
                }
            }
        ],
        "maxTokens": max_tokens,
        "temperature": 0.7
    })
}
