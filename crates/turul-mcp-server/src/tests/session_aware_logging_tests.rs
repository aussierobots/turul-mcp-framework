//! Session-Aware Logging Tests
//!
//! This module tests the session-aware logging functionality including:
//! - Per-session logging level storage and retrieval
//! - Logging level filtering in notify_log method
//! - LoggingHandler integration with session context
//! - Session isolation for logging levels
//! - Default logging level behavior

use std::sync::Arc;
use serde_json::json;
use crate::session::SessionManager;
use crate::handlers::{LoggingHandler, McpHandler};
use turul_mcp_protocol::{ServerCapabilities, logging::LoggingLevel};

/// Test session-aware logging level methods
#[cfg(test)]
mod logging_level_tests {
    use super::*;

    #[tokio::test]
    async fn test_default_logging_level() {
        let capabilities = ServerCapabilities::default();
        let manager = Arc::new(SessionManager::new(capabilities));
        
        let session_id = manager.create_session().await;
        let context = manager.create_session_context(&session_id).unwrap();
        
        // Default logging level should be Info
        let level = context.get_logging_level();
        assert_eq!(level, LoggingLevel::Info);
        
        // Should log messages at Info level or higher
        assert!(context.should_log(LoggingLevel::Info));
        assert!(context.should_log(LoggingLevel::Notice));
        assert!(context.should_log(LoggingLevel::Warning));
        assert!(context.should_log(LoggingLevel::Error));
        assert!(context.should_log(LoggingLevel::Critical));
        assert!(context.should_log(LoggingLevel::Alert));
        assert!(context.should_log(LoggingLevel::Emergency));
        
        // Should NOT log messages below Info level
        assert!(!context.should_log(LoggingLevel::Debug));
    }

    #[tokio::test]
    async fn test_set_and_get_logging_level() {
        let capabilities = ServerCapabilities::default();
        let manager = Arc::new(SessionManager::new(capabilities));
        
        let session_id = manager.create_session().await;
        let context = manager.create_session_context(&session_id).unwrap();
        
        // Test setting each logging level
        let test_levels = vec![
            LoggingLevel::Debug,
            LoggingLevel::Info,
            LoggingLevel::Notice,
            LoggingLevel::Warning,
            LoggingLevel::Error,
            LoggingLevel::Critical,
            LoggingLevel::Alert,
            LoggingLevel::Emergency,
        ];
        
        for level in test_levels {
            context.set_logging_level(level);
            let retrieved_level = context.get_logging_level();
            assert_eq!(retrieved_level, level);
        }
    }

    #[tokio::test]
    async fn test_logging_level_persistence_in_session_state() {
        let capabilities = ServerCapabilities::default();
        let manager = Arc::new(SessionManager::new(capabilities));
        
        let session_id = manager.create_session().await;
        let context = manager.create_session_context(&session_id).unwrap();
        
        // Set logging level
        context.set_logging_level(LoggingLevel::Warning);
        
        // Check that it's stored in session state
        let state_value = (context.get_state)("mcp:logging:level");
        assert_eq!(state_value, Some(json!("warning")));
        
        // Create new context for same session - should retrieve the same level
        let context2 = manager.create_session_context(&session_id).unwrap();
        let retrieved_level = context2.get_logging_level();
        assert_eq!(retrieved_level, LoggingLevel::Warning);
    }

    #[tokio::test]
    async fn test_should_log_filtering() {
        let capabilities = ServerCapabilities::default();
        let manager = Arc::new(SessionManager::new(capabilities));
        
        let session_id = manager.create_session().await;
        let context = manager.create_session_context(&session_id).unwrap();
        
        // Set session to Error level
        context.set_logging_level(LoggingLevel::Error);
        
        // Only Error and higher should pass
        assert!(!context.should_log(LoggingLevel::Debug));
        assert!(!context.should_log(LoggingLevel::Info));
        assert!(!context.should_log(LoggingLevel::Notice));
        assert!(!context.should_log(LoggingLevel::Warning));
        assert!(context.should_log(LoggingLevel::Error));
        assert!(context.should_log(LoggingLevel::Critical));
        assert!(context.should_log(LoggingLevel::Alert));
        assert!(context.should_log(LoggingLevel::Emergency));
    }

    #[tokio::test]
    async fn test_debug_level_allows_all() {
        let capabilities = ServerCapabilities::default();
        let manager = Arc::new(SessionManager::new(capabilities));
        
        let session_id = manager.create_session().await;
        let context = manager.create_session_context(&session_id).unwrap();
        
        // Set session to Debug level (lowest)
        context.set_logging_level(LoggingLevel::Debug);
        
        // All levels should pass
        assert!(context.should_log(LoggingLevel::Debug));
        assert!(context.should_log(LoggingLevel::Info));
        assert!(context.should_log(LoggingLevel::Notice));
        assert!(context.should_log(LoggingLevel::Warning));
        assert!(context.should_log(LoggingLevel::Error));
        assert!(context.should_log(LoggingLevel::Critical));
        assert!(context.should_log(LoggingLevel::Alert));
        assert!(context.should_log(LoggingLevel::Emergency));
    }

    #[tokio::test]
    async fn test_emergency_level_blocks_most() {
        let capabilities = ServerCapabilities::default();
        let manager = Arc::new(SessionManager::new(capabilities));
        
        let session_id = manager.create_session().await;
        let context = manager.create_session_context(&session_id).unwrap();
        
        // Set session to Emergency level (highest)
        context.set_logging_level(LoggingLevel::Emergency);
        
        // Only Emergency should pass
        assert!(!context.should_log(LoggingLevel::Debug));
        assert!(!context.should_log(LoggingLevel::Info));
        assert!(!context.should_log(LoggingLevel::Notice));
        assert!(!context.should_log(LoggingLevel::Warning));
        assert!(!context.should_log(LoggingLevel::Error));
        assert!(!context.should_log(LoggingLevel::Critical));
        assert!(!context.should_log(LoggingLevel::Alert));
        assert!(context.should_log(LoggingLevel::Emergency));
    }
}

/// Test session isolation for logging levels
#[cfg(test)]
mod session_isolation_tests {
    use super::*;

    #[tokio::test]
    async fn test_multiple_sessions_independent_logging_levels() {
        let capabilities = ServerCapabilities::default();
        let manager = Arc::new(SessionManager::new(capabilities));
        
        // Create multiple sessions
        let session1 = manager.create_session().await;
        let session2 = manager.create_session().await;
        let session3 = manager.create_session().await;
        
        let context1 = manager.create_session_context(&session1).unwrap();
        let context2 = manager.create_session_context(&session2).unwrap();
        let context3 = manager.create_session_context(&session3).unwrap();
        
        // Set different logging levels for each session
        context1.set_logging_level(LoggingLevel::Debug);
        context2.set_logging_level(LoggingLevel::Warning);
        context3.set_logging_level(LoggingLevel::Error);
        
        // Verify each session has its own level
        assert_eq!(context1.get_logging_level(), LoggingLevel::Debug);
        assert_eq!(context2.get_logging_level(), LoggingLevel::Warning);
        assert_eq!(context3.get_logging_level(), LoggingLevel::Error);
        
        // Verify filtering behavior is independent
        let test_level = LoggingLevel::Info;
        assert!(context1.should_log(test_level)); // Debug allows Info
        assert!(!context2.should_log(test_level)); // Warning blocks Info
        assert!(!context3.should_log(test_level)); // Error blocks Info
    }

    #[tokio::test]
    async fn test_session_level_changes_dont_affect_others() {
        let capabilities = ServerCapabilities::default();
        let manager = Arc::new(SessionManager::new(capabilities));
        
        let session1 = manager.create_session().await;
        let session2 = manager.create_session().await;
        
        let context1 = manager.create_session_context(&session1).unwrap();
        let context2 = manager.create_session_context(&session2).unwrap();
        
        // Initially both should have default level
        assert_eq!(context1.get_logging_level(), LoggingLevel::Info);
        assert_eq!(context2.get_logging_level(), LoggingLevel::Info);
        
        // Change one session's level
        context1.set_logging_level(LoggingLevel::Debug);
        
        // Other session should be unchanged
        assert_eq!(context1.get_logging_level(), LoggingLevel::Debug);
        assert_eq!(context2.get_logging_level(), LoggingLevel::Info);
        
        // Change second session's level
        context2.set_logging_level(LoggingLevel::Error);
        
        // Both sessions should maintain their own levels
        assert_eq!(context1.get_logging_level(), LoggingLevel::Debug);
        assert_eq!(context2.get_logging_level(), LoggingLevel::Error);
    }
}

/// Test LoggingHandler integration
#[cfg(test)]
mod logging_handler_tests {
    use super::*;

    #[tokio::test]
    async fn test_logging_handler_with_session_context() {
        let capabilities = ServerCapabilities::default();
        let manager = Arc::new(SessionManager::new(capabilities));
        
        let session_id = manager.create_session().await;
        let context = manager.create_session_context(&session_id);
        
        let handler = LoggingHandler;
        
        // Test SetLevelRequest params
        let params = json!({
            "level": "error"
        });
        
        // Call handler with session context
        let result = handler.handle_with_session(Some(params), context).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), json!({}));
        
        // Verify level was set in session
        let context2 = manager.create_session_context(&session_id).unwrap();
        assert_eq!(context2.get_logging_level(), LoggingLevel::Error);
    }

    #[tokio::test]
    async fn test_logging_handler_without_session_context() {
        let handler = LoggingHandler;
        
        let params = json!({
            "level": "warning"
        });
        
        // Call handler without session context (should still work but not store level)
        let result = handler.handle_with_session(Some(params), None).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), json!({}));
    }

    #[tokio::test]
    async fn test_logging_handler_with_invalid_params() {
        let capabilities = ServerCapabilities::default();
        let manager = Arc::new(SessionManager::new(capabilities));
        
        let session_id = manager.create_session().await;
        let context = manager.create_session_context(&session_id);
        
        let handler = LoggingHandler;
        
        // Test with missing params
        let result = handler.handle_with_session(None, context).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_logging_handler_with_all_levels() {
        let capabilities = ServerCapabilities::default();
        let manager = Arc::new(SessionManager::new(capabilities));
        
        let session_id = manager.create_session().await;
        
        let handler = LoggingHandler;
        
        let test_levels = vec![
            ("debug", LoggingLevel::Debug),
            ("info", LoggingLevel::Info),
            ("notice", LoggingLevel::Notice),
            ("warning", LoggingLevel::Warning),
            ("error", LoggingLevel::Error),
            ("critical", LoggingLevel::Critical),
            ("alert", LoggingLevel::Alert),
            ("emergency", LoggingLevel::Emergency),
        ];
        
        for (level_str, expected_level) in test_levels {
            let context = manager.create_session_context(&session_id);
            let params = json!({
                "level": level_str
            });
            
            let result = handler.handle_with_session(Some(params), context).await;
            assert!(result.is_ok(), "Failed to set level: {}", level_str);
            
            // Verify level was set correctly
            let context2 = manager.create_session_context(&session_id).unwrap();
            assert_eq!(context2.get_logging_level(), expected_level, "Wrong level for: {}", level_str);
        }
    }
}

/// Test notify_log filtering behavior
#[cfg(test)]
mod notify_log_filtering_tests {
    use super::*;

    #[tokio::test]
    async fn test_notify_log_respects_session_level() {
        let capabilities = ServerCapabilities::default();
        let manager = Arc::new(SessionManager::new(capabilities));
        
        let session_id = manager.create_session().await;
        let context = manager.create_session_context(&session_id).unwrap();
        
        // Set session to Warning level
        context.set_logging_level(LoggingLevel::Warning);
        
        // These calls should not panic, but filtering happens internally
        context.notify_log("debug", "This should be filtered out");
        context.notify_log("info", "This should be filtered out");
        context.notify_log("notice", "This should be filtered out");
        context.notify_log("warning", "This should pass through");
        context.notify_log("error", "This should pass through");
        context.notify_log("critical", "This should pass through");
        context.notify_log("alert", "This should pass through");
        context.notify_log("emergency", "This should pass through");
    }

    #[tokio::test]
    async fn test_notify_log_with_unknown_level() {
        let capabilities = ServerCapabilities::default();
        let manager = Arc::new(SessionManager::new(capabilities));
        
        let session_id = manager.create_session().await;
        let context = manager.create_session_context(&session_id).unwrap();
        
        // Unknown level should default to Info and be handled gracefully
        context.notify_log("unknown_level", "This should not panic");
    }

    #[tokio::test]
    async fn test_notify_log_filtering_with_different_session_levels() {
        let capabilities = ServerCapabilities::default();
        let manager = Arc::new(SessionManager::new(capabilities));
        
        // Create sessions with different logging levels
        let debug_session = manager.create_session().await;
        let error_session = manager.create_session().await;
        
        let debug_context = manager.create_session_context(&debug_session).unwrap();
        let error_context = manager.create_session_context(&error_session).unwrap();
        
        debug_context.set_logging_level(LoggingLevel::Debug);
        error_context.set_logging_level(LoggingLevel::Error);
        
        // Same message to both sessions - should be filtered differently
        debug_context.notify_log("info", "Info message to debug session"); // Should pass
        error_context.notify_log("info", "Info message to error session"); // Should be filtered
        
        debug_context.notify_log("error", "Error message to debug session"); // Should pass
        error_context.notify_log("error", "Error message to error session"); // Should pass
    }
}

/// Test edge cases and error handling
#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[tokio::test]
    async fn test_invalid_level_string_in_session_state() {
        let capabilities = ServerCapabilities::default();
        let manager = Arc::new(SessionManager::new(capabilities));
        
        let session_id = manager.create_session().await;
        let context = manager.create_session_context(&session_id).unwrap();
        
        // Manually set invalid level string in session state
        (context.set_state)("mcp:logging:level", json!("invalid_level"));
        
        // Should fall back to default Info level
        let level = context.get_logging_level();
        assert_eq!(level, LoggingLevel::Info);
    }

    #[tokio::test]
    async fn test_non_string_value_in_session_state() {
        let capabilities = ServerCapabilities::default();
        let manager = Arc::new(SessionManager::new(capabilities));
        
        let session_id = manager.create_session().await;
        let context = manager.create_session_context(&session_id).unwrap();
        
        // Set non-string value in session state
        (context.set_state)("mcp:logging:level", json!(42));
        
        // Should fall back to default Info level
        let level = context.get_logging_level();
        assert_eq!(level, LoggingLevel::Info);
    }

    #[tokio::test]
    async fn test_logging_level_boundary_conditions() {
        let capabilities = ServerCapabilities::default();
        let manager = Arc::new(SessionManager::new(capabilities));
        
        let session_id = manager.create_session().await;
        let context = manager.create_session_context(&session_id).unwrap();
        
        // Test each level as threshold
        let all_levels = vec![
            LoggingLevel::Debug,
            LoggingLevel::Info,
            LoggingLevel::Notice,
            LoggingLevel::Warning,
            LoggingLevel::Error,
            LoggingLevel::Critical,
            LoggingLevel::Alert,
            LoggingLevel::Emergency,
        ];
        
        for threshold in &all_levels {
            context.set_logging_level(*threshold);
            
            for message_level in &all_levels {
                let should_pass = message_level.priority() >= threshold.priority();
                let actual_pass = context.should_log(*message_level);
                assert_eq!(
                    actual_pass, should_pass,
                    "Level {:?} with threshold {:?}: expected {}, got {}",
                    message_level, threshold, should_pass, actual_pass
                );
            }
        }
    }
}