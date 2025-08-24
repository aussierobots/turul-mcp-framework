//! MCP Logging Protocol Types
//!
//! This module defines types for logging in MCP.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Log levels
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug,
    Info,
    Notice,
    Warning,
    Error,
    Critical,
    Alert,
    Emergency,
}

/// Log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    /// Log level
    pub level: LogLevel,
    /// Log message
    pub message: String,
    /// Optional additional data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    /// Optional logger name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logger: Option<String>,
}

impl LogEntry {
    pub fn new(level: LogLevel, message: impl Into<String>) -> Self {
        Self {
            level,
            message: message.into(),
            data: None,
            logger: None,
        }
    }

    pub fn with_data(mut self, data: Value) -> Self {
        self.data = Some(data);
        self
    }

    pub fn with_logger(mut self, logger: impl Into<String>) -> Self {
        self.logger = Some(logger.into());
        self
    }
}

/// Parameters for logging/setLevel request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetLevelParams {
    /// The log level to set
    pub level: LogLevel,
    /// Meta information (optional _meta field inside params)
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<std::collections::HashMap<String, serde_json::Value>>,
}

impl SetLevelParams {
    pub fn new(level: LogLevel) -> Self {
        Self { 
            level,
            meta: None,
        }
    }

    pub fn with_meta(mut self, meta: std::collections::HashMap<String, serde_json::Value>) -> Self {
        self.meta = Some(meta);
        self
    }
}

/// Complete logging/setLevel request (matches TypeScript SetLevelRequest interface)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetLevelRequest {
    /// Method name (always "logging/setLevel")
    pub method: String,
    /// Request parameters
    pub params: SetLevelParams,
}

impl SetLevelRequest {
    pub fn new(level: LogLevel) -> Self {
        Self {
            method: "logging/setLevel".to_string(),
            params: SetLevelParams::new(level),
        }
    }

    pub fn with_meta(mut self, meta: std::collections::HashMap<String, serde_json::Value>) -> Self {
        self.params = self.params.with_meta(meta);
        self
    }
}

// Trait implementations for logging

use crate::traits::*;

// Trait implementations for SetLevelParams
impl Params for SetLevelParams {}

impl HasSetLevelParams for SetLevelParams {
    fn level(&self) -> &LogLevel {
        &self.level
    }
}

impl HasMetaParam for SetLevelParams {
    fn meta(&self) -> Option<&std::collections::HashMap<String, serde_json::Value>> {
        self.meta.as_ref()
    }
}

// Trait implementations for SetLevelRequest
impl HasMethod for SetLevelRequest {
    fn method(&self) -> &str {
        &self.method
    }
}

impl HasParams for SetLevelRequest {
    fn params(&self) -> Option<&dyn Params> {
        Some(&self.params)
    }
}