//! Test utilities specific to elicitation protocol testing

use serde_json::{Value, json};
use std::collections::HashMap;

/// Helper to create elicitation capabilities for initialize
pub fn elicitation_capabilities() -> Value {
    json!({
        "tools": {}
    })
}

/// Helper to extract elicitation request from tool result
pub fn extract_elicitation_request(response: &HashMap<String, Value>) -> Option<Value> {
    response
        .get("result")
        .and_then(|r| r.as_object())
        .and_then(|obj| obj.get("content"))
        .and_then(|content| content.as_array())
        .and_then(|arr| arr.first())
        .and_then(|item| item.as_object())
        .and_then(|item_obj| item_obj.get("text"))
        .and_then(|text| text.as_str())
        .and_then(|text_str| {
            // Try to extract JSON from text (elicitation data might be embedded)
            if text_str.contains("elicitation_request") {
                serde_json::from_str::<Value>(text_str).ok()
            } else {
                None
            }
        })
}

/// Helper to validate elicitation workflow structure
pub fn validate_workflow_structure(workflow_data: &Value) -> bool {
    let workflow = match workflow_data.as_object() {
        Some(obj) => obj,
        None => return false,
    };

    // Should have workflow identification
    if !workflow.contains_key("workflow_id") && !workflow.contains_key("workflow_type") {
        return false;
    }

    // Should have current step information
    if !workflow.contains_key("current_step") {
        return false;
    }

    // Should have field count or schema information
    if !workflow.contains_key("field_count") && !workflow.contains_key("elicitation_request") {
        return false;
    }

    true
}
