//! Logging Builder for Runtime Logging Configuration
//!
//! This module provides builder patterns for creating logging notifications and requests
//! at runtime. Supports all MCP logging levels and message formatting.

use serde_json::Value;
use std::collections::HashMap;

// Import protocol types
use turul_mcp_protocol::logging::{LoggingLevel, LoggingMessageNotification, SetLevelRequest};

// Import framework traits from local crate
use crate::traits::{HasLogFormat, HasLogLevel, HasLogTransport, HasLoggingMetadata};

// Re-export the trait for convenience (defined below)
// pub use LoggingTarget;

/// Builder for creating logging messages at runtime
pub struct LoggingBuilder {
    level: LoggingLevel,
    data: Value,
    logger: Option<String>,
    meta: Option<HashMap<String, Value>>,
    // Filtering and transport settings
    batch_size: Option<usize>,
}

impl LoggingBuilder {
    /// Create a new logging builder with the given level and data
    pub fn new(level: LoggingLevel, data: Value) -> Self {
        Self {
            level,
            data,
            logger: None,
            meta: None,
            batch_size: None,
        }
    }

    /// Set the logger name/identifier
    pub fn logger(mut self, logger: impl Into<String>) -> Self {
        self.logger = Some(logger.into());
        self
    }

    /// Set meta information
    pub fn meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.meta = Some(meta);
        self
    }

    /// Add a meta key-value pair
    pub fn meta_value(mut self, key: impl Into<String>, value: Value) -> Self {
        if self.meta.is_none() {
            self.meta = Some(HashMap::new());
        }
        self.meta.as_mut().unwrap().insert(key.into(), value);
        self
    }

    /// Set batch size for log messages
    pub fn batch_size(mut self, size: usize) -> Self {
        self.batch_size = Some(size);
        self
    }

    /// Build the logging message notification
    pub fn build(self) -> LoggingMessageNotification {
        let mut notification = LoggingMessageNotification::new(self.level, self.data);
        if let Some(logger) = self.logger {
            notification = notification.with_logger(logger);
        }
        if let Some(meta) = self.meta {
            notification = notification.with_meta(meta);
        }
        notification
    }

    /// Build a dynamic logger that implements the definition traits
    pub fn build_dynamic(self) -> DynamicLogger {
        DynamicLogger {
            level: self.level,
            data: self.data,
            logger: self.logger,
            meta: self.meta,
            batch_size: self.batch_size,
        }
    }

    /// Create session-aware logger that can send messages directly to a session
    pub fn build_session_aware(self) -> SessionAwareLogger {
        SessionAwareLogger {
            level: self.level,
            data: self.data,
            logger: self.logger,
            meta: self.meta,
            batch_size: self.batch_size,
        }
    }
}

/// Dynamic logger created by LoggingBuilder
#[derive(Debug)]
pub struct DynamicLogger {
    level: LoggingLevel,
    data: Value,
    logger: Option<String>,
    #[allow(dead_code)]
    meta: Option<HashMap<String, Value>>,
    batch_size: Option<usize>,
}

// Implement all fine-grained traits for DynamicLogger
impl HasLoggingMetadata for DynamicLogger {
    fn method(&self) -> &str {
        "notifications/message"
    }

    fn logger_name(&self) -> Option<&str> {
        self.logger.as_deref()
    }
}

impl HasLogLevel for DynamicLogger {
    fn level(&self) -> LoggingLevel {
        self.level
    }
}

impl HasLogFormat for DynamicLogger {
    fn data(&self) -> &Value {
        &self.data
    }
}

impl HasLogTransport for DynamicLogger {
    fn batch_size(&self) -> Option<usize> {
        self.batch_size
    }

    fn should_deliver(&self, threshold_level: LoggingLevel) -> bool {
        self.level.should_log(threshold_level)
    }
}

// LoggerDefinition is automatically implemented via blanket impl!

/// Session-aware logger that can send messages to sessions with filtering
///
/// This logger is designed to work with session contexts that implement
/// logging level checking and message sending capabilities.
#[derive(Debug)]
pub struct SessionAwareLogger {
    level: LoggingLevel,
    data: Value,
    logger: Option<String>,
    #[allow(dead_code)]
    meta: Option<HashMap<String, Value>>,
    batch_size: Option<usize>,
}

/// Trait for logging targets that can check levels and send messages
pub trait LoggingTarget {
    /// Check if this target should receive a message at the given level
    fn should_log(&self, level: LoggingLevel) -> bool;

    /// Send a log message to this target
    fn notify_log(
        &self,
        level: LoggingLevel,
        data: serde_json::Value,
        logger: Option<String>,
        meta: Option<std::collections::HashMap<String, serde_json::Value>>,
    );
}

impl SessionAwareLogger {
    /// Send this log message to the specified target if it passes the target's logging level filter
    pub fn send_to_target<T: LoggingTarget>(&self, target: &T) {
        if target.should_log(self.level) {
            let message = self.format_message();
            target.notify_log(
                self.level,
                serde_json::json!(message),
                self.logger.clone(),
                self.meta.clone(),
            );
        }
    }

    /// Send this log message to multiple targets with per-target filtering
    pub fn send_to_targets<T: LoggingTarget>(&self, targets: &[&T]) {
        for &target in targets {
            self.send_to_target(target);
        }
    }

    /// Check if this message would be sent to the given target
    pub fn would_send_to_target<T: LoggingTarget>(&self, target: &T) -> bool {
        target.should_log(self.level)
    }

    /// Get the formatted message that would be sent
    pub fn format_message(&self) -> String {
        match &self.data {
            Value::String(s) => s.clone(),
            other => {
                serde_json::to_string(other).unwrap_or_else(|_| "<invalid log data>".to_string())
            }
        }
    }

    /// Convert logging level to string representation
    pub fn level_to_string(&self) -> &'static str {
        match self.level {
            LoggingLevel::Debug => "debug",
            LoggingLevel::Info => "info",
            LoggingLevel::Notice => "notice",
            LoggingLevel::Warning => "warning",
            LoggingLevel::Error => "error",
            LoggingLevel::Critical => "critical",
            LoggingLevel::Alert => "alert",
            LoggingLevel::Emergency => "emergency",
        }
    }
}

// Implement all fine-grained traits for SessionAwareLogger (same as DynamicLogger)
impl HasLoggingMetadata for SessionAwareLogger {
    fn method(&self) -> &str {
        "notifications/message"
    }

    fn logger_name(&self) -> Option<&str> {
        self.logger.as_deref()
    }
}

impl HasLogLevel for SessionAwareLogger {
    fn level(&self) -> LoggingLevel {
        self.level
    }
}

impl HasLogFormat for SessionAwareLogger {
    fn data(&self) -> &Value {
        &self.data
    }
}

impl HasLogTransport for SessionAwareLogger {
    fn batch_size(&self) -> Option<usize> {
        self.batch_size
    }

    fn should_deliver(&self, threshold_level: LoggingLevel) -> bool {
        self.level.should_log(threshold_level)
    }
}

// LoggerDefinition is automatically implemented via blanket impl for SessionAwareLogger too!

/// Builder for set level requests
pub struct SetLevelBuilder {
    level: LoggingLevel,
    meta: Option<HashMap<String, Value>>,
}

impl SetLevelBuilder {
    pub fn new(level: LoggingLevel) -> Self {
        Self { level, meta: None }
    }

    /// Set meta information
    pub fn meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.meta = Some(meta);
        self
    }

    /// Add a meta key-value pair
    pub fn meta_value(mut self, key: impl Into<String>, value: Value) -> Self {
        if self.meta.is_none() {
            self.meta = Some(HashMap::new());
        }
        self.meta.as_mut().unwrap().insert(key.into(), value);
        self
    }

    /// Build the set level request
    pub fn build(self) -> SetLevelRequest {
        let mut request = SetLevelRequest::new(self.level);
        if let Some(meta) = self.meta {
            request = request.with_meta(meta);
        }
        request
    }
}

/// Convenience methods for different log levels
impl LoggingBuilder {
    /// Create a debug level logging builder
    pub fn debug(data: Value) -> Self {
        Self::new(LoggingLevel::Debug, data)
    }

    /// Create an info level logging builder
    pub fn info(data: Value) -> Self {
        Self::new(LoggingLevel::Info, data)
    }

    /// Create a notice level logging builder
    pub fn notice(data: Value) -> Self {
        Self::new(LoggingLevel::Notice, data)
    }

    /// Create a warning level logging builder
    pub fn warning(data: Value) -> Self {
        Self::new(LoggingLevel::Warning, data)
    }

    /// Create an error level logging builder
    pub fn error(data: Value) -> Self {
        Self::new(LoggingLevel::Error, data)
    }

    /// Create a critical level logging builder
    pub fn critical(data: Value) -> Self {
        Self::new(LoggingLevel::Critical, data)
    }

    /// Create an alert level logging builder
    pub fn alert(data: Value) -> Self {
        Self::new(LoggingLevel::Alert, data)
    }

    /// Create an emergency level logging builder
    pub fn emergency(data: Value) -> Self {
        Self::new(LoggingLevel::Emergency, data)
    }

    /// Create a simple text log message
    pub fn text(level: LoggingLevel, message: impl Into<String>) -> Self {
        Self::new(level, Value::String(message.into()))
    }

    /// Create a structured log message with fields
    pub fn structured(level: LoggingLevel, fields: HashMap<String, Value>) -> Self {
        Self::new(
            level,
            serde_json::to_value(fields).unwrap_or(Value::Object(serde_json::Map::new())),
        )
    }

    /// Create a log message with message and context
    pub fn with_context(
        level: LoggingLevel,
        message: impl Into<String>,
        context: HashMap<String, Value>,
    ) -> Self {
        let mut data = context;
        data.insert("message".to_string(), Value::String(message.into()));
        Self::structured(level, data)
    }

    /// Create a set level request builder
    pub fn set_level(level: LoggingLevel) -> SetLevelBuilder {
        SetLevelBuilder::new(level)
    }
}

/// Logger level utility functions
pub struct LogLevel;

impl LogLevel {
    /// Parse a string to LoggingLevel
    pub fn parse(level: &str) -> Result<LoggingLevel, String> {
        match level.to_lowercase().as_str() {
            "debug" => Ok(LoggingLevel::Debug),
            "info" => Ok(LoggingLevel::Info),
            "notice" => Ok(LoggingLevel::Notice),
            "warning" | "warn" => Ok(LoggingLevel::Warning),
            "error" => Ok(LoggingLevel::Error),
            "critical" => Ok(LoggingLevel::Critical),
            "alert" => Ok(LoggingLevel::Alert),
            "emergency" => Ok(LoggingLevel::Emergency),
            _ => Err(format!("Invalid log level: {}", level)),
        }
    }

    /// Convert LoggingLevel to string
    pub fn to_string(level: LoggingLevel) -> String {
        match level {
            LoggingLevel::Debug => "debug",
            LoggingLevel::Info => "info",
            LoggingLevel::Notice => "notice",
            LoggingLevel::Warning => "warning",
            LoggingLevel::Error => "error",
            LoggingLevel::Critical => "critical",
            LoggingLevel::Alert => "alert",
            LoggingLevel::Emergency => "emergency",
        }
        .to_string()
    }

    /// Get all available log levels
    pub fn all() -> Vec<LoggingLevel> {
        vec![
            LoggingLevel::Debug,
            LoggingLevel::Info,
            LoggingLevel::Notice,
            LoggingLevel::Warning,
            LoggingLevel::Error,
            LoggingLevel::Critical,
            LoggingLevel::Alert,
            LoggingLevel::Emergency,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use crate::traits::LoggerDefinition;

    #[test]
    fn test_logging_builder_basic() {
        let data = json!({"message": "Test log message"});
        let notification = LoggingBuilder::new(LoggingLevel::Info, data.clone())
            .logger("test-logger")
            .meta_value("request_id", json!("req-123"))
            .build();

        assert_eq!(notification.method, "notifications/message");
        assert_eq!(notification.params.level, LoggingLevel::Info);
        assert_eq!(notification.params.data, data);
        assert_eq!(notification.params.logger, Some("test-logger".to_string()));

        let meta = notification.params.meta.expect("Expected meta");
        assert_eq!(meta.get("request_id"), Some(&json!("req-123")));
    }

    #[test]
    fn test_logging_level_convenience_methods() {
        let debug_log = LoggingBuilder::debug(json!({"debug": "info"})).build();
        assert_eq!(debug_log.params.level, LoggingLevel::Debug);

        let info_log = LoggingBuilder::info(json!({"info": "message"})).build();
        assert_eq!(info_log.params.level, LoggingLevel::Info);

        let warning_log = LoggingBuilder::warning(json!({"warning": "alert"})).build();
        assert_eq!(warning_log.params.level, LoggingLevel::Warning);

        let error_log = LoggingBuilder::error(json!({"error": "critical"})).build();
        assert_eq!(error_log.params.level, LoggingLevel::Error);
    }

    #[test]
    fn test_text_logging() {
        let notification = LoggingBuilder::text(LoggingLevel::Info, "Simple text message")
            .logger("text-logger")
            .build();

        assert_eq!(notification.params.level, LoggingLevel::Info);
        assert_eq!(notification.params.data, json!("Simple text message"));
        assert_eq!(notification.params.logger, Some("text-logger".to_string()));
    }

    #[test]
    fn test_structured_logging() {
        let mut fields = HashMap::new();
        fields.insert("user".to_string(), json!("alice"));
        fields.insert("action".to_string(), json!("login"));
        fields.insert("success".to_string(), json!(true));

        let notification = LoggingBuilder::structured(LoggingLevel::Notice, fields.clone())
            .logger("auth-logger")
            .build();

        assert_eq!(notification.params.level, LoggingLevel::Notice);
        // The data should be the JSON representation of the fields
        let expected_data = serde_json::to_value(fields).unwrap();
        assert_eq!(notification.params.data, expected_data);
    }

    #[test]
    fn test_with_context_logging() {
        let mut context = HashMap::new();
        context.insert("session_id".to_string(), json!("sess-123"));
        context.insert("ip_address".to_string(), json!("192.168.1.1"));

        let notification = LoggingBuilder::with_context(
            LoggingLevel::Info,
            "User logged in successfully",
            context.clone(),
        )
        .build();

        assert_eq!(notification.params.level, LoggingLevel::Info);

        // Verify the data contains the message and context
        if let Value::Object(data_obj) = &notification.params.data {
            assert_eq!(
                data_obj.get("message"),
                Some(&json!("User logged in successfully"))
            );
            assert_eq!(data_obj.get("session_id"), Some(&json!("sess-123")));
            assert_eq!(data_obj.get("ip_address"), Some(&json!("192.168.1.1")));
        } else {
            panic!("Expected object data");
        }
    }

    #[test]
    fn test_set_level_builder() {
        let request = LoggingBuilder::set_level(LoggingLevel::Warning)
            .meta_value("source", json!("admin_panel"))
            .build();

        assert_eq!(request.method, "logging/setLevel");
        assert_eq!(request.params.level, LoggingLevel::Warning);

        let meta = request.params.meta.expect("Expected meta");
        assert_eq!(meta.get("source"), Some(&json!("admin_panel")));
    }

    #[test]
    fn test_dynamic_logger_traits() {
        let logger = LoggingBuilder::info(json!({"message": "Test"}))
            .logger("test-logger")
            .batch_size(10)
            .build_dynamic();

        // Test HasLoggingMetadata
        assert_eq!(logger.method(), "notifications/message");
        assert_eq!(logger.logger_name(), Some("test-logger"));

        // Test HasLogLevel
        assert_eq!(logger.level(), LoggingLevel::Info);
        // With Info threshold (1), should log Error (4) but not Debug (0)
        assert!(!logger.should_log(LoggingLevel::Debug)); // Debug (0) < Info (1), so shouldn't log
        assert!(logger.should_log(LoggingLevel::Error)); // Error (4) >= Info (1), so should log

        // Test HasLogFormat
        assert_eq!(logger.data(), &json!({"message": "Test"}));
        assert_eq!(logger.format_message(), "{\"message\":\"Test\"}");

        // Test HasLogTransport
        assert_eq!(logger.batch_size(), Some(10));
        assert!(logger.should_deliver(LoggingLevel::Debug));

        // Test LoggerDefinition (auto-implemented)
        let message_notification = logger.to_message_notification();
        assert_eq!(message_notification.method, "notifications/message");
        assert_eq!(message_notification.params.level, LoggingLevel::Info);

        let set_level_request = logger.to_set_level_request();
        assert_eq!(set_level_request.method, "logging/setLevel");
        assert_eq!(set_level_request.params.level, LoggingLevel::Info);
    }

    #[test]
    fn test_log_level_utilities() {
        // Test parsing
        assert_eq!(LogLevel::parse("info").unwrap(), LoggingLevel::Info);
        assert_eq!(LogLevel::parse("WARNING").unwrap(), LoggingLevel::Warning);
        assert_eq!(LogLevel::parse("warn").unwrap(), LoggingLevel::Warning);
        assert!(LogLevel::parse("invalid").is_err());

        // Test to_string
        assert_eq!(LogLevel::to_string(LoggingLevel::Debug), "debug");
        assert_eq!(LogLevel::to_string(LoggingLevel::Emergency), "emergency");

        // Test all levels
        let all_levels = LogLevel::all();
        assert_eq!(all_levels.len(), 8);
        assert!(all_levels.contains(&LoggingLevel::Debug));
        assert!(all_levels.contains(&LoggingLevel::Emergency));
    }

    #[test]
    fn test_log_format_with_string_data() {
        let logger =
            LoggingBuilder::text(LoggingLevel::Info, "Simple string message").build_dynamic();

        assert_eq!(logger.format_message(), "Simple string message");
    }

    #[test]
    fn test_log_format_with_object_data() {
        let data = json!({"key": "value", "number": 42});
        let logger = LoggingBuilder::new(LoggingLevel::Info, data).build_dynamic();

        let formatted = logger.format_message();
        assert!(formatted.contains("key"));
        assert!(formatted.contains("value"));
        assert!(formatted.contains("42"));
    }
}
