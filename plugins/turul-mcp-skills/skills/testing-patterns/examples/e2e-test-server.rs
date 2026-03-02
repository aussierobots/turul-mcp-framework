// turul-mcp-server v0.3
// E2E test using TestServerManager + McpTestClient
// This pattern starts a real HTTP server and sends requests over the network.

use serde_json::json;

// Import from the mcp-e2e-shared crate (available in workspace tests)
use mcp_e2e_shared::e2e_utils::{McpTestClient, TestFixtures, TestServerManager};

#[tokio::test]
async fn test_tools_server_e2e() {
    // 1. Start the test server (auto-allocates ephemeral port)
    let server = TestServerManager::start("tools-test-server")
        .await
        .expect("Failed to start tools test server");

    // 2. Create test client
    let mut client = McpTestClient::new(server.port());

    // 3. Initialize session
    let init_result = client
        .initialize_with_capabilities(TestFixtures::tools_capabilities())
        .await
        .expect("Failed to initialize");

    // Verify initialization response
    TestFixtures::verify_initialization_response(&init_result);

    // 4. Complete strict lifecycle handshake
    client
        .send_initialized_notification()
        .await
        .expect("Failed to send initialized notification");

    // 5. List tools
    let tools_result = client.list_tools().await.expect("Failed to list tools");
    let tools = TestFixtures::extract_tools_list(&tools_result)
        .expect("Failed to extract tools list");

    assert!(!tools.is_empty(), "Server should have registered tools");

    // Verify tool structure
    for tool in &tools {
        let tool_obj = tool.as_object().unwrap();
        assert!(tool_obj.contains_key("name"), "Tool must have a name");
        assert!(
            tool_obj.contains_key("inputSchema"),
            "Tool must have inputSchema"
        );
    }

    // 6. Call a specific tool
    let result = client
        .call_tool("calculator_add", json!({"a": 10.0, "b": 20.0}))
        .await
        .expect("Failed to call tool");

    // Verify the result has content
    assert!(
        result.contains_key("result"),
        "Tool call should return a result"
    );

    // Extract structured content if the tool has outputSchema
    if let Some(structured) = TestFixtures::extract_tool_structured_content(&result) {
        assert!(structured.is_object(), "structuredContent should be an object");
    }

    // Server is auto-killed when `server` is dropped
}

#[tokio::test]
async fn test_error_handling_e2e() {
    let server = TestServerManager::start("tools-test-server")
        .await
        .expect("Failed to start server");

    let mut client = McpTestClient::new(server.port());
    client.initialize().await.expect("Failed to initialize");
    client
        .send_initialized_notification()
        .await
        .expect("Failed to send initialized");

    // Call a non-existent tool
    let result = client
        .call_tool("nonexistent_tool", json!({}))
        .await
        .expect("Request should succeed even for unknown tools");

    // Should get a JSON-RPC error response
    TestFixtures::verify_error_response(&result);
}
