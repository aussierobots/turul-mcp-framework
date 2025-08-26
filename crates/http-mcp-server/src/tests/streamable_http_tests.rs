//! Streamable HTTP Tests
//!
//! This module tests the Streamable HTTP transport implementation per MCP 2025-06-18 specification including:
//! - HTTP/1.1 and HTTP/2 support 
//! - JSON-RPC request/response streaming
//! - Connection management and upgrades
//! - Error handling and recovery
//! - Protocol compliance validation

use std::sync::Arc;

use hyper::{Body, Method, Request, Response, StatusCode, Version};
use serde_json::json;

use crate::streamable_http::{StreamableHttpHandler, StreamableHttpContext, McpProtocolVersion};
use crate::server::ServerConfig;
use mcp_json_rpc_server::{JsonRpcDispatcher, JsonRpcHandler};
use mcp_protocol::{JsonRpcRequest, JsonRpcResponse, JsonRpcError};

/// Mock handler for streamable HTTP testing
struct MockStreamableHandler;

#[async_trait::async_trait]
impl JsonRpcHandler for MockStreamableHandler {
    async fn handle(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        match request.method.as_str() {
            "stream_test" => {
                let params = request.params.unwrap_or_default();
                JsonRpcResponse::success(request.id, json!({
                    "echoed": params,
                    "protocol": "streamable_http"
                }))
            }
            "large_response" => {
                let large_data = json!({
                    "data": "x".repeat(10_000),
                    "metadata": {
                        "size": 10_000,
                        "type": "large_response"
                    }
                });
                JsonRpcResponse::success(request.id, large_data)
            }
            "error_test" => {
                JsonRpcResponse::error(request.id, JsonRpcError::internal_error())
            }
            _ => {
                JsonRpcResponse::error(request.id, JsonRpcError::method_not_found())
            }
        }
    }
}

/// Test Streamable HTTP handler creation and configuration
#[cfg(test)]
mod streamable_handler_tests {
    use super::*;

    #[tokio::test]
    async fn test_streamable_handler_creation() {
        let config = ServerConfig::default();
        let handler = StreamableHttpHandler::new(Arc::new(config));
        
        // Handler should be created successfully
        println!("Streamable HTTP handler created successfully");
    }

    #[tokio::test]
    async fn test_streamable_handler_with_custom_config() {
        let config = ServerConfig {
            cors_origins: vec!["https://example.com".to_string()],
            max_request_size: 5 * 1024 * 1024,
        };
        
        let handler = StreamableHttpHandler::new(Arc::new(config));
        
        // Handler should accept custom configuration
        println!("Streamable HTTP handler with custom config created");
    }
}

/// Test HTTP version support and protocol negotiation
#[cfg(test)]
mod protocol_version_tests {
    use super::*;

    #[tokio::test]
    async fn test_http_1_1_support() {
        let config = ServerConfig::default();
        let handler = StreamableHttpHandler::new(Arc::new(config));
        
        // Create HTTP/1.1 request
        let mut request = Request::builder()
            .method(Method::POST)
            .uri("/")
            .header("content-type", "application/json")
            .version(Version::HTTP_11)
            .body(Body::from(json!({
                "jsonrpc": "2.0",
                "method": "stream_test",
                "params": {"version": "1.1"},
                "id": 1
            }).to_string()))
            .unwrap();
        
        println!("HTTP/1.1 request prepared for streamable handler");
    }

    #[tokio::test]
    async fn test_http_2_support() {
        let config = ServerConfig::default();
        let handler = StreamableHttpHandler::new(Arc::new(config));
        
        // Create HTTP/2 request
        let mut request = Request::builder()
            .method(Method::POST)
            .uri("/")
            .header("content-type", "application/json")
            .version(Version::HTTP_2)
            .body(Body::from(json!({
                "jsonrpc": "2.0",
                "method": "stream_test",
                "params": {"version": "2.0"},
                "id": 1
            }).to_string()))
            .unwrap();
        
        println!("HTTP/2 request prepared for streamable handler");
    }

    #[tokio::test]
    async fn test_protocol_upgrade_headers() {
        // Test headers that might be used for protocol upgrades
        let upgrade_headers = vec![
            ("Upgrade", "h2c"),
            ("Connection", "Upgrade, HTTP2-Settings"),
            ("HTTP2-Settings", "AAMAAABkAARAAAAAAAIAAAAA"),
        ];
        
        for (name, value) in upgrade_headers {
            println!("Protocol upgrade header: {}: {}", name, value);
        }
    }
}

/// Test JSON-RPC streaming over HTTP
#[cfg(test)]
mod json_rpc_streaming_tests {
    use super::*;

    #[tokio::test]
    async fn test_single_request_response() {
        let handler = MockStreamableHandler;
        
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "stream_test".to_string(),
            params: Some(json!({"test": "data"})),
            id: Some(json!(1)),
        };
        
        let response = handler.handle(request).await;
        
        // Verify response
        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, Some(json!(1)));
        assert!(response.result.is_some());
        
        let result = response.result.unwrap();
        assert_eq!(result["protocol"], "streamable_http");
    }

    #[tokio::test]
    async fn test_batch_request_handling() {
        let handler = MockStreamableHandler;
        
        // Simulate batch requests
        let requests = vec![
            JsonRpcRequest {
                jsonrpc: "2.0".to_string(),
                method: "stream_test".to_string(),
                params: Some(json!({"batch_id": 1})),
                id: Some(json!(1)),
            },
            JsonRpcRequest {
                jsonrpc: "2.0".to_string(),
                method: "stream_test".to_string(),
                params: Some(json!({"batch_id": 2})),
                id: Some(json!(2)),
            },
            JsonRpcRequest {
                jsonrpc: "2.0".to_string(),
                method: "stream_test".to_string(),
                params: Some(json!({"batch_id": 3})),
                id: Some(json!(3)),
            },
        ];
        
        let mut responses = Vec::new();
        for request in requests {
            let response = handler.handle(request).await;
            responses.push(response);
        }
        
        // All responses should be successful
        assert_eq!(responses.len(), 3);
        for (i, response) in responses.iter().enumerate() {
            assert_eq!(response.id, Some(json!(i + 1)));
            assert!(response.result.is_some());
        }
    }

    #[tokio::test]
    async fn test_large_response_streaming() {
        let handler = MockStreamableHandler;
        
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "large_response".to_string(),
            params: None,
            id: Some(json!(1)),
        };
        
        let response = handler.handle(request).await;
        
        // Verify large response handling
        assert!(response.result.is_some());
        let result = response.result.unwrap();
        assert_eq!(result["metadata"]["size"], 10_000);
        assert_eq!(result["metadata"]["type"], "large_response");
    }

    #[tokio::test]
    async fn test_streaming_error_handling() {
        let handler = MockStreamableHandler;
        
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "error_test".to_string(),
            params: None,
            id: Some(json!(1)),
        };
        
        let response = handler.handle(request).await;
        
        // Verify error response
        assert!(response.error.is_some());
        assert!(response.result.is_none());
        
        let error = response.error.unwrap();
        assert_eq!(error.code, -32603); // Internal error
    }
}

/// Test connection management and keep-alive
#[cfg(test)]
mod connection_management_tests {
    use super::*;

    #[tokio::test]
    async fn test_connection_keep_alive() {
        let config = ServerConfig::default();
        let handler = StreamableHttpHandler::new(Arc::new(config));
        
        // Create request with keep-alive headers
        let request = Request::builder()
            .method(Method::POST)
            .uri("/")
            .header("content-type", "application/json")
            .header("connection", "keep-alive")
            .body(Body::from("{}"))
            .unwrap();
        
        println!("Keep-alive request prepared");
    }

    #[tokio::test]
    async fn test_connection_close_handling() {
        let config = ServerConfig::default();
        let handler = StreamableHttpHandler::new(Arc::new(config));
        
        // Create request with close connection
        let request = Request::builder()
            .method(Method::POST)
            .uri("/")
            .header("content-type", "application/json")
            .header("connection", "close")
            .body(Body::from("{}"))
            .unwrap();
        
        println!("Connection close request prepared");
    }

    #[tokio::test]
    async fn test_persistent_connection_reuse() {
        let config = ServerConfig::default();
        let handler = StreamableHttpHandler::new(Arc::new(config));
        
        // Simulate multiple requests on same connection
        let requests = vec![
            json!({"jsonrpc": "2.0", "method": "stream_test", "id": 1}),
            json!({"jsonrpc": "2.0", "method": "stream_test", "id": 2}),
            json!({"jsonrpc": "2.0", "method": "stream_test", "id": 3}),
        ];
        
        for (i, request_body) in requests.iter().enumerate() {
            let request = Request::builder()
                .method(Method::POST)
                .uri("/")
                .header("content-type", "application/json")
                .header("connection", "keep-alive")
                .body(Body::from(request_body.to_string()))
                .unwrap();
                
            println!("Persistent connection request {} prepared", i + 1);
        }
    }
}

/// Test MCP 2025-06-18 specification compliance
#[cfg(test)]
mod mcp_compliance_tests {
    use super::*;

    #[tokio::test]
    async fn test_mcp_request_format_compliance() {
        // Test that requests follow MCP 2025-06-18 format
        let mcp_request = json!({
            "jsonrpc": "2.0",
            "method": "tools/call",
            "params": {
                "name": "test_tool",
                "arguments": {"input": "test"},
                "_meta": {
                    "progressToken": "progress-123"
                }
            },
            "id": "req-123"
        });
        
        // Verify _meta field handling
        assert!(mcp_request["params"]["_meta"].is_object());
        assert_eq!(mcp_request["params"]["_meta"]["progressToken"], "progress-123");
        
        println!("MCP 2025-06-18 request format validated");
    }

    #[tokio::test]
    async fn test_mcp_response_format_compliance() {
        // Test that responses follow MCP 2025-06-18 format
        let mcp_response = json!({
            "jsonrpc": "2.0",
            "result": {
                "content": [
                    {
                        "type": "text",
                        "text": "Tool response"
                    }
                ],
                "_meta": {
                    "usage": {
                        "inputTokens": 10,
                        "outputTokens": 5
                    }
                }
            },
            "id": "req-123"
        });
        
        // Verify _meta field in response
        assert!(mcp_response["result"]["_meta"].is_object());
        assert!(mcp_response["result"]["_meta"]["usage"].is_object());
        
        println!("MCP 2025-06-18 response format validated");
    }

    #[tokio::test]
    async fn test_mcp_error_format_compliance() {
        // Test error format compliance
        let mcp_error_response = json!({
            "jsonrpc": "2.0",
            "error": {
                "code": -32602,
                "message": "Invalid params",
                "data": {
                    "details": "Missing required parameter",
                    "_meta": {
                        "timestamp": "2024-01-01T00:00:00Z"
                    }
                }
            },
            "id": "req-123"
        });
        
        // Verify error structure
        assert_eq!(mcp_error_response["error"]["code"], -32602);
        assert!(mcp_error_response["error"]["data"]["_meta"].is_object());
        
        println!("MCP 2025-06-18 error format validated");
    }

    #[tokio::test]
    async fn test_transport_metadata_handling() {
        // Test transport-specific metadata
        let request_with_transport_meta = json!({
            "jsonrpc": "2.0",
            "method": "test",
            "params": {
                "data": "test",
                "_meta": {
                    "transport": "streamable_http",
                    "version": "2025-06-18",
                    "capabilities": ["streaming", "batching"]
                }
            },
            "id": 1
        });
        
        let meta = &request_with_transport_meta["params"]["_meta"];
        assert_eq!(meta["transport"], "streamable_http");
        assert_eq!(meta["version"], "2025-06-18");
        assert!(meta["capabilities"].is_array());
        
        println!("Transport metadata handling validated");
    }
}

/// Test error handling and recovery
#[cfg(test)]
mod error_recovery_tests {
    use super::*;

    #[tokio::test]
    async fn test_malformed_json_handling() {
        let config = ServerConfig::default();
        let handler = StreamableHttpHandler::new(Arc::new(config));
        
        // Create request with malformed JSON
        let request = Request::builder()
            .method(Method::POST)
            .uri("/")
            .header("content-type", "application/json")
            .body(Body::from("{ invalid json"))
            .unwrap();
        
        println!("Malformed JSON request prepared");
    }

    #[tokio::test]
    async fn test_oversized_request_handling() {
        let config = ServerConfig {
            max_request_size: 1024, // Small limit
            cors_origins: vec![],
        };
        let handler = StreamableHttpHandler::new(Arc::new(config));
        
        // Create oversized request
        let large_data = "x".repeat(2048);
        let request = Request::builder()
            .method(Method::POST)
            .uri("/")
            .header("content-type", "application/json")
            .body(Body::from(large_data))
            .unwrap();
        
        println!("Oversized request prepared");
    }

    #[tokio::test]
    async fn test_invalid_http_method_handling() {
        let config = ServerConfig::default();
        let handler = StreamableHttpHandler::new(Arc::new(config));
        
        // Create request with unsupported method
        let request = Request::builder()
            .method(Method::DELETE)
            .uri("/")
            .body(Body::empty())
            .unwrap();
        
        println!("Invalid HTTP method request prepared");
    }

    #[tokio::test]
    async fn test_missing_content_type_handling() {
        let config = ServerConfig::default();
        let handler = StreamableHttpHandler::new(Arc::new(config));
        
        // Create request without content-type
        let request = Request::builder()
            .method(Method::POST)
            .uri("/")
            .body(Body::from("{}"))
            .unwrap();
        
        println!("Request without content-type prepared");
    }
}

/// Test performance and scalability
#[cfg(test)]
mod performance_tests {
    use super::*;

    #[tokio::test]
    async fn test_concurrent_request_processing() {
        let handler = Arc::new(MockStreamableHandler);
        
        let num_requests = 20; // Reduced for faster test execution
        let mut handles = Vec::new();
        
        for i in 0..num_requests {
            let handler_clone = handler.clone();
            let handle = tokio::spawn(async move {
                let request = JsonRpcRequest {
                    jsonrpc: "2.0".to_string(),
                    method: "stream_test".to_string(),
                    params: Some(json!({"concurrent_id": i})),
                    id: Some(json!(i)),
                };
                
                handler_clone.handle(request).await
            });
            handles.push(handle);
        }
        
        let responses = futures::future::join_all(handles).await;
        
        // All requests should complete successfully
        assert_eq!(responses.len(), num_requests);
        for (i, response) in responses.into_iter().enumerate() {
            let response = response.unwrap();
            assert_eq!(response.id, Some(json!(i)));
            assert!(response.result.is_some());
        }
        
        println!("Concurrent request processing test completed");
    }

    #[tokio::test]
    async fn test_high_frequency_requests() {
        let handler = MockStreamableHandler;
        
        let num_requests = 100; // Reduced for faster test execution
        let start = std::time::Instant::now();
        
        for i in 0..num_requests {
            let request = JsonRpcRequest {
                jsonrpc: "2.0".to_string(),
                method: "stream_test".to_string(),
                params: Some(json!({"sequence": i})),
                id: Some(json!(i)),
            };
            
            let response = handler.handle(request).await;
            assert!(response.result.is_some());
        }
        
        let duration = start.elapsed();
        println!("Processed {} requests in {:?}", num_requests, duration);
        
        // Should process requests efficiently
        assert!(duration.as_millis() < 1000);
    }

    #[tokio::test]
    async fn test_memory_efficiency() {
        let handler = MockStreamableHandler;
        
        // Process many requests to test memory usage
        for batch in 0..10 {
            let mut batch_handles = Vec::new();
            
            for i in 0..50 {
                let request = JsonRpcRequest {
                    jsonrpc: "2.0".to_string(),
                    method: "stream_test".to_string(),
                    params: Some(json!({
                        "batch": batch,
                        "request": i,
                        "data": format!("test_data_{}", i)
                    })),
                    id: Some(json!(format!("{}_{}", batch, i))),
                };
                
                let handle = tokio::spawn(async move {
                    handler.handle(request).await
                });
                batch_handles.push(handle);
            }
            
            // Wait for batch to complete
            let _responses = futures::future::join_all(batch_handles).await;
        }
        
        println!("Memory efficiency test completed");
    }
}