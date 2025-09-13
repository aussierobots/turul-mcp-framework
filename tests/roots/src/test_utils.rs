//! Test utilities specific to roots protocol testing

use std::collections::HashMap;
use serde_json::{json, Value};

/// Helper to create roots capabilities for initialize
pub fn roots_capabilities() -> Value {
    json!({
        "roots": {
            "listChanged": false
        }
    })
}

/// Helper to extract roots array from response
pub fn extract_roots_list(response: &HashMap<String, Value>) -> Option<Vec<Value>> {
    response
        .get("result")
        .and_then(|r| r.as_object())
        .and_then(|obj| obj.get("roots"))
        .and_then(|roots| roots.as_array())
        .map(|arr| arr.clone())
}

/// Helper to validate root object structure
pub fn validate_root_object(root: &Value) -> bool {
    let root_obj = match root.as_object() {
        Some(obj) => obj,
        None => return false,
    };

    // Required field: uri
    if !root_obj.contains_key("uri") {
        return false;
    }

    let uri = match root_obj.get("uri").and_then(|u| u.as_str()) {
        Some(uri_str) => uri_str,
        None => return false,
    };

    // URI should have valid scheme
    if !uri.contains("://") {
        return false;
    }

    // name field is optional but should be string if present
    if let Some(name) = root_obj.get("name") {
        if !name.is_string() {
            return false;
        }
    }

    true
}

/// Helper to check if URI is in allowed roots
pub fn is_uri_in_allowed_roots(uri: &str, allowed_roots: &[&str]) -> bool {
    allowed_roots.iter().any(|root| uri.starts_with(root))
}