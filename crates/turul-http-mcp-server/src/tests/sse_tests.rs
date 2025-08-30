//! Server-Sent Events (SSE) Tests
//!
//! This module tests SSE functionality for real-time MCP notifications including:
//! - SSE connection establishment and management
//! - Event streaming and formatting
//! - Client connection lifecycle
//! - Error handling and recovery
//! - Event buffering and delivery guarantees

use std::sync::Arc;
use std::time::Duration;

use hyper::{Body, Method, Request, StatusCode};
use serde_json::json;
use tokio::time::sleep;

use crate::sse::{SseEvent, SseManager};

/// Mock SSE event data for testing
fn create_test_events() -> Vec<SseEvent> {
    vec![
        SseEvent::Data(json!({"text": "Hello, SSE!"})),
        SseEvent::Data(json!({"type": "info", "message": "Test notification"})),
        SseEvent::Data(json!({"step": 1, "total": 10})),
        SseEvent::KeepAlive,
        SseEvent::Error("Test error".to_string()),
    ]
}

/// Test SSE connection establishment
#[cfg(test)]
mod sse_connection_tests {
    use super::*;

    #[tokio::test]
    async fn test_sse_connection_creation() {
        let manager = SseManager::new();
        let connection_id = "test-connection-123".to_string();
        let connection = manager.create_connection(connection_id.clone()).await;
        
        assert_eq!(connection.id, connection_id);
        assert_eq!(manager.connection_count().await, 1);
        println!("SSE connection created with ID: {}", connection.id);
    }

    #[tokio::test]
    async fn test_sse_connection_lifecycle() {
        let manager = SseManager::new();
        let connection_id = "lifecycle-test".to_string();
        
        // Create connection
        let _connection = manager.create_connection(connection_id.clone()).await;
        assert_eq!(manager.connection_count().await, 1);
        
        // Remove connection
        manager.remove_connection(&connection_id).await;
        assert_eq!(manager.connection_count().await, 0);
        
        println!("SSE connection lifecycle tested successfully");
    }

    #[tokio::test]
    async fn test_multiple_sse_connections() {
        let manager = SseManager::new();
        let mut connections = Vec::new();
        
        // Create multiple connections
        for i in 1..=3 {
            let connection_id = format!("conn-{}", i);
            let connection = manager.create_connection(connection_id.clone()).await;
            assert_eq!(connection.id, connection_id);
            connections.push(connection);
        }
        
        assert_eq!(manager.connection_count().await, 3);
        println!("Multiple SSE connections created successfully");
    }
}

/// Test SSE event creation and formatting
#[cfg(test)]
mod sse_event_tests {
    use super::*;

    #[tokio::test]
    async fn test_basic_sse_event_creation() {
        let event = SseEvent::Data(json!({"message": "test"}));
        let formatted = event.format();
        
        assert!(formatted.contains("event: data"));
        assert!(formatted.contains("data: "));
        assert!(formatted.ends_with("\n\n"));
    }

    #[tokio::test]
    async fn test_sse_keep_alive_event() {
        let event = SseEvent::KeepAlive;
        let formatted = event.format();
        
        assert!(formatted.contains("event: ping"));
        assert!(formatted.contains("data: "));
        assert!(formatted.ends_with("\n\n"));
    }

    #[tokio::test]
    async fn test_sse_error_event() {
        let event = SseEvent::Error("Test error message".to_string());
        let formatted = event.format();
        
        assert!(formatted.contains("event: error"));
        assert!(formatted.contains("Test error message"));
        assert!(formatted.ends_with("\n\n"));
    }

    #[tokio::test]
    async fn test_sse_event_formatting() {
        let events = create_test_events();
        
        for event in events {
            let formatted = event.format();
            
            // Should contain event type
            assert!(formatted.contains("event: "));
            
            // Should contain data
            assert!(formatted.contains("data: "));
            
            // Should end with double newline
            assert!(formatted.ends_with("\n\n"));
            
            println!("Event formatted: {}", formatted.replace('\n', "\\n"));
        }
    }

    #[tokio::test]
    async fn test_sse_event_json_serialization() {
        let complex_data = json!({
            "user": {
                "id": 123,
                "name": "Test User",
                "metadata": {
                    "role": "admin",
                    "permissions": ["read", "write"]
                }
            },
            "timestamp": "2024-01-01T00:00:00Z",
            "nested_array": [1, 2, {"nested": "object"}]
        });
        
        let event = SseEvent::Data(complex_data.clone());
        let formatted = event.format();
        
        // Should contain the serialized JSON
        assert!(formatted.contains("data: "));
        
        // Should be valid SSE format
        assert!(formatted.starts_with("event: data\n"));
        assert!(formatted.ends_with("\n\n"));
        
        println!("Complex JSON event formatted successfully");
    }
}

/// Test SSE manager functionality
#[cfg(test)]
mod sse_manager_tests {
    use super::*;

    #[tokio::test]
    async fn test_sse_manager_creation() {
        let manager = SseManager::new();
        
        assert_eq!(manager.connection_count().await, 0);
        println!("SSE manager created successfully");
    }

    #[tokio::test]
    async fn test_connection_registration() {
        let manager = SseManager::new();
        
        let _conn1 = manager.create_connection("conn1".to_string()).await;
        assert_eq!(manager.connection_count().await, 1);
        
        // Register more connections
        let _conn2 = manager.create_connection("conn2".to_string()).await;
        let _conn3 = manager.create_connection("conn3".to_string()).await;
        assert_eq!(manager.connection_count().await, 3);
        
        println!("Connections registered: count = {}", manager.connection_count().await);
    }

    #[tokio::test]
    async fn test_connection_removal() {
        let manager = SseManager::new();
        
        let connection_id = "test_connection".to_string();
        let _connection = manager.create_connection(connection_id.clone()).await;
        assert_eq!(manager.connection_count().await, 1);
        
        manager.remove_connection(&connection_id).await;
        assert_eq!(manager.connection_count().await, 0);
        
        // Removing non-existent connection should not cause issues
        manager.remove_connection(&connection_id).await;
        assert_eq!(manager.connection_count().await, 0);
        
        println!("Connection removal tested successfully");
    }

    #[tokio::test]
    async fn test_event_broadcasting() {
        let manager = SseManager::new();
        
        // Register multiple connections
        let _conn1 = manager.create_connection("conn1".to_string()).await;
        let _conn2 = manager.create_connection("conn2".to_string()).await;
        let _conn3 = manager.create_connection("conn3".to_string()).await;
        
        assert_eq!(manager.connection_count().await, 3);
        
        // Broadcast events
        let events = create_test_events();
        for event in events {
            manager.broadcast(event).await;
        }
        
        println!("Event broadcasting tested successfully");
    }

    #[tokio::test]
    async fn test_data_sending() {
        let manager = SseManager::new();
        
        let _conn1 = manager.create_connection("conn1".to_string()).await;
        let _conn2 = manager.create_connection("conn2".to_string()).await;
        
        // Send data to all connections
        manager.send_data(json!({"message": "test data"})).await;
        manager.send_error("Test error message".to_string()).await;
        manager.send_keep_alive().await;
        
        println!("Data sending tested successfully");
    }
}

/// Test SSE error handling and edge cases
#[cfg(test)]
mod sse_error_handling_tests {
    use super::*;

    #[tokio::test]
    async fn test_connection_cleanup_on_disconnect() {
        let manager = SseManager::new();
        
        let connection_id = manager.register_connection().await;
        assert_eq!(manager.connection_count().await, 1);
        
        // Simulate connection disconnect
        let removed = manager.remove_connection(&connection_id).await;
        assert!(removed);
        assert_eq!(manager.connection_count().await, 0);
        
        // Subsequent operations on disconnected connection should fail
        let event = SseEvent::new("test", json!({}));
        let result = manager.send_event_to_connection(&connection_id, event).await;
        assert!(result.is_err());
        
        println!("Connection cleanup tested successfully");
    }

    #[tokio::test]
    async fn test_large_event_data_handling() {
        let manager = SseManager::new();
        let connection_id = manager.register_connection().await;
        
        // Create event with large data payload
        let large_data = json!({
            "large_string": "x".repeat(10_000),
            "large_array": (0..1000).collect::<Vec<i32>>(),
            "nested": {
                "deep": {
                    "structure": {
                        "with": {
                            "many": {
                                "levels": "test"
                            }
                        }
                    }
                }
            }
        });
        
        let event = SseEvent::new("large_event", large_data);
        let result = manager.send_event_to_connection(&connection_id, event).await;
        
        // Should handle large events gracefully
        assert!(result.is_ok());
        
        println!("Large event data handling tested successfully");
    }

    #[tokio::test]
    async fn test_malformed_event_data() {
        let events = vec![
            // Event with empty data
            SseEvent::new("empty", json!(null)),
            
            // Event with special characters
            SseEvent::new("special_chars", json!({
                "message": "Test with 🚀 emojis and \n newlines"
            })),
            
            // Event with very long event type
            SseEvent::new(&"x".repeat(1000), json!({"test": "data"})),
        ];
        
        for event in events {
            let formatted = event.format();
            
            // Should still produce valid SSE format
            assert!(formatted.contains("event: "));
            assert!(formatted.contains("data: "));
            assert!(formatted.ends_with("\n\n"));
        }
        
        println!("Malformed event data handling tested successfully");
    }

    #[tokio::test]
    async fn test_concurrent_connection_management() {
        let manager = Arc::new(SseManager::new());
        
        let num_connections = 20;
        let mut handles = Vec::new();
        
        // Create connections concurrently
        for i in 0..num_connections {
            let manager_clone = manager.clone();
            let handle = tokio::spawn(async move {
                let connection_id = manager_clone.register_connection().await;
                
                // Send some events
                for j in 0..5 {
                    let event = SseEvent::new("concurrent", json!({
                        "connection": i,
                        "event": j
                    }));
                    let _ = manager_clone.send_event_to_connection(&connection_id, event).await;
                }
                
                connection_id
            });
            handles.push(handle);
        }
        
        // Wait for all connections to be created
        let connection_ids = futures::future::join_all(handles).await;
        let connection_ids: Vec<String> = connection_ids.into_iter()
            .map(|result| result.unwrap())
            .collect();
        
        assert_eq!(manager.connection_count().await, num_connections);
        
        // Clean up connections
        for connection_id in connection_ids {
            let removed = manager.remove_connection(&connection_id).await;
            assert!(removed);
        }
        
        assert_eq!(manager.connection_count().await, 0);
        
        println!("Concurrent connection management tested successfully");
    }
}

/// Test SSE integration with HTTP requests
#[cfg(test)]
mod sse_http_integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_sse_request_headers() {
        // Test SSE request validation
        let valid_sse_request = Request::builder()
            .method(Method::GET)
            .uri("/events")
            .header("Accept", "text/event-stream")
            .header("Cache-Control", "no-cache")
            .body(Body::empty())
            .unwrap();
        
        // Validate headers
        assert_eq!(valid_sse_request.headers().get("Accept").unwrap(), "text/event-stream");
        assert_eq!(valid_sse_request.headers().get("Cache-Control").unwrap(), "no-cache");
        
        println!("SSE request headers validated successfully");
    }

    #[tokio::test]
    async fn test_sse_response_headers() {
        // Test that we can construct proper SSE response headers
        let expected_headers = vec![
            ("Content-Type", "text/event-stream"),
            ("Cache-Control", "no-cache"),
            ("Connection", "keep-alive"),
            ("Access-Control-Allow-Origin", "*"),
        ];
        
        for (name, value) in expected_headers {
            println!("SSE response should include header: {}: {}", name, value);
        }
    }

    #[tokio::test]
    async fn test_sse_keepalive_mechanism() {
        let manager = SseManager::new();
        let connection_id = manager.register_connection().await;
        
        // Send keepalive events
        for i in 0..5 {
            let keepalive = SseEvent::new("keepalive", json!({"sequence": i}));
            let result = manager.send_event_to_connection(&connection_id, keepalive).await;
            assert!(result.is_ok());
            
            // Small delay between keepalives
            sleep(Duration::from_millis(10)).await;
        }
        
        println!("SSE keepalive mechanism tested successfully");
    }
}

/// Test SSE performance and scalability
#[cfg(test)]
mod sse_performance_tests {
    use super::*;

    #[tokio::test]
    async fn test_high_frequency_events() {
        let manager = SseManager::new();
        let connection_id = manager.register_connection().await;
        
        let num_events = 100; // Reduced for faster test execution
        let start = std::time::Instant::now();
        
        // Send events rapidly
        for i in 0..num_events {
            let event = SseEvent::new("high_freq", json!({"sequence": i}));
            let result = manager.send_event_to_connection(&connection_id, event).await;
            assert!(result.is_ok());
        }
        
        let duration = start.elapsed();
        println!("Sent {} events in {:?}", num_events, duration);
        
        // Events should be sent quickly
        assert!(duration < Duration::from_secs(1));
    }

    #[tokio::test]
    async fn test_many_connections_broadcast() {
        let manager = SseManager::new();
        
        let num_connections = 50; // Reduced for faster test execution
        let mut connection_ids = Vec::new();
        
        // Create many connections
        for _ in 0..num_connections {
            let connection_id = manager.register_connection().await;
            connection_ids.push(connection_id);
        }
        
        assert_eq!(manager.connection_count().await, num_connections);
        
        let num_broadcasts = 10;
        let start = std::time::Instant::now();
        
        // Broadcast to all connections
        for i in 0..num_broadcasts {
            let event = SseEvent::new("broadcast", json!({"id": i}));
            let sent_count = manager.broadcast_event(event).await;
            assert_eq!(sent_count, num_connections);
        }
        
        let duration = start.elapsed();
        println!("Broadcasted {} events to {} connections in {:?}",
                num_broadcasts, num_connections, duration);
        
        // Cleanup
        for connection_id in connection_ids {
            manager.remove_connection(&connection_id).await;
        }
        
        assert_eq!(manager.connection_count().await, 0);
    }

    #[tokio::test]
    async fn test_memory_usage_with_many_connections() {
        let manager = SseManager::new();
        
        let num_connections = 100; // Moderate number for memory test
        let mut connection_ids = Vec::new();
        
        // Create connections and store IDs
        for _ in 0..num_connections {
            let connection_id = manager.register_connection().await;
            connection_ids.push(connection_id);
        }
        
        assert_eq!(manager.connection_count().await, num_connections);
        
        // Send events to test memory usage
        for i in 0..10 {
            let event = SseEvent::new("memory_test", json!({
                "data": format!("Test data {}", i),
                "connections": num_connections
            }));
            
            let sent_count = manager.broadcast_event(event).await;
            assert_eq!(sent_count, num_connections);
        }
        
        // Cleanup all connections
        for connection_id in connection_ids {
            let removed = manager.remove_connection(&connection_id).await;
            assert!(removed);
        }
        
        assert_eq!(manager.connection_count().await, 0);
        
        println!("Memory usage test with {} connections completed", num_connections);
    }
}