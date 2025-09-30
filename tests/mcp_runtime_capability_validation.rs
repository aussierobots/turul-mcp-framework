//! MCP Runtime Capability Validation Tests
//!
//! These tests perform actual HTTP calls to running servers to validate
//! that capabilities are truthfully advertised at runtime, not just in code.

use serde_json::{Value, json};
use std::time::Duration;
use turul_mcp_derive::{McpPrompt, mcp_tool};
use turul_mcp_server::prelude::*;

/// Simple test tool for capability validation
#[mcp_tool(
    name = "test_tool",
    description = "Test tool for capability validation"
)]
async fn test_tool() -> McpResult<String> {
    Ok("test response".to_string())
}

/// Simple test prompt for capability validation
#[derive(McpPrompt)]
#[prompt(
    name = "test_prompt",
    description = "Test prompt for capability validation"
)]
struct TestPrompt {
    #[argument(name = "input", description = "Test input", required = true)]
    input: String,
}

#[async_trait]
impl McpPrompt for TestPrompt {
    async fn render(&self, _args: Option<HashMap<String, Value>>) -> McpResult<Vec<PromptMessage>> {
        Ok(vec![PromptMessage::user_text(format!(
            "Test prompt with input: {}",
            self.input
        ))])
    }
}

/// Test that a server with tools correctly advertises listChanged: false
#[tokio::test]
async fn test_tools_capability_truthfulness() {
    let port = portpicker::pick_unused_port().expect("No available port");

    // Start server with tools
    let server_handle = tokio::spawn(async move {
        let server = McpServer::builder()
            .name("capability-test-server")
            .version("1.0.0")
            .tool_fn(test_tool)
            .bind_address(format!("127.0.0.1:{}", port).parse().unwrap())
            .build()
            .expect("Failed to build server");

        server.run().await.ok();
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Call initialize endpoint
    let client = reqwest::Client::new();
    let response = client
        .post(format!("http://127.0.0.1:{}/mcp", port))
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {
                    "name": "test-client",
                    "version": "1.0.0"
                }
            }
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert!(response.status().is_success(), "Initialize request failed");

    let body: Value = response.json().await.expect("Failed to parse JSON");

    // Validate response structure
    assert_eq!(body["jsonrpc"], "2.0");
    assert_eq!(body["id"], 1);

    // Check capabilities truthfulness
    let capabilities = &body["result"]["capabilities"];

    // Tools capability should be listChanged: false for static framework
    assert_eq!(
        capabilities["tools"]["listChanged"], false,
        "tools.listChanged should be false for static framework, got: {}",
        capabilities["tools"]["listChanged"]
    );

    server_handle.abort();
}

/// Test that a server with prompts correctly advertises listChanged: false
#[tokio::test]
async fn test_prompts_capability_truthfulness() {
    let port = portpicker::pick_unused_port().expect("No available port");

    // Start server with prompts
    let server_handle = tokio::spawn(async move {
        let server = McpServer::builder()
            .name("prompts-capability-test-server")
            .version("1.0.0")
            .prompt(TestPrompt {
                input: "test".to_string(),
            })
            .bind_address(format!("127.0.0.1:{}", port).parse().unwrap())
            .build()
            .expect("Failed to build server");

        server.run().await.ok();
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Call initialize endpoint
    let client = reqwest::Client::new();
    let response = client
        .post(format!("http://127.0.0.1:{}/mcp", port))
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {
                    "name": "prompts-test-client",
                    "version": "1.0.0"
                }
            }
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert!(response.status().is_success(), "Initialize request failed");

    let body: Value = response.json().await.expect("Failed to parse JSON");

    // Validate response structure
    assert_eq!(body["jsonrpc"], "2.0");
    assert_eq!(body["id"], 1);

    // Check capabilities truthfulness
    let capabilities = &body["result"]["capabilities"];

    // Prompts capability should be listChanged: false for static framework
    assert_eq!(
        capabilities["prompts"]["listChanged"], false,
        "prompts.listChanged should be false for static framework, got: {}",
        capabilities["prompts"]["listChanged"]
    );

    server_handle.abort();
}

/// Test server with no components advertises no capabilities
#[tokio::test]
async fn test_empty_server_capabilities() {
    let port = portpicker::pick_unused_port().expect("No available port");

    // Start server with no components
    let server_handle = tokio::spawn(async move {
        let server = McpServer::builder()
            .name("empty-server")
            .version("1.0.0")
            .bind_address(format!("127.0.0.1:{}", port).parse().unwrap())
            .build()
            .expect("Failed to build server");

        server.run().await.ok();
    });

    tokio::time::sleep(Duration::from_millis(500)).await;

    let client = reqwest::Client::new();
    let response = client
        .post(format!("http://127.0.0.1:{}/mcp", port))
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {
                    "name": "test-client",
                    "version": "1.0.0"
                }
            }
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert!(response.status().is_success());

    let body: Value = response.json().await.expect("Failed to parse JSON");
    let capabilities = &body["result"]["capabilities"];

    // Empty server should not advertise capabilities it doesn't have
    assert!(
        capabilities.get("tools").is_none() || capabilities["tools"].is_null(),
        "Empty server should not advertise tools capability"
    );
    assert!(
        capabilities.get("prompts").is_none() || capabilities["prompts"].is_null(),
        "Empty server should not advertise prompts capability"
    );
    assert!(
        capabilities.get("resources").is_none() || capabilities["resources"].is_null(),
        "Empty server should not advertise resources capability"
    );

    server_handle.abort();
}

/// Test JSON-RPC protocol compliance
#[tokio::test]
async fn test_json_rpc_protocol_compliance() {
    let port = portpicker::pick_unused_port().expect("No available port");

    let server_handle = tokio::spawn(async move {
        let server = McpServer::builder()
            .name("protocol-test-server")
            .version("1.0.0")
            .tool_fn(test_tool)
            .bind_address(format!("127.0.0.1:{}", port).parse().unwrap())
            .build()
            .expect("Failed to build server");

        server.run().await.ok();
    });

    tokio::time::sleep(Duration::from_millis(500)).await;

    let client = reqwest::Client::new();
    let response = client
        .post(format!("http://127.0.0.1:{}/mcp", port))
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 42,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {
                    "name": "protocol-test",
                    "version": "1.0.0"
                }
            }
        }))
        .send()
        .await
        .expect("Failed to send request");

    let body: Value = response.json().await.expect("Failed to parse JSON");

    // Validate JSON-RPC 2.0 compliance
    assert_eq!(body["jsonrpc"], "2.0", "Must be JSON-RPC 2.0");
    assert_eq!(body["id"], 42, "ID must match request");
    assert!(body["result"].is_object(), "Must have result object");
    assert!(
        body["error"].is_null() || body.get("error").is_none(),
        "Should not have error on success"
    );

    // Validate MCP protocol version
    assert_eq!(
        body["result"]["protocolVersion"], "2025-06-18",
        "Must support MCP 2025-06-18"
    );

    // Validate server info structure
    let server_info = &body["result"]["serverInfo"];
    assert!(
        server_info["name"].is_string(),
        "Server name must be string"
    );
    assert!(
        server_info["version"].is_string(),
        "Server version must be string"
    );

    server_handle.abort();
}

#[cfg(test)]
mod integration {
    use super::*;

    /// Integration test to validate overall MCP compliance
    #[tokio::test]
    async fn test_full_mcp_compliance_integration() {
        let port = portpicker::pick_unused_port().expect("No available port");

        let server_handle = tokio::spawn(async move {
            let server = McpServer::builder()
                .name("compliance-test-server")
                .version("1.0.0")
                .tool_fn(test_tool)
                .bind_address(format!("127.0.0.1:{}", port).parse().unwrap())
                .build()
                .expect("Failed to build server");

            server.run().await.ok();
        });

        tokio::time::sleep(Duration::from_millis(500)).await;

        let client = reqwest::Client::new();
        let response = client
            .post(format!("http://127.0.0.1:{}/mcp", port))
            .json(&json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2025-06-18",
                    "capabilities": {},
                    "clientInfo": {
                        "name": "compliance-test",
                        "version": "1.0.0"
                    }
                }
            }))
            .send()
            .await
            .expect("Failed to send request");

        assert!(response.status().is_success());
        let body: Value = response.json().await.expect("Failed to parse JSON");

        // Comprehensive compliance check
        assert_eq!(body["jsonrpc"], "2.0");
        assert_eq!(body["result"]["protocolVersion"], "2025-06-18");

        let capabilities = &body["result"]["capabilities"];

        // Static framework compliance: all listChanged must be false
        if let Some(tools) = capabilities.get("tools") {
            assert_eq!(
                tools["listChanged"], false,
                "Static framework: tools.listChanged must be false"
            );
        }

        if let Some(prompts) = capabilities.get("prompts") {
            assert_eq!(
                prompts["listChanged"], false,
                "Static framework: prompts.listChanged must be false"
            );
        }

        if let Some(resources) = capabilities.get("resources") {
            assert_eq!(
                resources["listChanged"], false,
                "Static framework: resources.listChanged must be false"
            );
            // Resources subscribe should be false until implemented
            assert_eq!(
                resources["subscribe"], false,
                "resources.subscribe must be false until implemented"
            );
        }

        server_handle.abort();
    }
}
