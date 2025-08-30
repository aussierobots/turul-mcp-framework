//! MCP Logging Trait
//!
//! This module defines the high-level trait for implementing MCP logging.

use async_trait::async_trait;
use mcp_protocol::{McpResult, logging::{SetLevelRequest, LoggingMessageNotification}};
use mcp_protocol::logging::{LoggerDefinition, LoggingLevel};
use serde_json::Value;

/// High-level trait for implementing MCP logging
/// 
/// McpLogger extends LoggerDefinition with execution capabilities.
/// All metadata is provided by the LoggerDefinition trait, ensuring
/// consistency between concrete Logger structs and dynamic implementations.
#[async_trait]
pub trait McpLogger: LoggerDefinition + Send + Sync {
    /// Log a message (per MCP spec)
    /// 
    /// This method processes log messages and sends them via the appropriate
    /// transport mechanism (notifications/message).
    async fn log(&self, level: LoggingLevel, data: Value) -> McpResult<()>;

    /// Set the logging level (per MCP spec)
    /// 
    /// This method processes logging/setLevel requests to configure
    /// the minimum level for log message delivery.
    async fn set_level(&self, request: SetLevelRequest) -> McpResult<()>;

    /// Optional: Check if this logger can handle the given level
    /// 
    /// This allows for conditional logging based on logger capabilities,
    /// transport availability, or other factors.
    fn can_log(&self, level: LoggingLevel) -> bool {
        self.should_log(level)
    }

    /// Optional: Get logger priority for routing
    /// 
    /// Higher priority loggers are used first when multiple loggers
    /// can handle the same message.
    fn priority(&self) -> u32 {
        0
    }

    /// Optional: Validate the log data
    /// 
    /// This method can perform validation of log data before processing.
    async fn validate_data(&self, _data: &Value) -> McpResult<()> {
        // Basic validation - ensure data is not null
        if _data.is_null() {
            return Err(mcp_protocol::McpError::validation("Log data cannot be null"));
        }
        Ok(())
    }

    /// Optional: Transform log data before sending
    /// 
    /// This allows for data enrichment, filtering, or formatting
    /// before the log message is transmitted.
    async fn transform_data(&self, data: Value) -> McpResult<Value> {
        Ok(data)
    }

    /// Optional: Handle logging errors
    /// 
    /// This method is called when log delivery fails, allowing
    /// for retry logic, fallback logging, or error notifications.
    async fn handle_error(&self, error: &mcp_protocol::McpError) -> McpResult<()> {
        // Default: just propagate the error by creating a new error with the same message
        Err(mcp_protocol::McpError::validation(&format!("Logging error: {}", error)))
    }

    /// Optional: Batch multiple log messages
    /// 
    /// This method can be used to optimize log delivery by batching
    /// multiple messages together.
    async fn flush(&self) -> McpResult<()> {
        // Default: no-op for non-batching loggers
        Ok(())
    }
}

/// Convert an McpLogger trait object to a LoggingMessageNotification
/// 
/// This is a convenience function for converting logger definitions
/// to protocol notifications.
pub fn logger_to_notification(logger: &dyn McpLogger, level: LoggingLevel, data: Value) -> LoggingMessageNotification {
    let mut notification = LoggingMessageNotification::new(level, data);
    if let Some(logger_name) = logger.logger_name() {
        notification = notification.with_logger(logger_name);
    }
    notification
}

#[cfg(test)]
mod tests {
    use super::*;
    use mcp_protocol::logging::{
        HasLoggingMetadata, HasLogLevel, HasLogFormat, HasLogTransport
    };
    use serde_json::json;

    struct TestLogger {
        logger_name: String,
        level: LoggingLevel,
        test_data: Value,
    }

    // Implement fine-grained traits (MCP spec compliant)
    impl HasLoggingMetadata for TestLogger {
        fn method(&self) -> &str {
            "notifications/message"
        }
        
        fn logger_name(&self) -> Option<&str> {
            Some(&self.logger_name)
        }
    }

    impl HasLogLevel for TestLogger {
        fn level(&self) -> LoggingLevel {
            self.level
        }
    }

    impl HasLogFormat for TestLogger {
        fn data(&self) -> &Value {
            &self.test_data
        }
    }

    impl HasLogTransport for TestLogger {
        fn should_deliver(&self, level: LoggingLevel) -> bool {
            level.should_log(self.level)
        }
        
        fn batch_size(&self) -> Option<usize> {
            Some(10) // Test batching
        }
    }

    // LoggerDefinition automatically implemented via blanket impl!

    #[async_trait]
    impl McpLogger for TestLogger {
        async fn log(&self, level: LoggingLevel, _data: Value) -> McpResult<()> {
            // Simulate logging (could send to file, network, etc.)
            if self.can_log(level) {
                println!("[{}] {}: {}", 
                    self.logger_name, 
                    format!("{:?}", level).to_lowercase(),
                    self.format_message()
                );
            }
            Ok(())
        }

        async fn set_level(&self, _request: SetLevelRequest) -> McpResult<()> {
            // Simulate level setting
            Ok(())
        }
    }

    #[test]
    fn test_logger_trait() {
        let logger = TestLogger {
            logger_name: "test-logger".to_string(),
            level: LoggingLevel::Info,
            test_data: json!({"message": "test log"}),
        };
        
        assert_eq!(logger.method(), "notifications/message");
        assert_eq!(logger.logger_name(), Some("test-logger"));
        assert_eq!(logger.level(), LoggingLevel::Info);
        assert_eq!(logger.batch_size(), Some(10));
    }

    #[tokio::test]
    async fn test_logger_validation() {
        let logger = TestLogger {
            logger_name: "test-logger".to_string(),
            level: LoggingLevel::Warning,
            test_data: json!({"message": "valid data"}),
        };

        let result = logger.validate_data(&json!({"test": "data"})).await;
        assert!(result.is_ok());

        let null_result = logger.validate_data(&Value::Null).await;
        assert!(null_result.is_err());
    }

    #[tokio::test]
    async fn test_logging_levels() {
        let logger = TestLogger {
            logger_name: "level-test".to_string(),
            level: LoggingLevel::Warning,
            test_data: json!({"message": "test"}),
        };

        assert!(logger.can_log(LoggingLevel::Error));
        assert!(logger.can_log(LoggingLevel::Warning));
        assert!(!logger.can_log(LoggingLevel::Info));
        assert!(!logger.can_log(LoggingLevel::Debug));
    }

    #[tokio::test]
    async fn test_data_transformation() {
        let logger = TestLogger {
            logger_name: "transform-test".to_string(),
            level: LoggingLevel::Info,
            test_data: json!({"original": "data"}),
        };

        let input_data = json!({"transform": "me"});
        let result = logger.transform_data(input_data.clone()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), input_data);
    }
}