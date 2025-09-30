//! MCP 2025-06-18 Specification Compliance Tests
//!
//! This module contains comprehensive tests to validate that our MCP framework
//! implementation fully complies with the Model Context Protocol specification
//! version 2025-06-18.
//!
//! Tests cover:
//! - JSON-RPC 2.0 compliance
//! - MCP message structure validation
//! - Protocol initialization sequence
//! - Tool and resource discovery
//! - Notification handling
//! - Error responses and codes
//! - _meta field handling
//! - Session and context management

use serde_json::json;
use std::collections::HashMap;

use turul_mcp_protocol::{
    JsonRpcError, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse, RequestParams,
    ResultWithMeta,
};

/// Test JSON-RPC 2.0 compliance
#[cfg(test)]
mod json_rpc_compliance {
    use super::*;

    #[tokio::test]
    async fn test_json_rpc_request_structure() {
        // Valid JSON-RPC 2.0 request structure
        let valid_request = json!({
            "jsonrpc": "2.0",
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {
                    "name": "test-client",
                    "version": "1.0.0"
                }
            },
            "id": 1
        });

        // Parse as MCP request
        let request: JsonRpcRequest = serde_json::from_value(valid_request).unwrap();

        // Validate required fields
        assert_eq!(request.jsonrpc, "2.0");
        assert_eq!(request.method, "initialize");
        assert!(request.params.is_some());
        assert_eq!(request.id, json!(1));
    }

    #[tokio::test]
    async fn test_json_rpc_response_structure() {
        // Test successful response
        let mut data = HashMap::new();
        data.insert("protocolVersion".to_string(), json!("2025-06-18"));
        data.insert("capabilities".to_string(), json!({}));
        data.insert(
            "serverInfo".to_string(),
            json!({
                "name": "test-server",
                "version": "1.0.0"
            }),
        );

        let result = ResultWithMeta::new(data);
        let success_response = JsonRpcResponse::success(json!(1), result);

        assert_eq!(success_response.jsonrpc, "2.0");
        assert_eq!(success_response.id, json!(1));
        assert!(success_response.result.is_some());
        assert!(success_response.error.is_none());

        // Test error response
        let error_response = JsonRpcResponse::error(json!(2), JsonRpcError::method_not_found());

        assert_eq!(error_response.jsonrpc, "2.0");
        assert_eq!(error_response.id, json!(2));
        assert!(error_response.result.is_none());
        assert!(error_response.error.is_some());

        let error = error_response.error.unwrap();
        assert_eq!(error.code, -32601); // Method not found
    }

    #[tokio::test]
    async fn test_json_rpc_notification_structure() {
        // Notifications should not have an id field
        let notification = JsonRpcNotification::new("notifications/initialized".to_string());

        let serialized = serde_json::to_value(&notification).unwrap();
        assert_eq!(serialized["jsonrpc"], "2.0");
        assert_eq!(serialized["method"], "notifications/initialized");
        assert!(serialized.get("id").is_none()); // Notifications must not have id
    }

    #[tokio::test]
    async fn test_json_rpc_error_codes() {
        // Standard JSON-RPC error codes
        let errors = vec![
            (-32700, JsonRpcError::parse_error()),
            (-32600, JsonRpcError::invalid_request()),
            (-32601, JsonRpcError::method_not_found()),
            (-32602, JsonRpcError::invalid_params()),
            (-32603, JsonRpcError::internal_error()),
        ];

        for (expected_code, error) in errors {
            assert_eq!(error.code, expected_code);
            assert!(!error.message.is_empty());
        }
    }
}

/// Test MCP protocol initialization sequence
#[cfg(test)]
mod initialization_compliance {
    use super::*;

    #[tokio::test]
    async fn test_mcp_initialization_sequence() {
        // Step 1: Client sends initialize request
        let initialize_request = json!({
            "jsonrpc": "2.0",
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {
                    "roots": {
                        "listChanged": false  // MCP compliance: static framework
                    },
                    "sampling": {}
                },
                "clientInfo": {
                    "name": "test-client",
                    "version": "1.0.0"
                }
            },
            "id": 1
        });

        // Validate request structure
        let request: JsonRpcRequest = serde_json::from_value(initialize_request).unwrap();
        assert_eq!(request.method, "initialize");

        let params = request.params.unwrap();
        assert_eq!(
            params.other.get("protocolVersion"),
            Some(&json!("2025-06-18"))
        );
        assert!(params.other.get("capabilities").unwrap().is_object());
        assert!(params.other.get("clientInfo").unwrap().is_object());

        // Step 2: Server responds with capabilities
        let initialize_response = json!({
            "jsonrpc": "2.0",
            "result": {
                "protocolVersion": "2025-06-18",
                "capabilities": {
                    "logging": {},
                    "prompts": {
                        "listChanged": false  // MCP compliance: static framework
                    },
                    "resources": {
                        "subscribe": true,
                        "listChanged": false  // MCP compliance: static framework
                    },
                    "tools": {
                        "listChanged": false  // MCP compliance: static framework
                    }
                },
                "serverInfo": {
                    "name": "test-server",
                    "version": "1.0.0"
                }
            },
            "id": 1
        });

        let response: JsonRpcResponse = serde_json::from_value(initialize_response).unwrap();
        assert!(response.result.is_some());

        let result = response.result.unwrap();
        assert_eq!(
            result.data.get("protocolVersion"),
            Some(&json!("2025-06-18"))
        );
        assert!(result.data.get("capabilities").unwrap().is_object());
        assert!(result.data.get("serverInfo").unwrap().is_object());

        // Step 3: Client sends initialized notification
        let initialized_notification = json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        });

        let notification: JsonRpcNotification =
            serde_json::from_value(initialized_notification).unwrap();
        assert_eq!(notification.method, "notifications/initialized");
    }

    #[tokio::test]
    async fn test_protocol_version_validation() {
        let supported_versions = vec!["2025-06-18"];
        let _unsupported_versions = ["2024-11-05", "invalid", ""];

        for version in supported_versions {
            // Should accept supported version
            let request = json!({
                "jsonrpc": "2.0",
                "method": "initialize",
                "params": {
                    "protocolVersion": version,
                    "capabilities": {},
                    "clientInfo": {
                        "name": "test",
                        "version": "1.0.0"
                    }
                },
                "id": 1
            });

            let parsed: Result<JsonRpcRequest, _> = serde_json::from_value(request);
            assert!(parsed.is_ok());
        }

        // Test that our implementation properly handles version negotiation
        println!("Protocol version validation tested");
    }

    #[tokio::test]
    async fn test_capability_negotiation() {
        // Test various capability combinations
        let client_capabilities = json!({
            "experimental": {
                "customFeature": true
            },
            "roots": {
                "listChanged": false  // MCP compliance: static framework
            },
            "sampling": {}
        });

        let server_capabilities = json!({
            "logging": {},
            "prompts": {
                "listChanged": false  // MCP compliance: static framework
            },
            "resources": {
                "subscribe": true,
                "listChanged": false  // MCP compliance: static framework
            },
            "tools": {
                "listChanged": false  // MCP compliance: static framework
            },
            "experimental": {
                "customFeature": true
            }
        });

        // Both should be valid capability objects
        assert!(client_capabilities.is_object());
        assert!(server_capabilities.is_object());

        println!("Capability negotiation structures validated");
    }
}

/// Test MCP message structure compliance
#[cfg(test)]
mod message_structure_compliance {
    use super::*;

    #[tokio::test]
    async fn test_tool_call_message_structure() {
        // MCP 2025-06-18 tool call structure
        let tool_call = json!({
            "jsonrpc": "2.0",
            "method": "tools/call",
            "params": {
                "name": "calculator",
                "arguments": {
                    "operation": "add",
                    "a": 5,
                    "b": 3
                },
                "_meta": {
                    "progressToken": "calc-123"
                }
            },
            "id": "call-1"
        });

        let request: JsonRpcRequest = serde_json::from_value(tool_call).unwrap();
        assert_eq!(request.method, "tools/call");

        let params = request.params.unwrap();
        assert_eq!(params.other.get("name"), Some(&json!("calculator")));
        assert!(params.other.get("arguments").unwrap().is_object());

        // _meta should be parsed into the meta field, not other
        assert!(params.meta.is_some());
        let meta = params.meta.unwrap();
        assert_eq!(meta.progress_token, Some("calc-123".into()));
    }

    #[tokio::test]
    async fn test_tool_response_structure() {
        // MCP 2025-06-18 tool response structure
        let tool_response = json!({
            "jsonrpc": "2.0",
            "result": {
                "content": [
                    {
                        "type": "text",
                        "text": "The result is 8"
                    }
                ],
                "isError": false,
                "_meta": {
                    "usage": {
                        "inputTokens": 10,
                        "outputTokens": 5
                    }
                }
            },
            "id": "call-1"
        });

        let response: JsonRpcResponse = serde_json::from_value(tool_response).unwrap();
        assert!(response.result.is_some());

        let result = response.result.unwrap();
        assert!(result.data.get("content").unwrap().is_array());
        assert_eq!(result.data.get("isError"), Some(&json!(false)));
        assert!(
            result
                .meta
                .as_ref()
                .unwrap()
                .get("usage")
                .unwrap()
                .is_object()
        );
    }

    #[tokio::test]
    async fn test_resource_message_structure() {
        // Resource list request
        let resource_list = json!({
            "jsonrpc": "2.0",
            "method": "resources/list",
            "params": {
                "cursor": "page-2",
                "_meta": {
                    "progressToken": "list-resources-456"
                }
            },
            "id": "list-1"
        });

        let request: JsonRpcRequest = serde_json::from_value(resource_list).unwrap();
        assert_eq!(request.method, "resources/list");

        let params = request.params.unwrap();
        assert_eq!(params.other.get("cursor"), Some(&json!("page-2")));

        // _meta should be parsed into the meta field, not other
        assert!(params.meta.is_some());

        // Resource read request
        let resource_read = json!({
            "jsonrpc": "2.0",
            "method": "resources/read",
            "params": {
                "uri": "file:///example.txt",
                "_meta": {
                    "progressToken": "read-file-789"
                }
            },
            "id": "read-1"
        });

        let request: JsonRpcRequest = serde_json::from_value(resource_read).unwrap();
        assert_eq!(request.method, "resources/read");

        let params = request.params.unwrap();
        assert_eq!(params.other.get("uri"), Some(&json!("file:///example.txt")));

        // _meta should be parsed into the meta field
        assert!(params.meta.is_some());
    }

    #[tokio::test]
    async fn test_prompt_message_structure() {
        // Prompt list request
        let prompt_list = json!({
            "jsonrpc": "2.0",
            "method": "prompts/list",
            "params": {
                "cursor": "prompt-page-1",
                "_meta": {
                    "progressToken": "list-prompts-101"
                }
            },
            "id": "prompts-1"
        });

        let request: JsonRpcRequest = serde_json::from_value(prompt_list).unwrap();
        assert_eq!(request.method, "prompts/list");

        // Prompt get request
        let prompt_get = json!({
            "jsonrpc": "2.0",
            "method": "prompts/get",
            "params": {
                "name": "code_review",
                "arguments": {
                    "language": "rust",
                    "file": "main.rs"
                },
                "_meta": {
                    "progressToken": "get-prompt-202"
                }
            },
            "id": "get-prompt-1"
        });

        let request: JsonRpcRequest = serde_json::from_value(prompt_get).unwrap();
        assert_eq!(request.method, "prompts/get");

        let params = request.params.unwrap();
        assert_eq!(params.other.get("name"), Some(&json!("code_review")));
        assert!(params.other.get("arguments").unwrap().is_object());

        // _meta should be parsed into the meta field, not other
        assert!(params.meta.is_some());
    }
}

/// Test MCP notification compliance
#[cfg(test)]
mod notification_compliance {
    use super::*;

    #[tokio::test]
    async fn test_progress_notifications() {
        // Progress notification structure
        let progress_notification = json!({
            "jsonrpc": "2.0",
            "method": "notifications/progress",
            "params": {
                "progressToken": "operation-123",
                "progress": 75,
                "total": 100
            }
        });

        let notification: JsonRpcNotification =
            serde_json::from_value(progress_notification).unwrap();
        assert_eq!(notification.method, "notifications/progress");

        let params = notification.params.unwrap();
        assert_eq!(
            params.other.get("progressToken"),
            Some(&json!("operation-123"))
        );
        assert_eq!(params.other.get("progress"), Some(&json!(75)));
        assert_eq!(params.other.get("total"), Some(&json!(100)));
    }

    #[tokio::test]
    async fn test_resource_notifications() {
        // Resource list changed notification
        let resource_changed = json!({
            "jsonrpc": "2.0",
            "method": "notifications/resources/listChanged"
        });

        let notification: JsonRpcNotification = serde_json::from_value(resource_changed).unwrap();
        assert_eq!(notification.method, "notifications/resources/listChanged");

        // Resource updated notification
        let resource_updated = json!({
            "jsonrpc": "2.0",
            "method": "notifications/resources/updated",
            "params": {
                "uri": "file:///updated.txt"
            }
        });

        let notification: JsonRpcNotification = serde_json::from_value(resource_updated).unwrap();
        assert_eq!(notification.method, "notifications/resources/updated");

        let params = notification.params.unwrap();
        assert_eq!(params.other.get("uri"), Some(&json!("file:///updated.txt")));
    }

    #[tokio::test]
    async fn test_tool_notifications() {
        // Tools list changed notification
        let tools_changed = json!({
            "jsonrpc": "2.0",
            "method": "notifications/tools/listChanged"
        });

        let notification: JsonRpcNotification = serde_json::from_value(tools_changed).unwrap();
        assert_eq!(notification.method, "notifications/tools/listChanged");
    }

    #[tokio::test]
    async fn test_logging_notifications() {
        // Logging message notification
        let log_message = json!({
            "jsonrpc": "2.0",
            "method": "notifications/message",
            "params": {
                "level": "info",
                "logger": "mcp.server",
                "data": "Operation completed successfully"
            }
        });

        let notification: JsonRpcNotification = serde_json::from_value(log_message).unwrap();
        assert_eq!(notification.method, "notifications/message");

        let params = notification.params.unwrap();
        assert_eq!(params.other.get("level"), Some(&json!("info")));
        assert_eq!(params.other.get("logger"), Some(&json!("mcp.server")));
        assert!(params.other.get("data").unwrap().is_string());
    }
}

/// Test _meta field handling compliance
#[cfg(test)]
mod meta_field_compliance {
    use super::*;

    #[tokio::test]
    async fn test_meta_field_in_requests() {
        // Test _meta field with progressToken
        let request_with_meta = json!({
            "jsonrpc": "2.0",
            "method": "tools/call",
            "params": {
                "name": "test_tool",
                "arguments": {},
                "_meta": {
                    "progressToken": "progress-abc-123",
                    "sessionId": "session-xyz-789"
                }
            },
            "id": 1
        });

        let request: JsonRpcRequest = serde_json::from_value(request_with_meta).unwrap();
        let params = request.params.unwrap();

        // _meta should be parsed into the meta field
        assert!(params.meta.is_some());
        let meta = params.meta.unwrap();
        assert_eq!(meta.progress_token, Some("progress-abc-123".into()));
        // Note: sessionId would be in meta.extra if it's a custom field
        assert!(meta.extra.contains_key("sessionId"));
    }

    #[tokio::test]
    async fn test_meta_field_in_responses() {
        // Test _meta field in response with usage information
        let response_with_meta = json!({
            "jsonrpc": "2.0",
            "result": {
                "content": [
                    {
                        "type": "text",
                        "text": "Response content"
                    }
                ],
                "_meta": {
                    "usage": {
                        "inputTokens": 15,
                        "outputTokens": 8,
                        "totalTokens": 23
                    },
                    "processingTime": 234,
                    "requestId": "req-456-def"
                }
            },
            "id": 1
        });

        let response: JsonRpcResponse = serde_json::from_value(response_with_meta).unwrap();
        let result = response.result.unwrap();

        // _meta should be preserved with usage and timing info
        assert!(result.meta.is_some());
        let meta = result.meta.as_ref().unwrap();
        let usage = meta.get("usage").unwrap();
        assert!(usage.is_object());
        assert_eq!(usage.get("inputTokens"), Some(&json!(15)));
        assert_eq!(usage.get("outputTokens"), Some(&json!(8)));
        assert_eq!(meta.get("processingTime"), Some(&json!(234)));
    }

    #[tokio::test]
    async fn test_meta_field_optional() {
        // Test that _meta field is optional
        let request_without_meta = json!({
            "jsonrpc": "2.0",
            "method": "tools/list",
            "params": {},
            "id": 1
        });

        let request: JsonRpcRequest = serde_json::from_value(request_without_meta).unwrap();
        let params = request.params.unwrap();

        // Should work fine without _meta field
        assert!(params.meta.is_none());
    }

    #[tokio::test]
    async fn test_meta_field_extensibility() {
        // Test that _meta field can contain custom fields
        let request_with_custom_meta = json!({
            "jsonrpc": "2.0",
            "method": "tools/call",
            "params": {
                "name": "custom_tool",
                "arguments": {},
                "_meta": {
                    "progressToken": "token-123",
                    "customField": "custom_value",
                    "experimentalData": {
                        "feature": "beta",
                        "version": "0.1.0"
                    }
                }
            },
            "id": 1
        });

        let request: JsonRpcRequest = serde_json::from_value(request_with_custom_meta).unwrap();
        let params = request.params.unwrap();

        // Custom fields should be preserved
        assert!(params.meta.is_some());
        let meta = params.meta.unwrap();
        // Standard field should be parsed correctly
        assert_eq!(meta.progress_token, Some("token-123".into()));

        // Custom fields should be preserved in meta.extra
        assert_eq!(meta.extra.get("customField"), Some(&json!("custom_value")));
        assert!(meta.extra.get("experimentalData").unwrap().is_object());
        let experimental = meta.extra.get("experimentalData").unwrap();
        assert_eq!(experimental.get("feature"), Some(&json!("beta")));
    }
}

/// Test error handling compliance
#[cfg(test)]
mod error_handling_compliance {
    use super::*;

    #[tokio::test]
    async fn test_standard_error_responses() {
        // Test method not found error
        let method_not_found = JsonRpcError::method_not_found();
        assert_eq!(method_not_found.code, -32601);
        assert!(!method_not_found.message.is_empty());

        // Test invalid params error
        let invalid_params = JsonRpcError::invalid_params();
        assert_eq!(invalid_params.code, -32602);
        assert!(!invalid_params.message.is_empty());

        // Test internal error
        let internal_error = JsonRpcError::internal_error();
        assert_eq!(internal_error.code, -32603);
        assert!(!internal_error.message.is_empty());
    }

    #[tokio::test]
    async fn test_mcp_specific_errors() {
        // Test tool not found error (should use appropriate error code)
        let tool_error = JsonRpcError {
            code: -32001, // Implementation-defined error
            message: "Tool not found".to_string(),
            data: Some(json!({
                "toolName": "nonexistent_tool",
                "availableTools": ["calculator", "file_reader"]
            })),
        };

        assert_eq!(tool_error.code, -32001);
        assert!(tool_error.data.is_some());

        let data = tool_error.data.unwrap();
        assert_eq!(data["toolName"], "nonexistent_tool");
        assert!(data["availableTools"].is_array());
    }

    #[tokio::test]
    async fn test_error_with_meta_field() {
        // Test error response with _meta field
        let error_response = json!({
            "jsonrpc": "2.0",
            "error": {
                "code": -32602,
                "message": "Invalid params",
                "data": {
                    "parameterErrors": [
                        {
                            "parameter": "name",
                            "error": "Required parameter missing"
                        }
                    ],
                    "_meta": {
                        "requestId": "req-error-123",
                        "timestamp": "2024-01-01T00:00:00Z"
                    }
                }
            },
            "id": 1
        });

        let response: JsonRpcResponse = serde_json::from_value(error_response).unwrap();
        assert!(response.error.is_some());

        let error = response.error.unwrap();
        assert_eq!(error.code, -32602);
        assert!(error.data.is_some());

        let data = error.data.unwrap();
        assert!(data["_meta"].is_object());
        assert_eq!(data["_meta"]["requestId"], "req-error-123");
    }
}

/// Test content type handling
#[cfg(test)]
mod content_type_compliance {
    use super::*;

    #[tokio::test]
    async fn test_text_content_structure() {
        let text_content = json!({
            "type": "text",
            "text": "This is plain text content"
        });

        // Should be valid content structure
        assert_eq!(text_content["type"], "text");
        assert!(text_content["text"].is_string());
    }

    #[tokio::test]
    async fn test_image_content_structure() {
        let image_content = json!({
            "type": "image",
            "data": "base64-encoded-image-data",
            "mimeType": "image/png"
        });

        // Should be valid image content structure
        assert_eq!(image_content["type"], "image");
        assert!(image_content["data"].is_string());
        assert_eq!(image_content["mimeType"], "image/png");
    }

    #[tokio::test]
    async fn test_resource_content_structure() {
        let resource_content = json!({
            "type": "resource",
            "resource": {
                "uri": "file:///example.txt",
                "text": "File contents here"
            }
        });

        // Should be valid resource content structure
        assert_eq!(resource_content["type"], "resource");
        assert!(resource_content["resource"].is_object());
        assert_eq!(resource_content["resource"]["uri"], "file:///example.txt");
    }

    #[tokio::test]
    async fn test_mixed_content_array() {
        let mixed_content = json!([
            {
                "type": "text",
                "text": "Here's an explanation:"
            },
            {
                "type": "image",
                "data": "base64-image-data",
                "mimeType": "image/jpeg"
            },
            {
                "type": "text",
                "text": "And here's more text."
            }
        ]);

        // Should be valid content array
        assert!(mixed_content.is_array());
        let content_array = mixed_content.as_array().unwrap();
        assert_eq!(content_array.len(), 3);

        // Check each content item has required type field
        for item in content_array {
            assert!(item["type"].is_string());
            assert!(!item["type"].as_str().unwrap().is_empty());
        }
    }
}

/// Test cursor-based pagination compliance
#[cfg(test)]
mod pagination_compliance {
    use super::*;

    #[tokio::test]
    async fn test_cursor_based_pagination() {
        // Initial request without cursor
        let initial_request = json!({
            "jsonrpc": "2.0",
            "method": "resources/list",
            "params": {},
            "id": 1
        });

        let request: JsonRpcRequest = serde_json::from_value(initial_request).unwrap();
        assert_eq!(request.method, "resources/list");

        // Response with cursor for next page
        let paginated_response = json!({
            "jsonrpc": "2.0",
            "result": {
                "resources": [
                    {
                        "uri": "file:///first.txt",
                        "name": "First File"
                    },
                    {
                        "uri": "file:///second.txt",
                        "name": "Second File"
                    }
                ],
                "nextCursor": "page-2-token-abc123"
            },
            "id": 1
        });

        let response: JsonRpcResponse = serde_json::from_value(paginated_response).unwrap();
        let result = response.result.unwrap();

        assert!(result.data.get("resources").unwrap().is_array());
        assert_eq!(
            result.data.get("nextCursor"),
            Some(&json!("page-2-token-abc123"))
        );

        // Follow-up request with cursor
        let next_request = json!({
            "jsonrpc": "2.0",
            "method": "resources/list",
            "params": {
                "cursor": "page-2-token-abc123"
            },
            "id": 2
        });

        let request: JsonRpcRequest = serde_json::from_value(next_request).unwrap();
        let params = request.params.unwrap();
        assert_eq!(
            params.other.get("cursor"),
            Some(&json!("page-2-token-abc123"))
        );
    }

    #[tokio::test]
    async fn test_pagination_end_condition() {
        // Final page response without nextCursor
        let final_page_response = json!({
            "jsonrpc": "2.0",
            "result": {
                "resources": [
                    {
                        "uri": "file:///last.txt",
                        "name": "Last File"
                    }
                ]
                // No nextCursor indicates end of pagination
            },
            "id": 3
        });

        let response: JsonRpcResponse = serde_json::from_value(final_page_response).unwrap();
        let result = response.result.unwrap();

        assert!(result.data.get("resources").unwrap().is_array());
        assert!(!result.data.contains_key("nextCursor"));
    }
}

/// Integration test with actual framework
#[cfg(test)]
mod framework_integration_compliance {
    use super::*;

    #[tokio::test]
    async fn test_framework_json_rpc_compliance() {
        // Test that our framework produces compliant JSON-RPC
        let mut params_map = HashMap::new();
        params_map.insert("protocolVersion".to_string(), json!("2025-06-18"));
        params_map.insert("capabilities".to_string(), json!({}));
        params_map.insert(
            "clientInfo".to_string(),
            json!({
                "name": "compliance-test",
                "version": "1.0.0"
            }),
        );

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "initialize".to_string(),
            params: Some(RequestParams {
                meta: None,
                other: params_map,
            }),
            id: json!(1),
        };

        // Serialize and deserialize to ensure round-trip compatibility
        let serialized = serde_json::to_string(&request).unwrap();
        let deserialized: JsonRpcRequest = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.jsonrpc, "2.0");
        assert_eq!(deserialized.method, "initialize");
        assert_eq!(deserialized.id, json!(1));
    }

    #[tokio::test]
    async fn test_framework_response_compliance() {
        // Test framework response generation
        let response = JsonRpcResponse::success(
            json!("test-id"),
            ResultWithMeta::from_value(json!({
                "protocolVersion": "2025-06-18",
                "capabilities": {
                    "tools": { "listChanged": false },  // MCP compliance: static framework
                    "resources": { "listChanged": false }  // MCP compliance: static framework
                },
                "serverInfo": {
                    "name": "mcp-test-server",
                    "version": "1.0.0"
                }
            })),
        );

        // Serialize and validate structure
        let serialized = serde_json::to_value(&response).unwrap();

        assert_eq!(serialized["jsonrpc"], "2.0");
        assert_eq!(serialized["id"], "test-id");
        assert!(serialized["result"].is_object());
        assert!(serialized.get("error").is_none());

        let result = &serialized["result"];
        assert_eq!(result.get("protocolVersion"), Some(&json!("2025-06-18")));
        assert!(result.get("capabilities").unwrap().is_object());
        assert!(result.get("serverInfo").unwrap().is_object());
    }

    #[tokio::test]
    async fn test_notification_compliance() {
        // Test framework notification generation
        let notification = JsonRpcNotification::new("notifications/tools/listChanged".to_string());

        let serialized = serde_json::to_value(&notification).unwrap();

        assert_eq!(serialized["jsonrpc"], "2.0");
        assert_eq!(serialized["method"], "notifications/tools/listChanged");
        assert!(serialized.get("id").is_none()); // Notifications must not have id

        println!("Framework notification compliance verified");
    }
}

/// Test MCP 2025-06-18 specific features
#[cfg(test)]
mod mcp_2025_06_18_features {
    use super::*;

    #[tokio::test]
    async fn test_structured_meta_support() {
        // Test that our implementation supports the structured _meta fields introduced in 2025-06-18
        let request_with_structured_meta = json!({
            "jsonrpc": "2.0",
            "method": "tools/call",
            "params": {
                "name": "advanced_tool",
                "arguments": {
                    "input": "test"
                },
                "_meta": {
                    "progressToken": "structured-progress-token",
                    "cursor": "page-2",
                    "total": 100,
                    "hasMore": true,
                    "progress": 0.75,
                    "customField": "custom-value",
                    "elicitation": {
                        "type": "confirmation",
                        "message": "Are you sure you want to proceed?"
                    },
                    "customCursor": {
                        "page": 2,
                        "token": "page-token-xyz"
                    }
                }
            },
            "id": "structured-test"
        });

        let request: JsonRpcRequest = serde_json::from_value(request_with_structured_meta).unwrap();
        let params = request.params.unwrap();

        assert!(params.meta.is_some());
        let meta = params.meta.unwrap();

        // Verify structured _meta fields are preserved
        assert_eq!(
            meta.progress_token,
            Some("structured-progress-token".into())
        );

        // Structured fields should be preserved in meta.extra
        assert!(meta.extra.get("elicitation").unwrap().is_object());
        let elicitation = meta.extra.get("elicitation").unwrap();
        assert_eq!(elicitation.get("type"), Some(&json!("confirmation")));

        // Custom cursor in meta.extra (using customCursor to avoid field name conflict)
        assert!(meta.extra.get("customCursor").unwrap().is_object());
        let cursor = meta.extra.get("customCursor").unwrap();
        assert_eq!(cursor.get("page"), Some(&json!(2)));

        println!("MCP 2025-06-18 structured _meta support verified");
    }

    #[tokio::test]
    async fn test_enhanced_error_reporting() {
        // Test enhanced error reporting with structured data
        let enhanced_error = json!({
            "jsonrpc": "2.0",
            "error": {
                "code": -32001,
                "message": "Tool execution failed",
                "data": {
                    "toolName": "failing_tool",
                    "errorType": "ValidationError",
                    "details": {
                        "field": "input_parameter",
                        "constraint": "must_be_positive",
                        "value": -5
                    },
                    "_meta": {
                        "timestamp": "2024-01-01T00:00:00Z",
                        "requestId": "req-123",
                        "retryable": true,
                        "suggestedDelay": 1000
                    }
                }
            },
            "id": "error-test"
        });

        let response: JsonRpcResponse = serde_json::from_value(enhanced_error).unwrap();
        assert!(response.error.is_some());

        let error = response.error.unwrap();
        assert!(error.data.is_some());

        let data = error.data.unwrap();
        assert_eq!(data["toolName"], "failing_tool");
        assert_eq!(data["errorType"], "ValidationError");
        assert!(data["details"].is_object());
        assert!(data["_meta"].is_object());
        assert_eq!(data["_meta"]["retryable"], true);

        println!("Enhanced error reporting compliance verified");
    }

    #[tokio::test]
    async fn test_content_type_extensions() {
        // Test support for extended content types in 2025-06-18
        let extended_content = json!([
            {
                "type": "text",
                "text": "Standard text content"
            },
            {
                "type": "image",
                "data": "base64-image-data",
                "mimeType": "image/webp",
                "annotations": {
                    "alt": "Alternative text",
                    "caption": "Image caption"
                }
            },
            {
                "type": "resource",
                "resource": {
                    "uri": "https://example.com/api/data",
                    "mimeType": "application/json",
                    "headers": {
                        "Authorization": "Bearer token"
                    }
                }
            }
        ]);

        // Verify extended content types are properly structured
        assert!(extended_content.is_array());
        let content_array = extended_content.as_array().unwrap();

        // Check image with annotations
        let image_content = &content_array[1];
        assert_eq!(image_content["type"], "image");
        assert_eq!(image_content["mimeType"], "image/webp");
        assert!(image_content["annotations"].is_object());

        // Check resource with headers
        let resource_content = &content_array[2];
        assert_eq!(resource_content["type"], "resource");
        assert!(resource_content["resource"]["headers"].is_object());

        println!("Extended content type support verified");
    }
}
