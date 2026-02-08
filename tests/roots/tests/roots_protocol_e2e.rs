//! End-to-End Tests for MCP Roots Protocol
//!
//! Tests the roots/list endpoint implementation and root directory discovery.
//! Validates protocol compliance and file system access control.

use mcp_roots_tests::test_utils::{extract_roots_list, roots_capabilities, validate_root_object};
use mcp_roots_tests::{debug, info, json, McpTestClient, TestServerManager};

#[tokio::test]
async fn test_roots_list_endpoint() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_roots_server()
        .await
        .expect("Failed to start roots server");
    let mut client = McpTestClient::new(server.port());

    // Initialize with roots capabilities
    client
        .initialize_with_capabilities(roots_capabilities())
        .await
        .unwrap();

    // Call roots/list endpoint
    let roots_result = client
        .make_request("roots/list", json!({}), 50)
        .await
        .expect("Failed to list roots");

    debug!("Roots result: {:?}", roots_result);

    // Verify response structure
    assert!(
        roots_result.contains_key("result"),
        "Response should contain 'result'"
    );
    let result = roots_result.get("result").unwrap().as_object().unwrap();

    // Check for roots array
    assert!(
        result.contains_key("roots"),
        "Result should contain 'roots'"
    );
    let roots = result.get("roots").unwrap().as_array().unwrap();

    info!("✅ Found {} root directories", roots.len());

    // Verify each root has required fields
    for (i, root) in roots.iter().enumerate() {
        assert!(
            validate_root_object(root),
            "Root {} should have valid structure",
            i
        );

        let root_obj = root.as_object().unwrap();
        let uri = root_obj.get("uri").unwrap().as_str().unwrap();
        let name = root_obj
            .get("name")
            .map(|n| n.as_str().unwrap())
            .unwrap_or("unnamed");

        // Verify URI has valid scheme
        assert!(
            uri.contains("://"),
            "Root {} URI should have valid scheme: {}",
            i,
            uri
        );

        info!("✅ Root {}: '{}' -> '{}'", i, name, uri);
    }

    // Verify at least one root exists (roots server should provide roots)
    assert!(!roots.is_empty(), "Should have at least one root directory");
}

#[tokio::test]
async fn test_roots_list_with_pagination() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_roots_server()
        .await
        .expect("Failed to start roots server");
    let mut client = McpTestClient::new(server.port());

    client
        .initialize_with_capabilities(roots_capabilities())
        .await
        .unwrap();

    // Test pagination with cursor parameter
    let paginated_result = client
        .make_request(
            "roots/list",
            json!({
                "cursor": "test_cursor"
            }),
            51,
        )
        .await
        .expect("Failed to list roots with cursor");

    debug!("Paginated roots result: {:?}", paginated_result);

    assert!(
        paginated_result.contains_key("result"),
        "Response should contain 'result'"
    );
    let result = paginated_result.get("result").unwrap().as_object().unwrap();

    assert!(
        result.contains_key("roots"),
        "Result should contain 'roots'"
    );

    // Check for pagination metadata if present
    if result.contains_key("nextCursor") {
        let next_cursor = result.get("nextCursor").unwrap();
        assert!(
            next_cursor.is_string() || next_cursor.is_null(),
            "nextCursor should be string or null"
        );
        info!(
            "✅ Pagination metadata present: nextCursor={:?}",
            next_cursor
        );
    }

    // Check for _meta field if present (MCP supports _meta)
    if result.contains_key("_meta") {
        let meta = result.get("_meta").unwrap().as_object().unwrap();
        info!("✅ Meta information present: {:?}", meta);
    }
}

#[tokio::test]
async fn test_roots_structure_validation() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_roots_server()
        .await
        .expect("Failed to start roots server");
    let mut client = McpTestClient::new(server.port());

    client
        .initialize_with_capabilities(roots_capabilities())
        .await
        .unwrap();

    let roots_result = client
        .make_request("roots/list", json!({}), 52)
        .await
        .expect("Failed to list roots");

    let result = roots_result.get("result").unwrap().as_object().unwrap();
    let roots = result.get("roots").unwrap().as_array().unwrap();

    // Detailed validation of root structure
    let mut workspace_found = false;
    let mut data_found = false;
    let mut config_found = false;

    for root in roots {
        let root_obj = root.as_object().unwrap();

        // Validate URI format and structure
        let uri = root_obj.get("uri").unwrap().as_str().unwrap();

        // Should be valid URI with file:// scheme (for file system roots)
        assert!(
            uri.starts_with("file://"),
            "URI should have file:// scheme: {}",
            uri
        );

        // Check for expected root patterns
        if uri.contains("workspace") {
            workspace_found = true;
        } else if uri.contains("data") {
            data_found = true;
        } else if uri.contains("config") {
            config_found = true;
        }

        // Validate name field if present
        if let Some(name) = root_obj.get("name") {
            let name_str = name.as_str().unwrap();
            assert!(
                !name_str.is_empty(),
                "Root name should not be empty if present"
            );
        }

        info!("✅ Root validation passed: {}", uri);
    }

    // Verify we found expected root types
    assert!(
        workspace_found || data_found || config_found,
        "Should find at least one standard root type (workspace, data, or config)"
    );
    info!("✅ Found expected root directory types");
}

#[tokio::test]
async fn test_roots_json_rpc_compliance() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_roots_server()
        .await
        .expect("Failed to start roots server");
    let mut client = McpTestClient::new(server.port());

    client
        .initialize_with_capabilities(roots_capabilities())
        .await
        .unwrap();

    let roots_result = client
        .make_request("roots/list", json!({}), 53)
        .await
        .expect("Failed to list roots");

    // Verify JSON-RPC 2.0 compliance
    assert!(
        roots_result.contains_key("jsonrpc"),
        "Response should contain 'jsonrpc'"
    );
    assert_eq!(
        roots_result.get("jsonrpc").unwrap().as_str().unwrap(),
        "2.0",
        "JSON-RPC version should be 2.0"
    );

    assert!(
        roots_result.contains_key("id"),
        "Response should contain 'id'"
    );
    assert_eq!(
        roots_result.get("id").unwrap().as_i64().unwrap(),
        53,
        "Response ID should match request ID"
    );

    assert!(
        roots_result.contains_key("result"),
        "Response should contain 'result'"
    );
    assert!(
        !roots_result.contains_key("error"),
        "Successful response should not contain 'error'"
    );

    info!("✅ Roots endpoint fully JSON-RPC 2.0 compliant");
}

#[tokio::test]
async fn test_roots_capability_advertising() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_roots_server()
        .await
        .expect("Failed to start roots server");
    let mut client = McpTestClient::new(server.port());

    // Initialize and check if roots capabilities are advertised
    let init_result = client
        .initialize_with_capabilities(roots_capabilities())
        .await
        .unwrap();

    debug!("Initialize result: {:?}", init_result);

    // Check if server advertises roots capabilities
    if let Some(server_capabilities) = init_result.get("capabilities") {
        if let Some(roots_cap) = server_capabilities.get("roots") {
            info!("✅ Server advertises roots capabilities: {:?}", roots_cap);

            // Check listChanged capability
            if let Some(list_changed) = roots_cap.get("listChanged") {
                assert!(
                    !list_changed.as_bool().unwrap(),
                    "Static roots server should advertise listChanged=false"
                );
                info!("✅ Correct listChanged capability: false");
            }
        } else {
            info!("ℹ️  Server does not advertise roots capabilities (may be implicit)");
        }
    }

    // Verify the endpoint is actually available by calling it
    let test_result = client.make_request("roots/list", json!({}), 54).await;

    match test_result {
        Ok(response) => {
            if response.contains_key("result") {
                info!("✅ Roots endpoint is available and functional");
            } else {
                info!("ℹ️  Roots endpoint responded but with different format");
            }
        }
        Err(e) => {
            panic!(
                "Roots endpoint should be available if server is running: {:?}",
                e
            );
        }
    }
}

#[tokio::test]
async fn test_roots_error_handling() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_roots_server()
        .await
        .expect("Failed to start roots server");
    let mut client = McpTestClient::new(server.port());

    client
        .initialize_with_capabilities(roots_capabilities())
        .await
        .unwrap();

    // Test with invalid parameters (if any)
    let invalid_result = client
        .make_request(
            "roots/list",
            json!({
                "invalid_param": "should_be_ignored"
            }),
            55,
        )
        .await;

    match invalid_result {
        Ok(response) => {
            // Should either succeed (ignoring invalid params) or return structured error
            if response.contains_key("error") {
                let error = response.get("error").unwrap().as_object().unwrap();
                assert!(error.contains_key("code"), "Error should have code");
                assert!(error.contains_key("message"), "Error should have message");
                info!("✅ Invalid parameters properly handled with error response");
            } else {
                assert!(
                    response.contains_key("result"),
                    "Should have result if no error"
                );
                info!("✅ Invalid parameters gracefully ignored");
            }
        }
        Err(e) => {
            info!("✅ Invalid parameters rejected at HTTP level: {:?}", e);
        }
    }

    // Verify server is still responsive after error
    let recovery_test = client
        .make_request("roots/list", json!({}), 56)
        .await
        .expect("Server should be responsive after error handling");

    assert!(
        recovery_test.contains_key("result"),
        "Server should recover from error handling"
    );
    info!("✅ Server remains responsive after error handling");
}

#[tokio::test]
async fn test_roots_session_continuity() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_roots_server()
        .await
        .expect("Failed to start roots server");
    let mut client = McpTestClient::new(server.port());

    client
        .initialize_with_capabilities(roots_capabilities())
        .await
        .unwrap();

    // Test multiple roots requests in the same session
    for i in 1..=3 {
        let result = client
            .make_request("roots/list", json!({}), 56 + i)
            .await
            .expect("Roots request should succeed");

        assert!(
            result.contains_key("result"),
            "Each request should get a result"
        );

        let roots = extract_roots_list(&result).expect("Should extract roots list");

        assert!(!roots.is_empty(), "Should have roots in request {}", i);

        info!(
            "✅ Request {} completed successfully with {} roots",
            i,
            roots.len()
        );
    }

    info!("✅ Session continuity maintained across multiple roots requests");
}

#[tokio::test]
async fn test_roots_concurrent_access() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_roots_server()
        .await
        .expect("Failed to start roots server");

    // Create multiple clients for concurrent requests
    let mut clients = Vec::new();
    for _ in 0..3 {
        let mut client = McpTestClient::new(server.port());
        client
            .initialize_with_capabilities(roots_capabilities())
            .await
            .unwrap();
        clients.push(client);
    }

    // Send concurrent requests
    let mut handles = Vec::new();
    for (i, client) in clients.into_iter().enumerate() {
        let handle = tokio::spawn(async move {
            client
                .make_request("roots/list", json!({}), 60 + i as u64)
                .await
        });
        handles.push((i, handle));
    }

    // Wait for all requests to complete
    let mut successes = 0;

    for (i, handle) in handles {
        match handle.await {
            Ok(Ok(response)) => {
                if response.contains_key("result") {
                    successes += 1;
                    let roots = extract_roots_list(&response).unwrap_or_default();
                    info!(
                        "✅ Concurrent request {} succeeded with {} roots",
                        i + 1,
                        roots.len()
                    );
                }
            }
            Ok(Err(e)) => {
                info!("⚠️  Concurrent request {} failed: {:?}", i + 1, e);
            }
            Err(join_error) => {
                info!(
                    "⚠️  Concurrent request {} join error: {:?}",
                    i + 1,
                    join_error
                );
            }
        }
    }

    assert!(
        successes > 0,
        "At least one concurrent request should succeed"
    );
    info!(
        "📊 Concurrent roots access: {} successful requests",
        successes
    );
}
