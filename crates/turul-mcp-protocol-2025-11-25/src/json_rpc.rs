//! JSON-RPC 2.0 Implementation for MCP 2025-11-25
//!
//! This module provides JSON-RPC structures that are fully compliant with the
//! MCP 2025-11-25 specification, including proper _meta field handling.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::meta::{Meta, ProgressToken};
use crate::traits::{
    HasData, HasDataParam, HasMeta, HasMetaParam, HasProgressTokenParam, Params, RpcResult,
};

/// JSON-RPC version constant
pub const JSONRPC_VERSION: &str = "2.0";

/// JSON-RPC `params` object with optional `_meta` and method-specific arguments
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestParams {
    /// Optional MCP `_meta` section
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "_meta")]
    pub meta: Option<Meta>,

    /// All other method-specific parameters
    #[serde(flatten)]
    pub other: HashMap<String, Value>,
}

impl Params for RequestParams {}

impl HasMeta for RequestParams {
    fn meta(&self) -> Option<HashMap<String, Value>> {
        self.meta.as_ref().map(|m| {
            let mut map = HashMap::new();
            if let Some(ref token) = m.progress_token {
                map.insert(
                    "progressToken".to_string(),
                    Value::String(token.as_str().to_string()),
                );
            }
            if let Some(ref cursor) = m.cursor {
                map.insert(
                    "cursor".to_string(),
                    Value::String(cursor.as_str().to_string()),
                );
            }
            if let Some(total) = m.total {
                map.insert("total".to_string(), Value::Number(total.into()));
            }
            if let Some(has_more) = m.has_more {
                map.insert("hasMore".to_string(), Value::Bool(has_more));
            }
            if let Some(estimated_remaining) = m.estimated_remaining_seconds {
                map.insert(
                    "estimatedRemainingSeconds".to_string(),
                    Value::Number(serde_json::Number::from_f64(estimated_remaining).unwrap()),
                );
            }
            if let Some(progress) = m.progress {
                map.insert(
                    "progress".to_string(),
                    Value::Number(serde_json::Number::from_f64(progress).unwrap()),
                );
            }
            if let Some(current_step) = m.current_step {
                map.insert(
                    "currentStep".to_string(),
                    Value::Number(current_step.into()),
                );
            }
            if let Some(total_steps) = m.total_steps {
                map.insert("totalSteps".to_string(), Value::Number(total_steps.into()));
            }
            map
        })
    }
}

impl HasProgressTokenParam for RequestParams {
    fn progress_token(&self) -> Option<&ProgressToken> {
        self.meta.as_ref()?.progress_token.as_ref()
    }
}

impl HasDataParam for RequestParams {
    fn data(&self) -> &HashMap<String, Value> {
        &self.other
    }
}

impl HasMetaParam for RequestParams {
    fn meta(&self) -> Option<&HashMap<String, Value>> {
        // This is different from HasMeta::meta() - this returns a reference to raw meta
        // For now, we'll just return None since we store structured Meta
        None
    }
}

/// A generic result wrapper that combines data with optional _meta information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultWithMeta {
    /// The result data
    #[serde(flatten)]
    pub data: HashMap<String, Value>,

    /// Optional _meta information
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<HashMap<String, Value>>,
}

impl ResultWithMeta {
    pub fn new(data: HashMap<String, Value>) -> Self {
        Self { data, meta: None }
    }

    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.meta = Some(meta);
        self
    }

    pub fn from_value(value: Value) -> Self {
        match value {
            Value::Object(map) => Self {
                data: map.into_iter().collect(),
                meta: None,
            },
            _ => Self {
                data: HashMap::new(),
                meta: None,
            },
        }
    }
}

impl HasData for ResultWithMeta {
    fn data(&self) -> HashMap<String, Value> {
        self.data.clone()
    }
}

impl HasMeta for ResultWithMeta {
    fn meta(&self) -> Option<HashMap<String, Value>> {
        self.meta.clone()
    }
}

impl RpcResult for ResultWithMeta {}

/// A standard JSON-RPC 2.0 request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Value,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<RequestParams>,
}

impl JsonRpcRequest {
    pub fn new(id: Value, method: String) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id,
            method,
            params: None,
        }
    }

    pub fn with_params(mut self, params: RequestParams) -> Self {
        self.params = Some(params);
        self
    }
}

/// A standard JSON-RPC 2.0 response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<ResultWithMeta>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

impl JsonRpcResponse {
    pub fn success(id: Value, result: ResultWithMeta) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: Value, error: JsonRpcError) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id,
            result: None,
            error: Some(error),
        }
    }
}

/// JSON-RPC 2.0 error object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcError {
    pub fn new(code: i32, message: String) -> Self {
        Self {
            code,
            message,
            data: None,
        }
    }

    pub fn with_data(mut self, data: Value) -> Self {
        self.data = Some(data);
        self
    }

    // Standard JSON-RPC error codes
    pub fn parse_error() -> Self {
        Self::new(-32700, "Parse error".to_string())
    }

    pub fn invalid_request() -> Self {
        Self::new(-32600, "Invalid Request".to_string())
    }

    pub fn method_not_found() -> Self {
        Self::new(-32601, "Method not found".to_string())
    }

    pub fn invalid_params() -> Self {
        Self::new(-32602, "Invalid params".to_string())
    }

    pub fn internal_error() -> Self {
        Self::new(-32603, "Internal error".to_string())
    }
}

/// A JSON-RPC 2.0 notification (no response expected)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcNotification {
    pub jsonrpc: String,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<RequestParams>,
}

impl JsonRpcNotification {
    pub fn new(method: String) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            method,
            params: None,
        }
    }

    pub fn with_params(mut self, params: RequestParams) -> Self {
        self.params = Some(params);
        self
    }
}

/// Unified JSON-RPC message type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonRpcMessage {
    Request(JsonRpcRequest),
    Response(JsonRpcResponse),
    Notification(JsonRpcNotification),
    Error(JsonRpcError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_request_params_with_meta() {
        let params = RequestParams {
            meta: Some(Meta {
                progress_token: Some(ProgressToken::new("test-token")),
                cursor: Some(crate::meta::Cursor::new("cursor-123")),
                total: Some(100),
                has_more: Some(true),
                ..Default::default()
            }),
            other: {
                let mut map = HashMap::new();
                map.insert("name".to_string(), json!("test"));
                map
            },
        };

        // Test serialization
        let json_str = serde_json::to_string(&params).unwrap();
        assert!(json_str.contains("progressToken"));
        assert!(json_str.contains("test-token"));
        assert!(json_str.contains("cursor"));
        assert!(json_str.contains("cursor-123"));
        assert!(json_str.contains("name"));
        assert!(json_str.contains("test"));

        // Test deserialization
        let parsed: RequestParams = serde_json::from_str(&json_str).unwrap();
        assert!(parsed.meta.is_some());
        assert_eq!(
            parsed
                .meta
                .as_ref()
                .unwrap()
                .progress_token
                .as_ref()
                .unwrap()
                .as_str(),
            "test-token"
        );
    }

    #[test]
    fn test_result_with_meta() {
        let mut data = HashMap::new();
        data.insert("result".to_string(), json!("success"));

        let mut meta = HashMap::new();
        meta.insert("total".to_string(), json!(42));

        let result = ResultWithMeta::new(data).with_meta(meta);

        // Test traits
        assert!(result.data().contains_key("result"));
        assert!(result.meta().unwrap().contains_key("total"));

        // Test serialization
        let json_str = serde_json::to_string(&result).unwrap();
        assert!(json_str.contains("result"));
        assert!(json_str.contains("_meta"));
        assert!(json_str.contains("total"));
    }
}
