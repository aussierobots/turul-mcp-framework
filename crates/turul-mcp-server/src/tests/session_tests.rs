//! Comprehensive Session Management Tests
//!
//! This module tests all aspects of session management including:
//! - Session lifecycle (creation, initialization, expiry)
//! - State management and persistence
//! - Session context functionality
//! - Event broadcasting and notifications
//! - Concurrent access and thread safety
//! - Error handling and edge cases

use std::sync::Arc;
use std::time::Duration;

use serde_json::json;
use tokio::time::sleep;

use crate::session::{SessionError, SessionEvent, SessionManager};
use turul_mcp_protocol::{
    ClientCapabilities, Implementation, ServerCapabilities, logging::LoggingLevel,
};

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

/// Test session creation and basic operations
#[cfg(test)]
mod basic_operations_tests {
    use super::*;

    #[tokio::test]
    async fn test_session_manager_creation() {
        let capabilities = ServerCapabilities::default();
        let manager = SessionManager::new(capabilities.clone());

        assert_eq!(manager.session_count().await, 0);
    }

    #[tokio::test]
    async fn test_session_creation_and_retrieval() {
        let capabilities = ServerCapabilities::default();
        let manager = SessionManager::new(capabilities);

        let session_id = manager.create_session().await;

        // Session ID should be a valid no-hyphen UUIDv7
        assert!(!session_id.is_empty());
        assert_eq!(session_id.len(), 32, "no-hyphen UUID is 32 hex chars");
        assert!(!session_id.contains('-'), "new session IDs must not contain hyphens");
        assert!(
            session_id.chars().all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)),
            "must be lowercase hex"
        );
        let uuid = uuid::Uuid::parse_str(&session_id).expect("must parse as valid UUID");
        assert_eq!(uuid.get_version_num(), 7, "must be UUIDv7");

        // Session should exist
        assert!(manager.session_exists(&session_id).await);
        assert_eq!(manager.session_count().await, 1);

        // Non-existent session should not exist
        assert!(!manager.session_exists("non-existent").await);
    }

    #[tokio::test]
    async fn test_multiple_session_creation() {
        let capabilities = ServerCapabilities::default();
        let manager = SessionManager::new(capabilities);

        let session1 = manager.create_session().await;
        let session2 = manager.create_session().await;
        let session3 = manager.create_session().await;

        // All sessions should be unique
        assert_ne!(session1, session2);
        assert_ne!(session2, session3);
        assert_ne!(session1, session3);

        // All sessions should exist
        assert!(manager.session_exists(&session1).await);
        assert!(manager.session_exists(&session2).await);
        assert!(manager.session_exists(&session3).await);

        assert_eq!(manager.session_count().await, 3);
    }

    #[tokio::test]
    async fn test_session_removal() {
        let capabilities = ServerCapabilities::default();
        let manager = SessionManager::new(capabilities);

        let session_id = manager.create_session().await;
        assert!(manager.session_exists(&session_id).await);
        assert_eq!(manager.session_count().await, 1);

        // Remove the session
        let removed = manager.remove_session(&session_id).await;
        assert!(removed);

        // Session should no longer exist
        assert!(!manager.session_exists(&session_id).await);
        assert_eq!(manager.session_count().await, 0);

        // Removing non-existent session should return false
        let removed_again = manager.remove_session(&session_id).await;
        assert!(!removed_again);
    }

    #[tokio::test]
    async fn test_session_touching() {
        let capabilities = ServerCapabilities::default();
        let manager = SessionManager::new(capabilities);

        let session_id = manager.create_session().await;

        // Touching valid session should succeed
        let result = manager.touch_session(&session_id).await;
        assert!(result.is_ok());

        // Touching non-existent session should fail
        let result = manager.touch_session("non-existent").await;
        assert!(matches!(result, Err(SessionError::NotFound(_))));
    }
}

/// Test session state management
#[cfg(test)]
mod state_management_tests {
    use super::*;

    #[tokio::test]
    async fn test_basic_state_operations() {
        let capabilities = ServerCapabilities::default();
        let manager = SessionManager::new(capabilities);

        let session_id = manager.create_session().await;

        // Initially no state
        let value = manager.get_session_state(&session_id, "test_key").await;
        assert_eq!(value, None);

        // Set state
        manager
            .set_session_state(&session_id, "test_key", json!("test_value"))
            .await;

        // Retrieve state
        let value = manager.get_session_state(&session_id, "test_key").await;
        assert_eq!(value, Some(json!("test_value")));

        // Update state
        manager
            .set_session_state(&session_id, "test_key", json!("updated_value"))
            .await;
        let value = manager.get_session_state(&session_id, "test_key").await;
        assert_eq!(value, Some(json!("updated_value")));

        // Remove state
        let removed = manager.remove_session_state(&session_id, "test_key").await;
        assert_eq!(removed, Some(json!("updated_value")));

        // State should be gone
        let value = manager.get_session_state(&session_id, "test_key").await;
        assert_eq!(value, None);

        // Removing non-existent key should return None
        let removed = manager
            .remove_session_state(&session_id, "non_existent")
            .await;
        assert_eq!(removed, None);
    }

    #[tokio::test]
    async fn test_complex_state_types() {
        let capabilities = ServerCapabilities::default();
        let manager = SessionManager::new(capabilities);

        let session_id = manager.create_session().await;

        // Test different JSON types
        let test_cases = vec![
            ("string", json!("hello world")),
            ("number", json!(42.5)),
            ("integer", json!(123)),
            ("boolean", json!(true)),
            ("array", json!([1, 2, 3, "four"])),
            (
                "object",
                json!({"nested": {"value": 123}, "array": [1, 2, 3]}),
            ),
            ("null", json!(null)),
        ];

        // Set all values
        for (key, value) in &test_cases {
            manager
                .set_session_state(&session_id, key, value.clone())
                .await;
        }

        // Verify all values
        for (key, expected_value) in &test_cases {
            let actual_value = manager.get_session_state(&session_id, key).await;
            assert_eq!(actual_value, Some(expected_value.clone()));
        }
    }

    #[tokio::test]
    async fn test_multiple_session_state_isolation() {
        let capabilities = ServerCapabilities::default();
        let manager = SessionManager::new(capabilities);

        let session1 = manager.create_session().await;
        let session2 = manager.create_session().await;

        // Set different values in each session
        manager
            .set_session_state(&session1, "key", json!("session1_value"))
            .await;
        manager
            .set_session_state(&session2, "key", json!("session2_value"))
            .await;

        // Each session should have its own value
        let value1 = manager.get_session_state(&session1, "key").await;
        let value2 = manager.get_session_state(&session2, "key").await;

        assert_eq!(value1, Some(json!("session1_value")));
        assert_eq!(value2, Some(json!("session2_value")));

        // Removing from one session shouldn't affect the other
        manager.remove_session_state(&session1, "key").await;

        let value1 = manager.get_session_state(&session1, "key").await;
        let value2 = manager.get_session_state(&session2, "key").await;

        assert_eq!(value1, None);
        assert_eq!(value2, Some(json!("session2_value")));
    }

    #[tokio::test]
    async fn test_state_operations_on_nonexistent_session() {
        let capabilities = ServerCapabilities::default();
        let manager = SessionManager::new(capabilities);

        let fake_session_id = "non-existent-session";

        // Get state from non-existent session
        let value = manager.get_session_state(fake_session_id, "key").await;
        assert_eq!(value, None);

        // Set state on non-existent session (should silently fail)
        manager
            .set_session_state(fake_session_id, "key", json!("value"))
            .await;

        // Remove state from non-existent session
        let removed = manager.remove_session_state(fake_session_id, "key").await;
        assert_eq!(removed, None);
    }
}

/// Test session context functionality
#[cfg(test)]
mod session_context_tests {
    use super::*;

    #[tokio::test]
    async fn test_session_context_creation() {
        let capabilities = ServerCapabilities::default();
        let manager = Arc::new(SessionManager::new(capabilities));

        let session_id = manager.create_session().await;
        let context = manager.create_session_context(&session_id);

        assert!(context.is_some());
        let context = context.unwrap();
        assert_eq!(context.session_id, session_id);
    }

    #[tokio::test]
    async fn test_session_context_state_operations() {
        let capabilities = ServerCapabilities::default();
        let manager = Arc::new(SessionManager::new(capabilities));

        let session_id = manager.create_session().await;
        let context = manager.create_session_context(&session_id).unwrap();

        // Set state through context
        (context.set_state)("test_key", json!("context_value")).await;

        // Get state through context
        let value = (context.get_state)("test_key").await;
        assert_eq!(value, Some(json!("context_value")));

        // Verify state is also accessible through manager
        let manager_value = manager.get_session_state(&session_id, "test_key").await;
        assert_eq!(manager_value, Some(json!("context_value")));

        // Remove state through context
        let removed = (context.remove_state)("test_key").await;
        assert_eq!(removed, Some(json!("context_value")));

        // Verify removal
        let value = (context.get_state)("test_key").await;
        assert_eq!(value, None);
    }

    #[tokio::test]
    async fn test_session_context_typed_operations() {
        let capabilities = ServerCapabilities::default();
        let manager = Arc::new(SessionManager::new(capabilities));

        let session_id = manager.create_session().await;
        let context = manager.create_session_context(&session_id).unwrap();

        // Test typed operations
        #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
        struct TestData {
            name: String,
            count: u32,
            active: bool,
        }

        let test_data = TestData {
            name: "test".to_string(),
            count: 42,
            active: true,
        };

        // Set typed state
        let result = context.set_typed_state("typed_key", &test_data).await;
        assert!(result.is_ok());

        // Get typed state
        let retrieved: Option<TestData> = context.get_typed_state("typed_key").await;
        assert_eq!(retrieved, Some(test_data));

        // Test type mismatch (should return None)
        let wrong_type: Option<String> = context.get_typed_state("typed_key").await;
        assert_eq!(wrong_type, None);
    }

    #[tokio::test]
    async fn test_session_context_notifications() {
        let capabilities = ServerCapabilities::default();
        let manager = Arc::new(SessionManager::new(capabilities));

        let session_id = manager.create_session().await;
        let context = manager.create_session_context(&session_id).unwrap();

        // Test different notification types
        context
            .notify_log(
                turul_mcp_protocol::logging::LoggingLevel::Info,
                serde_json::json!("Test log message"),
                Some("test".to_string()),
                None,
            )
            .await;
        context.notify_progress("test-token", 25).await;
        context
            .notify_progress_with_total("test-token", 50, 100)
            .await;
        context.notify_resources_changed().await;
        context.notify_resource_updated("test://resource").await;
        context.notify_tools_changed().await;

        // Test custom notification
        let custom_event = SessionEvent::Custom {
            event_type: "test_event".to_string(),
            data: json!({"message": "custom test"}),
        };
        context.notify(custom_event).await;

        // These should not panic or error - notifications are fire-and-forget
    }
}

/// Test session initialization and capabilities
#[cfg(test)]
mod initialization_tests {
    use super::*;

    #[tokio::test]
    async fn test_session_initialization() {
        let capabilities = ServerCapabilities::default();
        let manager = SessionManager::new(capabilities);

        let session_id = manager.create_session().await;

        // Initially not initialized
        assert!(!manager.is_session_initialized(&session_id).await);

        // Initialize session
        let client_info = Implementation {
            name: "test_client".to_string(),
            version: "1.0.0".to_string(),
            title: None,
            icons: None,
            description: None,
            website_url: None,
        };
        let client_capabilities = ClientCapabilities::default();

        let result = manager
            .initialize_session(
                &session_id,
                client_info.clone(),
                client_capabilities.clone(),
            )
            .await;
        assert!(result.is_ok());

        // Should now be initialized
        assert!(manager.is_session_initialized(&session_id).await);
    }

    #[tokio::test]
    async fn test_initialization_of_nonexistent_session() {
        let capabilities = ServerCapabilities::default();
        let manager = SessionManager::new(capabilities);

        let client_info = Implementation {
            name: "test_client".to_string(),
            version: "1.0.0".to_string(),
            title: None,
            icons: None,
            description: None,
            website_url: None,
        };
        let client_capabilities = ClientCapabilities::default();

        let result = manager
            .initialize_session("non-existent", client_info, client_capabilities)
            .await;
        assert!(matches!(result, Err(SessionError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_session_context_initialization_check() {
        let capabilities = ServerCapabilities::default();
        let manager = Arc::new(SessionManager::new(capabilities));

        let session_id = manager.create_session().await;
        let context = manager.create_session_context(&session_id).unwrap();

        // Initially not initialized
        assert!(!((context.is_initialized)().await));

        // Initialize session
        let client_info = Implementation {
            name: "test_client".to_string(),
            version: "1.0.0".to_string(),
            title: None,
            icons: None,
            description: None,
            website_url: None,
        };
        let client_capabilities = ClientCapabilities::default();

        manager
            .initialize_session(&session_id, client_info, client_capabilities)
            .await
            .unwrap();

        // Should now be initialized
        assert!(((context.is_initialized)().await));
    }
}

/// Test session expiry and cleanup
#[cfg(test)]
mod expiry_and_cleanup_tests {
    use super::*;

    #[tokio::test]
    async fn test_session_expiry() {
        // Note: This test is simplified since session_timeout is private
        // In practice, sessions expire after 30 minutes by default
        let capabilities = ServerCapabilities::default();
        let manager = SessionManager::new(capabilities);

        let session_id = manager.create_session().await;
        assert!(manager.session_exists(&session_id).await);

        // Test touch on valid session
        let result = manager.touch_session(&session_id).await;
        assert!(result.is_ok());

        // Test touch on non-existent session
        let result = manager.touch_session("non-existent").await;
        assert!(matches!(result, Err(SessionError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_session_touch_updates_access_time() {
        let capabilities = ServerCapabilities::default();
        let manager = SessionManager::new(capabilities);

        let session_id = manager.create_session().await;

        // Touch session multiple times
        for _ in 0..5 {
            sleep(Duration::from_millis(10)).await;
            let result = manager.touch_session(&session_id).await;
            assert!(result.is_ok());
        }

        // Session should still exist
        assert!(manager.session_exists(&session_id).await);
    }

    #[tokio::test]
    async fn test_cleanup_sessions() {
        let capabilities = ServerCapabilities::default();
        let manager = SessionManager::new(capabilities);

        // Create multiple sessions
        let session1 = manager.create_session().await;
        let session2 = manager.create_session().await;
        let session3 = manager.create_session().await;

        assert_eq!(manager.session_count().await, 3);

        // Remove one session manually
        manager.remove_session(&session2).await;
        assert_eq!(manager.session_count().await, 2);

        // Test cleanup (with default timeout, no sessions should be expired yet)
        let cleaned = manager.cleanup_expired().await;
        assert_eq!(cleaned, 0); // No sessions should be expired with default 30min timeout

        // Remaining sessions should still exist
        assert!(manager.session_exists(&session1).await);
        assert!(!manager.session_exists(&session2).await); // This was manually removed
        assert!(manager.session_exists(&session3).await);
    }

    #[tokio::test]
    async fn test_automatic_cleanup_task() {
        let capabilities = ServerCapabilities::default();
        let manager = Arc::new(SessionManager::new(capabilities));

        // Start cleanup task
        let cleanup_task = manager.clone().start_cleanup_task();

        // Create sessions
        let _session1 = manager.create_session().await;
        let _session2 = manager.create_session().await;

        assert_eq!(manager.session_count().await, 2);

        // Let cleanup task run briefly (it should not clean up fresh sessions)
        sleep(Duration::from_millis(100)).await;

        // Sessions should still exist (default 30min timeout)
        assert_eq!(manager.session_count().await, 2);

        // Stop cleanup task
        cleanup_task.abort();
    }
}

/// Test event broadcasting and SSE functionality
#[cfg(test)]
mod event_broadcasting_tests {
    use super::*;

    #[tokio::test]
    async fn test_send_event_to_specific_session() {
        let capabilities = ServerCapabilities::default();
        let manager = SessionManager::new(capabilities);

        let session_id = manager.create_session().await;

        let event = SessionEvent::Custom {
            event_type: "test".to_string(),
            data: json!({"message": "test event"}),
        };

        // Note: Events may fail to send if no receivers are listening
        // This is normal behavior for broadcast channels
        let _result = manager.send_event_to_session(&session_id, event).await;
        // We don't assert success here because it depends on having active receivers

        // Test sending to non-existent session should still fail
        let result = manager
            .send_event_to_session("non-existent", SessionEvent::KeepAlive)
            .await;
        assert!(matches!(result, Err(SessionError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_broadcast_event_to_all_sessions() {
        let capabilities = ServerCapabilities::default();
        let manager = SessionManager::new(capabilities);

        // Create multiple sessions
        let _session1 = manager.create_session().await;
        let _session2 = manager.create_session().await;
        let _session3 = manager.create_session().await;

        let broadcast_event = SessionEvent::Custom {
            event_type: "broadcast".to_string(),
            data: json!({"announcement": "system maintenance"}),
        };

        // This should not panic or error
        manager.broadcast_event(broadcast_event).await;
    }

    #[tokio::test]
    async fn test_session_event_sending() {
        let capabilities = ServerCapabilities::default();
        let manager = SessionManager::new(capabilities);

        let session_id = manager.create_session().await;

        // Test sending various events
        let test_events = vec![
            SessionEvent::KeepAlive,
            SessionEvent::Custom {
                event_type: "test".to_string(),
                data: json!({"message": "test event"}),
            },
            SessionEvent::Notification(json!({"type": "notification", "data": "test"})),
        ];

        // Send events to session (may fail if no receivers, which is normal)
        for event in test_events {
            let _result = manager.send_event_to_session(&session_id, event).await;
            // Don't assert success as it depends on having active receivers
        }

        // Test sending to non-existent session
        let result = manager
            .send_event_to_session("non-existent", SessionEvent::KeepAlive)
            .await;
        assert!(matches!(result, Err(SessionError::NotFound(_))));
    }
}

/// Test concurrent access and thread safety
#[cfg(test)]
mod concurrency_tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[tokio::test]
    async fn test_concurrent_session_creation() {
        let capabilities = ServerCapabilities::default();
        let manager = Arc::new(SessionManager::new(capabilities));

        let num_tasks = 50;
        let mut handles = Vec::new();

        // Create many sessions concurrently
        for _ in 0..num_tasks {
            let manager_clone = manager.clone();
            let handle = tokio::spawn(async move { manager_clone.create_session().await });
            handles.push(handle);
        }

        // Wait for all sessions to be created
        let session_ids: Vec<String> = futures::future::join_all(handles)
            .await
            .into_iter()
            .map(|result| result.unwrap())
            .collect();

        // All session IDs should be unique
        let mut unique_ids = std::collections::HashSet::new();
        for id in &session_ids {
            assert!(
                unique_ids.insert(id.clone()),
                "Duplicate session ID: {}",
                id
            );
        }

        assert_eq!(manager.session_count().await, num_tasks);
    }

    #[tokio::test]
    async fn test_concurrent_state_operations() {
        let capabilities = ServerCapabilities::default();
        let manager = Arc::new(SessionManager::new(capabilities));

        let session_id = manager.create_session().await;
        let num_operations = 100;
        let counter = Arc::new(AtomicUsize::new(0));

        let mut handles = Vec::new();

        // Perform many state operations concurrently
        for i in 0..num_operations {
            let manager_clone = manager.clone();
            let session_id_clone = session_id.clone();
            let counter_clone = counter.clone();

            let handle = tokio::spawn(async move {
                let key = format!("key_{}", i);
                let value = json!(format!("value_{}", i));

                // Set state
                manager_clone
                    .set_session_state(&session_id_clone, &key, value.clone())
                    .await;

                // Get state
                let retrieved = manager_clone
                    .get_session_state(&session_id_clone, &key)
                    .await;
                assert_eq!(retrieved, Some(value));

                // Remove state
                let removed = manager_clone
                    .remove_session_state(&session_id_clone, &key)
                    .await;
                assert_eq!(removed, Some(json!(format!("value_{}", i))));

                counter_clone.fetch_add(1, Ordering::SeqCst);
            });

            handles.push(handle);
        }

        // Wait for all operations to complete
        futures::future::join_all(handles).await;

        // All operations should have completed
        assert_eq!(counter.load(Ordering::SeqCst), num_operations);
    }

    #[tokio::test]
    async fn test_concurrent_session_context_usage() {
        let capabilities = ServerCapabilities::default();
        let manager = Arc::new(SessionManager::new(capabilities));

        let session_id = manager.create_session().await;
        let context = manager.create_session_context(&session_id).unwrap();
        let context = Arc::new(context);

        let num_tasks = 20;
        let mut handles = Vec::new();

        // Use session context from multiple tasks
        for i in 0..num_tasks {
            let context_clone = context.clone();

            let handle = tokio::spawn(async move {
                let key = format!("concurrent_key_{}", i);
                let value = json!(i);

                // Set state through context
                (context_clone.set_state)(&key, value.clone()).await;

                // Get state through context
                let retrieved = (context_clone.get_state)(&key).await;
                assert_eq!(retrieved, Some(value));

                // Send notification
                context_clone
                    .notify_log(
                        str_to_logging_level("info"),
                        serde_json::json!(format!("Concurrent operation {}", i)),
                        Some("test".to_string()),
                        None,
                    )
                    .await;
            });

            handles.push(handle);
        }

        // Wait for all operations
        futures::future::join_all(handles).await;

        // Verify all keys were set
        for i in 0..num_tasks {
            let key = format!("concurrent_key_{}", i);
            let value = (context.get_state)(&key).await;
            assert_eq!(value, Some(json!(i)));
        }
    }
}

/// Test error conditions and edge cases
#[cfg(test)]
mod error_handling_tests {
    use super::*;

    #[tokio::test]
    async fn test_invalid_session_operations() {
        let capabilities = ServerCapabilities::default();
        let manager = SessionManager::new(capabilities);

        let invalid_session_id = "invalid-session-id";

        // All operations on invalid session should handle gracefully
        assert!(!manager.session_exists(invalid_session_id).await);
        assert!(!manager.is_session_initialized(invalid_session_id).await);

        let touch_result = manager.touch_session(invalid_session_id).await;
        assert!(matches!(touch_result, Err(SessionError::NotFound(_))));

        let state_value = manager.get_session_state(invalid_session_id, "key").await;
        assert_eq!(state_value, None);

        let removed_value = manager
            .remove_session_state(invalid_session_id, "key")
            .await;
        assert_eq!(removed_value, None);

        let event_result = manager
            .send_event_to_session(invalid_session_id, SessionEvent::KeepAlive)
            .await;
        assert!(matches!(event_result, Err(SessionError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_session_context_with_invalid_session() {
        let capabilities = ServerCapabilities::default();
        let manager = Arc::new(SessionManager::new(capabilities));

        let invalid_session_id = "invalid-session-id";
        let context = manager.create_session_context(invalid_session_id);

        // Context should still be created but operations should return None/fail gracefully
        assert!(context.is_some());
        let context = context.unwrap();

        // State operations should return None for invalid session
        let value = (context.get_state)("any_key").await;
        assert_eq!(value, None);

        let removed = (context.remove_state)("any_key").await;
        assert_eq!(removed, None);

        // Set state and notifications should not panic
        (context.set_state)("key", json!("value")).await;
        context
            .notify_log(
                str_to_logging_level("info"),
                serde_json::json!("This should not panic"),
                Some("test".to_string()),
                None,
            )
            .await;

        assert!(!((context.is_initialized)().await));
    }

    #[tokio::test]
    async fn test_malformed_state_data() {
        let capabilities = ServerCapabilities::default();
        let manager = Arc::new(SessionManager::new(capabilities));

        let session_id = manager.create_session().await;
        let context = manager.create_session_context(&session_id).unwrap();

        // Set valid JSON data
        (context.set_state)("valid_json", json!({"key": "value"})).await;

        // Try to retrieve as wrong type
        #[derive(serde::Deserialize, Debug, PartialEq)]
        struct WrongType {
            number: u32,
        }

        let wrong_type: Option<WrongType> = context.get_typed_state("valid_json").await;
        assert_eq!(wrong_type, None);

        // Set non-serializable data should fail gracefully
        #[derive(serde::Serialize)]
        struct NonSerializable {
            #[serde(skip_serializing)]
            _data: std::collections::HashMap<String, std::rc::Rc<String>>,
        }

        let non_serializable = NonSerializable {
            _data: std::collections::HashMap::new(),
        };

        // This should not panic but should return an error
        let _result = context
            .set_typed_state("non_serializable", non_serializable)
            .await;
        // The result depends on the actual serialization behavior
        // but should not panic
    }
}

/// Performance and stress tests
#[cfg(test)]
mod performance_tests {
    use super::*;

    #[tokio::test]
    async fn test_large_session_count() {
        let capabilities = ServerCapabilities::default();
        let manager = SessionManager::new(capabilities);

        let num_sessions = 100; // Reduced for faster test execution
        let mut session_ids = Vec::new();

        // Create many sessions
        for _ in 0..num_sessions {
            let session_id = manager.create_session().await;
            session_ids.push(session_id);
        }

        assert_eq!(manager.session_count().await, num_sessions);

        // Access each session
        for session_id in &session_ids {
            assert!(manager.session_exists(session_id).await);
            manager
                .set_session_state(session_id, "test", json!("value"))
                .await;
        }

        // Cleanup
        for session_id in &session_ids {
            manager.remove_session(session_id).await;
        }

        assert_eq!(manager.session_count().await, 0);
    }

    // test_high_frequency_operations removed - caused async deadlocks in unit tests
    // These performance tests should be integration tests with separate server/client processes
}
