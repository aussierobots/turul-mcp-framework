use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;
use thiserror::Error;

use crate::types::RequestId;

/// JSON-RPC error codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonRpcErrorCode {
    ParseError,
    InvalidRequest,
    MethodNotFound,
    InvalidParams,
    InternalError,
    ServerError(i64), // -32099 to -32000
}

impl JsonRpcErrorCode {
    pub fn code(&self) -> i64 {
        match self {
            JsonRpcErrorCode::ParseError => -32700,
            JsonRpcErrorCode::InvalidRequest => -32600,
            JsonRpcErrorCode::MethodNotFound => -32601,
            JsonRpcErrorCode::InvalidParams => -32602,
            JsonRpcErrorCode::InternalError => -32603,
            JsonRpcErrorCode::ServerError(code) => *code,
        }
    }

    pub fn message(&self) -> &'static str {
        match self {
            JsonRpcErrorCode::ParseError => "Parse error",
            JsonRpcErrorCode::InvalidRequest => "Invalid Request",
            JsonRpcErrorCode::MethodNotFound => "Method not found",
            JsonRpcErrorCode::InvalidParams => "Invalid params",
            JsonRpcErrorCode::InternalError => "Internal error",
            JsonRpcErrorCode::ServerError(_) => "Server error",
        }
    }
}

impl fmt::Display for JsonRpcErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code(), self.message())
    }
}

/// JSON-RPC Error object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcErrorObject {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcErrorObject {
    pub fn new(code: JsonRpcErrorCode, message: Option<String>, data: Option<Value>) -> Self {
        Self {
            code: code.code(),
            message: message.unwrap_or_else(|| code.message().to_string()),
            data,
        }
    }

    pub fn parse_error(data: Option<Value>) -> Self {
        Self::new(JsonRpcErrorCode::ParseError, None, data)
    }

    pub fn invalid_request(data: Option<Value>) -> Self {
        Self::new(JsonRpcErrorCode::InvalidRequest, None, data)
    }

    pub fn method_not_found(method: &str) -> Self {
        Self::new(
            JsonRpcErrorCode::MethodNotFound,
            Some(format!("Method '{}' not found", method)),
            None,
        )
    }

    pub fn invalid_params(message: &str) -> Self {
        Self::new(
            JsonRpcErrorCode::InvalidParams,
            Some(message.to_string()),
            None,
        )
    }

    pub fn internal_error(message: Option<String>) -> Self {
        Self::new(JsonRpcErrorCode::InternalError, message, None)
    }

    pub fn server_error(code: i64, message: &str, data: Option<Value>) -> Self {
        assert!(
            (-32099..=-32000).contains(&code),
            "Server error code must be in range -32099 to -32000"
        );
        Self::new(
            JsonRpcErrorCode::ServerError(code),
            Some(message.to_string()),
            data,
        )
    }
}

/// JSON-RPC Error response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    #[serde(rename = "jsonrpc")]
    pub version: String,
    pub id: Option<RequestId>,
    pub error: JsonRpcErrorObject,
}

impl JsonRpcError {
    pub fn new(id: Option<RequestId>, error: JsonRpcErrorObject) -> Self {
        Self {
            version: crate::JSONRPC_VERSION.to_string(),
            id,
            error,
        }
    }

    pub fn parse_error() -> Self {
        Self::new(None, JsonRpcErrorObject::parse_error(None))
    }

    pub fn invalid_request(id: Option<RequestId>) -> Self {
        Self::new(id, JsonRpcErrorObject::invalid_request(None))
    }

    pub fn method_not_found(id: RequestId, method: &str) -> Self {
        Self::new(Some(id), JsonRpcErrorObject::method_not_found(method))
    }

    pub fn invalid_params(id: RequestId, message: &str) -> Self {
        Self::new(Some(id), JsonRpcErrorObject::invalid_params(message))
    }

    pub fn internal_error(id: Option<RequestId>, message: Option<String>) -> Self {
        Self::new(id, JsonRpcErrorObject::internal_error(message))
    }
}

impl fmt::Display for JsonRpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "JSON-RPC Error {}: {}",
            self.error.code, self.error.message
        )
    }
}

impl std::error::Error for JsonRpcError {}

/// Transport-level errors for JSON-RPC processing (no domain logic)
#[derive(Debug, Error)]
pub enum JsonRpcTransportError {
    #[error("JSON parse error: {0}")]
    JsonParseError(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Protocol error: {0}")]
    ProtocolError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_codes() {
        assert_eq!(JsonRpcErrorCode::ParseError.code(), -32700);
        assert_eq!(JsonRpcErrorCode::MethodNotFound.code(), -32601);
    }

    #[test]
    fn test_error_serialization() {
        let error = JsonRpcError::method_not_found(RequestId::Number(1), "test");
        let json = serde_json::to_string(&error).unwrap();
        assert!(json.contains("Method 'test' not found"));
    }
}
