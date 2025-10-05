//! Test utilities specific to roots protocol testing

use serde_json::{json, Value};
use std::collections::HashMap;

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
        .cloned()
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
    if let Some(name) = root_obj.get("name")
        && !name.is_string() {
            return false;
        }

    true
}

/// Helper to check if URI is in allowed roots
/// Handles path traversal attacks by normalizing paths
pub fn is_uri_in_allowed_roots(uri: &str, allowed_roots: &[&str]) -> bool {
    // Extract the path component from the URI
    let path = if let Some(path) = uri.strip_prefix("file://") {
        path
    } else {
        uri
    };

    // Normalize path by resolving .. and . components
    let normalized = normalize_path(path);

    // Check if normalized path starts with any allowed root
    allowed_roots.iter().any(|root| {
        let root_path = root.strip_prefix("file://").unwrap_or(root);
        normalized.starts_with(root_path)
    })
}

/// Normalize a path by resolving . and .. components
fn normalize_path(path: &str) -> String {
    let mut components = Vec::new();

    for component in path.split('/') {
        match component {
            "" | "." => continue,
            ".." => {
                components.pop();
            }
            c => components.push(c),
        }
    }

    format!("/{}", components.join("/"))
}
