//! Pagination Integration Tests
//!
//! Tests for MCP 2025-06-18 pagination features.

use crate::handlers::{ResourcesListHandler, McpHandler};
use crate::resource::McpResource;
use turul_mcp_protocol::resources::{ResourceContent, ListResourcesResult};
use turul_mcp_protocol::meta::PaginatedResponse;
use serde_json::{json, Value};
use async_trait::async_trait;

// Simple test resource for pagination
#[derive(Clone)]
struct TestResource {
    id: String,
    uri: String,
}

impl TestResource {
    fn new(id: impl Into<String>) -> Self {
        let id_str = id.into();
        Self { 
            uri: format!("test://item/{}", id_str),
            id: id_str,
        }
    }
}

#[async_trait]
impl McpResource for TestResource {
    async fn read(&self, _params: Option<Value>) -> crate::McpResult<Vec<ResourceContent>> {
        Ok(vec![ResourceContent::text(
            &format!("test://item/{}", self.id),
            &format!("Test content for {}", self.id)
        )])
    }
}

// Required trait implementations for TestResource
use turul_mcp_protocol::resources::{
    HasResourceMetadata, HasResourceDescription, HasResourceUri, 
    HasResourceMimeType, HasResourceSize, HasResourceAnnotations, HasResourceMeta
};

impl HasResourceMetadata for TestResource {
    fn name(&self) -> &str { "test_resource" }
}

impl HasResourceDescription for TestResource {
    fn description(&self) -> Option<&str> { Some("Test resource for pagination") }
}

impl HasResourceUri for TestResource {
    fn uri(&self) -> &str { 
        &self.uri
    }
}

impl HasResourceMimeType for TestResource {}
impl HasResourceSize for TestResource {}
impl HasResourceAnnotations for TestResource {}
impl HasResourceMeta for TestResource {}

#[tokio::test]
async fn test_resources_list_pagination() {
    // Create handler with multiple resources
    let mut handler = ResourcesListHandler::new();
    
    // Add resources in predictable order (will be sorted by URI)
    for i in 1..=75 {
        let resource = TestResource::new(format!("item_{:03}", i));
        handler = handler.add_resource(resource);
    }
    
    // Test first page (no cursor)
    let page1_response = handler.handle(None).await.unwrap();
    let page1_data: PaginatedResponse<ListResourcesResult> = 
        serde_json::from_value(page1_response).unwrap();
    
    // Validate first page
    assert_eq!(page1_data.data.resources.len(), 50); // Default page size
    assert!(page1_data.meta.is_some());
    
    let page1_meta = page1_data.meta.as_ref().unwrap();
    assert_eq!(page1_meta.total, Some(75));
    assert_eq!(page1_meta.has_more, Some(true));
    assert!(page1_meta.cursor.is_some());
    
    // Test second page using cursor
    let cursor = page1_meta.cursor.as_ref().unwrap();
    let page2_params = json!({ "cursor": cursor.as_str() });
    let page2_response = handler.handle(Some(page2_params)).await.unwrap();
    let page2_data: PaginatedResponse<ListResourcesResult> = 
        serde_json::from_value(page2_response).unwrap();
    
    // Validate second page
    assert_eq!(page2_data.data.resources.len(), 25); // Remaining items
    assert!(page2_data.meta.is_some());
    
    let page2_meta = page2_data.meta.as_ref().unwrap();
    assert_eq!(page2_meta.total, Some(75));
    assert_eq!(page2_meta.has_more, Some(false)); // No more pages
    assert!(page2_meta.cursor.is_none()); // No next cursor
    
    // Verify no overlap between pages
    let page1_uris: Vec<&String> = page1_data.data.resources.iter().map(|r| &r.uri).collect();
    let page2_uris: Vec<&String> = page2_data.data.resources.iter().map(|r| &r.uri).collect();
    
    for page1_uri in &page1_uris {
        assert!(!page2_uris.contains(page1_uri), "Pages should not overlap");
    }
}

#[tokio::test]
async fn test_pagination_cursor_consistency() {
    // Test that cursors work consistently
    let mut handler = ResourcesListHandler::new();
    
    // Add fewer resources to test edge cases
    for i in 1..=10 {
        let resource = TestResource::new(format!("cursor_test_{:02}", i));
        handler = handler.add_resource(resource);
    }
    
    // All resources should fit in one page
    let response = handler.handle(None).await.unwrap();
    let data: PaginatedResponse<ListResourcesResult> = 
        serde_json::from_value(response).unwrap();
    
    assert_eq!(data.data.resources.len(), 10);
    assert!(data.meta.is_some());
    
    let meta = data.meta.as_ref().unwrap();
    assert_eq!(meta.total, Some(10));
    assert_eq!(meta.has_more, Some(false)); // No more pages
    assert!(meta.cursor.is_none()); // No next cursor when all fit in one page
}

#[tokio::test]
async fn test_pagination_with_invalid_cursor() {
    // Test that invalid cursors are handled gracefully
    let mut handler = ResourcesListHandler::new();
    
    for i in 1..=5 {
        let resource = TestResource::new(format!("invalid_test_{}", i));
        handler = handler.add_resource(resource);
    }
    
    // Test with invalid cursor (should start from beginning)
    let invalid_params = json!({ "cursor": "invalid_cursor_value" });
    let response = handler.handle(Some(invalid_params)).await.unwrap();
    let data: PaginatedResponse<ListResourcesResult> = 
        serde_json::from_value(response).unwrap();
    
    // Should return all resources (graceful fallback)
    assert_eq!(data.data.resources.len(), 5);
    assert!(data.meta.is_some());
    assert_eq!(data.meta.as_ref().unwrap().has_more, Some(false));
}

#[tokio::test]
async fn test_empty_resources_pagination() {
    // Test pagination with no resources
    let handler = ResourcesListHandler::new();
    
    let response = handler.handle(None).await.unwrap();
    let data: PaginatedResponse<ListResourcesResult> = 
        serde_json::from_value(response).unwrap();
    
    // Empty list should have proper pagination metadata
    assert_eq!(data.data.resources.len(), 0);
    assert!(data.meta.is_some());
    
    let meta = data.meta.as_ref().unwrap();
    assert_eq!(meta.total, Some(0));
    assert_eq!(meta.has_more, Some(false));
    assert!(meta.cursor.is_none());
}