//! MCP Logging Protocol Types
//!
//! This module defines types for logging in MCP.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Logging levels (per MCP spec)
/// Maps to syslog message severities as specified in RFC-5424
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LoggingLevel {
    Debug,
    Info,
    Notice,
    Warning,
    Error,
    Critical,
    Alert,
    Emergency,
}

/// Type alias for compatibility (per MCP spec)
pub type LogLevel = LoggingLevel;

/// Parameters for notifications/message logging (per MCP spec)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoggingMessageParams {
    /// Log level
    pub level: LoggingLevel,
    /// Optional logger name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logger: Option<String>,
    /// Log data (any serializable type)
    pub data: Value,
    /// Meta information (optional _meta field inside params)
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<HashMap<String, Value>>,
}

/// Complete logging message notification (per MCP spec)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoggingMessageNotification {
    /// Method name (always "notifications/message")
    pub method: String,
    /// Notification parameters
    pub params: LoggingMessageParams,
}

impl LoggingMessageParams {
    pub fn new(level: LoggingLevel, data: Value) -> Self {
        Self {
            level,
            logger: None,
            data,
            meta: None,
        }
    }

    pub fn with_logger(mut self, logger: impl Into<String>) -> Self {
        self.logger = Some(logger.into());
        self
    }

    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.meta = Some(meta);
        self
    }
}

impl LoggingMessageNotification {
    pub fn new(level: LoggingLevel, data: Value) -> Self {
        Self {
            method: "notifications/message".to_string(),
            params: LoggingMessageParams::new(level, data),
        }
    }

    pub fn with_logger(mut self, logger: impl Into<String>) -> Self {
        self.params = self.params.with_logger(logger);
        self
    }

    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.params = self.params.with_meta(meta);
        self
    }
}

/// Parameters for logging/setLevel request (per MCP spec)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetLevelParams {
    /// The log level to set
    pub level: LoggingLevel,
    /// Meta information (optional _meta field inside params)
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<HashMap<String, Value>>,
}

impl SetLevelParams {
    pub fn new(level: LoggingLevel) -> Self {
        Self { level, meta: None }
    }

    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
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
    pub fn new(level: LoggingLevel) -> Self {
        Self {
            method: "logging/setLevel".to_string(),
            params: SetLevelParams::new(level),
        }
    }

    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.params = self.params.with_meta(meta);
        self
    }
}

/// Convenience constructors for LoggingLevel
impl LoggingLevel {
    /// Get logging level priority (0 = debug, 7 = emergency)
    pub fn priority(&self) -> u8 {
        match self {
            LoggingLevel::Debug => 0,
            LoggingLevel::Info => 1,
            LoggingLevel::Notice => 2,
            LoggingLevel::Warning => 3,
            LoggingLevel::Error => 4,
            LoggingLevel::Critical => 5,
            LoggingLevel::Alert => 6,
            LoggingLevel::Emergency => 7,
        }
    }

    /// Check if this level should be logged at the given threshold
    pub fn should_log(&self, threshold: LoggingLevel) -> bool {
        self.priority() >= threshold.priority()
    }
}

// Trait implementations for protocol compliance
use crate::traits::*;

// Params trait implementations
impl Params for SetLevelParams {}
impl Params for LoggingMessageParams {}

// SetLevelParams specific traits
impl HasLevelParam for SetLevelParams {
    fn level(&self) -> &LoggingLevel {
        &self.level
    }
}

impl HasMetaParam for SetLevelParams {
    fn meta(&self) -> Option<&HashMap<String, Value>> {
        self.meta.as_ref()
    }
}

// SetLevelRequest traits
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

// LoggingMessageParams specific traits
impl HasLevelParam for LoggingMessageParams {
    fn level(&self) -> &LoggingLevel {
        &self.level
    }
}

impl HasLoggerParam for LoggingMessageParams {
    fn logger(&self) -> Option<&String> {
        self.logger.as_ref()
    }
}

impl HasMetaParam for LoggingMessageParams {
    fn meta(&self) -> Option<&HashMap<String, Value>> {
        self.meta.as_ref()
    }
}

// LoggingMessageNotification traits
impl HasMethod for LoggingMessageNotification {
    fn method(&self) -> &str {
        &self.method
    }
}

impl HasParams for LoggingMessageNotification {
    fn params(&self) -> Option<&dyn Params> {
        Some(&self.params)
    }
}

// ===========================================
// === Fine-Grained Logging Traits ===
// ===========================================

/// Trait for logging metadata (method, logger name)
pub trait HasLoggingMetadata {
    /// The logging method name
    fn method(&self) -> &str;

    /// Optional logger name/identifier
    fn logger_name(&self) -> Option<&str> {
        None
    }
}

/// Trait for logging level configuration
pub trait HasLogLevel {
    /// The current or target logging level
    fn level(&self) -> LoggingLevel;

    /// Check if a message at the given level should be logged
    fn should_log(&self, message_level: LoggingLevel) -> bool {
        message_level.should_log(self.level())
    }
}

/// Trait for log message formatting and data
pub trait HasLogFormat {
    /// Get the log data
    fn data(&self) -> &Value;

    /// Format the log message for output
    fn format_message(&self) -> String {
        // Default: try to format as string, fallback to JSON
        match self.data() {
            Value::String(s) => s.clone(),
            other => {
                serde_json::to_string(other).unwrap_or_else(|_| "<invalid log data>".to_string())
            }
        }
    }
}

/// Trait for logging transport and delivery
pub trait HasLogTransport {
    /// Optional filtering criteria
    fn should_deliver(&self, _level: LoggingLevel) -> bool {
        true
    }

    /// Optional batching configuration
    fn batch_size(&self) -> Option<usize> {
        None
    }
}

/// Composed logging definition trait (automatically implemented via blanket impl)
pub trait LoggerDefinition:
    HasLoggingMetadata + HasLogLevel + HasLogFormat + HasLogTransport
{
    /// Convert this logger definition to a LoggingMessageNotification
    fn to_message_notification(&self) -> LoggingMessageNotification {
        let mut notification = LoggingMessageNotification::new(self.level(), self.data().clone());
        if let Some(logger) = self.logger_name() {
            notification = notification.with_logger(logger);
        }
        notification
    }

    /// Convert this logger definition to a SetLevelRequest
    fn to_set_level_request(&self) -> SetLevelRequest {
        SetLevelRequest::new(self.level())
    }
}

// Blanket implementation: any type implementing the fine-grained traits automatically gets LoggerDefinition
impl<T> LoggerDefinition for T where
    T: HasLoggingMetadata + HasLogLevel + HasLogFormat + HasLogTransport
{
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_logging_level_priority() {
        assert_eq!(LoggingLevel::Debug.priority(), 0);
        assert_eq!(LoggingLevel::Emergency.priority(), 7);

        assert!(LoggingLevel::Error.should_log(LoggingLevel::Warning));
        assert!(!LoggingLevel::Info.should_log(LoggingLevel::Error));
    }

    #[test]
    fn test_set_level_request() {
        let request = SetLevelRequest::new(LoggingLevel::Warning);

        assert_eq!(request.method, "logging/setLevel");
        assert_eq!(request.params.level, LoggingLevel::Warning);
    }

    #[test]
    fn test_logging_message_notification() {
        let data = json!({"message": "Test log message", "context": "test"});
        let notification = LoggingMessageNotification::new(LoggingLevel::Info, data.clone())
            .with_logger("test-logger");

        assert_eq!(notification.method, "notifications/message");
        assert_eq!(notification.params.level, LoggingLevel::Info);
        assert_eq!(notification.params.logger, Some("test-logger".to_string()));
        assert_eq!(notification.params.data, data);
    }

    #[test]
    fn test_serialization() {
        let request = SetLevelRequest::new(LoggingLevel::Error);
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("logging/setLevel"));
        assert!(json.contains("error"));

        let parsed: SetLevelRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.method, "logging/setLevel");
        assert_eq!(parsed.params.level, LoggingLevel::Error);
    }
}
