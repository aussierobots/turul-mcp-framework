//! MCP Resources Integration Tests
//!
//! Comprehensive integration tests for all MCP 2025-11-25 resources features
//! including pagination, security, URI templates, and notification integration.

use turul_mcp_server::prelude::*;
use turul_mcp_protocol::resources::*;
use turul_mcp_protocol::meta::*;
use turul_mcp_server::handlers::{ResourcesListHandler, ResourcesReadHandler};
use turul_mcp_server::security::{SecurityMiddleware, RateLimitConfig, ResourceAccessControl, AccessLevel};
use turul_mcp_server::uri_template::{UriTemplate, UriTemplateRegistry};
use turul_mcp_derive::McpResource;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

// Test helper resource for pagination testing
#[derive(McpResource, Clone)]
#[resource(
    name = "test_pagination_resource",
    description = "Test resource for pagination"
)]
struct TestPaginationResource {
    pub id: String,
}

impl TestPaginationResource {
    fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }

    async fn execute(&self, _params: Option<Value>) -> McpResult<Vec<ResourceContent>> {
        Ok(vec![ResourceContent::text(
            &format!("test://resource/{}", self.id),
            &format!("Test resource data for ID: {}", self.id)
        )])
    }
}

// Test helper resource for URI template testing
#[derive(McpResource, Clone)]
#[resource(
    name = "template_test_resource",
    description = "Test resource for URI templates"
)]
struct TemplateTestResource {
    pub category: String,
    pub item_id: String,
}

impl TemplateTestResource {
    fn new(category: impl Into<String>, item_id: impl Into<String>) -> Self {
        Self {
            category: category.into(),
            item_id: item_id.into(),
        }
    }

    async fn execute(&self, params: Option<Value>) -> McpResult<Vec<ResourceContent>> {
        // Extract template variables if provided
        let template_vars = params
            .as_ref()
            .and_then(|p| p.get("template_variables"))
            .and_then(|tv| tv.as_object());

        let response_text = if let Some(vars) = template_vars {
            format!(
                "Template resource - category: {}, item_id: {}, template_vars: {:?}",
                self.category, self.item_id, vars
            )
        } else {
            format!(
                "Template resource - category: {}, item_id: {}",
                self.category, self.item_id
            )
        };

        Ok(vec![ResourceContent::text(
            &format!("test://items/{}/{}", self.category, self.item_id),
            &response_text
        )])
    }
}

#[tokio::test]
async fn test_resources_pagination_integration() {
    // === Test Complete Pagination Integration ===

    // Create multiple test resources for pagination
    let mut handler = ResourcesListHandler::new();

    // Add resources with predictable ordering (sorted by URI)
    for i in 1..=100 {
        let resource = TestPaginationResource::new(format!("item_{:03}", i));
        handler = handler.add_resource(resource);
    }

    // Test page 1 (no cursor)
    let page1_params = json!({});
    let page1_response = handler.handle(Some(page1_params)).await.unwrap();
    let page1_data: PaginatedResponse<ListResourcesResult> =
        serde_json::from_value(page1_response).unwrap();

    // Validate page 1 results
    assert_eq!(page1_data.data.resources.len(), 50); // Default page size
    assert!(page1_data.meta.is_some());

    let page1_meta = page1_data.meta.as_ref().unwrap();
    assert_eq!(page1_meta.total, Some(100));
    assert_eq!(page1_meta.has_more, Some(true));
    assert!(page1_meta.cursor.is_some());

    // Verify stable ordering (sorted by URI)
    let first_uri = &page1_data.data.resources[0].uri;
    let last_uri = &page1_data.data.resources[49].uri;
    assert!(first_uri < last_uri);

    // Test page 2 using cursor from page 1
    let cursor = page1_meta.cursor.as_ref().unwrap();
    let page2_params = json!({
        "cursor": cursor.as_str()
    });
    let page2_response = handler.handle(Some(page2_params)).await.unwrap();
    let page2_data: PaginatedResponse<ListResourcesResult> =
        serde_json::from_value(page2_response).unwrap();

    // Validate page 2 results
    assert_eq!(page2_data.data.resources.len(), 50); // Remaining items
    assert!(page2_data.meta.is_some());

    let page2_meta = page2_data.meta.as_ref().unwrap();
    assert_eq!(page2_meta.total, Some(100));
    assert_eq!(page2_meta.has_more, Some(false)); // No more pages
    assert!(page2_meta.cursor.is_none()); // No next cursor when no more items

    // Verify no overlap between pages
    let page1_uris: Vec<&String> = page1_data.data.resources.iter().map(|r| &r.uri).collect();
    let page2_uris: Vec<&String> = page2_data.data.resources.iter().map(|r| &r.uri).collect();

    for page1_uri in &page1_uris {
        assert!(!page2_uris.contains(page1_uri), "No overlap between pages");
    }

    // Verify pagination consistency (all items from page 1 < all items from page 2)
    let max_page1_uri = page1_uris.iter().max().unwrap();
    let min_page2_uri = page2_uris.iter().min().unwrap();
    assert!(max_page1_uri < min_page2_uri, "Page ordering consistency");
}

#[tokio::test]
async fn test_uri_template_integration() {
    // === Test Complete URI Template Integration ===

    // Create URI template for dynamic resources
    let template = UriTemplate::new("test://items/{category}/{item_id}")
        .expect("Valid URI template");

    // Create handler with template resource
    let template_resource = TemplateTestResource::new("electronics", "laptop_001");
    let mut handler = ResourcesReadHandler::new()
        .add_template_resource(template.clone(), template_resource);

    // Test URI template matching and variable extraction
    let read_params = json!({
        "uri": "test://items/electronics/laptop_001"
    });

    let response = handler.handle(Some(read_params)).await.unwrap();
    let read_result: ReadResourceResult = serde_json::from_value(response).unwrap();

    // Validate template resolution worked
    assert_eq!(read_result.contents.len(), 1);
    match &read_result.contents[0] {
        ResourceContent::Text(text_content) => {
            assert!(text_content.text.contains("Template resource"));
            assert!(text_content.text.contains("category: electronics"));
            assert!(text_content.text.contains("item_id: laptop_001"));
            assert!(text_content.text.contains("template_vars:"));
        }
        _ => panic!("Expected text resource content")
    }

    // Test URI template with different variables
    let read_params2 = json!({
        "uri": "test://items/books/novel_042"
    });

    let response2 = handler.handle(Some(read_params2)).await.unwrap();
    let read_result2: ReadResourceResult = serde_json::from_value(response2).unwrap();

    match &read_result2.contents[0] {
        ResourceContent::Text(text_content) => {
            // Template should extract new variables
            assert!(text_content.text.contains("template_vars:"));
            // The template variables should include the extracted values
            assert!(text_content.text.contains("books") || text_content.text.contains("novel_042"));
        }
        _ => panic!("Expected text resource content")
    }
}

#[tokio::test]
async fn test_security_middleware_integration() {
    // === Test Complete Security Middleware Integration ===

    // Create security middleware with restrictions
    let mut access_control = ResourceAccessControl::new();
    access_control.set_access_level(AccessLevel::ReadOnly);
    access_control.add_allowed_mime_type("text/plain");
    access_control.set_max_content_size(1024); // 1KB limit

    let rate_limit = RateLimitConfig::new()
        .with_requests_per_minute(10)
        .with_burst_capacity(5);

    let security_middleware = SecurityMiddleware::new()
        .with_rate_limiting(rate_limit)
        .with_resource_access_control(access_control);

    // Create handler with security middleware
    let test_resource = TestPaginationResource::new("secure_test");
    let handler = ResourcesReadHandler::new()
        .with_security(Arc::new(security_middleware))
        .add_resource(test_resource);

    // Test valid request passes security
    let read_params = json!({
        "uri": "test://resource/secure_test"
    });

    let response = handler.handle(Some(read_params)).await;
    assert!(response.is_ok(), "Valid request should pass security");

    // Test security validation (this should work since our test resource produces small text content)
    let read_result: ReadResourceResult = serde_json::from_value(response.unwrap()).unwrap();
    assert!(!read_result.contents.is_empty());
}

#[tokio::test]
async fn test_notification_integration_with_session() {
    // === Test Notification Integration with Session Context ===

    // This test verifies that resource operations can send notifications
    // when executed within a session context

    let test_resource = TestPaginationResource::new("notification_test");
    let handler = ResourcesReadHandler::new()
        .add_resource(test_resource);

    // Test without session context (should work without notifications)
    let read_params = json!({
        "uri": "test://resource/notification_test"
    });

    let response = handler.handle_with_session(Some(read_params.clone()), None).await;
    assert!(response.is_ok(), "Resource read should work without session");

    // Note: Full session context testing requires session manager setup
    // which is tested in other integration test suites. This verifies
    // the handler interface supports session-aware operations.
}

#[tokio::test]
async fn test_end_to_end_resource_workflow() {
    // === Test Complete End-to-End Resource Workflow ===

    // Create a complete resource server setup
    let mut list_handler = ResourcesListHandler::new();
    let mut read_handler = ResourcesReadHandler::new();

    // Add multiple resource types
    for i in 1..=25 {
        let resource = TestPaginationResource::new(format!("workflow_{:02}", i));
        list_handler = list_handler.add_resource(resource.clone());
        read_handler = read_handler.add_resource(resource);
    }

    // Add template resources
    let template = UriTemplate::new("test://dynamic/{category}/{id}")
        .expect("Valid URI template");
    let template_resource = TemplateTestResource::new("test_cat", "test_id");
    read_handler = read_handler.add_template_resource(template, template_resource);

    // Step 1: List resources (pagination test)
    let list_response = list_handler.handle(None).await.unwrap();
    let list_data: PaginatedResponse<ListResourcesResult> =
        serde_json::from_value(list_response).unwrap();

    assert_eq!(list_data.data.resources.len(), 25); // All fit in one page
    assert!(list_data.meta.is_some());
    assert_eq!(list_data.meta.as_ref().unwrap().has_more, Some(false));

    // Step 2: Read specific resource
    let first_resource_uri = &list_data.data.resources[0].uri;
    let read_params = json!({
        "uri": first_resource_uri
    });

    let read_response = read_handler.handle(Some(read_params)).await.unwrap();
    let read_data: ReadResourceResult = serde_json::from_value(read_response).unwrap();

    assert_eq!(read_data.contents.len(), 1);
    match &read_data.contents[0] {
        ResourceContent::Text(text_content) => {
            assert_eq!(&text_content.uri, first_resource_uri);
            assert!(!text_content.text.is_empty());
        }
        _ => panic!("Expected text content")
    }

    // Step 3: Test template resource reading
    let template_params = json!({
        "uri": "test://dynamic/electronics/laptop"
    });

    let template_response = read_handler.handle(Some(template_params)).await.unwrap();
    let template_data: ReadResourceResult = serde_json::from_value(template_response).unwrap();

    assert_eq!(template_data.contents.len(), 1);
    match &template_data.contents[0] {
        ResourceContent::Text(text_content) => {
            assert!(text_content.text.contains("template_vars"));
            assert!(text_content.uri.contains("test://items/")); // Template resource's actual URI
        }
        _ => panic!("Expected text content")
    }
}

#[tokio::test]
async fn test_mcp_error_handling_integration() {
    // === Test MCP Error Handling Integration ===

    let handler = ResourcesReadHandler::new();

    // Test missing resource (should return proper MCP error)
    let invalid_params = json!({
        "uri": "test://nonexistent/resource"
    });

    let error_response = handler.handle(Some(invalid_params)).await;
    assert!(error_response.is_err(), "Should return error for nonexistent resource");

    let error = error_response.unwrap_err();
    match error {
        McpError::InvalidParams { .. } => {
            // Correct error type for invalid resource URI
        }
        _ => panic!("Expected InvalidParams error for nonexistent resource")
    }

    // Test missing parameters (should return proper MCP error)
    let missing_params_response = handler.handle(None).await;
    assert!(missing_params_response.is_err(), "Should return error for missing params");

    // Test invalid URI format
    let invalid_uri_params = json!({
        "uri": ""  // Empty URI
    });

    let invalid_uri_response = handler.handle(Some(invalid_uri_params)).await;
    assert!(invalid_uri_response.is_err(), "Should return error for empty URI");
}

#[tokio::test]
async fn test_resource_templates_list_pagination() {
    // === Test Resource Templates List Pagination ===

    use turul_mcp_server::handlers::ResourceTemplatesHandler;

    let handler = ResourceTemplatesHandler;

    // Test empty list pagination
    let response = handler.handle(None).await.unwrap();
    let paginated_data: PaginatedResponse<ListResourceTemplatesResult> =
        serde_json::from_value(response).unwrap();

    // Empty list should have proper pagination metadata
    assert_eq!(paginated_data.data.resource_templates.len(), 0);
    assert!(paginated_data.meta.is_some());

    let meta = paginated_data.meta.as_ref().unwrap();
    assert_eq!(meta.total, Some(0));
    assert_eq!(meta.has_more, Some(false));
    assert!(meta.cursor.is_none()); // No cursor for empty list

    // Test with cursor parameter (should handle gracefully)
    let cursor_params = json!({
        "cursor": "test-cursor"
    });

    let cursor_response = handler.handle(Some(cursor_params)).await.unwrap();
    let cursor_data: PaginatedResponse<ListResourceTemplatesResult> =
        serde_json::from_value(cursor_response).unwrap();

    // Should still return empty list with proper metadata
    assert_eq!(cursor_data.data.resource_templates.len(), 0);
    assert!(cursor_data.meta.is_some());
}