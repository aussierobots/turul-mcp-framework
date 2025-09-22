//! Simple HTTP MCP Server Tests
//!
//! Basic tests for HTTP transport and SSE functionality that use the actual available APIs.

use serde_json::json;
use std::sync::Arc;

use crate::server::{HttpMcpServerBuilder, ServerConfig};
use crate::sse::{SseEvent, SseManager};
use crate::streamable_http::{McpProtocolVersion, StreamableHttpHandler};

/// Test basic server configuration
#[cfg(test)]
mod basic_tests {
    use super::*;

    #[tokio::test]
    async fn test_server_config_creation() {
        let config = ServerConfig::default();

        // Config should have default values
        assert!(config.enable_cors);
        assert_eq!(config.mcp_path, "/mcp");
        assert_eq!(config.max_body_size, 1024 * 1024);

        println!("Server config created with defaults");
    }

    #[tokio::test]
    async fn test_server_config_customization() {
        let config = ServerConfig {
            enable_cors: false,
            mcp_path: "/custom".to_string(),
            max_body_size: 2 * 1024 * 1024,
            ..Default::default()
        };

        assert!(!config.enable_cors);
        assert_eq!(config.mcp_path, "/custom");
        assert_eq!(config.max_body_size, 2 * 1024 * 1024);

        println!("Server config customized successfully");
    }

    #[tokio::test]
    async fn test_server_builder_creation() {
        let _builder = HttpMcpServerBuilder::new();

        // Builder should be created successfully
        println!("HTTP MCP server builder created");
    }

    #[tokio::test]
    async fn test_streamable_handler_creation() {
        let config = ServerConfig::default();
        let _handler = StreamableHttpHandler::new(Arc::new(config));

        // Handler should be created successfully
        println!("Streamable HTTP handler created");
    }
}

/// Test SSE functionality
#[cfg(test)]
mod sse_tests {
    use super::*;

    #[tokio::test]
    async fn test_sse_manager_creation() {
        let manager = SseManager::new();

        assert_eq!(manager.connection_count().await, 0);
        println!("SSE manager created successfully");
    }

    #[tokio::test]
    async fn test_sse_connection_management() {
        let manager = SseManager::new();

        // Create connections
        let _conn1 = manager.create_connection("conn1".to_string()).await;
        let _conn2 = manager.create_connection("conn2".to_string()).await;

        assert_eq!(manager.connection_count().await, 2);

        // Remove a connection
        manager.remove_connection("conn1").await;
        assert_eq!(manager.connection_count().await, 1);

        println!("SSE connection management tested");
    }

    #[tokio::test]
    async fn test_sse_event_formatting() {
        let events = vec![
            SseEvent::Connected,
            SseEvent::Data(json!({"message": "test"})),
            SseEvent::Error("Test error".to_string()),
            SseEvent::KeepAlive,
        ];

        for event in events {
            let formatted = event.format();

            // All events should be properly formatted
            assert!(formatted.contains("event: "));
            assert!(formatted.contains("data: "));
            assert!(formatted.ends_with("\n\n"));

            println!("Event formatted: {}", formatted.replace('\n', "\\n"));
        }
    }

    #[tokio::test]
    async fn test_sse_broadcasting() {
        let manager = SseManager::new();

        // Create some connections
        let _conn1 = manager.create_connection("conn1".to_string()).await;
        let _conn2 = manager.create_connection("conn2".to_string()).await;

        // Test different broadcast methods
        manager.send_data(json!({"test": "data"})).await;
        manager.send_error("Test error".to_string()).await;
        manager.send_keep_alive().await;

        // Direct broadcast
        manager.broadcast(SseEvent::Connected).await;

        println!("SSE broadcasting tested successfully");
    }
}

/// Test MCP protocol version handling
#[cfg(test)]
mod protocol_tests {
    use super::*;

    #[tokio::test]
    async fn test_protocol_version_parsing() {
        let versions = vec![
            ("2024-11-05", Some(McpProtocolVersion::V2024_11_05)),
            ("2025-03-26", Some(McpProtocolVersion::V2025_03_26)),
            ("2025-06-18", Some(McpProtocolVersion::V2025_06_18)),
            ("invalid", None),
        ];

        for (input, expected) in versions {
            let parsed = McpProtocolVersion::parse_version(input);
            assert_eq!(parsed, expected);

            if let Some(version) = parsed {
                assert_eq!(version.as_str(), input);
            }
        }

        println!("Protocol version parsing tested successfully");
    }

    #[tokio::test]
    async fn test_protocol_version_comparison() {
        let v1 = McpProtocolVersion::V2024_11_05;
        let v2 = McpProtocolVersion::V2025_03_26;
        let v3 = McpProtocolVersion::V2025_06_18;

        // Test equality
        assert_eq!(v1, McpProtocolVersion::V2024_11_05);
        assert_eq!(v2, McpProtocolVersion::V2025_03_26);
        assert_eq!(v3, McpProtocolVersion::V2025_06_18);

        // Test inequality
        assert_ne!(v1, v2);
        assert_ne!(v2, v3);
        assert_ne!(v1, v3);

        println!("Protocol version comparison tested successfully");
    }
}

/// Test concurrent operations
#[cfg(test)]
mod concurrency_tests {
    use super::*;

    #[tokio::test]
    async fn test_concurrent_sse_connections() {
        let manager = Arc::new(SseManager::new());

        let num_connections = 20;
        let mut handles = Vec::new();

        // Create connections concurrently
        for i in 0..num_connections {
            let manager_clone = manager.clone();
            let handle = tokio::spawn(async move {
                let connection_id = format!("conn_{}", i);
                manager_clone.create_connection(connection_id).await
            });
            handles.push(handle);
        }

        // Wait for all connections to be created
        let _connections = futures::future::join_all(handles).await;

        assert_eq!(manager.connection_count().await, num_connections);

        // Test concurrent broadcasting
        let num_events = 10;
        let mut broadcast_handles = Vec::new();

        for i in 0..num_events {
            let manager_clone = manager.clone();
            let handle = tokio::spawn(async move {
                let data = json!({"event": i, "message": format!("Event {}", i)});
                manager_clone.send_data(data).await;
            });
            broadcast_handles.push(handle);
        }

        futures::future::join_all(broadcast_handles).await;

        println!("Concurrent SSE operations tested successfully");
    }

    #[tokio::test]
    async fn test_concurrent_handler_creation() {
        let num_handlers = 10;
        let mut handles = Vec::new();

        for i in 0..num_handlers {
            let handle = tokio::spawn(async move {
                let config = ServerConfig {
                    mcp_path: format!("/mcp_{}", i),
                    ..Default::default()
                };

                let _handler = StreamableHttpHandler::new(Arc::new(config));
                format!("Handler {} created", i)
            });
            handles.push(handle);
        }

        let results = futures::future::join_all(handles).await;

        // All handlers should be created successfully
        assert_eq!(results.len(), num_handlers);
        for result in results {
            assert!(result.is_ok());
            println!("{}", result.unwrap());
        }
    }
}

/// Test error handling
#[cfg(test)]
mod error_tests {
    use super::*;

    #[tokio::test]
    async fn test_sse_error_event_handling() {
        let manager = SseManager::new();
        let _conn = manager.create_connection("test".to_string()).await;

        // Test various error scenarios
        manager.send_error("".to_string()).await; // Empty error
        manager
            .send_error("Test error with special chars: 🚀".to_string())
            .await;
        manager
            .send_error("Error with \"quotes\" and \n newlines".to_string())
            .await;

        println!("SSE error handling tested successfully");
    }

    #[tokio::test]
    async fn test_large_data_handling() {
        let manager = SseManager::new();
        let _conn = manager
            .create_connection("large_data_test".to_string())
            .await;

        // Test with large JSON data
        let large_data = json!({
            "large_string": "x".repeat(10_000),
            "large_array": (0..1000).collect::<Vec<i32>>(),
            "nested": {
                "deep": {
                    "structure": "test"
                }
            }
        });

        manager.send_data(large_data).await;

        println!("Large data handling tested successfully");
    }

    #[tokio::test]
    async fn test_invalid_protocol_versions() {
        let invalid_versions = vec!["", "2024", "invalid-version", "2025-99-99", "1.0.0"];

        for version in invalid_versions {
            let parsed = McpProtocolVersion::parse_version(version);
            assert!(parsed.is_none(), "Version '{}' should not parse", version);
        }

        println!("Invalid protocol version handling tested successfully");
    }
}

/// Test performance characteristics
#[cfg(test)]
mod performance_tests {
    use super::*;
    use std::time::Instant;

    #[tokio::test]
    async fn test_sse_performance() {
        let manager = SseManager::new();

        // Create connections
        let num_connections = 50;
        for i in 0..num_connections {
            let _conn = manager.create_connection(format!("perf_conn_{}", i)).await;
        }

        assert_eq!(manager.connection_count().await, num_connections);

        // Test broadcast performance
        let num_events = 100;
        let start = Instant::now();

        for i in 0..num_events {
            let data = json!({"event_id": i, "timestamp": start.elapsed().as_millis()});
            manager.send_data(data).await;
        }

        let duration = start.elapsed();
        println!(
            "Sent {} events to {} connections in {:?}",
            num_events, num_connections, duration
        );

        // Should complete reasonably quickly
        assert!(duration.as_millis() < 1000);
    }

    #[tokio::test]
    async fn test_handler_creation_performance() {
        let num_handlers = 100;
        let start = Instant::now();

        let mut handlers = Vec::new();
        for i in 0..num_handlers {
            let config = ServerConfig {
                mcp_path: format!("/perf_{}", i),
                ..Default::default()
            };

            let handler = StreamableHttpHandler::new(Arc::new(config));
            handlers.push(handler);
        }

        let duration = start.elapsed();
        println!("Created {} handlers in {:?}", num_handlers, duration);

        assert_eq!(handlers.len(), num_handlers);
        assert!(duration.as_millis() < 100);
    }
}
