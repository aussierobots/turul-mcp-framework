//! Resource Templates E2E Testing
//!
//! Comprehensive testing for resources/templates/list endpoint
//! Addresses Codex Review Action Item #4: Resource templates E2E coverage

use mcp_e2e_shared::{McpTestClient, TestServerManager, TestFixtures};
use serde_json::json;
use tracing::{debug, info, warn};

#[tokio::test]
async fn test_resource_templates_list_endpoint() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_resource_server().await.expect("Failed to start resource server");
    let mut client = McpTestClient::new(server.port());

    // Initialize with resource capabilities
    client.initialize_with_capabilities(TestFixtures::resource_capabilities()).await.unwrap();

    // Call resources/templates/list endpoint
    let templates_result = client.make_request("resources/templates/list", json!({}), 10).await
        .expect("Failed to list resource templates");

    debug!("Resource templates result: {:?}", templates_result);

    // Verify response structure
    assert!(templates_result.contains_key("result"), "Response should contain 'result'");
    let result = templates_result.get("result").unwrap().as_object().unwrap();

    // Debug: Print the actual response structure
    debug!("Actual result keys: {:?}", result.keys().collect::<Vec<_>>());
    debug!("Full result object: {:#?}", result);

    // Check for resourceTemplates array (handle different possible field names)
    let templates = if result.contains_key("resourceTemplates") {
        result.get("resourceTemplates").unwrap().as_array().unwrap()
    } else if result.contains_key("templates") {
        result.get("templates").unwrap().as_array().unwrap()
    } else if result.contains_key("resources") {
        // Fallback to resources if templates not available
        warn!("⚠️  'resourceTemplates' not found, using 'resources' field");
        result.get("resources").unwrap().as_array().unwrap()
    } else {
        panic!("Result should contain 'resourceTemplates', 'templates', or 'resources'. Found keys: {:?}", result.keys().collect::<Vec<_>>());
    };

    info!("✅ Found {} resource templates", templates.len());

    // Verify each template has required fields
    for (i, template) in templates.iter().enumerate() {
        let template_obj = template.as_object().unwrap();
        
        // Required fields per MCP spec
        assert!(template_obj.contains_key("uriTemplate"), 
               "Template {} missing required 'uriTemplate' field", i);
        assert!(template_obj.contains_key("name"), 
               "Template {} missing required 'name' field", i);
        
        // Optional but commonly present fields
        if template_obj.contains_key("description") {
            assert!(template_obj.get("description").unwrap().is_string(),
                   "Template {} description should be string", i);
        }

        if template_obj.contains_key("mimeType") {
            assert!(template_obj.get("mimeType").unwrap().is_string(),
                   "Template {} mimeType should be string", i);
        }

        let uri_template = template_obj.get("uriTemplate").unwrap().as_str().unwrap();
        let name = template_obj.get("name").unwrap().as_str().unwrap();
        
        // Verify URI template format (should contain variables like {id})
        assert!(uri_template.contains("://"), 
               "Template {} URI should have valid scheme: {}", i, uri_template);
        
        info!("✅ Template {}: '{}' -> '{}'", i, name, uri_template);
    }

    // Verify at least one template exists (resource-test-server should provide templates)
    assert!(!templates.is_empty(), "Should have at least one resource template");
}

#[tokio::test]
async fn test_resource_templates_list_with_pagination() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_resource_server().await.expect("Failed to start resource server");
    let mut client = McpTestClient::new(server.port());

    client.initialize_with_capabilities(TestFixtures::resource_capabilities()).await.unwrap();

    // Test pagination with cursor parameter
    let paginated_result = client.make_request("resources/templates/list", json!({
        "cursor": "test_cursor"
    }), 11).await.expect("Failed to list resource templates with cursor");

    debug!("Paginated templates result: {:?}", paginated_result);

    assert!(paginated_result.contains_key("result"), "Response should contain 'result'");
    let result = paginated_result.get("result").unwrap().as_object().unwrap();
    
    assert!(result.contains_key("resourceTemplates"), "Result should contain 'resourceTemplates'");

    // Check for pagination metadata if present
    if result.contains_key("nextCursor") {
        let next_cursor = result.get("nextCursor").unwrap();
        assert!(next_cursor.is_string() || next_cursor.is_null(),
               "nextCursor should be string or null");
        info!("✅ Pagination metadata present: nextCursor={:?}", next_cursor);
    }

    // Check for _meta field if present (MCP 2025-06-18 supports _meta)
    if result.contains_key("_meta") {
        let meta = result.get("_meta").unwrap().as_object().unwrap();
        info!("✅ Meta information present: {:?}", meta);
    }
}

#[tokio::test]
async fn test_resource_templates_structure_validation() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_resource_server().await.expect("Failed to start resource server");
    let mut client = McpTestClient::new(server.port());

    client.initialize_with_capabilities(TestFixtures::resource_capabilities()).await.unwrap();

    let templates_result = client.make_request("resources/templates/list", json!({}), 12).await
        .expect("Failed to list resource templates");

    let result = templates_result.get("result").unwrap().as_object().unwrap();
    let templates = result.get("resourceTemplates").unwrap().as_array().unwrap();

    // Detailed validation of template structure
    for template in templates {
        let template_obj = template.as_object().unwrap();
        
        // Validate uriTemplate format and structure
        let uri_template = template_obj.get("uriTemplate").unwrap().as_str().unwrap();
        
        // Should be valid URI with template variables
        assert!(uri_template.starts_with("file://") || 
               uri_template.starts_with("memory://") || 
               uri_template.starts_with("template://") ||
               uri_template.starts_with("http://") ||
               uri_template.starts_with("https://"),
               "URI template should have valid scheme: {}", uri_template);

        // Should contain template variables (RFC 6570 style)
        if uri_template.contains("{") && uri_template.contains("}") {
            info!("✅ Template contains variables: {}", uri_template);
            
            // Basic validation of variable syntax
            let open_count = uri_template.matches('{').count();
            let close_count = uri_template.matches('}').count();
            assert_eq!(open_count, close_count, 
                      "Mismatched braces in URI template: {}", uri_template);
        }

        // Validate name field
        let name = template_obj.get("name").unwrap().as_str().unwrap();
        assert!(!name.is_empty(), "Template name should not be empty");
        
        // Check for additional MCP-compliant fields
        if let Some(description) = template_obj.get("description") {
            assert!(description.is_string(), "Description should be string");
            assert!(!description.as_str().unwrap().is_empty(), 
                   "Description should not be empty if present");
        }
        
        info!("✅ Template validation passed: {}", name);
    }
}

#[tokio::test]
async fn test_resource_templates_uri_variable_patterns() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_resource_server().await.expect("Failed to start resource server");
    let mut client = McpTestClient::new(server.port());

    client.initialize_with_capabilities(TestFixtures::resource_capabilities()).await.unwrap();

    let templates_result = client.make_request("resources/templates/list", json!({}), 13).await
        .expect("Failed to list resource templates");

    let result = templates_result.get("result").unwrap().as_object().unwrap();
    let templates = result.get("resourceTemplates").unwrap().as_array().unwrap();

    let mut variable_patterns_found = Vec::new();

    // Analyze URI template patterns
    for template in templates {
        let template_obj = template.as_object().unwrap();
        let uri_template = template_obj.get("uriTemplate").unwrap().as_str().unwrap();
        let name = template_obj.get("name").unwrap().as_str().unwrap();

        // Extract variable patterns
        let mut variables = Vec::new();
        let mut in_variable = false;
        let mut current_var = String::new();
        
        for char in uri_template.chars() {
            match char {
                '{' => {
                    in_variable = true;
                    current_var.clear();
                }
                '}' => {
                    if in_variable && !current_var.is_empty() {
                        variables.push(current_var.clone());
                    }
                    in_variable = false;
                }
                c if in_variable => {
                    current_var.push(c);
                }
                _ => {}
            }
        }

        if !variables.is_empty() {
            variable_patterns_found.push((name.to_string(), uri_template.to_string(), variables.clone()));
            info!("✅ Template '{}' has variables: {:?}", name, variables);
        }
    }

    // Verify we found some templates with variables
    assert!(!variable_patterns_found.is_empty(), 
           "Should find at least one template with URI variables");

    // Common variable patterns validation
    for (name, _uri_template, variables) in variable_patterns_found {
        for var in variables {
            // Variables should be valid identifiers
            assert!(!var.is_empty(), "Variable name should not be empty in template '{}'", name);
            assert!(var.chars().all(|c| c.is_alphanumeric() || c == '_'), 
                   "Variable '{}' should be alphanumeric in template '{}'", var, name);
            
            info!("✅ Valid variable pattern '{}' in template '{}'", var, name);
        }
    }
}

#[tokio::test]
async fn test_resource_templates_json_rpc_compliance() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_resource_server().await.expect("Failed to start resource server");
    let mut client = McpTestClient::new(server.port());

    client.initialize_with_capabilities(TestFixtures::resource_capabilities()).await.unwrap();

    let templates_result = client.make_request("resources/templates/list", json!({}), 14).await
        .expect("Failed to list resource templates");

    // Verify JSON-RPC 2.0 compliance
    assert!(templates_result.contains_key("jsonrpc"), "Response should contain 'jsonrpc'");
    assert_eq!(templates_result.get("jsonrpc").unwrap().as_str().unwrap(), "2.0",
              "JSON-RPC version should be 2.0");

    assert!(templates_result.contains_key("id"), "Response should contain 'id'");
    assert_eq!(templates_result.get("id").unwrap().as_i64().unwrap(), 14,
              "Response ID should match request ID");

    assert!(templates_result.contains_key("result"), "Response should contain 'result'");
    assert!(!templates_result.contains_key("error"), "Successful response should not contain 'error'");

    info!("✅ Resource templates endpoint fully JSON-RPC 2.0 compliant");
}

#[tokio::test]
async fn test_resource_templates_error_handling() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_resource_server().await.expect("Failed to start resource server");
    let mut client = McpTestClient::new(server.port());

    client.initialize_with_capabilities(TestFixtures::resource_capabilities()).await.unwrap();

    // Test with invalid parameters (if any)
    let invalid_result = client.make_request("resources/templates/list", json!({
        "invalid_param": "should_be_ignored"
    }), 15).await;

    match invalid_result {
        Ok(response) => {
            // Should either succeed (ignoring invalid params) or return structured error
            if response.contains_key("error") {
                let error = response.get("error").unwrap().as_object().unwrap();
                assert!(error.contains_key("code"), "Error should have code");
                assert!(error.contains_key("message"), "Error should have message");
                info!("✅ Invalid parameters properly handled with error response");
            } else {
                assert!(response.contains_key("result"), "Should have result if no error");
                info!("✅ Invalid parameters gracefully ignored");
            }
        }
        Err(e) => {
            info!("✅ Invalid parameters rejected at HTTP level: {:?}", e);
        }
    }

    // Verify server is still responsive after error
    let recovery_test = client.make_request("resources/templates/list", json!({}), 16).await
        .expect("Server should be responsive after error handling");
    
    assert!(recovery_test.contains_key("result"), "Server should recover from error handling");
    info!("✅ Server remains responsive after error handling");
}