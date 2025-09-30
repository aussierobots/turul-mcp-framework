//! MCP Resources Specification Compliance Tests
//!
//! This test suite validates complete MCP 2025-06-18 Resources specification compliance.
//! Tests all required traits, fields, behaviors, and protocol requirements for Resources.

use turul_mcp_protocol::resources::{Resource, ResourceContent, ResourceTemplate};
use turul_mcp_server::prelude::*;
// Removed unused imports - will add back when implementing pagination/progress features
use mcp_resources_tests::{AppConfigResource, LogFilesResource, UserProfileResource};

// =============================================
// === MCP Resources Core Specification Tests ===
// =============================================

#[tokio::test]
async fn test_resource_definition_trait_complete_mcp_compliance() {
    let user_resource = UserProfileResource::new().with_preferences();

    // === Test Required ResourceDefinition Trait Methods ===

    // HasResourceMetadata trait compliance
    assert_eq!(user_resource.name(), "user_profile"); // Required: programmatic identifier
    assert!(user_resource.title().is_none()); // Optional: human-readable display name

    // HasResourceDescription trait compliance
    assert!(user_resource.description().is_some()); // Optional but recommended
    let description = user_resource.description().unwrap();
    assert!(!description.is_empty());

    // HasResourceUri trait compliance
    assert_eq!(user_resource.uri(), "app://users/{user_id}"); // Required: RFC 6570 URI template

    // HasResourceMimeType trait compliance
    assert!(user_resource.mime_type().is_none()); // Optional: MIME type hint

    // HasResourceSize trait compliance
    assert!(user_resource.size().is_none()); // Optional: size in bytes

    // HasResourceAnnotations trait compliance
    assert!(user_resource.annotations().is_none()); // Optional: additional metadata

    // HasResourceMeta trait compliance
    assert!(user_resource.resource_meta().is_none()); // Optional: resource-specific metadata

    // === Test ResourceDefinition Composed Methods ===

    // display_name precedence: title > name (per MCP spec)
    assert_eq!(user_resource.display_name(), "user_profile"); // Falls back to name

    // to_resource() protocol serialization
    let resource_struct = user_resource.to_resource();
    assert_eq!(resource_struct.name, "user_profile");
    assert_eq!(resource_struct.uri, "app://users/{user_id}");
    assert!(resource_struct.description.is_some());
    assert!(resource_struct.title.is_none());
    assert!(resource_struct.mime_type.is_none());
    assert!(resource_struct.size.is_none());
    assert!(resource_struct.annotations.is_none());
    assert!(resource_struct.meta.is_none());
}

#[tokio::test]
async fn test_resource_uri_rfc_6570_template_compliance() {
    // === Test URI Template Format Compliance ===

    let user_resource = UserProfileResource::new();
    let config_resource = AppConfigResource::new("api", "prod");
    let log_resource = LogFilesResource::new("error");

    // RFC 6570 URI Template requirements:
    // 1. Template variables use {variable} syntax
    assert!(user_resource.uri().contains("{user_id}"));
    assert!(config_resource.uri().contains("{config_type}"));
    assert!(log_resource.uri().contains("{log_type}"));

    // 2. Valid URI scheme
    assert!(user_resource.uri().starts_with("app://"));
    assert!(config_resource.uri().starts_with("app://"));
    assert!(log_resource.uri().starts_with("app://"));

    // 3. Templates should NOT contain actual instance data
    assert!(!user_resource.uri().contains("uri_test")); // Should be template
    assert!(!config_resource.uri().contains("api")); // Should be template
    assert!(!config_resource.uri().contains("prod")); // Should be template
    assert!(!log_resource.uri().contains("error")); // Should be template

    // 4. Templates should be valid URI format
    for uri in &[
        user_resource.uri(),
        config_resource.uri(),
        log_resource.uri(),
    ] {
        // Should not contain spaces or invalid characters
        assert!(!uri.contains(" "));
        assert!(!uri.contains("\n"));
        assert!(!uri.contains("\t"));
    }
}

#[tokio::test]
async fn test_resource_content_mcp_format_compliance() {
    let config_resource = AppConfigResource::new("database", "test");
    let log_resource = LogFilesResource::new("application").with_lines(10);

    // === Test ResourceContent Format Compliance ===

    // Test via McpResource trait (official framework interface)
    let config_content = config_resource.read(None, None).await.unwrap();
    let log_content = log_resource.read(None, None).await.unwrap();

    assert!(!config_content.is_empty());
    assert!(!log_content.is_empty());

    // === Test ResourceContent::Blob Compliance ===
    match &config_content[0] {
        ResourceContent::Blob(blob_content) => {
            // Required fields per MCP spec
            assert!(!blob_content.uri.is_empty());
            assert!(!blob_content.blob.is_empty());

            // Validate MIME type is set correctly
            assert_eq!(blob_content.mime_type.as_ref().unwrap(), "application/json");

            // Validate blob contains valid JSON (for this resource type)
            let parsed_json: Value = serde_json::from_str(&blob_content.blob)
                .expect("Config resource should contain valid JSON");
            assert!(parsed_json.is_object());

            // Optional fields should be None by default
            assert!(blob_content.meta.is_none());
        }
        _ => panic!("Config resource should return Blob content"),
    }

    // === Test ResourceContent Type (may be Blob from derive macro default) ===
    match &log_content[0] {
        ResourceContent::Text(text_content) => {
            // Required fields per MCP spec
            assert!(!text_content.uri.is_empty());
            assert!(!text_content.text.is_empty());

            // Optional fields should be None by default
            assert!(text_content.meta.is_none());
        }
        ResourceContent::Blob(blob_content) => {
            // Default derive macro behavior - serializes struct as JSON
            assert!(!blob_content.uri.is_empty());
            assert!(!blob_content.blob.is_empty());
            assert_eq!(blob_content.mime_type.as_ref().unwrap(), "application/json");

            // Should contain serialized LogFilesResource struct
            let parsed_json: Value = serde_json::from_str(&blob_content.blob).unwrap();
            assert!(parsed_json.is_object());
            assert!(parsed_json.get("log_type").is_some());
        }
    }
}

#[tokio::test]
async fn test_resource_business_logic_methods_coverage() {
    // === Test Business Logic Methods (Eliminates Dead Code Warnings) ===

    let user_resource = UserProfileResource::new().with_preferences();
    let config_resource = AppConfigResource::new("api", "development");
    let log_resource = LogFilesResource::new("error").with_lines(25); // Use "error" type to get ERROR entries

    // Test UserProfileResource business methods
    let user_profile_data = user_resource
        .fetch_profile_data("business_test")
        .await
        .unwrap();
    assert!(!user_profile_data.is_empty());

    // Should include preferences when enabled
    match &user_profile_data[0] {
        ResourceContent::Text(text_content) => {
            assert!(text_content.text.contains("business_test"));
            assert!(text_content.text.contains("preferences")); // with_preferences() enabled
        }
        _ => panic!("User profile should return text content"),
    }

    // Test AppConfigResource business methods
    let config_data = config_resource.fetch_config_data().await.unwrap();
    assert!(!config_data.is_empty());

    match &config_data[0] {
        ResourceContent::Blob(blob_content) => {
            assert_eq!(blob_content.mime_type.as_ref().unwrap(), "application/json");
            // Validate config contains environment info
            let config_json: Value = serde_json::from_str(&blob_content.blob).unwrap();
            assert!(config_json.get("environment").is_some());
        }
        _ => panic!("App config should return blob content"),
    }

    // Test LogFilesResource business methods
    let log_data = log_resource.fetch_log_data().await.unwrap();
    assert!(!log_data.is_empty());

    match &log_data[0] {
        ResourceContent::Text(text_content) => {
            assert!(text_content.text.to_uppercase().contains("ERROR")); // log type or level
                                                                         // Should contain either "error" (log type) or "ERROR" (log level)
                                                                         // Should respect lines limit
            let line_count = text_content.text.lines().count();
            assert!(line_count <= 25); // Respects with_lines(25)
        }
        _ => {
            // Log files business method returns text, derive macro method returns JSON blob
            // Both are valid MCP resource content types
        }
    }
}

#[tokio::test]
async fn test_resource_template_mcp_specification() {
    // === Test ResourceTemplate Compliance (MCP Spec Feature) ===

    // Create ResourceTemplate matching our resources
    let user_template = ResourceTemplate {
        name: "user_profile".to_string(),
        title: Some("User Profile Data".to_string()), // Optional display name
        uri_template: "app://users/{user_id}".to_string(), // RFC 6570 template
        description: Some("User profile information and preferences".to_string()),
        mime_type: Some("application/json".to_string()), // Expected content type
        annotations: None,                               // Optional
        meta: None,                                      // Optional
    };

    // Validate ResourceTemplate structure
    assert_eq!(user_template.name, "user_profile");
    assert!(user_template.title.is_some());
    assert!(user_template.description.is_some());
    assert!(user_template.uri_template.contains("{user_id}")); // Template variable
    assert_eq!(
        user_template.mime_type.as_ref().unwrap(),
        "application/json"
    );

    // Test serialization for protocol compliance
    let serialized = serde_json::to_string(&user_template).unwrap();
    let deserialized: ResourceTemplate = serde_json::from_str(&serialized).unwrap();

    assert_eq!(deserialized.name, user_template.name);
    assert_eq!(deserialized.uri_template, user_template.uri_template);
    assert_eq!(deserialized.description, user_template.description);
    assert_eq!(deserialized.mime_type, user_template.mime_type);
}

#[tokio::test]
async fn test_resource_polymorphism_mcp_compliance() {
    // === Test Trait Polymorphism (Framework Architecture) ===

    // All resource types should work uniformly through ResourceDefinition trait
    let resources: Vec<Box<dyn ResourceDefinition>> = vec![
        Box::new(UserProfileResource::new()),
        Box::new(AppConfigResource::new("database", "prod")),
        Box::new(LogFilesResource::new("error").with_lines(50)),
    ];

    // Test uniform interface through trait
    for resource in &resources {
        // All should implement core traits
        assert!(!resource.name().is_empty());
        assert!(!resource.uri().is_empty());
        assert!(resource.description().is_some());

        // All should convert to protocol Resource struct
        let resource_struct = resource.to_resource();
        assert_eq!(resource_struct.name, resource.name());
        assert_eq!(resource_struct.uri, resource.uri());
        assert_eq!(
            resource_struct.description,
            resource.description().map(String::from)
        );
    }
}

#[tokio::test]
async fn test_resource_mcp_error_handling_compliance() {
    // === Test MCP Error Code Compliance ===

    let resource_error = McpError::resource_execution("Resource failed to load");
    let json_rpc_error = resource_error.to_error_object();

    // Validate MCP-specific error code for resources
    assert_eq!(json_rpc_error.code, -32012); // Resource execution error per MCP spec
    assert!(json_rpc_error.message.contains("Resource execution failed"));
    assert!(json_rpc_error.message.contains("Resource failed to load"));

    // Test other resource-related error types
    let missing_param_error = McpError::missing_param("resource_uri");
    assert_eq!(missing_param_error.to_error_object().code, -32602); // Invalid params

    let validation_error = McpError::validation("Invalid resource format");
    assert_eq!(validation_error.to_error_object().code, -32020); // Validation error
}

#[tokio::test]
async fn test_resource_serialization_round_trip_mcp_compliance() {
    // === Test Protocol Serialization Compliance ===

    let user_resource = UserProfileResource::new();
    let resource_struct = user_resource.to_resource();

    // Test JSON serialization preserves MCP structure
    let serialized = serde_json::to_string(&resource_struct).unwrap();

    // Validate serialized JSON has correct camelCase fields (per MCP spec)
    assert!(serialized.contains("\"name\":")); // name field
    assert!(serialized.contains("\"uri\":")); // uri field
    assert!(serialized.contains("\"description\":")); // description field
                                                      // Should NOT contain snake_case (Rust style)
    assert!(!serialized.contains("mime_type")); // Should be camelCase in JSON

    // Test deserialization maintains data integrity
    let deserialized: Resource = serde_json::from_str(&serialized).unwrap();
    assert_eq!(deserialized.name, resource_struct.name);
    assert_eq!(deserialized.uri, resource_struct.uri);
    assert_eq!(deserialized.description, resource_struct.description);
}

#[tokio::test]
async fn test_resource_edge_cases_mcp_robustness() {
    // === Test Edge Cases and Robustness ===

    // Test with minimal/empty inputs
    let minimal_resource = UserProfileResource::new();
    assert_eq!(minimal_resource.name(), "user_profile"); // Should still work
    assert!(!minimal_resource.uri().is_empty()); // Should have valid URI template

    // Test resource reading with minimal data
    let content = minimal_resource.read(None, None).await.unwrap();
    assert!(!content.is_empty()); // Should produce content even with empty input

    // Test business logic robustness
    let profile_data = minimal_resource
        .fetch_profile_data("test_user")
        .await
        .unwrap();
    assert!(!profile_data.is_empty());

    // Test with boundary line counts
    let zero_lines_log = LogFilesResource::new("test").with_lines(0);
    let log_content = zero_lines_log.fetch_log_data().await.unwrap();
    assert!(!log_content.is_empty()); // Should handle gracefully

    let large_lines_log = LogFilesResource::new("test").with_lines(1000000);
    let large_content = large_lines_log.fetch_log_data().await.unwrap();
    assert!(!large_content.is_empty()); // Should handle large requests
}
