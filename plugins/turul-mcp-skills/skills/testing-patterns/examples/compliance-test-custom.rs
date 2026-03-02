// turul-mcp-server v0.3
// Writing custom compliance assertions for MCP specification conformance
// These patterns verify that your server correctly implements MCP protocol rules.

use serde_json::json;
use mcp_e2e_shared::e2e_utils::{McpTestClient, TestFixtures, TestServerManager};

#[tokio::test]
async fn test_protocol_version_negotiation() {
    let server = TestServerManager::start("tools-test-server")
        .await
        .expect("Failed to start server");

    let mut client = McpTestClient::new(server.port());

    // Server should negotiate to 2025-11-25 (default)
    let result = client.initialize().await.expect("Failed to initialize");
    let protocol_version = result["result"]["protocolVersion"]
        .as_str()
        .expect("Missing protocolVersion");

    assert_eq!(
        protocol_version, "2025-11-25",
        "Server should negotiate to MCP 2025-11-25"
    );
}

#[tokio::test]
async fn test_strict_lifecycle_enforcement() {
    let server = TestServerManager::start("tools-test-server")
        .await
        .expect("Failed to start server");

    let mut client = McpTestClient::new(server.port());

    // Initialize but do NOT send initialized notification
    client.initialize().await.expect("Failed to initialize");

    // Attempt to call a tool before completing handshake
    let result = client
        .call_tool("calculator_add", json!({"a": 1.0, "b": 2.0}))
        .await
        .expect("Request should succeed at HTTP level");

    // Should get error -32031 (SessionError: not yet initialized)
    assert!(result.contains_key("error"), "Should get error before initialized");
    let error_code = result["error"]["code"].as_i64().unwrap();
    assert_eq!(
        error_code, -32031,
        "Expected SessionError code -32031, got {}",
        error_code
    );
}

#[tokio::test]
async fn test_capability_truthfulness() {
    let server = TestServerManager::start("tools-test-server")
        .await
        .expect("Failed to start server");

    let mut client = McpTestClient::new(server.port());
    let init_result = client.initialize().await.expect("Failed to initialize");
    client
        .send_initialized_notification()
        .await
        .expect("Failed to send initialized");

    let capabilities = init_result["result"]["capabilities"]
        .as_object()
        .expect("Missing capabilities");

    // If server advertises tools capability, it must respond to tools/list
    if capabilities.contains_key("tools") {
        let tools_result = client.list_tools().await.expect("tools/list should work");
        assert!(
            tools_result.contains_key("result"),
            "tools/list should return a valid result when tools capability is advertised"
        );
    }

    // If server does NOT advertise prompts, prompts/list should still work
    // (returning empty) but should NOT be advertised in capabilities
    if !capabilities.contains_key("prompts") {
        // Server should still respond but may return empty or error
        let prompts_result = client.list_prompts().await;
        // The key test: no capability → server has no obligation to support it
        assert!(
            prompts_result.is_ok(),
            "Server should handle prompts/list gracefully even without capability"
        );
    }
}

#[tokio::test]
async fn test_structured_content_consistency() {
    let server = TestServerManager::start("tools-test-server")
        .await
        .expect("Failed to start server");

    let mut client = McpTestClient::new(server.port());
    client.initialize().await.expect("Failed to initialize");
    client
        .send_initialized_notification()
        .await
        .expect("Failed to send initialized");

    // Get tool list to check which tools have outputSchema
    let tools_result = client.list_tools().await.expect("Failed to list tools");
    let tools = TestFixtures::extract_tools_list(&tools_result).unwrap_or_default();

    for tool in &tools {
        let tool_name = tool["name"].as_str().unwrap();
        let has_output_schema = tool.get("outputSchema").is_some();

        // Call the tool with some basic args (tool-specific)
        let call_result = client
            .call_tool(tool_name, json!({"a": 1.0, "b": 2.0}))
            .await;

        if let Ok(result) = call_result {
            if result.contains_key("result") {
                let has_structured = result["result"].get("structuredContent").is_some();

                if has_output_schema {
                    // MCP spec: tools with outputSchema MUST provide structuredContent
                    assert!(
                        has_structured,
                        "Tool '{}' has outputSchema but missing structuredContent",
                        tool_name
                    );
                }
            }
        }
    }
}
