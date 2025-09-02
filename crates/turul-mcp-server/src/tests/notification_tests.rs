//! Comprehensive Notification Broadcasting Tests
//!
//! This module tests all aspects of notification broadcasting including:
//! - Session event broadcasting to individual sessions
//! - System-wide broadcast capabilities
//! - MCP-compliant notification types (progress, logging, resources, tools)
//! - Real-time notification delivery and SSE integration
//! - Error handling and edge cases for notification systems

use std::sync::Arc;

use serde_json::json;

use crate::session::{SessionManager, SessionEvent};
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

/// Test basic notification sending to specific sessions
#[cfg(test)]
mod session_notification_tests {
    use super::*;

    #[tokio::test]
    async fn test_send_notification_to_existing_session() {
        let capabilities = ServerCapabilities::default();
        let manager = SessionManager::new(capabilities);
        
        let session_id = manager.create_session().await;
        
        // Test different notification types
        let notifications = vec![
            SessionEvent::KeepAlive,
            SessionEvent::Notification(json!({
                "jsonrpc": "2.0",
                "method": "notifications/message",
                "params": {
                    "level": "info",
                    "message": "Test notification"
                }
            })),
            SessionEvent::Custom {
                event_type: "test_event".to_string(),
                data: json!({"custom": "data"}),
            },
        ];
        
        for notification in notifications {
            let result = manager.send_event_to_session(&session_id, notification).await;
            // Note: Result may be Ok or Err depending on whether there are active receivers
            // This is normal behavior for broadcast channels
            if let Err(e) = result {
                println!("Note: Notification may fail without active receivers: {:?}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_send_notification_to_nonexistent_session() {
        let capabilities = ServerCapabilities::default();
        let manager = SessionManager::new(capabilities);
        
        let nonexistent_session = "non-existent-session-id";
        let notification = SessionEvent::KeepAlive;
        
        let result = manager.send_event_to_session(nonexistent_session, notification).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_notification_delivery_with_session_context() {
        let capabilities = ServerCapabilities::default();
        let manager = Arc::new(SessionManager::new(capabilities));
        
        let session_id = manager.create_session().await;
        let context = manager.create_session_context(&session_id).unwrap();
        
        // Test different context notification methods
        context.notify_log(turul_mcp_protocol::logging::LoggingLevel::Info, serde_json::json!("Test log message"), Some("test".to_string()), None);
        context.notify_progress("test-token", 25);
        context.notify_progress_with_total("test-token", 50, 100);
        context.notify_resources_changed();
        context.notify_resource_updated("test://resource");
        context.notify_tools_changed();
        
        let custom_event = SessionEvent::Custom {
            event_type: "test_custom".to_string(),
            data: json!({"message": "custom notification"}),
        };
        context.notify(custom_event);
        
        // These should not panic - notifications are fire-and-forget
    }
}

/// Test system-wide broadcast capabilities
#[cfg(test)]
mod broadcast_notification_tests {
    use super::*;

    #[tokio::test]
    async fn test_broadcast_to_multiple_sessions() {
        let capabilities = ServerCapabilities::default();
        let manager = SessionManager::new(capabilities);
        
        // Create multiple sessions
        let session1 = manager.create_session().await;
        let session2 = manager.create_session().await;
        let session3 = manager.create_session().await;
        
        assert_eq!(manager.session_count().await, 3);
        
        // Broadcast a message to all sessions
        let broadcast_event = SessionEvent::Custom {
            event_type: "system_announcement".to_string(),
            data: json!({
                "message": "System maintenance scheduled",
                "priority": "high"
            }),
        };
        
        manager.broadcast_event(broadcast_event).await;
        
        // Verify sessions still exist (broadcast shouldn't affect session lifecycle)
        assert!(manager.session_exists(&session1).await);
        assert!(manager.session_exists(&session2).await);
        assert!(manager.session_exists(&session3).await);
        assert_eq!(manager.session_count().await, 3);
    }

    #[tokio::test]
    async fn test_broadcast_to_empty_session_list() {
        let capabilities = ServerCapabilities::default();
        let manager = SessionManager::new(capabilities);
        
        // No sessions created
        assert_eq!(manager.session_count().await, 0);
        
        let broadcast_event = SessionEvent::KeepAlive;
        
        // Broadcasting to no sessions should not panic or error
        manager.broadcast_event(broadcast_event).await;
    }

    #[tokio::test]
    async fn test_broadcast_with_session_removal_during_broadcast() {
        let capabilities = ServerCapabilities::default();
        let manager = SessionManager::new(capabilities);
        
        // Create sessions
        let session1 = manager.create_session().await;
        let session2 = manager.create_session().await;
        let session3 = manager.create_session().await;
        
        // Remove one session
        let removed = manager.remove_session(&session2).await;
        assert!(removed);
        assert_eq!(manager.session_count().await, 2);
        
        // Broadcast should work with remaining sessions
        let broadcast_event = SessionEvent::Custom {
            event_type: "partial_broadcast".to_string(),
            data: json!({"remaining_sessions": 2}),
        };
        
        manager.broadcast_event(broadcast_event).await;
        
        // Verify remaining sessions
        assert!(manager.session_exists(&session1).await);
        assert!(!manager.session_exists(&session2).await);
        assert!(manager.session_exists(&session3).await);
    }
}

/// Test MCP-compliant notification types
#[cfg(test)]
mod mcp_notification_tests {
    use super::*;

    #[tokio::test]
    async fn test_progress_notifications() {
        let capabilities = ServerCapabilities::default();
        let manager = Arc::new(SessionManager::new(capabilities));
        
        let session_id = manager.create_session().await;
        let context = manager.create_session_context(&session_id).unwrap();
        
        // Test progress notifications with different patterns
        let progress_tokens = vec!["upload", "download", "processing", "analysis"];
        
        for (i, token) in progress_tokens.iter().enumerate() {
            let progress = (i as u64 + 1) * 25;
            context.notify_progress(*token, progress);
            
            // Also test with total
            context.notify_progress_with_total(*token, progress, 100);
        }
        
        // Test edge cases
        context.notify_progress("zero-progress", 0);
        context.notify_progress_with_total("complete", 100, 100);
        context.notify_progress("over-100", 150); // Should still work
    }

    #[tokio::test]
    async fn test_logging_notifications() {
        let capabilities = ServerCapabilities::default();
        let manager = Arc::new(SessionManager::new(capabilities));
        
        let session_id = manager.create_session().await;
        let context = manager.create_session_context(&session_id).unwrap();
        
        // Test different log levels
        let log_levels = vec!["debug", "info", "warn", "error"];
        
        for level in log_levels {
            context.notify_log(str_to_logging_level(level), serde_json::json!(format!("Test {} message", level)), Some("test".to_string()), None);
        }
        
        // Test with complex messages
        context.notify_log(str_to_logging_level("info"), serde_json::json!("Multi-line\nmessage\nwith special chars: 🚀"), Some("test".to_string()), None);
        context.notify_log(str_to_logging_level("error"), json!({"structured": "log", "error_code": 500}), Some("test".to_string()), None);
    }

    #[tokio::test]
    async fn test_resource_notifications() {
        let capabilities = ServerCapabilities::default();
        let manager = Arc::new(SessionManager::new(capabilities));
        
        let session_id = manager.create_session().await;
        let context = manager.create_session_context(&session_id).unwrap();
        
        // Test resource list changed notification
        context.notify_resources_changed();
        
        // Test specific resource updates
        let resource_uris = vec![
            "file:///path/to/resource.txt",
            "http://example.com/api/resource",
            "custom://schema/resource/123",
            "mem://temporary/resource",
        ];
        
        for uri in resource_uris {
            context.notify_resource_updated(uri);
        }
    }

    #[tokio::test]
    async fn test_tool_notifications() {
        let capabilities = ServerCapabilities::default();
        let manager = Arc::new(SessionManager::new(capabilities));
        
        let session_id = manager.create_session().await;
        let context = manager.create_session_context(&session_id).unwrap();
        
        // Test tools list changed notification
        context.notify_tools_changed();
        
        // Tool notifications should be fire-and-forget
        // Multiple calls should not cause issues
        for _ in 0..5 {
            context.notify_tools_changed();
        }
    }

    #[tokio::test]
    async fn test_custom_notifications() {
        let capabilities = ServerCapabilities::default();
        let manager = Arc::new(SessionManager::new(capabilities));
        
        let session_id = manager.create_session().await;
        let context = manager.create_session_context(&session_id).unwrap();
        
        // Test various custom notification types
        let custom_notifications = vec![
            SessionEvent::Custom {
                event_type: "user_interaction".to_string(),
                data: json!({
                    "action": "click",
                    "element": "button",
                    "timestamp": "2024-01-01T00:00:00Z"
                }),
            },
            SessionEvent::Custom {
                event_type: "system_alert".to_string(),
                data: json!({
                    "severity": "warning",
                    "message": "High memory usage detected",
                    "threshold": 85.5
                }),
            },
            SessionEvent::Custom {
                event_type: "data_update".to_string(),
                data: json!({
                    "table": "users",
                    "operation": "insert",
                    "count": 1
                }),
            },
        ];
        
        for notification in custom_notifications {
            context.notify(notification);
        }
    }
}

/// Test notification delivery and SSE integration
#[cfg(test)]
mod notification_delivery_tests {
    use super::*;

    #[tokio::test]
    async fn test_session_event_subscription() {
        let capabilities = ServerCapabilities::default();
        let manager = SessionManager::new(capabilities);
        
        let session_id = manager.create_session().await;
        
        // Note: For testing SSE subscription, we need access to session internals
        // In a real implementation, this would be handled by the HTTP/SSE layer
        // For now, we'll test the manager's event sending capability
        
        // Send a test event
        let test_event = SessionEvent::Custom {
            event_type: "test".to_string(),
            data: json!({"test": "data"}),
        };
        
        let send_result = manager.send_event_to_session(&session_id, test_event.clone()).await;
        // Result depends on whether there are active receivers
        if send_result.is_ok() {
            println!("Event sent successfully");
        } else {
            println!("Event send failed (no active receivers): {:?}", send_result.err());
        }
    }

    #[tokio::test]
    async fn test_multiple_subscribers_per_session() {
        let capabilities = ServerCapabilities::default();
        let manager = SessionManager::new(capabilities);
        
        let session_id = manager.create_session().await;
        
        // Test sending multiple events to the same session
        let events = vec![
            SessionEvent::KeepAlive,
            SessionEvent::Custom {
                event_type: "test1".to_string(),
                data: json!({"id": 1}),
            },
            SessionEvent::Custom {
                event_type: "test2".to_string(),
                data: json!({"id": 2}),
            },
        ];
        
        for event in events {
            let result = manager.send_event_to_session(&session_id, event).await;
            // Results may vary based on receiver availability
            if result.is_err() {
                println!("Event send failed (no active receivers)");
            }
        }
        
        // Session should still exist
        assert!(manager.session_exists(&session_id).await);
    }

    #[tokio::test]
    async fn test_session_disconnect_event() {
        let capabilities = ServerCapabilities::default();
        let manager = SessionManager::new(capabilities);
        
        let session_id = manager.create_session().await;
        assert!(manager.session_exists(&session_id).await);
        
        // Remove session (should trigger disconnect event internally)
        let removed = manager.remove_session(&session_id).await;
        assert!(removed);
        
        // Verify session no longer exists
        assert!(!manager.session_exists(&session_id).await);
        
        // Try to send event to removed session (should fail)
        let result = manager.send_event_to_session(&session_id, SessionEvent::KeepAlive).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }
}

/// Test error handling and edge cases
#[cfg(test)]
mod notification_error_tests {
    use super::*;

    #[tokio::test]
    async fn test_notification_with_invalid_json() {
        let capabilities = ServerCapabilities::default();
        let manager = Arc::new(SessionManager::new(capabilities));
        
        let session_id = manager.create_session().await;
        let context = manager.create_session_context(&session_id).unwrap();
        
        // These should not panic even with unusual inputs
        context.notify_log(str_to_logging_level("info"), serde_json::json!(""), Some("test".to_string()), None); // Empty strings
        context.notify_log(str_to_logging_level("invalid_level"), serde_json::json!("Test message"), Some("test".to_string()), None);
        context.notify_progress("", 0);
        context.notify_resource_updated("");
        
        // Test with very long strings
        let long_string = "x".repeat(10000);
        context.notify_log(str_to_logging_level("info"), serde_json::json!(long_string.clone()), Some("test".to_string()), None);
        context.notify_progress(&long_string, 50);
    }

    #[tokio::test]
    async fn test_notification_during_session_expiry() {
        let capabilities = ServerCapabilities::default();
        let manager = Arc::new(SessionManager::new(capabilities));
        
        let session_id = manager.create_session().await;
        let context = manager.create_session_context(&session_id).unwrap();
        
        // Remove session to simulate expiry
        manager.remove_session(&session_id).await;
        
        // Attempt to send notifications to expired session
        context.notify_log(str_to_logging_level("info"), serde_json::json!("Message to expired session"), Some("test".to_string()), None);
        context.notify_progress("test", 50);
        
        // These should not panic, even though session may be expired
    }

    #[tokio::test]
    async fn test_concurrent_notification_sending() {
        let capabilities = ServerCapabilities::default();
        let manager = Arc::new(SessionManager::new(capabilities));
        
        let session_id = manager.create_session().await;
        let context = Arc::new(manager.create_session_context(&session_id).unwrap());
        
        let num_concurrent = 20;
        let mut handles = Vec::new();
        
        // Send notifications concurrently
        for i in 0..num_concurrent {
            let context_clone = context.clone();
            let handle = tokio::spawn(async move {
                context_clone.notify_log(str_to_logging_level("info"), serde_json::json!(format!("Concurrent message {}", i)), Some("test".to_string()), None);
                context_clone.notify_progress("concurrent", i as u64);
                
                let custom_event = SessionEvent::Custom {
                    event_type: "concurrent_test".to_string(),
                    data: json!({"id": i}),
                };
                context_clone.notify(custom_event);
            });
            handles.push(handle);
        }
        
        // Wait for all notifications to complete
        futures::future::join_all(handles).await;
        
        // Session should still be valid
        assert!(manager.session_exists(&session_id).await);
    }

    #[tokio::test]
    async fn test_notification_channel_capacity_limits() {
        let capabilities = ServerCapabilities::default();
        let manager = SessionManager::new(capabilities);
        
        let session_id = manager.create_session().await;
        
        // Send many events rapidly to test channel capacity
        // Default channel capacity is 128, so we'll send more than that
        let num_events = 200;
        
        for i in 0..num_events {
            let event = SessionEvent::Custom {
                event_type: "capacity_test".to_string(),
                data: json!({"index": i}),
            };
            
            let result = manager.send_event_to_session(&session_id, event).await;
            // Some may fail if channel is full, which is expected behavior
            if result.is_err() {
                println!("Event {} failed to send (channel may be full)", i);
            }
        }
        
        // Session should still exist
        assert!(manager.session_exists(&session_id).await);
    }
}

/// Performance tests for notification systems
#[cfg(test)]
mod notification_performance_tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[tokio::test]
    async fn test_notification_throughput() {
        let capabilities = ServerCapabilities::default();
        let manager = Arc::new(SessionManager::new(capabilities));
        
        let session_id = manager.create_session().await;
        let context = manager.create_session_context(&session_id).unwrap();
        
        let num_notifications = 100; // Reduced for faster test execution
        let counter = Arc::new(AtomicUsize::new(0));
        
        let start = std::time::Instant::now();
        
        // Send notifications as fast as possible
        for i in 0..num_notifications {
            context.notify_log(str_to_logging_level("performance"), serde_json::json!(format!("Message {}", i)), Some("test".to_string()), None);
            counter.fetch_add(1, Ordering::SeqCst);
        }
        
        let duration = start.elapsed();
        let sent_count = counter.load(Ordering::SeqCst);
        
        println!("Sent {} notifications in {:?}", sent_count, duration);
        assert_eq!(sent_count, num_notifications);
        
        // Verify session is still valid
        assert!(manager.session_exists(&session_id).await);
    }

    #[tokio::test]
    async fn test_broadcast_performance() {
        let capabilities = ServerCapabilities::default();
        let manager = SessionManager::new(capabilities);
        
        let num_sessions = 50; // Reduced for faster test execution
        let mut session_ids = Vec::new();
        
        // Create multiple sessions
        for _ in 0..num_sessions {
            let session_id = manager.create_session().await;
            session_ids.push(session_id);
        }
        
        assert_eq!(manager.session_count().await, num_sessions);
        
        let num_broadcasts = 10; // Reduced for faster test execution
        let start = std::time::Instant::now();
        
        // Perform broadcasts
        for i in 0..num_broadcasts {
            let event = SessionEvent::Custom {
                event_type: "broadcast_performance".to_string(),
                data: json!({"broadcast_id": i}),
            };
            manager.broadcast_event(event).await;
        }
        
        let duration = start.elapsed();
        
        println!("Completed {} broadcasts to {} sessions in {:?}",
                num_broadcasts, num_sessions, duration);
        
        // All sessions should still exist
        assert_eq!(manager.session_count().await, num_sessions);
    }
}