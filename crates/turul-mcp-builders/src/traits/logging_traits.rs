//! Framework traits for MCP logging construction
//!
//! **IMPORTANT**: These are framework features, NOT part of the MCP specification.

// The Logging surface is SEP-2577-deprecated; these traits carry it
// through the migration window.
#![allow(deprecated)]

use serde_json::Value;
use turul_mcp_protocol::logging::LoggingLevel;

#[cfg(feature = "protocol-2025-11-25")]
use turul_mcp_protocol::logging::{LoggingMessageNotification, SetLevelRequest};

/// Supplies logging metadata: method, logger name and `_meta`.
pub trait HasLoggingMetadata {
    /// Returns the logging method name.
    fn method(&self) -> &str;

    /// Returns the optional logger name.
    fn logger_name(&self) -> Option<&str> {
        None
    }

    /// Returns the optional `_meta` sent on the notification.
    fn meta(&self) -> Option<&std::collections::HashMap<String, Value>> {
        None
    }
}

/// Supplies the logging level and level-based filtering.
pub trait HasLogLevel {
    /// Returns the current or target logging level.
    fn level(&self) -> LoggingLevel;

    /// Returns whether a message at `message_level` should be logged at this level.
    fn should_log(&self, message_level: LoggingLevel) -> bool {
        message_level.should_log(self.level())
    }
}

/// Supplies log data and formats it as a message.
pub trait HasLogFormat {
    /// Returns the log data.
    fn data(&self) -> &Value;

    /// Formats the log data as a string, using JSON text for non-string values.
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

/// Controls logging delivery and batching.
pub trait HasLogTransport {
    /// Returns whether a message at `_level` should be delivered.
    fn should_deliver(&self, _level: LoggingLevel) -> bool {
        true
    }

    /// Returns the batch size, or `None` when batching is not configured.
    fn batch_size(&self) -> Option<usize> {
        None
    }
}

/// Defines a complete MCP logger.
///
/// When you implement the required traits, you automatically get
/// `LoggerDefinition` for free via blanket implementation.
#[cfg(feature = "protocol-2025-11-25")]
pub trait LoggerDefinition:
    HasLoggingMetadata + HasLogLevel + HasLogFormat + HasLogTransport
{
    /// Converts this logger definition to a `LoggingMessageNotification`.
    fn to_message_notification(&self) -> LoggingMessageNotification {
        let mut notification = LoggingMessageNotification::new(self.level(), self.data().clone());
        if let Some(logger) = self.logger_name() {
            notification = notification.with_logger(logger);
        }
        if let Some(meta) = self.meta() {
            notification = notification.with_meta(meta.clone());
        }
        notification
    }

    /// Converts this logger definition to a `SetLevelRequest`.
    fn to_set_level_request(&self) -> SetLevelRequest {
        SetLevelRequest::new(self.level())
    }
}

// Blanket implementation: any type implementing the fine-grained traits automatically gets LoggerDefinition
#[cfg(feature = "protocol-2025-11-25")]
impl<T> LoggerDefinition for T where
    T: HasLoggingMetadata + HasLogLevel + HasLogFormat + HasLogTransport
{
}
