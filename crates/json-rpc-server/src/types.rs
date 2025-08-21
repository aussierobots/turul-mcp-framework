use std::fmt;
use serde::{Deserialize, Serialize};

/// A uniquely identifying ID for a JSON-RPC request.
/// Can be a string or a number, but never null.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RequestId {
    String(String),
    Number(i64),
}

impl fmt::Display for RequestId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RequestId::String(s) => write!(f, "{}", s),
            RequestId::Number(n) => write!(f, "{}", n),
        }
    }
}

impl From<i64> for RequestId {
    fn from(n: i64) -> Self {
        RequestId::Number(n)
    }
}

impl From<&str> for RequestId {
    fn from(s: &str) -> Self {
        RequestId::String(s.to_string())
    }
}

impl From<String> for RequestId {
    fn from(s: String) -> Self {
        RequestId::String(s)
    }
}

impl RequestId {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            RequestId::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            RequestId::Number(n) => Some(*n),
            _ => None,
        }
    }
}

/// JSON-RPC version
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JsonRpcVersion {
    V2_0,
}

impl JsonRpcVersion {
    pub fn as_str(&self) -> &'static str {
        match self {
            JsonRpcVersion::V2_0 => "2.0",
        }
    }
}

impl Default for JsonRpcVersion {
    fn default() -> Self {
        JsonRpcVersion::V2_0
    }
}

impl fmt::Display for JsonRpcVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl Serialize for JsonRpcVersion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for JsonRpcVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "2.0" => Ok(JsonRpcVersion::V2_0),
            _ => Err(serde::de::Error::custom(format!("Invalid JSON-RPC version: {}", s))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_request_id_serialization() {
        let id_str = RequestId::String("test".to_string());
        let id_num = RequestId::Number(42);
        
        assert_eq!(serde_json::to_string(&id_str).unwrap(), r#""test""#);
        assert_eq!(serde_json::to_string(&id_num).unwrap(), "42");
    }

    #[test]
    fn test_json_rpc_version() {
        let version = JsonRpcVersion::V2_0;
        assert_eq!(version.as_str(), "2.0");
        assert_eq!(serde_json::to_string(&version).unwrap(), r#""2.0""#);
    }
}