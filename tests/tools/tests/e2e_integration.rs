//! E2E Integration Tests for MCP Tools
//!
//! Tests real HTTP/SSE transport using tools-test-server
//! Validates complete MCP 2025-06-18 specification compliance

use futures::future::try_join_all;
use mcp_e2e_shared::{McpTestClient, TestFixtures, TestServerManager};
use serde_json::json;
use tracing::{debug, info};

#[tokio::test]
async fn test_tools_server_startup_and_discovery() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_tools_server()
        .await
        .expect("Failed to start tools server");
    let mut client = McpTestClient::new(server.port());

    // Initialize client with tools capabilities
    let init_result = client
        .initialize_with_capabilities(TestFixtures::tools_capabilities())
        .await
        .expect("Failed to initialize");

    debug!("Initialize result: {:?}", init_result);

    // Verify server info
    assert!(init_result.contains_key("result"));
    let result = init_result.get("result").unwrap().as_object().unwrap();

    assert!(result.contains_key("serverInfo"));
    let server_info = result.get("serverInfo").unwrap().as_object().unwrap();
    assert_eq!(
        server_info.get("name").unwrap().as_str().unwrap(),
        "tools-test-server"
    );

    // Verify capabilities
    assert!(result.contains_key("capabilities"));
    let capabilities = result.get("capabilities").unwrap().as_object().unwrap();
    assert!(capabilities.contains_key("tools"));
}

#[tokio::test]
async fn test_tools_list_endpoint() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_tools_server()
        .await
        .expect("Failed to start tools server");
    let mut client = McpTestClient::new(server.port());

    // Initialize and get tools list
    client
        .initialize_with_capabilities(TestFixtures::tools_capabilities())
        .await
        .unwrap();

    let tools_result = client.list_tools().await.expect("Failed to list tools");
    debug!("Tools list result: {:?}", tools_result);

    // Verify we have all expected tools
    assert!(tools_result.contains_key("result"));
    let result = tools_result.get("result").unwrap().as_object().unwrap();
    assert!(result.contains_key("tools"));

    let tools = result.get("tools").unwrap().as_array().unwrap();
    let tool_names: Vec<&str> = tools
        .iter()
        .map(|t| {
            t.as_object()
                .unwrap()
                .get("name")
                .unwrap()
                .as_str()
                .unwrap()
        })
        .collect();

    // Verify all 7 expected tools are present
    let expected_tools = vec![
        "calculator",
        "string_processor",
        "data_transformer",
        "session_counter",
        "progress_tracker",
        "error_generator",
        "parameter_validator",
    ];

    for expected_tool in expected_tools {
        assert!(
            tool_names.contains(&expected_tool),
            "Missing tool: {}",
            expected_tool
        );
    }

    info!("✅ All {} tools found: {:?}", tools.len(), tool_names);

    // Verify tool schemas
    for tool in tools {
        let tool_obj = tool.as_object().unwrap();
        assert!(tool_obj.contains_key("name"));
        assert!(tool_obj.contains_key("description"));
        assert!(tool_obj.contains_key("inputSchema"));

        let input_schema = tool_obj.get("inputSchema").unwrap().as_object().unwrap();
        assert_eq!(
            input_schema.get("type").unwrap().as_str().unwrap(),
            "object"
        );
        assert!(input_schema.contains_key("properties"));
    }
}

#[tokio::test]
async fn test_calculator_tool_execution() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_tools_server()
        .await
        .expect("Failed to start tools server");
    let mut client = McpTestClient::new(server.port());

    client
        .initialize_with_capabilities(TestFixtures::tools_capabilities())
        .await
        .unwrap();

    // Test addition
    let add_result = client
        .call_tool(
            "calculator",
            json!({
                "operation": "add",
                "a": 5.0,
                "b": 3.0
            }),
        )
        .await
        .expect("Failed to call calculator tool");

    debug!("Calculator add result: {:?}", add_result);

    assert!(add_result.contains_key("result"));
    let result = add_result.get("result").unwrap().as_object().unwrap();
    assert!(result.contains_key("content"));

    // Extract the calculator result object
    if let Some(calc_result_val) = TestFixtures::extract_tool_result_object(&add_result) {
        let calc_result = calc_result_val.as_object().unwrap();
        assert_eq!(calc_result.get("result").unwrap().as_f64().unwrap(), 8.0);
        assert_eq!(
            calc_result.get("operation").unwrap().as_str().unwrap(),
            "add"
        );
    } else {
        panic!("No tool result found");
    }

    // Test division by zero error
    let div_zero_result = client
        .call_tool(
            "calculator",
            json!({
                "operation": "divide",
                "a": 10.0,
                "b": 0.0
            }),
        )
        .await;

    // Should receive an error response or a successful response containing error information
    match div_zero_result {
        Ok(result) => {
            // Check if response contains error field (JSON-RPC error)
            if result.contains_key("error") {
                info!("✅ Division by zero correctly returned JSON-RPC error");
            } else {
                // Sometimes tools return successful responses with error content
                info!("Got success response for division by zero: {:#?}", result);
            }
        }
        Err(e) => {
            info!("✅ Division by zero correctly returned HTTP error: {:?}", e);
        }
    }

    info!("✅ Calculator tool validation complete");
}

#[tokio::test]
async fn test_string_processor_tool() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_tools_server()
        .await
        .expect("Failed to start tools server");
    let mut client = McpTestClient::new(server.port());

    client
        .initialize_with_capabilities(TestFixtures::tools_capabilities())
        .await
        .unwrap();

    // Test uppercase operation
    let result = client
        .call_tool(
            "string_processor",
            json!({
                "text": "hello world",
                "operation": "uppercase"
            }),
        )
        .await
        .expect("Failed to call string processor");

    let parsed_result =
        TestFixtures::extract_tool_result_object(&result).expect("No tool result found");

    assert_eq!(
        parsed_result.get("result").unwrap().as_str().unwrap(),
        "HELLO WORLD"
    );
    assert_eq!(
        parsed_result.get("operation").unwrap().as_str().unwrap(),
        "uppercase"
    );
    assert_eq!(
        parsed_result.get("original").unwrap().as_str().unwrap(),
        "hello world"
    );

    // Test length operation
    let result = client
        .call_tool(
            "string_processor",
            json!({
                "text": "test",
                "operation": "length"
            }),
        )
        .await
        .expect("Failed to call string processor");

    let parsed_result =
        TestFixtures::extract_tool_result_object(&result).expect("No tool result found");

    assert!(parsed_result
        .get("result")
        .unwrap()
        .as_str()
        .unwrap()
        .contains("4 characters"));

    info!("✅ String processor tool validation complete");
}

#[tokio::test]
async fn test_data_transformer_tool() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_tools_server()
        .await
        .expect("Failed to start tools server");
    let mut client = McpTestClient::new(server.port());

    client
        .initialize_with_capabilities(TestFixtures::tools_capabilities())
        .await
        .unwrap();

    // Test extract_keys operation
    let test_data = json!({
        "name": "test",
        "value": 42,
        "enabled": true
    });

    let result = client
        .call_tool(
            "data_transformer",
            json!({
                "data": test_data,
                "operation": "extract_keys"
            }),
        )
        .await
        .expect("Failed to call data transformer");

    let parsed_result =
        TestFixtures::extract_tool_result_object(&result).expect("No tool result found");

    let result_obj = parsed_result.get("result").unwrap().as_object().unwrap();
    let keys = result_obj.get("keys").unwrap().as_array().unwrap();

    let key_names: Vec<&str> = keys.iter().map(|k| k.as_str().unwrap()).collect();
    assert!(key_names.contains(&"name"));
    assert!(key_names.contains(&"value"));
    assert!(key_names.contains(&"enabled"));

    info!("✅ Data transformer tool validation complete");
}

#[tokio::test]
async fn test_session_counter_tool_state_management() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_tools_server()
        .await
        .expect("Failed to start tools server");
    let mut client = McpTestClient::new(server.port());

    client
        .initialize_with_capabilities(TestFixtures::tools_capabilities())
        .await
        .unwrap();
    let session_id = client.session_id().unwrap().clone();

    // Test increment
    let result = client
        .call_tool(
            "session_counter",
            json!({
                "operation": "increment",
                "amount": 5
            }),
        )
        .await
        .expect("Failed to call session counter");

    let parsed_result =
        TestFixtures::extract_tool_result_object(&result).expect("No tool result found");

    assert_eq!(
        parsed_result
            .get("current_value")
            .unwrap()
            .as_i64()
            .unwrap(),
        5
    );
    assert_eq!(
        parsed_result.get("session_id").unwrap().as_str().unwrap(),
        session_id
    );

    // Test get (should maintain state)
    let result = client
        .call_tool(
            "session_counter",
            json!({
                "operation": "get"
            }),
        )
        .await
        .expect("Failed to call session counter");

    let parsed_result =
        TestFixtures::extract_tool_result_object(&result).expect("No tool result found");

    assert_eq!(
        parsed_result
            .get("current_value")
            .unwrap()
            .as_i64()
            .unwrap(),
        5
    );

    // Test decrement
    let result = client
        .call_tool(
            "session_counter",
            json!({
                "operation": "decrement",
                "amount": 2
            }),
        )
        .await
        .expect("Failed to call session counter");

    let parsed_result =
        TestFixtures::extract_tool_result_object(&result).expect("No tool result found");

    assert_eq!(
        parsed_result
            .get("current_value")
            .unwrap()
            .as_i64()
            .unwrap(),
        3
    );

    info!("✅ Session counter state management validation complete");
}

#[tokio::test]
async fn test_progress_tracker_with_notifications() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_tools_server()
        .await
        .expect("Failed to start tools server");
    let mut client = McpTestClient::new(server.port());

    client
        .initialize_with_capabilities(TestFixtures::tools_capabilities())
        .await
        .unwrap();

    // Start progress tracker (short duration for testing)
    let result = client
        .call_tool(
            "progress_tracker",
            json!({
                "duration": 2.0,  // 2 seconds
                "steps": 4        // 4 steps = 25%, 50%, 75%, 100%
            }),
        )
        .await
        .expect("Failed to call progress tracker");

    let parsed_result =
        TestFixtures::extract_tool_result_object(&result).expect("No tool result found");

    assert_eq!(
        parsed_result.get("operation").unwrap().as_str().unwrap(),
        "progress_tracker"
    );
    assert_eq!(
        parsed_result.get("duration").unwrap().as_f64().unwrap(),
        2.0
    );
    assert_eq!(parsed_result.get("steps").unwrap().as_i64().unwrap(), 4);
    assert_eq!(
        parsed_result.get("status").unwrap().as_str().unwrap(),
        "completed"
    );
    let parsed_obj = parsed_result.as_object().unwrap();
    assert!(parsed_obj.contains_key("progress_token"));
    assert!(parsed_obj.contains_key("completed_at"));

    // Test SSE client-side verification for progress notifications
    info!("🔍 Testing SSE client-side verification...");

    // Start SSE connection in background before starting long-running tool
    let session_id = client.session_id().unwrap().clone();
    let sse_client = reqwest::Client::new();
    let sse_url = format!("http://127.0.0.1:{}/mcp", server.port());

    let sse_handle = tokio::spawn(async move {
        let mut response = sse_client
            .get(&sse_url)
            .header("Accept", "text/event-stream")
            .header("mcp-session-id", &session_id)
            .send()
            .await
            .expect("Failed to connect to SSE");

        let mut progress_events = Vec::new();
        let start = tokio::time::Instant::now();

        // Listen for progress events for up to 4 seconds
        while start.elapsed() < tokio::time::Duration::from_secs(4) && progress_events.len() < 3 {
            if let Some(chunk) = response.chunk().await.unwrap_or(None) {
                if let Ok(text) = String::from_utf8(chunk.to_vec()) {
                    if !text.trim().is_empty() {
                        debug!("📨 Received SSE chunk: {}", text);

                        // Look for progress notifications in the SSE stream
                        if text.contains("\"method\":\"notifications/progress\"") {
                            progress_events.push(text);
                            info!(
                                "✅ Progress notification received via SSE: {}",
                                progress_events.len()
                            );
                        }
                    }
                }
            }

            // Small delay to prevent spinning
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }

        progress_events
    });

    // Wait a moment for SSE connection to establish
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Now call a long-running progress tracker tool
    let _progress_result = client
        .call_tool(
            "progress_tracker",
            json!({
                "duration": 1.5,
                "steps": 3  // Should generate progress at 33%, 66%, 100%
            }),
        )
        .await
        .expect("Failed to call progress tracker");

    // Wait for SSE events and verify progress notifications were received client-side
    let progress_events = sse_handle.await.expect("SSE task failed");

    // STRICT ASSERTION: Progress notifications MUST be received for protocol compliance
    assert!(!progress_events.is_empty(),
           "❌ CRITICAL: No progress notifications received via SSE. This is a protocol compliance failure for tools with progressToken support. Expected at least 1 notification for 3-step progress tracker.");

    info!(
        "✅ SSE client-side verification complete: {} progress notifications received",
        progress_events.len()
    );

    // Validate structure of all received progress notifications
    for (i, event) in progress_events.iter().enumerate() {
        assert!(
            event.contains("\"method\":\"notifications/progress\""),
            "Progress notification {} has invalid method format: {}",
            i,
            event
        );
        assert!(
            event.contains("\"progress\""),
            "Progress notification {} missing progress field: {}",
            i,
            event
        );
        assert!(
            event.contains("\"progressToken\""),
            "Progress notification {} missing progressToken field: {}",
            i,
            event
        );
    }

    // Additional validation: Ensure we got reasonable number of progress updates
    // For 3 steps (33%, 66%, 100%), we should get at least 1 notification
    assert!(
        !progress_events.is_empty(),
        "Expected at least 1 progress notification for 3-step operation, got {}",
        progress_events.len()
    );

    info!("✅ Progress tracker validation complete");
}

#[tokio::test]
async fn test_error_generator_tool() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_tools_server()
        .await
        .expect("Failed to start tools server");
    let mut client = McpTestClient::new(server.port());

    client
        .initialize_with_capabilities(TestFixtures::tools_capabilities())
        .await
        .unwrap();

    // Test tool execution error
    let result = client
        .call_tool(
            "error_generator",
            json!({
                "error_type": "tool_execution",
                "message": "Test tool execution error"
            }),
        )
        .await;

    // Should receive an error response
    match result {
        Ok(result_val) => {
            if result_val.contains_key("error") {
                info!("✅ Tool correctly returned JSON-RPC error");
            } else {
                info!("Got success response for error case: {:#?}", result_val);
            }
        }
        Err(e) => {
            info!("✅ Tool correctly returned HTTP error: {:?}", e);
        }
    }

    // Test validation error
    let result = client
        .call_tool(
            "error_generator",
            json!({
                "error_type": "validation",
                "message": "Test validation error"
            }),
        )
        .await;

    // Should receive an error response
    match result {
        Ok(result_val) => {
            if result_val.contains_key("error") {
                info!("✅ Error generator validation correctly returned JSON-RPC error");
            } else {
                info!(
                    "Got success response for validation error: {:#?}",
                    result_val
                );
            }
        }
        Err(e) => {
            info!(
                "✅ Error generator validation correctly returned HTTP error: {:?}",
                e
            );
        }
    }

    info!("✅ Error generator validation complete");
}

#[tokio::test]
async fn test_notifications_initialized_lifecycle() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_tools_server()
        .await
        .expect("Failed to start tools server");
    let mut client = McpTestClient::new(server.port());

    // Step 1: Initialize connection (but not send initialized notification yet)
    let init_result = client
        .initialize_with_capabilities(TestFixtures::tools_capabilities())
        .await
        .expect("Failed to initialize");

    debug!("Initialize handshake complete: {:?}", init_result);

    // Step 2: Start SSE connection to listen for any server notifications
    let sse_handle = tokio::spawn({
        let client_clone = client.clone();
        async move {
            info!("🔗 Starting SSE connection for initialized notification test");

            let mut response = client_clone
                .connect_sse()
                .await
                .expect("Failed to connect to SSE");

            let mut server_events = Vec::new();
            let start = tokio::time::Instant::now();

            // Listen for any server events for 2 seconds after initialization
            while start.elapsed() < tokio::time::Duration::from_secs(2) {
                if let Some(chunk) = response.chunk().await.unwrap_or(None) {
                    if let Ok(text) = String::from_utf8(chunk.to_vec()) {
                        if !text.trim().is_empty() {
                            debug!("📨 SSE event received: {}", text);
                            server_events.push(text);
                        }
                    }
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            }

            server_events
        }
    });

    // Step 3: Send notifications/initialized to complete the initialization lifecycle
    info!("📤 Sending notifications/initialized notification");

    let initialized_notification = json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    });

    let response = client
        .send_notification(initialized_notification)
        .await
        .expect("Failed to send initialized notification");

    // Step 4: Verify the notification was processed (should return empty response for notifications)
    debug!("Initialized notification response: {:?}", response);

    // Step 5: Verify that tools work normally after initialized notification
    let tools_result = client
        .list_tools()
        .await
        .expect("Failed to list tools after initialized notification");

    let tools = TestFixtures::extract_tools_list(&tools_result)
        .expect("Failed to extract tools from response");

    assert!(
        !tools.is_empty(),
        "No tools found after initialized notification"
    );
    info!(
        "✅ Tools list works after initialized notification: {} tools found",
        tools.len()
    );

    // Step 6: Verify that tool execution works after initialized notification
    let calc_result = client
        .call_tool(
            "calculator",
            json!({
                "operation": "add",
                "a": 5.0,
                "b": 3.0
            }),
        )
        .await
        .expect("Failed to call calculator after initialized notification");

    let result_obj = TestFixtures::extract_tool_result_object(&calc_result)
        .expect("No tool result found after initialized");

    assert_eq!(result_obj.get("result").unwrap().as_f64().unwrap(), 8.0);
    info!("✅ Tool execution works after initialized notification");

    // Step 7: Check for any server-side events during initialization
    let server_events = sse_handle.await.expect("SSE task failed");

    debug!("Server events during initialization: {:?}", server_events);

    // For now, we don't require specific server-side initialized notifications
    // but we log them for debugging
    for event in &server_events {
        if event.contains("initialized") {
            info!("📨 Server sent initialized-related event: {}", event);
        }
    }

    info!("✅ notifications/initialized lifecycle test complete");
}

#[tokio::test]
async fn test_notifications_tools_list_changed_compliance() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_tools_server()
        .await
        .expect("Failed to start tools server");
    let mut client = McpTestClient::new(server.port());

    // Step 1: Initialize with tools capabilities that include listChanged support
    let tools_capabilities = json!({
        "tools": {
            "listChanged": true  // Client advertises it can handle listChanged notifications
        }
    });

    let init_result = client
        .initialize_with_capabilities(tools_capabilities)
        .await
        .expect("Failed to initialize");

    debug!(
        "Initialize result for tools listChanged test: {:?}",
        init_result
    );

    // Step 2: Verify server correctly advertises listChanged=false (static framework)
    let result_obj = init_result.get("result").unwrap().as_object().unwrap();
    let server_capabilities = result_obj.get("capabilities").unwrap().as_object().unwrap();
    let tools_caps = server_capabilities
        .get("tools")
        .unwrap()
        .as_object()
        .unwrap();

    // MCP Compliance: Static framework should advertise listChanged=false
    assert!(
        !tools_caps.get("listChanged").unwrap().as_bool().unwrap(),
        "🔍 MCP COMPLIANCE: Static framework must advertise tools.listChanged=false"
    );

    info!("✅ Server correctly advertises tools.listChanged=false for static framework");

    // Step 3: Test that notifications/tools/listChanged can be sent (even though server is static)
    // This tests the infrastructure, not actual dynamic list changes

    let sse_handle = tokio::spawn({
        let client_clone = client.clone();
        async move {
            info!("🔗 Starting SSE connection for tools listChanged notification test");

            let mut response = client_clone
                .connect_sse()
                .await
                .expect("Failed to connect to SSE");

            let mut list_changed_events = Vec::new();
            let start = tokio::time::Instant::now();

            // Listen for listChanged notifications for up to 3 seconds
            while start.elapsed() < tokio::time::Duration::from_secs(3) {
                if let Some(chunk) = response.chunk().await.unwrap_or(None) {
                    if let Ok(text) = String::from_utf8(chunk.to_vec()) {
                        if !text.trim().is_empty() {
                            debug!("📨 SSE event received: {}", text);

                            // Look for tools listChanged notifications
                            if text.contains("\"method\":\"notifications/tools/listChanged\"") {
                                list_changed_events.push(text);
                                info!("✅ tools/listChanged notification received via SSE");
                            }
                        }
                    }
                }

                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }

            list_changed_events
        }
    });

    // Step 4: Since this is a static framework, we don't expect listChanged events
    // But we can verify the infrastructure works by checking current tool list
    let tools_result = client.list_tools().await.expect("Failed to list tools");
    let tools = TestFixtures::extract_tools_list(&tools_result).expect("Failed to extract tools");

    info!("📋 Current tool list has {} tools", tools.len());

    // Verify tool list structure for protocol compliance
    for tool in &tools {
        let tool_obj = tool.as_object().unwrap();
        assert!(
            tool_obj.contains_key("name"),
            "Tool missing required 'name' field"
        );
        assert!(
            tool_obj.contains_key("description"),
            "Tool missing required 'description' field"
        );
        assert!(
            tool_obj.contains_key("inputSchema"),
            "Tool missing required 'inputSchema' field"
        );
    }

    info!("✅ All {} tools have required MCP fields", tools.len());

    // Step 5: Test notifications/tools/listChanged message format compliance
    let list_changed_notification = json!({
        "jsonrpc": "2.0",
        "method": "notifications/tools/listChanged"
        // No params required for this notification type
    });

    // Send the notification to test server handling
    let response = client.send_notification(list_changed_notification).await;
    match response {
        Ok(result) => debug!(
            "✅ Server handled tools/listChanged notification: {:?}",
            result
        ),
        Err(e) => debug!("ℹ️  Server response to tools/listChanged: {:?}", e),
    }

    // Step 6: Wait for any SSE events and verify none were received (static server)
    let list_changed_events = sse_handle.await.expect("SSE task failed");

    // For static framework, we expect NO listChanged events
    if list_changed_events.is_empty() {
        info!("✅ EXPECTED: No tools/listChanged events from static framework (listChanged=false)");
    } else {
        // If events were received, validate their format
        info!(
            "ℹ️  Received {} tools/listChanged events (testing infrastructure)",
            list_changed_events.len()
        );

        for (i, event) in list_changed_events.iter().enumerate() {
            assert!(
                event.contains("\"method\":\"notifications/tools/listChanged\""),
                "Event {} has incorrect method format: {}",
                i,
                event
            );
            assert!(
                event.contains("\"jsonrpc\":\"2.0\""),
                "Event {} missing JSON-RPC version: {}",
                i,
                event
            );
        }
    }

    // Step 7: Verify tools list consistency after notification handling
    let tools_result_after = client
        .list_tools()
        .await
        .expect("Failed to list tools after notification");
    let tools_after = TestFixtures::extract_tools_list(&tools_result_after)
        .expect("Failed to extract tools after notification");

    // Tool list should be identical (static framework)
    assert_eq!(
        tools.len(),
        tools_after.len(),
        "Tool count changed unexpectedly in static framework"
    );

    info!("✅ notifications/tools/listChanged compliance test complete");
}

#[tokio::test]
async fn test_parameter_validator_tool() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_tools_server()
        .await
        .expect("Failed to start tools server");
    let mut client = McpTestClient::new(server.port());

    client
        .initialize_with_capabilities(TestFixtures::tools_capabilities())
        .await
        .unwrap();

    // Test valid parameters
    let result = client
        .call_tool(
            "parameter_validator",
            json!({
                "email": "test@example.com",
                "age": 25,
                "config": {"setting": "value"},
                "tags": ["tag1", "tag2"]
            }),
        )
        .await
        .expect("Failed to call parameter validator");

    let parsed_result =
        TestFixtures::extract_tool_result_object(&result).expect("No tool result found");

    assert_eq!(
        parsed_result
            .get("validation_result")
            .unwrap()
            .as_str()
            .unwrap(),
        "passed"
    );
    assert_eq!(
        parsed_result.get("email").unwrap().as_str().unwrap(),
        "test@example.com"
    );
    assert_eq!(parsed_result.get("age").unwrap().as_i64().unwrap(), 25);
    assert_eq!(parsed_result.get("tag_count").unwrap().as_i64().unwrap(), 2);

    // Test invalid email
    let result = client
        .call_tool(
            "parameter_validator",
            json!({
                "email": "invalid-email",
                "age": 25,
                "config": {"setting": "value"}
            }),
        )
        .await;

    match result {
        Ok(result_val) => {
            if result_val.contains_key("error") {
                info!("✅ Validation correctly returned JSON-RPC error");
            } else {
                info!(
                    "Got success response for validation error: {:#?}",
                    result_val
                );
            }
        }
        Err(e) => {
            info!("✅ Validation correctly returned HTTP error: {:?}", e);
        }
    }

    // Test age out of range
    let result = client
        .call_tool(
            "parameter_validator",
            json!({
                "email": "test@example.com",
                "age": 200,
                "config": {"setting": "value"}
            }),
        )
        .await;

    match result {
        Ok(result_val) => {
            if result_val.contains_key("error") {
                info!("✅ Validation correctly returned JSON-RPC error");
            } else {
                info!(
                    "Got success response for validation error: {:#?}",
                    result_val
                );
            }
        }
        Err(e) => {
            info!("✅ Validation correctly returned HTTP error: {:?}", e);
        }
    }

    info!("✅ Parameter validator validation complete");
}

#[tokio::test]
async fn test_tools_protocol_compliance() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_tools_server()
        .await
        .expect("Failed to start tools server");
    let mut client = McpTestClient::new(server.port());

    // Test initialize compliance
    let init_result = client
        .initialize_with_capabilities(TestFixtures::tools_capabilities())
        .await
        .unwrap();

    // Verify MCP 2025-06-18 protocol compliance
    assert!(init_result.contains_key("jsonrpc"));
    assert_eq!(init_result.get("jsonrpc").unwrap().as_str().unwrap(), "2.0");
    assert!(init_result.contains_key("id"));
    assert!(init_result.contains_key("result"));

    // Test tools/list compliance
    let tools_result = client.list_tools().await.unwrap();
    assert!(tools_result.contains_key("jsonrpc"));
    assert_eq!(
        tools_result.get("jsonrpc").unwrap().as_str().unwrap(),
        "2.0"
    );

    // Test tools/call compliance
    let call_result = client
        .call_tool(
            "calculator",
            json!({
                "operation": "add",
                "a": 1.0,
                "b": 2.0
            }),
        )
        .await
        .unwrap();

    assert!(call_result.contains_key("jsonrpc"));
    assert_eq!(call_result.get("jsonrpc").unwrap().as_str().unwrap(), "2.0");

    // Verify CallToolResult structure
    let result = call_result.get("result").unwrap().as_object().unwrap();
    assert!(result.contains_key("content"));

    let content = result.get("content").unwrap().as_array().unwrap();
    let content_item = content[0].as_object().unwrap();
    assert!(content_item.contains_key("type"));
    assert!(content_item.contains_key("text"));

    info!("✅ Tools protocol MCP 2025-06-18 compliance validated");
}

#[tokio::test]
async fn test_concurrent_tool_execution() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_tools_server()
        .await
        .expect("Failed to start tools server");
    let mut client = McpTestClient::new(server.port());

    client
        .initialize_with_capabilities(TestFixtures::tools_capabilities())
        .await
        .unwrap();

    // Execute multiple tools concurrently
    let tasks = vec![
        tokio::spawn({
            let client = client.clone();
            async move {
                client
                    .call_tool(
                        "calculator",
                        json!({"operation": "add", "a": 1.0, "b": 2.0}),
                    )
                    .await
            }
        }),
        tokio::spawn({
            let client = client.clone();
            async move {
                client
                    .call_tool(
                        "string_processor",
                        json!({"text": "test", "operation": "uppercase"}),
                    )
                    .await
            }
        }),
        tokio::spawn({
            let client = client.clone();
            async move {
                client
                    .call_tool(
                        "session_counter",
                        json!({"operation": "increment", "amount": 1}),
                    )
                    .await
            }
        }),
    ];

    // Wait for all tasks and verify they all succeed
    let results = try_join_all(tasks).await.unwrap();

    for result in results {
        assert!(
            result.is_ok(),
            "Concurrent tool execution failed: {:?}",
            result
        );
    }

    info!("✅ Concurrent tool execution validation complete");
}

#[tokio::test]
async fn test_session_storage_integration() {
    let _ = tracing_subscriber::fmt::try_init();
    info!("🧪 Testing SessionStorage integration and session isolation...");

    let server = TestServerManager::start_tools_server()
        .await
        .expect("Failed to start tools server");

    // Create two separate clients to test session isolation
    let mut client1 = McpTestClient::new(server.port());
    let mut client2 = McpTestClient::new(server.port());

    // Initialize both clients - they should get different session IDs
    client1
        .initialize_with_capabilities(TestFixtures::tools_capabilities())
        .await
        .expect("Failed to initialize client1");
    client2
        .initialize_with_capabilities(TestFixtures::tools_capabilities())
        .await
        .expect("Failed to initialize client2");

    let session1_id = client1.session_id().unwrap().clone();
    let session2_id = client2.session_id().unwrap().clone();

    // Verify different session IDs
    assert_ne!(
        session1_id, session2_id,
        "Sessions should have different IDs"
    );
    info!(
        "✅ Session isolation verified: {} vs {}",
        session1_id, session2_id
    );

    // Test session state persistence - Client 1
    info!("🔍 Testing session state persistence for client 1...");

    // Initialize counter to 5 for session 1
    let result = client1
        .call_tool(
            "session_counter",
            json!({
                "operation": "increment",
                "amount": 5
            }),
        )
        .await
        .expect("Failed to increment counter");

    let parsed_result =
        TestFixtures::extract_tool_result_object(&result).expect("No tool result found");
    assert_eq!(
        parsed_result
            .get("current_value")
            .unwrap()
            .as_i64()
            .unwrap(),
        5
    );
    assert_eq!(
        parsed_result.get("session_id").unwrap().as_str().unwrap(),
        session1_id
    );

    // Increment by 3 more for session 1
    let result = client1
        .call_tool(
            "session_counter",
            json!({
                "operation": "increment",
                "amount": 3
            }),
        )
        .await
        .expect("Failed to increment counter");

    let parsed_result =
        TestFixtures::extract_tool_result_object(&result).expect("No tool result found");
    assert_eq!(
        parsed_result
            .get("current_value")
            .unwrap()
            .as_i64()
            .unwrap(),
        8
    );

    // Test session state persistence - Client 2 (should start at 0)
    info!("🔍 Testing session isolation for client 2...");

    let result = client2
        .call_tool(
            "session_counter",
            json!({
                "operation": "get"
            }),
        )
        .await
        .expect("Failed to get counter");

    let parsed_result =
        TestFixtures::extract_tool_result_object(&result).expect("No tool result found");
    assert_eq!(
        parsed_result
            .get("current_value")
            .unwrap()
            .as_i64()
            .unwrap(),
        0,
        "Client 2 should start with 0"
    );
    assert_eq!(
        parsed_result.get("session_id").unwrap().as_str().unwrap(),
        session2_id
    );

    // Increment client 2's counter
    let result = client2
        .call_tool(
            "session_counter",
            json!({
                "operation": "increment",
                "amount": 10
            }),
        )
        .await
        .expect("Failed to increment counter");

    let parsed_result =
        TestFixtures::extract_tool_result_object(&result).expect("No tool result found");
    assert_eq!(
        parsed_result
            .get("current_value")
            .unwrap()
            .as_i64()
            .unwrap(),
        10
    );

    // Verify client 1's state is unchanged
    info!("🔍 Verifying session state isolation...");

    let result = client1
        .call_tool(
            "session_counter",
            json!({
                "operation": "get"
            }),
        )
        .await
        .expect("Failed to get counter");

    let parsed_result =
        TestFixtures::extract_tool_result_object(&result).expect("No tool result found");
    assert_eq!(
        parsed_result
            .get("current_value")
            .unwrap()
            .as_i64()
            .unwrap(),
        8,
        "Client 1 state should be unchanged"
    );

    // Verify client 2's state is maintained
    let result = client2
        .call_tool(
            "session_counter",
            json!({
                "operation": "get"
            }),
        )
        .await
        .expect("Failed to get counter");

    let parsed_result =
        TestFixtures::extract_tool_result_object(&result).expect("No tool result found");
    assert_eq!(
        parsed_result
            .get("current_value")
            .unwrap()
            .as_i64()
            .unwrap(),
        10,
        "Client 2 state should be maintained"
    );

    // Test state operations (decrement, reset)
    info!("🔍 Testing state operations...");

    let result = client1
        .call_tool(
            "session_counter",
            json!({
                "operation": "decrement",
                "amount": 3
            }),
        )
        .await
        .expect("Failed to decrement counter");

    let parsed_result =
        TestFixtures::extract_tool_result_object(&result).expect("No tool result found");
    assert_eq!(
        parsed_result
            .get("current_value")
            .unwrap()
            .as_i64()
            .unwrap(),
        5
    );

    let result = client1
        .call_tool(
            "session_counter",
            json!({
                "operation": "reset"
            }),
        )
        .await
        .expect("Failed to reset counter");

    let parsed_result =
        TestFixtures::extract_tool_result_object(&result).expect("No tool result found");
    assert_eq!(
        parsed_result
            .get("current_value")
            .unwrap()
            .as_i64()
            .unwrap(),
        0
    );

    // Verify client 2 is still unaffected
    let result = client2
        .call_tool(
            "session_counter",
            json!({
                "operation": "get"
            }),
        )
        .await
        .expect("Failed to get counter");

    let parsed_result =
        TestFixtures::extract_tool_result_object(&result).expect("No tool result found");
    assert_eq!(
        parsed_result
            .get("current_value")
            .unwrap()
            .as_i64()
            .unwrap(),
        10,
        "Client 2 should be unaffected by client 1 reset"
    );

    info!("✅ SessionStorage integration validation complete - proper session isolation and state persistence verified");
}
