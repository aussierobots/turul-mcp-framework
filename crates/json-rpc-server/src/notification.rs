use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::{request::RequestParams, types::JsonRpcVersion};

/// A JSON-RPC notification (request without an id)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcNotification {
    #[serde(rename = "jsonrpc")]
    pub version: JsonRpcVersion,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<RequestParams>,
}

impl JsonRpcNotification {
    pub fn new(method: String, params: Option<RequestParams>) -> Self {
        Self {
            version: JsonRpcVersion::V2_0,
            method,
            params,
        }
    }

    /// Create a new notification with no parameters
    pub fn new_no_params(method: String) -> Self {
        Self::new(method, None)
    }

    /// Create a new notification with object parameters
    pub fn new_with_object_params(
        method: String,
        params: HashMap<String, Value>,
    ) -> Self {
        Self::new(method, Some(RequestParams::Object(params)))
    }

    /// Create a new notification with array parameters
    pub fn new_with_array_params(method: String, params: Vec<Value>) -> Self {
        Self::new(method, Some(RequestParams::Array(params)))
    }

    /// Get a parameter by name (if params are an object)
    pub fn get_param(&self, name: &str) -> Option<&Value> {
        self.params.as_ref()?.get(name)
    }

    /// Get a parameter by index (if params are an array)
    pub fn get_param_index(&self, index: usize) -> Option<&Value> {
        self.params.as_ref()?.get_index(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, from_str, to_string};

    #[test]
    fn test_notification_serialization() {
        let notification = JsonRpcNotification::new_no_params("test_notification".to_string());

        let json_str = to_string(&notification).unwrap();
        let parsed: JsonRpcNotification = from_str(&json_str).unwrap();

        assert_eq!(parsed.method, "test_notification");
        assert!(parsed.params.is_none());
    }

    #[test]
    fn test_notification_with_params() {
        let mut params = HashMap::new();
        params.insert("message".to_string(), json!("Hello"));
        params.insert("level".to_string(), json!("info"));

        let notification = JsonRpcNotification::new_with_object_params(
            "log".to_string(),
            params,
        );

        assert_eq!(notification.get_param("message"), Some(&json!("Hello")));
        assert_eq!(notification.get_param("level"), Some(&json!("info")));
    }

    #[test]
    fn test_notification_json_format() {
        let notification = JsonRpcNotification::new_no_params("ping".to_string());
        let json_str = to_string(&notification).unwrap();
        
        // Should not contain an "id" field
        assert!(!json_str.contains("\"id\""));
        assert!(json_str.contains("\"jsonrpc\":\"2.0\""));
        assert!(json_str.contains("\"method\":\"ping\""));
    }
}