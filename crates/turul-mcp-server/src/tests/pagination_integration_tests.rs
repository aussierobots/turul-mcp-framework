//! Pagination Integration Tests
//!
//! Tests for MCP pagination features (current spec: 2025-11-25).

use crate::handlers::{McpHandler, ResourcesListHandler};
use async_trait::async_trait;
use serde_json::{Value, json};
use turul_mcp_protocol::meta::PaginatedResponse;
use turul_mcp_protocol::resources::{ListResourcesResult, ResourceContent};

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
impl crate::McpResource for TestResource {
    async fn read(
        &self,
        _params: Option<Value>,
        _session: Option<&crate::SessionContext>,
    ) -> crate::McpResult<Vec<ResourceContent>> {
        Ok(vec![ResourceContent::text(
            format!("test://item/{}", self.id),
            format!("Test content for {}", self.id),
        )])
    }
}

// Required trait implementations for TestResource
use turul_mcp_builders::prelude::*; // HasResourceMetadata, HasResourceDescription, etc.

impl HasResourceMetadata for TestResource {
    fn name(&self) -> &str {
        "test_resource"
    }
}

impl HasResourceDescription for TestResource {
    fn description(&self) -> Option<&str> {
        Some("Test resource for pagination")
    }
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
impl HasIcons for TestResource {}

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
    let data: PaginatedResponse<ListResourcesResult> = serde_json::from_value(response).unwrap();

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
    let data: PaginatedResponse<ListResourcesResult> = serde_json::from_value(response).unwrap();

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
    let data: PaginatedResponse<ListResourcesResult> = serde_json::from_value(response).unwrap();

    // Empty list should have proper pagination metadata
    assert_eq!(data.data.resources.len(), 0);
    assert!(data.meta.is_some());

    let meta = data.meta.as_ref().unwrap();
    assert_eq!(meta.total, Some(0));
    assert_eq!(meta.has_more, Some(false));
    assert!(meta.cursor.is_none());
}

#[tokio::test]
async fn test_pagination_sorting_order_validation() {
    // Test that resources are actually sorted by URI as required by MCP spec
    let mut handler = ResourcesListHandler::new();

    // Add resources in deliberately unsorted order
    let test_uris = vec![
        "test://zebra/item",
        "test://alpha/item",
        "test://beta/item",
        "test://gamma/item",
        "test://delta/item",
    ];

    for uri in &test_uris {
        let resource = TestResource {
            id: uri.replace("test://", "").replace("/item", ""),
            uri: uri.to_string(),
        };
        handler = handler.add_resource(resource);
    }

    // Get all resources in one page
    let response = handler.handle(None).await.unwrap();
    let data: PaginatedResponse<ListResourcesResult> = serde_json::from_value(response).unwrap();

    // Extract URIs from response
    let returned_uris: Vec<&String> = data.data.resources.iter().map(|r| &r.uri).collect();

    // URIs should be sorted alphabetically
    let mut expected_uris = test_uris.clone();
    expected_uris.sort();

    assert_eq!(returned_uris.len(), expected_uris.len());
    for (returned, expected) in returned_uris.iter().zip(expected_uris.iter()) {
        assert_eq!(returned, expected, "Resources should be sorted by URI");
    }
}

#[tokio::test]
async fn test_pagination_cursor_robustness() {
    // Test cursor with special characters and edge cases
    let mut handler = ResourcesListHandler::new();

    // Add resources with special characters in URIs
    let special_uris = vec![
        "file:///test/item-001.json",
        "file:///test/item_002.json",
        "file:///test/item%20003.json",
        "file:///test/item+004.json",
        "file:///test/item@005.json",
    ];

    for uri in &special_uris {
        let resource = TestResource {
            id: uri.replace("file:///test/", "").replace(".json", ""),
            uri: uri.to_string(),
        };
        handler = handler.add_resource(resource);
    }

    // Get first page with small page size to force pagination
    // Since we can't change page size easily, add more resources to force pagination
    for i in 6..=55 {
        let resource = TestResource {
            id: format!("item_{:03}", i),
            uri: format!("file:///test/item_{:03}.json", i),
        };
        handler = handler.add_resource(resource);
    }

    // Get first page
    let page1_response = handler.handle(None).await.unwrap();
    let page1_data: PaginatedResponse<ListResourcesResult> =
        serde_json::from_value(page1_response).unwrap();

    assert!(page1_data.meta.as_ref().unwrap().cursor.is_some());
    let cursor = page1_data.meta.as_ref().unwrap().cursor.as_ref().unwrap();

    // Test cursor persistence - same cursor should return same results
    let page2_params = json!({ "cursor": cursor.as_str() });
    let page2_response1 = handler.handle(Some(page2_params.clone())).await.unwrap();
    let page2_response2 = handler.handle(Some(page2_params)).await.unwrap();

    // Should get identical results with same cursor
    assert_eq!(
        serde_json::to_string(&page2_response1).unwrap(),
        serde_json::to_string(&page2_response2).unwrap(),
        "Same cursor should return identical results"
    );
}

#[tokio::test]
async fn test_pagination_page_size_behavior() {
    // Test that the default page size (50) is actually respected
    let mut handler = ResourcesListHandler::new();

    // Add exactly 75 resources to test page boundaries
    for i in 1..=75 {
        let resource = TestResource::new(format!("pagesize_test_{:03}", i));
        handler = handler.add_resource(resource);
    }

    // First page should have exactly 50 resources
    let page1_response = handler.handle(None).await.unwrap();
    let page1_data: PaginatedResponse<ListResourcesResult> =
        serde_json::from_value(page1_response).unwrap();

    assert_eq!(
        page1_data.data.resources.len(),
        50,
        "First page should have 50 resources"
    );
    assert_eq!(page1_data.meta.as_ref().unwrap().has_more, Some(true));

    // Second page should have remaining 25 resources
    let cursor = page1_data.meta.as_ref().unwrap().cursor.as_ref().unwrap();
    let page2_params = json!({ "cursor": cursor.as_str() });
    let page2_response = handler.handle(Some(page2_params)).await.unwrap();
    let page2_data: PaginatedResponse<ListResourcesResult> =
        serde_json::from_value(page2_response).unwrap();

    assert_eq!(
        page2_data.data.resources.len(),
        25,
        "Second page should have 25 resources"
    );
    assert_eq!(page2_data.meta.as_ref().unwrap().has_more, Some(false));

    // Test with exactly 50 resources
    let mut handler_50 = ResourcesListHandler::new();
    for i in 1..=50 {
        let resource = TestResource::new(format!("exact_50_{:03}", i));
        handler_50 = handler_50.add_resource(resource);
    }

    let response_50 = handler_50.handle(None).await.unwrap();
    let data_50: PaginatedResponse<ListResourcesResult> =
        serde_json::from_value(response_50).unwrap();

    assert_eq!(
        data_50.data.resources.len(),
        50,
        "Should return all 50 resources in one page"
    );
    assert_eq!(
        data_50.meta.as_ref().unwrap().has_more,
        Some(false),
        "Should not have more pages"
    );
    assert!(
        data_50.meta.as_ref().unwrap().cursor.is_none(),
        "Should not have next cursor"
    );
}

#[tokio::test]
async fn test_pagination_resource_content_correctness() {
    // Verify actual resource URIs match expected values and appear in correct order
    let mut handler = ResourcesListHandler::new();

    // Add resources with predictable naming pattern
    let expected_ids = (1..=10)
        .map(|i| format!("content_test_{:02}", i))
        .collect::<Vec<_>>();
    let expected_uris = expected_ids
        .iter()
        .map(|id| format!("test://item/{}", id))
        .collect::<Vec<_>>();

    // Add in reverse order to test sorting
    for id in expected_ids.iter().rev() {
        let resource = TestResource {
            id: id.clone(),
            uri: format!("test://item/{}", id),
        };
        handler = handler.add_resource(resource);
    }

    let response = handler.handle(None).await.unwrap();
    let data: PaginatedResponse<ListResourcesResult> = serde_json::from_value(response).unwrap();

    // Verify all resources are present and correctly ordered
    assert_eq!(data.data.resources.len(), expected_uris.len());

    // Check each resource has correct URI and metadata
    for (i, resource) in data.data.resources.iter().enumerate() {
        assert_eq!(
            resource.uri, expected_uris[i],
            "Resource {} should have correct URI",
            i
        );
        assert_eq!(
            resource.name, "test_resource",
            "Resource should have correct name"
        );
        assert_eq!(
            resource.description,
            Some("Test resource for pagination".to_string())
        );
        assert_eq!(resource.mime_type, None); // TestResource returns None for mime_type
    }
}

#[tokio::test]
async fn test_pagination_boundary_conditions() {
    // Test edge cases: 0, 1, 49, 50, 51, 100, 101 resources
    let test_cases = vec![1, 49, 50, 51, 100, 101];

    for &resource_count in &test_cases {
        let mut handler = ResourcesListHandler::new();

        // Add specified number of resources
        for i in 1..=resource_count {
            let resource = TestResource::new(format!("boundary_test_{}_{:03}", resource_count, i));
            handler = handler.add_resource(resource);
        }

        // Get first page
        let page1_response = handler.handle(None).await.unwrap();
        let page1_data: PaginatedResponse<ListResourcesResult> =
            serde_json::from_value(page1_response).unwrap();

        let expected_page1_size = std::cmp::min(resource_count, 50);
        let expected_has_more = resource_count > 50;

        assert_eq!(
            page1_data.data.resources.len(),
            expected_page1_size,
            "First page size should be correct for {} resources",
            resource_count
        );

        assert_eq!(
            page1_data.meta.as_ref().unwrap().has_more,
            Some(expected_has_more),
            "has_more should be correct for {} resources",
            resource_count
        );

        assert_eq!(
            page1_data.meta.as_ref().unwrap().total,
            Some(resource_count as u64),
            "Total count should be correct for {} resources",
            resource_count
        );

        // Test cursor behavior
        if expected_has_more {
            assert!(
                page1_data.meta.as_ref().unwrap().cursor.is_some(),
                "Should have cursor when more pages exist for {} resources",
                resource_count
            );

            // Test second page
            let cursor = page1_data.meta.as_ref().unwrap().cursor.as_ref().unwrap();
            let page2_params = json!({ "cursor": cursor.as_str() });
            let page2_response = handler.handle(Some(page2_params)).await.unwrap();
            let page2_data: PaginatedResponse<ListResourcesResult> =
                serde_json::from_value(page2_response).unwrap();

            let remaining_resources = resource_count - 50;
            let expected_page2_size = std::cmp::min(remaining_resources, 50);
            let expected_page2_has_more = remaining_resources > 50;

            assert_eq!(
                page2_data.data.resources.len(),
                expected_page2_size,
                "Second page size should be correct for {} resources",
                resource_count
            );

            assert_eq!(
                page2_data.meta.as_ref().unwrap().has_more,
                Some(expected_page2_has_more),
                "Second page has_more should be correct for {} resources",
                resource_count
            );
        } else {
            assert!(
                page1_data.meta.as_ref().unwrap().cursor.is_none(),
                "Should not have cursor when no more pages for {} resources",
                resource_count
            );
        }
    }
}
