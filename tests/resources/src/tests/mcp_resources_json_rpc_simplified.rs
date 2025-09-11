//! Simplified MCP Resources JSON-RPC Integration Tests
//!
//! End-to-end integration tests using direct handler calls to verify
//! JSON-RPC compliance and MCP specification adherence.

use turul_mcp_server::prelude::*;
use turul_mcp_server::handlers::{ResourcesListHandler, ResourcesReadHandler, McpHandler};
use turul_mcp_protocol::resources::*;
use turul_mcp_protocol::meta::*;
use serde_json::{json, Value};

// Test resource for JSON-RPC integration
#[derive(Clone)]
struct SimpleJsonRpcTestResource {
    pub id: String,
    pub uri: String,
}

impl SimpleJsonRpcTestResource {
    fn new(id: impl Into<String>) -> Self {
        let id_str = id.into();
        Self {
            uri: format!("file:///tmp/test_{}.txt", id_str),
            id: id_str,
        }
    }
}

#[async_trait::async_trait]
impl McpResource for SimpleJsonRpcTestResource {
    async fn read(&self, _params: Option<Value>) -> McpResult<Vec<ResourceContent>> {
        Ok(vec![ResourceContent::text(
            &self.uri,
            &format!("JSON-RPC test content for {}", self.id)
        )])
    }
}

// Required trait implementations
use turul_mcp_protocol::resources::{
    HasResourceMetadata, HasResourceDescription, HasResourceUri,
    HasResourceMimeType, HasResourceSize, HasResourceAnnotations, HasResourceMeta
};

impl HasResourceMetadata for SimpleJsonRpcTestResource {
    fn name(&self) -> &str { "json_rpc_test_resource" }
}

impl HasResourceDescription for SimpleJsonRpcTestResource {
    fn description(&self) -> Option<&str> { Some("Test resource for JSON-RPC integration") }
}

impl HasResourceUri for SimpleJsonRpcTestResource {
    fn uri(&self) -> &str { &self.uri }
}

impl HasResourceMimeType for SimpleJsonRpcTestResource {}
impl HasResourceSize for SimpleJsonRpcTestResource {}
impl HasResourceAnnotations for SimpleJsonRpcTestResource {}
impl HasResourceMeta for SimpleJsonRpcTestResource {}

#[tokio::test]
async fn test_resources_list_handler_json_structure() {
    let mut handler = ResourcesListHandler::new();
    
    // Add resources for testing
    for i in 1..=75 {
        let resource = SimpleJsonRpcTestResource::new(format!("item_{:03}", i));
        handler = handler.add_resource(resource);
    }
    
    // Test resources/list with no parameters (first page)
    let response = handler.handle(None).await.unwrap();
    let response_obj: PaginatedResponse<ListResourcesResult> = 
        serde_json::from_value(response).unwrap();
    
    // Verify pagination structure
    assert_eq!(response_obj.data.resources.len(), 50); // Default page size
    assert!(response_obj.meta.is_some());
    
    let meta = response_obj.meta.as_ref().unwrap();
    assert_eq!(meta.total, Some(75));
    assert_eq!(meta.has_more, Some(true));
    assert!(meta.cursor.is_some());
    
    // Verify resource structure
    let first_resource = &response_obj.data.resources[0];
    assert!(!first_resource.uri.is_empty());
    assert_eq!(first_resource.name, "json_rpc_test_resource");
    assert!(first_resource.description.is_some());
}

#[tokio::test]
async fn test_resources_list_pagination_handler() {
    let mut handler = ResourcesListHandler::new();
    
    // Add resources for testing
    for i in 1..=75 {
        let resource = SimpleJsonRpcTestResource::new(format!("item_{:03}", i));
        handler = handler.add_resource(resource);
    }
    
    // Get first page
    let page1_response = handler.handle(None).await.unwrap();
    let page1_data: PaginatedResponse<ListResourcesResult> = 
        serde_json::from_value(page1_response).unwrap();
    
    let page1_meta = page1_data.meta.as_ref().unwrap();
    let cursor = page1_meta.cursor.as_ref().unwrap();
    
    // Get second page using cursor
    let page2_params = json!({ "cursor": cursor.as_str() });
    let page2_response = handler.handle(Some(page2_params)).await.unwrap();
    let page2_data: PaginatedResponse<ListResourcesResult> = 
        serde_json::from_value(page2_response).unwrap();
    
    // Verify second page
    assert_eq!(page2_data.data.resources.len(), 25); // Remaining items
    
    let page2_meta = page2_data.meta.as_ref().unwrap();
    assert_eq!(page2_meta.has_more, Some(false));
    assert!(page2_meta.cursor.is_none());
    
    // Verify no overlap
    let page1_uris: Vec<&String> = page1_data.data.resources.iter().map(|r| &r.uri).collect();
    let page2_uris: Vec<&String> = page2_data.data.resources.iter().map(|r| &r.uri).collect();
    
    for page1_uri in &page1_uris {
        assert!(!page2_uris.contains(page1_uri), "Pages should not overlap");
    }
}

#[tokio::test]
async fn test_resources_read_json_rpc_structure() {
    // Test direct resource read JSON structure (simpler than handler)
    let resource = SimpleJsonRpcTestResource::new("read_test");
    
    // Test direct resource read
    let contents = resource.read(None).await.unwrap();
    
    // Verify read result structure
    assert_eq!(contents.len(), 1);
    
    match &contents[0] {
        ResourceContent::Text(text_content) => {
            assert_eq!(text_content.uri, "file:///tmp/test_read_test.txt");
            assert!(text_content.text.contains("JSON-RPC test content"));
            assert_eq!(text_content.mime_type.as_ref().unwrap(), "text/plain");
        }
        _ => panic!("Expected text resource content")
    }
    
    // Test JSON serialization structure
    let json_content = serde_json::to_value(&contents[0]).unwrap();
    // Check that it has the expected ResourceContent structure
    assert!(json_content.get("Text").is_some() || json_content.get("uri").is_some());
}

#[tokio::test]
async fn test_resources_list_empty_response_structure() {
    let handler = ResourcesListHandler::new(); // Empty handler
    
    let response = handler.handle(None).await.unwrap();
    let response_obj: PaginatedResponse<ListResourcesResult> = 
        serde_json::from_value(response).unwrap();
    
    // Verify empty list structure
    assert_eq!(response_obj.data.resources.len(), 0);
    assert!(response_obj.meta.is_some());
    
    let meta = response_obj.meta.as_ref().unwrap();
    assert_eq!(meta.total, Some(0));
    assert_eq!(meta.has_more, Some(false));
    assert!(meta.cursor.is_none());
}

#[tokio::test]
async fn test_resources_read_error_handling() {
    let handler = ResourcesReadHandler::new(); // Empty handler
    
    // Test reading nonexistent resource
    let read_params = json!({
        "uri": "test://nonexistent/resource"
    });
    
    let result = handler.handle(Some(read_params)).await;
    assert!(result.is_err(), "Should return error for nonexistent resource");
    
    // Test missing parameters
    let result = handler.handle(None).await;
    assert!(result.is_err(), "Should return error for missing params");
}

#[tokio::test]
async fn test_stable_uri_ordering() {
    let mut handler = ResourcesListHandler::new();
    
    // Add resources with predictable ordering
    let test_uris = vec!["test://c", "test://a", "test://b"];
    for (i, base_uri) in test_uris.iter().enumerate() {
        let mut resource = SimpleJsonRpcTestResource::new(format!("item_{}", i));
        resource.uri = base_uri.to_string();
        handler = handler.add_resource(resource);
    }
    
    // Get list multiple times
    let response1 = handler.handle(None).await.unwrap();
    let response2 = handler.handle(None).await.unwrap();
    
    let data1: PaginatedResponse<ListResourcesResult> = serde_json::from_value(response1).unwrap();
    let data2: PaginatedResponse<ListResourcesResult> = serde_json::from_value(response2).unwrap();
    
    // Verify consistent ordering
    assert_eq!(data1.data.resources.len(), data2.data.resources.len());
    for (r1, r2) in data1.data.resources.iter().zip(data2.data.resources.iter()) {
        assert_eq!(r1.uri, r2.uri, "Resource ordering must be stable");
    }
    
    // Verify resources are sorted by URI
    let uris: Vec<&String> = data1.data.resources.iter().map(|r| &r.uri).collect();
    let mut sorted_uris = uris.clone();
    sorted_uris.sort();
    assert_eq!(uris, sorted_uris, "Resources should be sorted by URI");
}

#[tokio::test]
async fn test_mcp_meta_field_structure() {
    let mut handler = ResourcesListHandler::new();
    handler = handler.add_resource(SimpleJsonRpcTestResource::new("meta_test"));
    
    let response = handler.handle(None).await.unwrap();
    let response_obj: PaginatedResponse<ListResourcesResult> = 
        serde_json::from_value(response).unwrap();
    
    // Verify _meta field structure follows MCP spec
    assert!(response_obj.meta.is_some());
    let meta = response_obj.meta.as_ref().unwrap();
    
    // Check required pagination fields
    assert!(meta.total.is_some());
    assert!(meta.has_more.is_some());
    
    // Verify serialization includes camelCase fields
    let serialized = serde_json::to_string(&response_obj).unwrap();
    assert!(serialized.contains("\"_meta\""));
    assert!(serialized.contains("\"hasMore\""));
    assert!(serialized.contains("\"total\""));
}