//! Comprehensive MCP Resources Protocol Coverage Tests
//!
//! This test file covers ALL structs from turul-mcp-protocol-2025-06-18/src/resources.rs
//! ensuring complete MCP 2025-06-18 specification compliance.

use serde_json::json;
use std::collections::HashMap;
use turul_mcp_protocol::meta::{Annotations, Cursor};
use turul_mcp_protocol::resources::*;
use turul_mcp_builders::prelude::*;  // ResourceDefinition and all framework traits

#[tokio::test]
async fn test_uri_template_file_user_json() {
    // Test URI template "file:///user/{user_id}.json" as requested
    let template = ResourceTemplate::new("user_file", "file:///user/{user_id}.json")
        .with_title("User JSON File")
        .with_description("User data stored in JSON format")
        .with_mime_type("application/json");

    assert_eq!(template.name, "user_file");
    assert_eq!(template.uri_template, "file:///user/{user_id}.json");
    assert_eq!(template.title, Some("User JSON File".to_string()));
    assert_eq!(template.mime_type, Some("application/json".to_string()));
}

#[tokio::test]
async fn test_text_resource_contents_json_example() {
    // Demonstrate TextResourceContents with JSON data for user file
    let user_data = json!({
        "name": "Alice Johnson",
        "id": 123,
        "email": "alice@example.com",
        "preferences": {
            "theme": "dark",
            "notifications": true
        }
    });

    let text_content = TextResourceContents {
        uri: "file:///user/123.json".to_string(),
        mime_type: Some("application/json".to_string()),
        meta: None,
        text: serde_json::to_string_pretty(&user_data).unwrap(),
    };

    assert_eq!(text_content.uri, "file:///user/123.json");
    assert_eq!(text_content.mime_type, Some("application/json".to_string()));
    assert!(text_content.text.contains("Alice Johnson"));
    assert!(text_content.text.contains("\"id\": 123"));
}

#[tokio::test]
async fn test_text_resource_contents_plain_text() {
    // Test TextResourceContents with plain text
    let text_content = TextResourceContents {
        uri: "file:///readme.txt".to_string(),
        mime_type: Some("text/plain".to_string()),
        meta: Some(HashMap::from([("version".to_string(), json!("1.0"))])),
        text: "This is a plain text file\nwith multiple lines\nof content.".to_string(),
    };

    assert_eq!(text_content.uri, "file:///readme.txt");
    assert_eq!(text_content.mime_type, Some("text/plain".to_string()));
    assert!(text_content.text.contains("plain text file"));
    assert!(text_content.meta.is_some());
}

#[tokio::test]
async fn test_blob_resource_contents_image() {
    // Demonstrate BlobResourceContents with base64 image data
    let base64_png = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";

    let blob_content = BlobResourceContents {
        uri: "file:///avatar.png".to_string(),
        mime_type: Some("image/png".to_string()),
        meta: Some(HashMap::from([
            ("size".to_string(), json!(95)),
            ("dimensions".to_string(), json!("1x1")),
        ])),
        blob: base64_png.to_string(),
    };

    assert_eq!(blob_content.uri, "file:///avatar.png");
    assert_eq!(blob_content.mime_type, Some("image/png".to_string()));
    assert_eq!(blob_content.blob, base64_png);
    assert!(blob_content.meta.is_some());
}

#[tokio::test]
async fn test_blob_resource_contents_pdf() {
    // Test BlobResourceContents with PDF data
    let base64_pdf = "JVBERi0xLjQKJeLjz9MKMSAwIG9iago8PAovVHlwZSAvQ2F0YWxvZwovUGFnZXMgMiAwIFIKPj4KZW5kb2JqCjIgMCBvYmoKPDwKL1R5cGUgL1BhZ2VzCi9LaWRzIFszIDAgUl0KL0NvdW50IDEKPD4KZW5kb2JqCjMgMCBvYmoKPDwKL1R5cGUgL1BhZ2UKL1BhcmVudCAyIDAgUgovTWVkaWFCb3ggWzAgMCA2MTIgNzkyXQo+PgplbmRvYmoKeHJlZgowIDQKMDAwMDAwMDAwMCA2NTUzNSBmCjAwMDAwMDAwMDkgMDAwMDAgbgowMDAwMDAwMDc0IDAwMDAwIG4KMDAwMDAwMDEyMCAwMDAwMCBuCnRyYWlsZXIKPDwKL1NpemUgNAovUm9vdCAxIDAgUgo+PgpzdGFydHhyZWYKMTc5CiUlRU9G";

    let blob_content = BlobResourceContents {
        uri: "file:///document.pdf".to_string(),
        mime_type: Some("application/pdf".to_string()),
        meta: None,
        blob: base64_pdf.to_string(),
    };

    assert_eq!(blob_content.uri, "file:///document.pdf");
    assert_eq!(blob_content.mime_type, Some("application/pdf".to_string()));
    assert!(blob_content.blob.starts_with("JVBERi0x")); // PDF header in base64
}

#[tokio::test]
async fn test_resource_content_enum_variants() {
    // Test ResourceContent enum with both Text and Blob variants
    let text_resource = ResourceContent::text("file:///config.json", "{\"version\": \"1.0\"}");
    let blob_resource = ResourceContent::blob("file:///logo.png", "iVBORw0...", "image/png");

    match text_resource {
        ResourceContent::Text(text_content) => {
            assert_eq!(text_content.uri, "file:///config.json");
            assert_eq!(text_content.mime_type, Some("text/plain".to_string()));
            assert!(text_content.text.contains("version"));
        }
        ResourceContent::Blob(_) => panic!("Expected Text variant"),
    }

    match blob_resource {
        ResourceContent::Blob(blob_content) => {
            assert_eq!(blob_content.uri, "file:///logo.png");
            assert_eq!(blob_content.mime_type, Some("image/png".to_string()));
            assert_eq!(blob_content.blob, "iVBORw0...");
        }
        ResourceContent::Text(_) => panic!("Expected Blob variant"),
    }
}

#[tokio::test]
async fn test_resource_template_rfc_6570_compliance() {
    // Test ResourceTemplate with multiple RFC 6570 variables
    let template = ResourceTemplate::new(
        "multi_param",
        "file:///projects/{project_id}/files/{file_name}.{ext}",
    )
    .with_description("Project files with multiple parameters")
    .with_annotations(Annotations::new().with_title("Project Files"));

    assert_eq!(
        template.uri_template,
        "file:///projects/{project_id}/files/{file_name}.{ext}"
    );
    assert!(template.uri_template.contains("{project_id}"));
    assert!(template.uri_template.contains("{file_name}"));
    assert!(template.uri_template.contains("{ext}"));
    assert!(template.annotations.is_some());
}

#[tokio::test]
async fn test_resource_struct_complete_fields() {
    // Test Resource struct with all optional fields populated
    let annotations = Annotations::new().with_title("Complete Resource");

    let mut meta = HashMap::new();
    meta.insert("created_by".to_string(), json!("system"));
    meta.insert("version".to_string(), json!(2));

    let resource = Resource::new("file:///complete.json", "complete_resource")
        .with_title("Complete Resource Example")
        .with_description("A resource demonstrating all available fields")
        .with_mime_type("application/json")
        .with_size(1024)
        .with_annotations(annotations)
        .with_meta(meta);

    assert_eq!(resource.uri, "file:///complete.json");
    assert_eq!(resource.name, "complete_resource");
    assert_eq!(
        resource.title,
        Some("Complete Resource Example".to_string())
    );
    assert_eq!(
        resource.description,
        Some("A resource demonstrating all available fields".to_string())
    );
    assert_eq!(resource.mime_type, Some("application/json".to_string()));
    assert_eq!(resource.size, Some(1024));
    assert!(resource.annotations.is_some());
    assert!(resource.meta.is_some());
}

#[tokio::test]
async fn test_list_resources_request_params() {
    // Test ListResourcesRequest and ListResourcesParams
    let cursor = Cursor::from("next_page_token_123");
    let mut meta = HashMap::new();
    meta.insert("client_id".to_string(), json!("test_client"));

    let request = ListResourcesRequest::new()
        .with_cursor(cursor.clone())
        .with_meta(meta);

    assert_eq!(request.method, "resources/list");
    assert!(request.params.cursor.is_some());
    assert!(request.params.meta.is_some());
    assert_eq!(
        request.params.cursor.as_ref().unwrap().as_str(),
        "next_page_token_123"
    );
}

#[tokio::test]
async fn test_list_resources_result_pagination() {
    // Test ListResourcesResult with pagination
    let resources = vec![
        Resource::new("file:///test1.json", "test1"),
        Resource::new("file:///test2.json", "test2"),
        Resource::new("file:///test3.json", "test3"),
    ];

    let next_cursor = Cursor::from("page_2_token");
    let mut meta = HashMap::new();
    meta.insert("total_count".to_string(), json!(10));

    let result = ListResourcesResult::new(resources)
        .with_next_cursor(next_cursor)
        .with_meta(meta);

    assert_eq!(result.resources.len(), 3);
    assert!(result.next_cursor.is_some());
    assert_eq!(
        result.next_cursor.as_ref().unwrap().as_str(),
        "page_2_token"
    );
    assert!(result.meta.is_some());
}

#[tokio::test]
async fn test_read_resource_request_params() {
    // Test ReadResourceRequest and ReadResourceParams
    let mut meta = HashMap::new();
    meta.insert("include_metadata".to_string(), json!(true));

    let request = ReadResourceRequest::new("file:///user/456.json").with_meta(meta);

    assert_eq!(request.method, "resources/read");
    assert_eq!(request.params.uri, "file:///user/456.json");
    assert!(request.params.meta.is_some());
}

#[tokio::test]
async fn test_read_resource_result_multiple_contents() {
    // Test ReadResourceResult with multiple ResourceContent items
    let contents = vec![
        ResourceContent::text("file:///user/789.json", "{\"name\":\"Bob\"}"),
        ResourceContent::blob(
            "file:///user/789/avatar.jpg",
            "base64imagedata",
            "image/jpeg",
        ),
    ];

    let mut meta = HashMap::new();
    meta.insert("read_time".to_string(), json!("2024-01-01T12:00:00Z"));

    let result = ReadResourceResult::new(contents).with_meta(meta);

    assert_eq!(result.contents.len(), 2);
    assert!(result.meta.is_some());

    // Verify content types
    match &result.contents[0] {
        ResourceContent::Text(text) => assert!(text.text.contains("Bob")),
        ResourceContent::Blob(_) => panic!("Expected Text content"),
    }

    match &result.contents[1] {
        ResourceContent::Blob(blob) => assert_eq!(blob.mime_type, Some("image/jpeg".to_string())),
        ResourceContent::Text(_) => panic!("Expected Blob content"),
    }
}

#[tokio::test]
async fn test_resource_templates_list_request() {
    // Test ListResourceTemplatesRequest
    let cursor = Cursor::from("templates_page_1");
    let mut meta = HashMap::new();
    meta.insert("filter".to_string(), json!("user_files"));

    let params = ListResourceTemplatesParams::new()
        .with_cursor(cursor)
        .with_meta(meta);

    let request = ListResourceTemplatesRequest::new();
    assert_eq!(request.method, "resources/templates/list");

    // Test params separately since we can't modify the request after creation
    assert!(params.cursor.is_some());
    assert!(params.meta.is_some());
}

#[tokio::test]
async fn test_resource_templates_list_result() {
    // Test ListResourceTemplatesResult
    let templates = vec![
        ResourceTemplate::new("user_data", "file:///users/{user_id}.json"),
        ResourceTemplate::new("user_avatar", "file:///users/{user_id}/avatar.{format}"),
    ];

    let result = ListResourceTemplatesResult::new(templates)
        .with_next_cursor(Cursor::from("templates_next"))
        .with_meta(HashMap::from([("count".to_string(), json!(2))]));

    assert_eq!(result.resource_templates.len(), 2);
    assert!(result.next_cursor.is_some());
    assert!(result.meta.is_some());
}

#[tokio::test]
async fn test_subscription_requests() {
    // Test SubscribeRequest and UnsubscribeRequest
    let subscribe_meta = HashMap::from([("priority".to_string(), json!("high"))]);
    let subscribe_request =
        SubscribeRequest::new("file:///monitor/config.json").with_meta(subscribe_meta);

    assert_eq!(subscribe_request.method, "resources/subscribe");
    assert_eq!(subscribe_request.params.uri, "file:///monitor/config.json");
    assert!(subscribe_request.params.meta.is_some());

    let unsubscribe_request = UnsubscribeRequest::new("file:///monitor/config.json");
    assert_eq!(unsubscribe_request.method, "resources/unsubscribe");
    assert_eq!(
        unsubscribe_request.params.uri,
        "file:///monitor/config.json"
    );
}

#[tokio::test]
async fn test_resource_subscription() {
    // Test ResourceSubscription
    let subscription = ResourceSubscription::new("file:///watch/changes.log");
    assert_eq!(subscription.uri, "file:///watch/changes.log");
}

#[tokio::test]
async fn test_resource_trait_implementations() {
    // Test that Resource struct implements all the required traits
    let resource = Resource::new("file:///test.txt", "test_resource")
        .with_description("Test resource for trait validation");

    // Test ResourceDefinition trait methods - MUST use trait accessors to verify impls!
    // Use method call syntax to test trait implementations
    assert_eq!(resource.name(), "test_resource");
    assert_eq!(resource.title(), None);
    assert_eq!(
        resource.description(),
        Some("Test resource for trait validation")
    );
    assert_eq!(resource.uri(), "file:///test.txt");
    assert_eq!(resource.mime_type(), None);
    assert_eq!(resource.size(), None);
    assert!(resource.annotations().is_none());
    assert!(resource.resource_meta().is_none());

    // Test display_name method
    assert_eq!(resource.display_name(), "test_resource");

    // Test conversion to Resource struct
    let converted = resource.to_resource();
    assert_eq!(converted.name, "test_resource");
    assert_eq!(converted.uri, "file:///test.txt");
}

#[tokio::test]
async fn test_serialization_round_trip_all_structs() {
    // Test serialization/deserialization for all major structs

    // ResourceTemplate
    let template =
        ResourceTemplate::new("test", "file:///test/{id}").with_description("Test template");
    let template_json = serde_json::to_string(&template).unwrap();
    let template_parsed: ResourceTemplate = serde_json::from_str(&template_json).unwrap();
    assert_eq!(template.name, template_parsed.name);

    // Resource
    let resource = Resource::new("file:///test.txt", "test").with_title("Test Resource");
    let resource_json = serde_json::to_string(&resource).unwrap();
    let resource_parsed: Resource = serde_json::from_str(&resource_json).unwrap();
    assert_eq!(resource.name, resource_parsed.name);

    // TextResourceContents
    let text_content = TextResourceContents {
        uri: "file:///test.txt".to_string(),
        mime_type: Some("text/plain".to_string()),
        meta: None,
        text: "Test content".to_string(),
    };
    let text_json = serde_json::to_string(&text_content).unwrap();
    let text_parsed: TextResourceContents = serde_json::from_str(&text_json).unwrap();
    assert_eq!(text_content.text, text_parsed.text);

    // BlobResourceContents
    let blob_content = BlobResourceContents {
        uri: "file:///test.bin".to_string(),
        mime_type: Some("application/octet-stream".to_string()),
        meta: None,
        blob: "dGVzdA==".to_string(), // "test" in base64
    };
    let blob_json = serde_json::to_string(&blob_content).unwrap();
    let blob_parsed: BlobResourceContents = serde_json::from_str(&blob_json).unwrap();
    assert_eq!(blob_content.blob, blob_parsed.blob);
}

// #[tokio::test]
// async fn test_mcp_error_handling_resource_not_found() {
//     // Test proper error handling with MCP-compliant error codes
//     use turul_mcp_protocol::json_rpc::JsonRpcError;
//
//     let error = JsonRpcError::resource_not_found("file:///nonexistent.json");
//     assert_eq!(error.code(), -32012); // MCP-specific code for resource not found
//     assert!(error.message().contains("Resource not found"));
//     assert!(error.data().is_some());
// }

#[tokio::test]
async fn test_edge_cases_and_robustness() {
    // Test edge cases for robustness

    // Empty URI templates
    let empty_template = ResourceTemplate::new("empty", "");
    assert_eq!(empty_template.uri_template, "");

    // Very long URIs
    let long_uri = "file:///".to_string() + &"a".repeat(1000) + ".txt";
    let long_resource = Resource::new(&long_uri, "long_resource");
    assert_eq!(long_resource.uri.len(), long_uri.len());

    // Special characters in URIs
    let special_resource = Resource::new("file:///test%20file%2Bspecial.txt", "special");
    assert!(special_resource.uri.contains("%20"));

    // Empty text content
    let empty_text = ResourceContent::text("file:///empty.txt", "");
    match empty_text {
        ResourceContent::Text(text) => assert_eq!(text.text, ""),
        ResourceContent::Blob(_) => panic!("Expected Text content"),
    }

    // Minimal blob content
    let minimal_blob =
        ResourceContent::blob("file:///minimal.bin", "AA==", "application/octet-stream");
    match minimal_blob {
        ResourceContent::Blob(blob) => assert_eq!(blob.blob, "AA=="),
        ResourceContent::Text(_) => panic!("Expected Blob content"),
    }
}
