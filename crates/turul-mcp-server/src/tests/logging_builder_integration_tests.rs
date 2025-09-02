//! LoggingBuilder Integration Tests
//!
//! This module tests the integration between turul-mcp-builders LoggingBuilder
//! and the session-aware logging functionality in turul-mcp-server.

use std::sync::Arc;
use serde_json::json;
use crate::session::SessionManager;
use turul_mcp_protocol::{ServerCapabilities, logging::{LoggingLevel, HasLogLevel, HasLogFormat, HasLoggingMetadata}};
use turul_mcp_builders::logging::LoggingBuilder;

/// Test LoggingTarget trait implementation for SessionContext
#[cfg(test)]
mod logging_target_tests {
    use super::*;

    #[tokio::test]
    async fn test_session_context_implements_logging_target() {
        let capabilities = ServerCapabilities::default();
        let manager = Arc::new(SessionManager::new(capabilities));
        
        let session_id = manager.create_session().await;
        let context = manager.create_session_context(&session_id).unwrap();
        
        // Set session to Warning level
        context.set_logging_level(LoggingLevel::Warning);
        
        // Test should_log method
        assert!(context.should_log(LoggingLevel::Warning));
        assert!(context.should_log(LoggingLevel::Error));
        assert!(!context.should_log(LoggingLevel::Info));
        assert!(!context.should_log(LoggingLevel::Debug));
        
        // Test notify_log method (should not panic)
        context.notify_log(
            turul_mcp_protocol::logging::LoggingLevel::Error, 
            serde_json::json!("Test message"),
            Some("test".to_string()),
            None
        );
    }
}

/// Test SessionAwareLogger functionality
#[cfg(test)]
mod session_aware_logger_tests {
    use super::*;

    #[tokio::test]
    async fn test_session_aware_logger_creation() {
        let logger = LoggingBuilder::error(json!("Test error message"))
            .logger("test-logger")
            .build_session_aware();
        
        assert_eq!(logger.level_to_string(), "error");
        assert_eq!(logger.format_message(), "Test error message");
    }

    #[tokio::test]
    async fn test_session_aware_logger_level_filtering() {
        let capabilities = ServerCapabilities::default();
        let manager = Arc::new(SessionManager::new(capabilities));
        
        // Create two sessions with different logging levels
        let debug_session = manager.create_session().await;
        let error_session = manager.create_session().await;
        
        let debug_context = manager.create_session_context(&debug_session).unwrap();
        let error_context = manager.create_session_context(&error_session).unwrap();
        
        debug_context.set_logging_level(LoggingLevel::Debug);
        error_context.set_logging_level(LoggingLevel::Error);
        
        // Create loggers at different levels
        let info_logger = LoggingBuilder::info(json!("Info message")).build_session_aware();
        let error_logger = LoggingBuilder::error(json!("Error message")).build_session_aware();
        
        // Test would_send_to_target
        assert!(info_logger.would_send_to_target(&debug_context)); // Debug allows Info
        assert!(!info_logger.would_send_to_target(&error_context)); // Error blocks Info
        
        assert!(error_logger.would_send_to_target(&debug_context)); // Debug allows Error
        assert!(error_logger.would_send_to_target(&error_context)); // Error allows Error
        
        // Test actual sending (should not panic)
        info_logger.send_to_target(&debug_context);
        info_logger.send_to_target(&error_context); // Should be filtered out
        
        error_logger.send_to_target(&debug_context);
        error_logger.send_to_target(&error_context);
    }

    #[tokio::test]
    async fn test_session_aware_logger_multiple_targets() {
        let capabilities = ServerCapabilities::default();
        let manager = Arc::new(SessionManager::new(capabilities));
        
        // Create multiple sessions with different levels
        let debug_session = manager.create_session().await;
        let warning_session = manager.create_session().await;
        let error_session = manager.create_session().await;
        
        let debug_context = manager.create_session_context(&debug_session).unwrap();
        let warning_context = manager.create_session_context(&warning_session).unwrap();
        let error_context = manager.create_session_context(&error_session).unwrap();
        
        debug_context.set_logging_level(LoggingLevel::Debug);
        warning_context.set_logging_level(LoggingLevel::Warning);
        error_context.set_logging_level(LoggingLevel::Error);
        
        // Create an info level logger
        let info_logger = LoggingBuilder::info(json!("Broadcast info message"))
            .logger("broadcast")
            .build_session_aware();
        
        // Send to all sessions
        let targets = vec![&debug_context, &warning_context, &error_context];
        info_logger.send_to_targets(&targets);
        
        // Only debug_context should receive the message (others filter it out)
        assert!(info_logger.would_send_to_target(&debug_context));
        assert!(!info_logger.would_send_to_target(&warning_context));
        assert!(!info_logger.would_send_to_target(&error_context));
    }

    #[tokio::test]
    async fn test_all_logging_levels_with_session_aware_logger() {
        let capabilities = ServerCapabilities::default();
        let manager = Arc::new(SessionManager::new(capabilities));
        
        let session_id = manager.create_session().await;
        let context = manager.create_session_context(&session_id).unwrap();
        
        // Set session to Debug level (allows all)
        context.set_logging_level(LoggingLevel::Debug);
        
        // Test all logging levels
        let test_cases = vec![
            (LoggingBuilder::debug(json!("Debug message")), "debug"),
            (LoggingBuilder::info(json!("Info message")), "info"),
            (LoggingBuilder::notice(json!("Notice message")), "notice"),
            (LoggingBuilder::warning(json!("Warning message")), "warning"),
            (LoggingBuilder::error(json!("Error message")), "error"),
            (LoggingBuilder::critical(json!("Critical message")), "critical"),
            (LoggingBuilder::alert(json!("Alert message")), "alert"),
            (LoggingBuilder::emergency(json!("Emergency message")), "emergency"),
        ];
        
        for (builder, expected_level) in test_cases {
            let logger = builder.build_session_aware();
            assert_eq!(logger.level_to_string(), expected_level);
            assert!(logger.would_send_to_target(&context));
            
            // Should not panic
            logger.send_to_target(&context);
        }
    }

    #[tokio::test]
    async fn test_complex_message_formatting() {
        let capabilities = ServerCapabilities::default();
        let manager = Arc::new(SessionManager::new(capabilities));
        
        let session_id = manager.create_session().await;
        let context = manager.create_session_context(&session_id).unwrap();
        
        // Test different data types
        let test_cases = vec![
            (json!("Simple string"), "Simple string"),
            (json!({"error": "Database connection failed", "code": 500}), "{\"code\":500,\"error\":\"Database connection failed\"}"),
            (json!([1, 2, 3, "test"]), "[1,2,3,\"test\"]"),
            (json!(42), "42"),
            (json!(true), "true"),
            (json!(null), "null"),
        ];
        
        for (data, expected_message) in test_cases {
            let logger = LoggingBuilder::info(data)
                .logger("formatter-test")
                .build_session_aware();
            
            assert_eq!(logger.format_message(), expected_message);
            
            // Should send successfully
            logger.send_to_target(&context);
        }
    }
}

/// Test integration with existing LoggingBuilder functionality
#[cfg(test)]
mod builder_integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_builder_chain_to_session_aware() {
        let capabilities = ServerCapabilities::default();
        let manager = Arc::new(SessionManager::new(capabilities));
        
        let session_id = manager.create_session().await;
        let context = manager.create_session_context(&session_id).unwrap();
        
        // Build a complex logger using builder pattern
        let logger = LoggingBuilder::error(json!({
            "error": "Critical system failure",
            "component": "database",
            "timestamp": "2024-01-01T00:00:00Z",
            "details": {
                "connection_pool": "exhausted",
                "retry_attempts": 3,
                "last_error": "timeout"
            }
        }))
        .logger("system-monitor")
        .meta_value("session_id", json!("test-session"))
        .meta_value("request_id", json!("req-12345"))
        .batch_size(10)
        .build_session_aware();
        
        // Test properties
        assert_eq!(logger.level_to_string(), "error");
        assert!(logger.format_message().contains("Critical system failure"));
        
        // Test session integration
        context.set_logging_level(LoggingLevel::Warning);
        assert!(logger.would_send_to_target(&context)); // Error passes Warning threshold
        
        logger.send_to_target(&context);
    }

    #[tokio::test]
    async fn test_build_both_regular_and_session_aware() {
        let capabilities = ServerCapabilities::default();
        let manager = Arc::new(SessionManager::new(capabilities));
        
        let session_id = manager.create_session().await;
        let context = manager.create_session_context(&session_id).unwrap();
        
        // Create separate builders (LoggingBuilder doesn't implement Clone)
        let regular_logger = LoggingBuilder::warning(json!("Test message"))
            .logger("dual-test")
            .build_dynamic();
        
        let session_logger = LoggingBuilder::warning(json!("Test message"))
            .logger("dual-test")
            .build_session_aware();
        
        // Both should have same level and message
        assert_eq!(regular_logger.level(), session_logger.level());
        assert_eq!(regular_logger.data(), session_logger.data());
        assert_eq!(regular_logger.logger_name(), Some("dual-test"));
        assert_eq!(session_logger.level_to_string(), "warning");
        
        // Session-aware logger can send to sessions
        session_logger.send_to_target(&context);
    }
}