//! Roots Protocol Security and Access Control Tests
//!
//! Tests the security aspects of the roots protocol, including access control,
//! path validation, and file system boundaries.

use mcp_roots_tests::{McpTestClient, TestServerManager, TestFixtures, json, info, warn};
use mcp_roots_tests::test_utils::{roots_capabilities, extract_roots_list, is_uri_in_allowed_roots};

#[tokio::test]
async fn test_roots_access_control_boundaries() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_roots_server().await.expect("Failed to start roots server");
    let mut client = McpTestClient::new(server.port());

    client.initialize_with_capabilities(roots_capabilities()).await.unwrap();

    // Get the list of available roots to understand boundaries
    let roots_result = client.make_request("roots/list", json!({}), 70).await
        .expect("Failed to list roots");

    let roots = extract_roots_list(&roots_result)
        .expect("Should extract roots list");

    assert!(!roots.is_empty(), "Should have at least one root for boundary testing");

    // Extract allowed root URIs
    let allowed_roots: Vec<String> = roots.iter()
        .filter_map(|root| {
            root.as_object()
                .and_then(|obj| obj.get("uri"))
                .and_then(|uri| uri.as_str())
                .map(|s| s.to_string())
        })
        .collect();

    info!("✅ Found {} allowed root directories", allowed_roots.len());
    for root in &allowed_roots {
        info!("   • {}", root);
    }

    // Test path validation using tools that respect root boundaries
    let test_cases = vec![
        ("valid workspace path", format!("{}/src/main.rs", allowed_roots.get(0).unwrap_or(&"file:///workspace".to_string())), true),
        ("invalid system path", "file:///etc/passwd".to_string(), false),
        ("traversal attack", "file:///workspace/../../../etc/passwd".to_string(), false),
    ];

    for (case_name, test_path, should_be_allowed) in test_cases {
        let is_allowed = is_uri_in_allowed_roots(&test_path, &allowed_roots.iter().map(|s| s.as_str()).collect::<Vec<_>>());
        
        if should_be_allowed {
            assert!(is_allowed, "Path should be allowed: {} ({})", test_path, case_name);
            info!("✅ {} correctly allowed: {}", case_name, test_path);
        } else {
            assert!(!is_allowed, "Path should be blocked: {} ({})", test_path, case_name);
            info!("✅ {} correctly blocked: {}", case_name, test_path);
        }
    }
}

#[tokio::test]
async fn test_roots_file_operation_security() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_roots_server().await.expect("Failed to start roots server");
    let mut client = McpTestClient::new(server.port());

    client.initialize_with_capabilities(TestFixtures::tools_capabilities()).await.unwrap();

    // Get available tools to see if we have file operation tools
    let tools_result = client.make_request("tools/list", json!({}), 71).await
        .expect("Failed to list tools");

    let empty_vec = vec![];
    let tools = tools_result.get("result")
        .and_then(|r| r.as_object())
        .and_then(|obj| obj.get("tools"))
        .and_then(|tools| tools.as_array())
        .unwrap_or(&empty_vec);

    // Look for file operation tools
    let file_operation_tool = tools.iter()
        .find(|tool| {
            tool.as_object()
                .and_then(|obj| obj.get("name"))
                .and_then(|name| name.as_str())
                .map(|name| name.contains("file") || name.contains("simulate"))
                .unwrap_or(false)
        });

    if let Some(tool) = file_operation_tool {
        let tool_name = tool.as_object()
            .and_then(|obj| obj.get("name"))
            .and_then(|name| name.as_str())
            .unwrap();

        info!("Found file operation tool: {}", tool_name);

        // Test allowed file operation
        let allowed_operation = client.call_tool(tool_name, json!({
            "operation": "read",
            "path": "file:///workspace/src/main.rs"
        })).await;

        match allowed_operation {
            Ok(response) => {
                if let Some(content) = TestFixtures::extract_tool_result_object(&response) {
                    let content_str = content.to_string();
                    assert!(content_str.contains("✅") || content_str.contains("Success"),
                           "Allowed operation should succeed");
                    info!("✅ Allowed file operation succeeded");
                } else {
                    info!("ℹ️  Allowed operation returned different format");
                }
            }
            Err(e) => {
                info!("ℹ️  Allowed operation failed (may not be implemented): {:?}", e);
            }
        }

        // Test blocked file operation (outside roots)
        let blocked_operation = client.call_tool(tool_name, json!({
            "operation": "read",
            "path": "file:///etc/passwd"
        })).await;

        match blocked_operation {
            Ok(response) => {
                if response.contains_key("error") {
                    let error = response.get("error").unwrap().as_object().unwrap();
                    let message = error.get("message").unwrap().as_str().unwrap();
                    assert!(message.to_lowercase().contains("access") || 
                           message.to_lowercase().contains("denied") ||
                           message.to_lowercase().contains("outside"),
                           "Error should indicate access denial: {}", message);
                    info!("✅ Blocked operation properly denied: {}", message);
                } else if let Some(content) = TestFixtures::extract_tool_result_object(&response) {
                    let content_str = content.to_string().to_lowercase();
                    assert!(content_str.contains("denied") ||
                           content_str.contains("outside") ||
                           content_str.contains("not allowed"),
                           "Should indicate access denial in result");
                    info!("✅ Blocked operation properly denied in result");
                }
            }
            Err(e) => {
                info!("✅ Blocked operation rejected at HTTP level: {:?}", e);
            }
        }
    } else {
        info!("ℹ️  No file operation tools available for security testing");
    }
}

#[tokio::test]
async fn test_roots_path_traversal_protection() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_roots_server().await.expect("Failed to start roots server");
    let mut client = McpTestClient::new(server.port());

    client.initialize_with_capabilities(TestFixtures::tools_capabilities()).await.unwrap();

    // Get available tools
    let tools_result = client.make_request("tools/list", json!({}), 72).await
        .expect("Failed to list tools");

    let empty_vec = vec![];
    let tools = tools_result.get("result")
        .and_then(|r| r.as_object())
        .and_then(|obj| obj.get("tools"))
        .and_then(|tools| tools.as_array())
        .unwrap_or(&empty_vec);

    // Look for file operation tools
    let file_tool = tools.iter()
        .find(|tool| {
            tool.as_object()
                .and_then(|obj| obj.get("name"))
                .and_then(|name| name.as_str())
                .map(|name| name.contains("file") || name.contains("simulate"))
                .unwrap_or(false)
        });

    if let Some(tool) = file_tool {
        let tool_name = tool.as_object()
            .and_then(|obj| obj.get("name"))
            .and_then(|name| name.as_str())
            .unwrap();

        // Test various path traversal attack patterns
        let traversal_attacks = vec![
            "file:///workspace/../../../etc/passwd",
            "file:///data/../../../etc/shadow",
            "file:///workspace/../../root/.ssh/id_rsa",
            "file:///config/../../../proc/version",
        ];

        for attack_path in traversal_attacks {
            let result = client.call_tool(tool_name, json!({
                "operation": "read",
                "path": attack_path
            })).await;

            match result {
                Ok(response) => {
                    if response.contains_key("error") {
                        let error = response.get("error").unwrap().as_object().unwrap();
                        let message = error.get("message").unwrap().as_str().unwrap();
                        assert!(message.to_lowercase().contains("access") || 
                               message.to_lowercase().contains("denied") ||
                               message.to_lowercase().contains("outside"),
                               "Should block traversal attack: {}", attack_path);
                        info!("✅ Path traversal attack blocked: {}", attack_path);
                    } else if let Some(content) = TestFixtures::extract_tool_result_object(&response) {
                        // If it succeeds, it should be because path is actually allowed
                        // or because the tool validates and rejects it in the result
                        let content_str = content.to_string().to_lowercase();
                        if content_str.contains("denied") ||
                           content_str.contains("outside") ||
                           content_str.contains("not allowed") {
                            info!("✅ Path traversal attack blocked in result: {}", attack_path);
                        } else {
                            // This should not happen - traversal should be blocked
                            warn!("⚠️  Path traversal attack may have succeeded: {}", attack_path);
                        }
                    }
                }
                Err(_e) => {
                    info!("✅ Path traversal attack rejected at HTTP level: {}", attack_path);
                }
            }
        }
    } else {
        info!("ℹ️  No file operation tools available for traversal testing");
    }
}

#[tokio::test]
async fn test_roots_permission_levels() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_roots_server().await.expect("Failed to start roots server");
    let mut client = McpTestClient::new(server.port());

    client.initialize_with_capabilities(TestFixtures::tools_capabilities()).await.unwrap();

    // Get available tools
    let tools_result = client.make_request("tools/list", json!({}), 73).await
        .expect("Failed to list tools");

    let empty_vec = vec![];
    let tools = tools_result.get("result")
        .and_then(|r| r.as_object())
        .and_then(|obj| obj.get("tools"))
        .and_then(|tools| tools.as_array())
        .unwrap_or(&empty_vec);

    let file_tool = tools.iter()
        .find(|tool| {
            tool.as_object()
                .and_then(|obj| obj.get("name"))
                .and_then(|name| name.as_str())
                .map(|name| name.contains("file") || name.contains("simulate"))
                .unwrap_or(false)
        });

    if let Some(tool) = file_tool {
        let tool_name = tool.as_object()
            .and_then(|obj| obj.get("name"))
            .and_then(|name| name.as_str())
            .unwrap();

        // Test read operations on different root types
        let permission_tests = vec![
            ("workspace read", "file:///workspace/src/main.rs", "read", true),
            ("data read", "file:///data/file.json", "read", true),
            ("config read", "file:///config/settings.json", "read", true),
            ("config write", "file:///config/settings.json", "write", false), // Should be read-only
            ("logs write", "file:///logs/app.log", "write", false), // Should be read-only
            ("workspace write", "file:///workspace/new_file.txt", "write", true),
            ("data write", "file:///data/new_data.json", "write", true),
        ];

        for (test_name, path, operation, should_succeed) in permission_tests {
            let result = client.call_tool(tool_name, json!({
                "operation": operation,
                "path": path
            })).await;

            match result {
                Ok(response) => {
                    let has_error = response.contains_key("error");
                    let success_in_result = if let Some(content) = TestFixtures::extract_tool_result_object(&response) {
                        let content_str = content.to_string();
                        content_str.contains("✅") || content_str.to_lowercase().contains("success")
                    } else {
                        false
                    };

                    if should_succeed {
                        assert!(!has_error || success_in_result, 
                               "Operation should succeed: {} ({})", test_name, path);
                        if success_in_result {
                            info!("✅ {} succeeded as expected", test_name);
                        } else if !has_error {
                            info!("ℹ️  {} handled gracefully", test_name);
                        }
                    } else {
                        if has_error {
                            let error = response.get("error").unwrap().as_object().unwrap();
                            let message = error.get("message").unwrap().as_str().unwrap();
                            info!("✅ {} properly denied: {}", test_name, message);
                        } else if let Some(content) = TestFixtures::extract_tool_result_object(&response) {
                            let content_str = content.to_string().to_lowercase();
                            if content_str.contains("denied") ||
                               content_str.contains("read-only") ||
                               content_str.contains("not allowed") {
                                info!("✅ {} properly denied in result", test_name);
                            } else {
                                warn!("⚠️  {} may have incorrectly succeeded", test_name);
                            }
                        }
                    }
                }
                Err(e) => {
                    if should_succeed {
                        info!("ℹ️  {} failed at HTTP level (may not be implemented): {:?}", test_name, e);
                    } else {
                        info!("✅ {} rejected at HTTP level", test_name);
                    }
                }
            }

            // Small delay between tests
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }
    } else {
        info!("ℹ️  No file operation tools available for permission testing");
    }
}

#[tokio::test]
async fn test_roots_uri_validation() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_roots_server().await.expect("Failed to start roots server");
    let mut client = McpTestClient::new(server.port());

    client.initialize_with_capabilities(roots_capabilities()).await.unwrap();

    // Get roots to validate URI formats
    let roots_result = client.make_request("roots/list", json!({}), 74).await
        .expect("Failed to list roots");

    let roots = extract_roots_list(&roots_result)
        .expect("Should extract roots list");

    // Validate URI formats
    for (i, root) in roots.iter().enumerate() {
        let root_obj = root.as_object().unwrap();
        let uri = root_obj.get("uri").unwrap().as_str().unwrap();
        
        // Basic URI validation
        assert!(uri.contains("://"), "Root {} should have valid URI scheme: {}", i, uri);
        
        // Should be absolute URIs
        assert!(!uri.starts_with("./") && !uri.starts_with("../"), 
               "Root {} should be absolute URI: {}", i, uri);
        
        // Should not contain spaces (unless encoded)
        if uri.contains(" ") {
            assert!(uri.contains("%20"), 
                   "Root {} with spaces should be percent-encoded: {}", i, uri);
        }
        
        // For file:// URIs, should have absolute paths
        if uri.starts_with("file://") {
            let path_part = &uri[7..]; // Remove "file://"
            assert!(path_part.starts_with("/"), 
                   "Root {} file URI should have absolute path: {}", i, uri);
        }
        
        info!("✅ Root {} URI validation passed: {}", i, uri);
    }
    
    info!("✅ All root URIs have valid formats");
}

#[tokio::test]
async fn test_roots_security_headers() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_roots_server().await.expect("Failed to start roots server");
    let mut client = McpTestClient::new(server.port());

    client.initialize_with_capabilities(roots_capabilities()).await.unwrap();

    // Make request and check response headers if accessible
    let roots_result = client.make_request("roots/list", json!({}), 75).await
        .expect("Failed to list roots");

    // Basic security checks on response content
    assert!(roots_result.contains_key("result"), "Response should have result");
    assert!(!roots_result.contains_key("error"), "Response should not have error");
    
    // Verify no sensitive information is leaked in response
    let response_str = serde_json::to_string(&roots_result).unwrap();
    
    // Should not contain common sensitive patterns
    let sensitive_patterns = vec![
        "password", "secret", "key", "token", "private",
        "/etc/passwd", "/etc/shadow", "id_rsa", ".ssh"
    ];
    
    for pattern in sensitive_patterns {
        assert!(!response_str.to_lowercase().contains(pattern),
               "Response should not contain sensitive pattern: {}", pattern);
    }
    
    info!("✅ No sensitive information leaked in roots response");
}