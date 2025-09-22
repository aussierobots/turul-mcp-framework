use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::types::{JsonRpcVersion, RequestId};

/// Parameters for a JSON-RPC request
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum RequestParams {
    /// Positional parameters as an array
    Array(Vec<Value>),
    /// Named parameters as an object
    Object(HashMap<String, Value>),
}

impl RequestParams {
    /// Get a parameter by index (for array params) or name (for object params)
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            RequestParams::Object(map) => map.get(key),
            RequestParams::Array(_) => None, // Can't get by name from array
        }
    }

    /// Get a parameter by index (for array params only)
    pub fn get_index(&self, index: usize) -> Option<&Value> {
        match self {
            RequestParams::Array(vec) => vec.get(index),
            RequestParams::Object(_) => None, // Can't get by index from object
        }
    }

    /// Convert to HashMap for easier processing (arrays become indexed keys)
    pub fn to_map(&self) -> HashMap<String, Value> {
        match self {
            RequestParams::Object(map) => map.clone(),
            RequestParams::Array(vec) => vec
                .iter()
                .enumerate()
                .map(|(i, v)| (i.to_string(), v.clone()))
                .collect(),
        }
    }

    /// Check if parameters are empty
    pub fn is_empty(&self) -> bool {
        match self {
            RequestParams::Object(map) => map.is_empty(),
            RequestParams::Array(vec) => vec.is_empty(),
        }
    }

    /// Convert to a serde_json::Value for serialization
    pub fn to_value(&self) -> Value {
        match self {
            RequestParams::Object(map) => {
                Value::Object(map.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            }
            RequestParams::Array(arr) => Value::Array(arr.clone()),
        }
    }
}

impl From<HashMap<String, Value>> for RequestParams {
    fn from(map: HashMap<String, Value>) -> Self {
        RequestParams::Object(map)
    }
}

impl From<Vec<Value>> for RequestParams {
    fn from(vec: Vec<Value>) -> Self {
        RequestParams::Array(vec)
    }
}

/// A JSON-RPC request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    #[serde(rename = "jsonrpc")]
    pub version: JsonRpcVersion,
    pub id: RequestId,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<RequestParams>,
}

impl JsonRpcRequest {
    pub fn new(id: RequestId, method: String, params: Option<RequestParams>) -> Self {
        Self {
            version: JsonRpcVersion::V2_0,
            id,
            method,
            params,
        }
    }

    /// Create a new request with no parameters
    pub fn new_no_params(id: RequestId, method: String) -> Self {
        Self::new(id, method, None)
    }

    /// Create a new request with object parameters
    pub fn new_with_object_params(
        id: RequestId,
        method: String,
        params: HashMap<String, Value>,
    ) -> Self {
        Self::new(id, method, Some(RequestParams::Object(params)))
    }

    /// Create a new request with array parameters
    pub fn new_with_array_params(id: RequestId, method: String, params: Vec<Value>) -> Self {
        Self::new(id, method, Some(RequestParams::Array(params)))
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
    use serde_json::{from_str, json, to_string};

    #[test]
    fn test_request_serialization() {
        let request =
            JsonRpcRequest::new_no_params(RequestId::Number(1), "test_method".to_string());

        let json = to_string(&request).unwrap();
        let parsed: JsonRpcRequest = from_str(&json).unwrap();

        assert_eq!(parsed.id, RequestId::Number(1));
        assert_eq!(parsed.method, "test_method");
        assert!(parsed.params.is_none());
    }

    #[test]
    fn test_request_with_object_params() {
        let mut params = HashMap::new();
        params.insert("name".to_string(), json!("test"));
        params.insert("value".to_string(), json!(42));

        let request = JsonRpcRequest::new_with_object_params(
            RequestId::String("req1".to_string()),
            "set_value".to_string(),
            params,
        );

        assert_eq!(request.get_param("name"), Some(&json!("test")));
        assert_eq!(request.get_param("value"), Some(&json!(42)));
        assert_eq!(request.get_param("missing"), None);
    }

    #[test]
    fn test_request_with_array_params() {
        let params = vec![json!("test"), json!(42), json!(true)];

        let request = JsonRpcRequest::new_with_array_params(
            RequestId::Number(2),
            "process".to_string(),
            params,
        );

        assert_eq!(request.get_param_index(0), Some(&json!("test")));
        assert_eq!(request.get_param_index(1), Some(&json!(42)));
        assert_eq!(request.get_param_index(2), Some(&json!(true)));
        assert_eq!(request.get_param_index(3), None);
    }

    #[test]
    fn test_params_to_map() {
        let object_params = RequestParams::Object({
            let mut map = HashMap::new();
            map.insert("key".to_string(), json!("value"));
            map
        });

        let array_params = RequestParams::Array(vec![json!("first"), json!("second")]);

        let object_map = object_params.to_map();
        assert_eq!(object_map.get("key"), Some(&json!("value")));

        let array_map = array_params.to_map();
        assert_eq!(array_map.get("0"), Some(&json!("first")));
        assert_eq!(array_map.get("1"), Some(&json!("second")));
    }
}
