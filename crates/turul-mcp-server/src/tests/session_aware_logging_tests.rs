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

/// Helper function to convert string level to LoggingLevel enum for tests
fn str_to_logging_level(level: &str) -> LoggingLevel {
    match level.to_lowercase().as_str() {
        "debug" => LoggingLevel::Debug,
        "info" => LoggingLevel::Info,
        "notice" => LoggingLevel::Notice,
        "warning" => LoggingLevel::Warning,
        "error" => LoggingLevel::Error,
        "critical" => LoggingLevel::Critical,
        "alert" => LoggingLevel::Alert,
        "emergency" => LoggingLevel::Emergency,
        _ => LoggingLevel::Info, // Default fallback
    }
}

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
        let level = context.get_logging_level().await;
        assert_eq!(level, LoggingLevel::Info);
        
        // Should log messages at Info level or higher
        assert!(context.should_log(LoggingLevel::Info).await);
        assert!(context.should_log(LoggingLevel::Notice).await);
        assert!(context.should_log(LoggingLevel::Warning).await);
        assert!(context.should_log(LoggingLevel::Error).await);
        assert!(context.should_log(LoggingLevel::Critical).await);
        assert!(context.should_log(LoggingLevel::Alert).await);
        assert!(context.should_log(LoggingLevel::Emergency).await);
        
        // Should NOT log messages below Info level
        assert!(!context.should_log(LoggingLevel::Debug).await);
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
            context.set_logging_level(level).await;
            let retrieved_level = context.get_logging_level().await;
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
        context.set_logging_level(LoggingLevel::Warning).await;
        
        // Check that it's stored in session state
        let state_value = (context.get_state)("mcp:logging:level").await;
        assert_eq!(state_value, Some(json!("warning")));
        
        // Create new context for same session - should retrieve the same level
        let context2 = manager.create_session_context(&session_id).unwrap();
        let retrieved_level = context2.get_logging_level().await;
        assert_eq!(retrieved_level, LoggingLevel::Warning);
    }

    #[tokio::test]
    async fn test_should_log_filtering() {
        let capabilities = ServerCapabilities::default();
        let manager = Arc::new(SessionManager::new(capabilities));
        
        let session_id = manager.create_session().await;
        let context = manager.create_session_context(&session_id).unwrap();
        
        // Set session to Error level
        context.set_logging_level(LoggingLevel::Error).await;
        
        // Only Error and higher should pass
        assert!(!context.should_log(LoggingLevel::Debug).await);
        assert!(!context.should_log(LoggingLevel::Info).await);
        assert!(!context.should_log(LoggingLevel::Notice).await);
        assert!(!context.should_log(LoggingLevel::Warning).await);
        assert!(context.should_log(LoggingLevel::Error).await);
        assert!(context.should_log(LoggingLevel::Critical).await);
        assert!(context.should_log(LoggingLevel::Alert).await);
        assert!(context.should_log(LoggingLevel::Emergency).await);
    }

    #[tokio::test]
    async fn test_debug_level_allows_all() {
        let capabilities = ServerCapabilities::default();
        let manager = Arc::new(SessionManager::new(capabilities));
        
        let session_id = manager.create_session().await;
        let context = manager.create_session_context(&session_id).unwrap();
        
        // Set session to Debug level (lowest)
        context.set_logging_level(LoggingLevel::Debug).await;
        
        // All levels should pass
        assert!(context.should_log(LoggingLevel::Debug).await);
        assert!(context.should_log(LoggingLevel::Info).await);
        assert!(context.should_log(LoggingLevel::Notice).await);
        assert!(context.should_log(LoggingLevel::Warning).await);
        assert!(context.should_log(LoggingLevel::Error).await);
        assert!(context.should_log(LoggingLevel::Critical).await);
        assert!(context.should_log(LoggingLevel::Alert).await);
        assert!(context.should_log(LoggingLevel::Emergency).await);
    }

    #[tokio::test]
    async fn test_emergency_level_blocks_most() {
        let capabilities = ServerCapabilities::default();
        let manager = Arc::new(SessionManager::new(capabilities));
        
        let session_id = manager.create_session().await;
        let context = manager.create_session_context(&session_id).unwrap();
        
        // Set session to Emergency level (highest)
        context.set_logging_level(LoggingLevel::Emergency).await;
        
        // Only Emergency should pass
        assert!(!context.should_log(LoggingLevel::Debug).await);
        assert!(!context.should_log(LoggingLevel::Info).await);
        assert!(!context.should_log(LoggingLevel::Notice).await);
        assert!(!context.should_log(LoggingLevel::Warning).await);
        assert!(!context.should_log(LoggingLevel::Error).await);
        assert!(!context.should_log(LoggingLevel::Critical).await);
        assert!(!context.should_log(LoggingLevel::Alert).await);
        assert!(context.should_log(LoggingLevel::Emergency).await);
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
        context1.set_logging_level(LoggingLevel::Debug).await;
        context2.set_logging_level(LoggingLevel::Warning).await;
        context3.set_logging_level(LoggingLevel::Error).await;
        
        // Verify each session has its own level
        assert_eq!(context1.get_logging_level().await, LoggingLevel::Debug);
        assert_eq!(context2.get_logging_level().await, LoggingLevel::Warning);
        assert_eq!(context3.get_logging_level().await, LoggingLevel::Error);
        
        // Verify filtering behavior is independent
        let test_level = LoggingLevel::Info;
        assert!(context1.should_log_sync(test_level)); // Debug allows Info
        assert!(!context2.should_log_sync(test_level)); // Warning blocks Info
        assert!(!context3.should_log_sync(test_level)); // Error blocks Info
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
        assert_eq!(context1.get_logging_level().await, LoggingLevel::Info);
        assert_eq!(context2.get_logging_level().await, LoggingLevel::Info);
        
        // Change one session's level
        context1.set_logging_level(LoggingLevel::Debug).await;
        
        // Other session should be unchanged
        assert_eq!(context1.get_logging_level().await, LoggingLevel::Debug);
        assert_eq!(context2.get_logging_level().await, LoggingLevel::Info);
        
        // Change second session's level
        context2.set_logging_level(LoggingLevel::Error).await;
        
        // Both sessions should maintain their own levels
        assert_eq!(context1.get_logging_level().await, LoggingLevel::Debug);
        assert_eq!(context2.get_logging_level().await, LoggingLevel::Error);
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
        
        // Initialize the session (required before setting logging level)
        let client_info = turul_mcp_protocol::Implementation {
            name: "test-client".to_string(),
            version: "1.0.0".to_string(),
            title: Some("Test Client".to_string()),
        };
        let client_capabilities = turul_mcp_protocol::ClientCapabilities::default();
        manager.initialize_session(&session_id, client_info, client_capabilities).await.unwrap();
        
        let context = manager.create_session_context(&session_id);
        
        let handler = LoggingHandler;
        
        // Test SetLevelRequest params
        let params = json!({
            "level": "error"
        });
        
        // Call handler with session context
        let result = handler.handle_with_session(Some(params), context).await;
        match result {
            Ok(value) => assert_eq!(value, json!({})),
            Err(e) => panic!("Expected Ok but got error: {:?}", e),
        }
        
        // Verify level was set in session
        let context2 = manager.create_session_context(&session_id).unwrap();
        assert_eq!(context2.get_logging_level().await, LoggingLevel::Error);
    }

    #[tokio::test]
    async fn test_logging_handler_without_session_context() {
        let handler = LoggingHandler;
        
        let params = json!({
            "level": "warning"
        });
        
        // Call handler without session context (should fail with session required error)
        let result = handler.handle_with_session(Some(params), None).await;
        assert!(result.is_err());
        // Verify it's the expected "Session required" error
        assert!(result.unwrap_err().to_string().contains("Session required"));
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
        
        // Initialize the session (required before setting logging level)
        let client_info = turul_mcp_protocol::Implementation {
            name: "test-client".to_string(),
            version: "1.0.0".to_string(),
            title: Some("Test Client".to_string()),
        };
        let client_capabilities = turul_mcp_protocol::ClientCapabilities::default();
        manager.initialize_session(&session_id, client_info, client_capabilities).await.unwrap();
        
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
            assert_eq!(context2.get_logging_level().await, expected_level, "Wrong level for: {}", level_str);
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
        context.set_logging_level(LoggingLevel::Warning).await;
        
        // These calls should not panic, but filtering happens internally
        context.notify_log(str_to_logging_level("debug"), serde_json::json!("This should be filtered out"), Some("test".to_string()), None).await;
        context.notify_log(str_to_logging_level("info"), serde_json::json!("This should be filtered out"), Some("test".to_string()), None).await;
        context.notify_log(str_to_logging_level("notice"), serde_json::json!("This should be filtered out"), Some("test".to_string()), None).await;
        context.notify_log(str_to_logging_level("warning"), serde_json::json!("This should pass through"), Some("test".to_string()), None).await;
        context.notify_log(str_to_logging_level("error"), serde_json::json!("This should pass through"), Some("test".to_string()), None).await;
        context.notify_log(str_to_logging_level("critical"), serde_json::json!("This should pass through"), Some("test".to_string()), None).await;
        context.notify_log(str_to_logging_level("alert"), serde_json::json!("This should pass through"), Some("test".to_string()), None).await;
        context.notify_log(str_to_logging_level("emergency"), serde_json::json!("This should pass through"), Some("test".to_string()), None).await;
    }

    #[tokio::test]
    async fn test_notify_log_with_unknown_level() {
        let capabilities = ServerCapabilities::default();
        let manager = Arc::new(SessionManager::new(capabilities));
        
        let session_id = manager.create_session().await;
        let context = manager.create_session_context(&session_id).unwrap();
        
        // Unknown level should default to Info and be handled gracefully
        context.notify_log(str_to_logging_level("unknown_level"), serde_json::json!("This should not panic"), Some("test".to_string()), None).await;
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
        
        debug_context.set_logging_level(LoggingLevel::Debug).await;
        error_context.set_logging_level(LoggingLevel::Error).await;
        
        // Same message to both sessions - should be filtered differently
        debug_context.notify_log(
            str_to_logging_level("info"), 
            serde_json::json!("Info message to debug session"),
            Some("test".to_string()),
            None
        ).await; // Should pass
        error_context.notify_log(
            str_to_logging_level("info"), 
            serde_json::json!("Info message to error session"),
            Some("test".to_string()),
            None
        ).await; // Should be filtered
        
        debug_context.notify_log(
            str_to_logging_level("error"), 
            serde_json::json!("Error message to debug session"),
            Some("test".to_string()),
            None
        ).await; // Should pass
        error_context.notify_log(
            str_to_logging_level("error"), 
            serde_json::json!("Error message to error session"),
            Some("test".to_string()),
            None
        ).await; // Should pass
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
        let level = context.get_logging_level().await;
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
        let level = context.get_logging_level().await;
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
            context.set_logging_level(*threshold).await;
            
            for message_level in &all_levels {
                let should_pass = message_level.priority() >= threshold.priority();
                let actual_pass = context.should_log(*message_level).await;
                assert_eq!(
                    actual_pass, should_pass,
                    "Level {:?} with threshold {:?}: expected {}, got {}",
                    message_level, threshold, should_pass, actual_pass
                );
            }
        }
    }
}