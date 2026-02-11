//! Framework traits for MCP logging construction
//!
//! **IMPORTANT**: These are framework features, NOT part of the MCP specification.

use serde_json::Value;
use turul_mcp_protocol::logging::{LoggingLevel, LoggingMessageNotification, SetLevelRequest};

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

/// Complete MCP Logger Definition trait
///
/// This trait represents a complete, working MCP logger.
/// When you implement the required traits, you automatically get
/// `LoggerDefinition` for free via blanket implementation.
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
