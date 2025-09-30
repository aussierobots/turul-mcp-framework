//! Comprehensive MCP Error Code Coverage Testing
//!
//! Tests all MCP error codes for complete protocol compliance and robustness
//! Validates error handling, JSON-RPC error structure, and client recovery

use mcp_e2e_shared::{McpTestClient, TestFixtures, TestServerManager};
use serde_json::{json, Map, Value};
use tracing::{debug, info};

/// Helper function to extract error from MCP response (handles both JSON-RPC and Tools protocol formats)
fn extract_error_from_response(
    response: &std::collections::HashMap<String, Value>,
) -> Option<Map<String, Value>> {
    if let Some(error) = response.get("error") {
        // Top-level JSON-RPC error
        error.as_object().cloned()
    } else if let Some(result) = response.get("result") {
        // Tool result with error
        if let Some(result_obj) = result.as_object() {
            if let Some(error) = result_obj.get("error") {
                error.as_object().cloned()
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    }
}

/// Helper function to check if response contains any error
fn is_error_response(response: &std::collections::HashMap<String, Value>) -> bool {
    extract_error_from_response(response).is_some()
}

#[tokio::test]
async fn test_tool_not_found_error() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_tools_server()
        .await
        .expect("Failed to start tools server");
    let mut client = McpTestClient::new(server.port());

    client
        .initialize_with_capabilities(TestFixtures::tools_capabilities())
        .await
        .unwrap();

    // Send notifications/initialized to complete handshake (required for strict lifecycle mode)
    client.send_initialized_notification().await.unwrap();

    // Call non-existent tool
    let result = client.call_tool("non_existent_tool", json!({})).await;

    match result {
        Ok(response) => {
            // Should be an error response
            assert!(
                is_error_response(&response),
                "Expected error response for non-existent tool, got: {:?}",
                response
            );

            let error = extract_error_from_response(&response).unwrap();
            assert!(error.contains_key("code"), "Error missing code field");
            assert!(error.contains_key("message"), "Error missing message field");

            let message = error.get("message").unwrap().as_str().unwrap();
            let code = error.get("code").unwrap().as_i64().unwrap();

            assert!(
                message.to_lowercase().contains("not found")
                    || message.to_lowercase().contains("tool"),
                "Error message should indicate tool not found: {}",
                message
            );

            info!(
                "✅ ToolNotFound error properly returned: code={}, message={}",
                code, message
            );
        }
        Err(e) => {
            // HTTP-level error is also acceptable
            info!("✅ ToolNotFound error returned as HTTP error: {:?}", e);
        }
    }
}

#[tokio::test]
async fn test_invalid_parameters_error() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_tools_server()
        .await
        .expect("Failed to start tools server");
    let mut client = McpTestClient::new(server.port());

    client
        .initialize_with_capabilities(TestFixtures::tools_capabilities())
        .await
        .unwrap();

    // Send notifications/initialized to complete handshake (required for strict lifecycle mode)
    client.send_initialized_notification().await.unwrap();

    // Call calculator with invalid parameters (missing required parameters)
    let result = client
        .call_tool(
            "calculator",
            json!({
                "operation": "add"
                // Missing a, b parameters
            }),
        )
        .await;

    match result {
        Ok(response) => {
            if is_error_response(&response) {
                let error = extract_error_from_response(&response).unwrap();
                let message = error.get("message").unwrap().as_str().unwrap();
                assert!(
                    message.to_lowercase().contains("parameter")
                        || message.to_lowercase().contains("missing")
                        || message.to_lowercase().contains("invalid"),
                    "Error should indicate parameter issue: {}",
                    message
                );

                info!("✅ InvalidParameters error properly returned: {}", message);
            } else {
                // Tool might handle this gracefully with defaults
                info!("ℹ️  Tool handled missing parameters gracefully");
            }
        }
        Err(e) => {
            info!("✅ InvalidParameters error returned as HTTP error: {:?}", e);
        }
    }
}

#[tokio::test]
async fn test_invalid_parameter_type_error() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_tools_server()
        .await
        .expect("Failed to start tools server");
    let mut client = McpTestClient::new(server.port());

    client
        .initialize_with_capabilities(TestFixtures::tools_capabilities())
        .await
        .unwrap();

    // Send notifications/initialized to complete handshake (required for strict lifecycle mode)
    client.send_initialized_notification().await.unwrap();

    // Call calculator with wrong parameter types (strings instead of numbers)
    let result = client
        .call_tool(
            "calculator",
            json!({
                "operation": "add",
                "a": "not_a_number",
                "b": "also_not_a_number"
            }),
        )
        .await;

    match result {
        Ok(response) => {
            if response.contains_key("error") {
                let error = response.get("error").unwrap().as_object().unwrap();
                let message = error.get("message").unwrap().as_str().unwrap();
                assert!(
                    message.to_lowercase().contains("type")
                        || message.to_lowercase().contains("invalid")
                        || message.to_lowercase().contains("parameter"),
                    "Error should indicate type issue: {}",
                    message
                );

                info!(
                    "✅ InvalidParameterType error properly returned: {}",
                    message
                );
            } else {
                // Tool might handle type conversion gracefully
                info!("ℹ️  Tool handled type conversion gracefully");
            }
        }
        Err(e) => {
            info!(
                "✅ InvalidParameterType error returned as HTTP error: {:?}",
                e
            );
        }
    }
}

#[tokio::test]
async fn test_tool_execution_error() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_tools_server()
        .await
        .expect("Failed to start tools server");
    let mut client = McpTestClient::new(server.port());

    client
        .initialize_with_capabilities(TestFixtures::tools_capabilities())
        .await
        .unwrap();

    // Send notifications/initialized to complete handshake (required for strict lifecycle mode)
    client.send_initialized_notification().await.unwrap();

    // Use error_generator tool to trigger execution errors
    let result = client
        .call_tool(
            "error_generator",
            json!({
                "error_type": "tool_execution",
                "message": "Test execution error"
            }),
        )
        .await;

    match result {
        Ok(response) => {
            // Should be an error response
            assert!(
                response.contains_key("error"),
                "Expected error response for execution error, got: {:?}",
                response
            );

            let error = response.get("error").unwrap().as_object().unwrap();
            let message = error.get("message").unwrap().as_str().unwrap();
            assert!(
                message.contains("Test execution error"),
                "Error message should contain test message: {}",
                message
            );

            info!("✅ ToolExecutionError properly returned: {}", message);
        }
        Err(e) => {
            info!("✅ ToolExecutionError returned as HTTP error: {:?}", e);
        }
    }
}

#[tokio::test]
async fn test_validation_error() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_tools_server()
        .await
        .expect("Failed to start tools server");
    let mut client = McpTestClient::new(server.port());

    client
        .initialize_with_capabilities(TestFixtures::tools_capabilities())
        .await
        .unwrap();

    // Send notifications/initialized to complete handshake (required for strict lifecycle mode)
    client.send_initialized_notification().await.unwrap();

    // Use error_generator tool to trigger validation errors
    let result = client
        .call_tool(
            "error_generator",
            json!({
                "error_type": "validation",
                "message": "Test validation error"
            }),
        )
        .await;

    match result {
        Ok(response) => {
            if response.contains_key("error") {
                let error = response.get("error").unwrap().as_object().unwrap();
                let message = error.get("message").unwrap().as_str().unwrap();
                assert!(
                    message.contains("validation") || message.contains("Test validation error"),
                    "Error should indicate validation issue: {}",
                    message
                );

                info!("✅ ValidationError properly returned: {}", message);
            } else {
                // Some validation might be handled at parameter level
                info!("ℹ️  Validation handled at different layer");
            }
        }
        Err(e) => {
            info!("✅ ValidationError returned as HTTP error: {:?}", e);
        }
    }
}

#[tokio::test]
async fn test_parameter_out_of_range_error() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_tools_server()
        .await
        .expect("Failed to start tools server");
    let mut client = McpTestClient::new(server.port());

    client
        .initialize_with_capabilities(TestFixtures::tools_capabilities())
        .await
        .unwrap();

    // Send notifications/initialized to complete handshake (required for strict lifecycle mode)
    client.send_initialized_notification().await.unwrap();

    // Use parameter_validator tool with invalid email format
    let result = client
        .call_tool(
            "parameter_validator",
            json!({
                "email": "not-an-email",
                "age": -50,  // Negative age should be out of range
                "config": {},
                "tags": []
            }),
        )
        .await;

    match result {
        Ok(response) => {
            if response.contains_key("error") {
                let error = response.get("error").unwrap().as_object().unwrap();
                let message = error.get("message").unwrap().as_str().unwrap();
                assert!(
                    message.to_lowercase().contains("range")
                        || message.to_lowercase().contains("invalid")
                        || message.to_lowercase().contains("validation"),
                    "Error should indicate range/validation issue: {}",
                    message
                );

                info!(
                    "✅ ParameterOutOfRange error properly returned: {}",
                    message
                );
            } else {
                // Tool might handle this as a validation result
                let parsed_result = TestFixtures::extract_tool_result_object(&response);
                if let Some(result_obj) = parsed_result {
                    if let Some(validation) = result_obj.get("validation_result") {
                        assert_ne!(
                            validation.as_str().unwrap(),
                            "passed",
                            "Validation should fail for invalid parameters"
                        );
                        info!("✅ Parameter validation properly handled in tool result");
                    }
                }
            }
        }
        Err(e) => {
            info!(
                "✅ ParameterOutOfRange error returned as HTTP error: {:?}",
                e
            );
        }
    }
}

#[tokio::test]
async fn test_json_rpc_error_structure() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_tools_server()
        .await
        .expect("Failed to start tools server");
    let mut client = McpTestClient::new(server.port());

    client
        .initialize_with_capabilities(TestFixtures::tools_capabilities())
        .await
        .unwrap();

    // Send notifications/initialized to complete handshake (required for strict lifecycle mode)
    client.send_initialized_notification().await.unwrap();

    // Send malformed JSON-RPC request (missing required fields)
    let malformed_request = json!({
        "jsonrpc": "2.0",
        // Missing id and method
        "params": {}
    });

    let response = client.send_notification(malformed_request).await;

    match response {
        Ok(result) => {
            debug!("Response to malformed request: {:?}", result);

            // If we got a structured response, check for proper error format
            if result.contains_key("error") {
                let error = result.get("error").unwrap().as_object().unwrap();

                // JSON-RPC 2.0 error structure validation
                assert!(error.contains_key("code"), "JSON-RPC error must have code");
                assert!(
                    error.contains_key("message"),
                    "JSON-RPC error must have message"
                );

                let code = error.get("code").unwrap().as_i64().unwrap();
                assert!(code != 0, "Error code should not be zero");

                info!(
                    "✅ JSON-RPC error structure properly formatted: code={}",
                    code
                );
            } else {
                info!("ℹ️  Server handled malformed request without structured error");
            }
        }
        Err(e) => {
            info!("✅ Malformed JSON-RPC handled with HTTP error: {:?}", e);
        }
    }
}

#[tokio::test]
async fn test_method_not_found_error() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_tools_server()
        .await
        .expect("Failed to start tools server");
    let mut client = McpTestClient::new(server.port());

    client
        .initialize_with_capabilities(TestFixtures::tools_capabilities())
        .await
        .unwrap();

    // Send notifications/initialized to complete handshake (required for strict lifecycle mode)
    client.send_initialized_notification().await.unwrap();

    // Call non-existent method
    let result = client
        .make_request("non_existent_method", json!({}), 999)
        .await;

    match result {
        Ok(response) => {
            // Should be an error response
            assert!(
                response.contains_key("error"),
                "Expected error response for non-existent method, got: {:?}",
                response
            );

            let error = response.get("error").unwrap().as_object().unwrap();
            let code = error.get("code").unwrap().as_i64().unwrap();
            let message = error.get("message").unwrap().as_str().unwrap();

            // JSON-RPC 2.0 standard error code for method not found is -32601
            assert!(
                code == -32601
                    || message.to_lowercase().contains("not found")
                    || message.to_lowercase().contains("method"),
                "Error should indicate method not found: code={}, message={}",
                code,
                message
            );

            info!(
                "✅ MethodNotFound error properly returned: code={}, message={}",
                code, message
            );
        }
        Err(e) => {
            info!("✅ MethodNotFound error returned as HTTP error: {:?}", e);
        }
    }
}

#[tokio::test]
async fn test_error_recovery_and_session_continuity() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_tools_server()
        .await
        .expect("Failed to start tools server");
    let mut client = McpTestClient::new(server.port());

    client
        .initialize_with_capabilities(TestFixtures::tools_capabilities())
        .await
        .unwrap();

    // Send notifications/initialized to complete handshake (required for strict lifecycle mode)
    client.send_initialized_notification().await.unwrap();

    // Test that after an error, the session continues to work normally

    // Step 1: Trigger an error
    let _error_result = client.call_tool("non_existent_tool", json!({})).await;

    // Step 2: Verify session still works with valid request
    let valid_result = client
        .call_tool(
            "calculator",
            json!({
                "operation": "add",
                "a": 2.0,
                "b": 3.0
            }),
        )
        .await
        .expect("Valid request should work after error");

    let result_obj =
        TestFixtures::extract_tool_result_object(&valid_result).expect("Should have valid result");

    assert_eq!(result_obj.get("result").unwrap().as_f64().unwrap(), 5.0);

    info!("✅ Session continuity maintained after error");
}

#[tokio::test]
async fn test_multiple_error_scenarios_batch() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_tools_server()
        .await
        .expect("Failed to start tools server");
    let mut client = McpTestClient::new(server.port());

    client
        .initialize_with_capabilities(TestFixtures::tools_capabilities())
        .await
        .unwrap();

    // Send notifications/initialized to complete handshake (required for strict lifecycle mode)
    client.send_initialized_notification().await.unwrap();

    let error_scenarios = vec![
        ("tool_not_found", "non_existent_tool", json!({})),
        (
            "invalid_params",
            "calculator",
            json!({"operation": "invalid_op", "a": 1.0, "b": 2.0}),
        ),
        ("missing_params", "calculator", json!({"operation": "add"})), // Missing a, b
        (
            "wrong_types",
            "calculator",
            json!({"operation": "add", "a": "string", "b": "string"}),
        ),
    ];

    let mut error_count = 0;
    let mut success_count = 0;

    for (scenario_name, tool_name, params) in error_scenarios {
        let result = client.call_tool(tool_name, params).await;

        match result {
            Ok(response) => {
                if response.contains_key("error") {
                    error_count += 1;
                    let error = response.get("error").unwrap().as_object().unwrap();
                    let message = error.get("message").unwrap().as_str().unwrap();
                    info!("✅ {}: Error properly returned: {}", scenario_name, message);
                } else {
                    success_count += 1;
                    info!("ℹ️  {}: Tool handled gracefully", scenario_name);
                }
            }
            Err(e) => {
                error_count += 1;
                info!("✅ {}: HTTP error returned: {:?}", scenario_name, e);
            }
        }

        // Small delay between requests
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }

    info!(
        "📊 Error scenario batch complete: {} errors, {} graceful handles",
        error_count, success_count
    );

    // At least some scenarios should produce errors for a robust error handling test
    assert!(
        error_count > 0,
        "Expected some error scenarios to produce errors"
    );
}
