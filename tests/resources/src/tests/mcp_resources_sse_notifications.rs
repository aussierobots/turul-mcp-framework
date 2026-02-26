//! SSE Notification Structure Tests (Option A)
//!
//! Tests that verify MCP notification structures are correct for SSE compliance:
//! - Underscore naming per MCP 2025-11-25 spec (list_changed not listChanged)
//! - Proper JSON-RPC 2.0 format
//! - Correct SSE event type mapping
//!
//! This is Option A: Structure testing only (no actual SSE streaming)
//! See WORKING_MEMORY.md for Options B & C (future phases)

use serde_json::json;
use std::collections::HashMap;
use turul_mcp_protocol::notifications::*;
use turul_mcp_server::prelude::*;

#[tokio::test]
async fn test_resource_notifications_method_string_compliance() {
    // Test that all resource notification methods use underscore convention per MCP 2025-11-25
    let list_changed = ResourceListChangedNotification::new();
    let updated = ResourceUpdatedNotification::new("test://resource/1".to_string());

    assert_eq!(list_changed.method, "notifications/resources/list_changed");
    assert_eq!(updated.method, "notifications/resources/updated");

    // Verify underscore convention
    assert!(list_changed.method.contains("list_changed"));
    assert!(!updated.method.contains("resource_updated"));
}

#[tokio::test]
async fn test_all_notification_types_method_string_compliance() {
    // Test all notification method names use underscore convention per MCP 2025-11-25
    let resource_list = ResourceListChangedNotification::new();
    let tool_list = ToolListChangedNotification::new();
    let prompt_list = PromptListChangedNotification::new();
    let roots_list = RootsListChangedNotification::new();
    let progress = ProgressNotification::new("token123".to_string(), 50.0);

    let methods = vec![
        resource_list.method,
        tool_list.method,
        prompt_list.method,
        roots_list.method,
        progress.method,
    ];

    for method in methods {
        // All should contain underscore "list_changed" if applicable
        if method.contains("list") {
            assert!(method.contains("list_changed"));
            assert!(!method.contains("listChanged"));
        }
        // All should start with notifications/
        assert!(method.starts_with("notifications/"));
    }
}

#[tokio::test]
async fn test_notification_json_serialization_format() {
    // Test that notifications serialize to correct JSON-RPC format
    let notification = ResourceListChangedNotification::new();
    let json_value = serde_json::to_value(&notification).unwrap();

    // Should have method field
    assert_eq!(json_value["method"], "notifications/resources/list_changed");

    // Should NOT have jsonrpc or id fields (those are added by transport layer)
    assert!(json_value.get("jsonrpc").is_none());
    assert!(json_value.get("id").is_none());

    // Check if params field is present - it might be skipped if None due to skip_serializing_if
    let has_params = json_value.get("params").is_some();
    if has_params {
        // If present, should be null for basic notifications
        assert!(json_value["params"].is_null());
    }
}

#[tokio::test]
async fn test_sse_event_type_mapping_correctness() {
    // Test that notification methods would map correctly to SSE event types
    // Per ADR-005: event type should match method name exactly

    let test_cases = vec![
        (
            ResourceListChangedNotification::new().method,
            "notifications/resources/list_changed",
        ),
        (
            ToolListChangedNotification::new().method,
            "notifications/tools/list_changed",
        ),
        (
            PromptListChangedNotification::new().method,
            "notifications/prompts/list_changed",
        ),
        (
            RootsListChangedNotification::new().method,
            "notifications/roots/list_changed",
        ),
        (
            ProgressNotification::new("test".to_string(), 0.0).method,
            "notifications/progress",
        ),
    ];

    for (actual_method, expected_event_type) in test_cases {
        // In proper SSE implementation: event: <method_name>
        assert_eq!(actual_method, expected_event_type);

        // Verify underscore convention for list_changed methods
        if actual_method.contains("list") {
            assert!(actual_method.contains("list_changed"));
            assert!(!actual_method.contains("listChanged"));
        }
    }
}

#[tokio::test]
async fn test_progress_notification_structure() {
    // Test progress notification with all fields
    let progress = ProgressNotification::new("progress_token_123".to_string(), 75.0);

    assert_eq!(progress.method, "notifications/progress");
    assert_eq!(
        progress.params.progress_token,
        ProgressTokenValue::String("progress_token_123".to_string())
    );
    assert_eq!(progress.params.progress, 75.0);
    // Note: message field set via builder pattern if needed

    // Test JSON serialization uses camelCase for field names
    let json_value = serde_json::to_value(&progress).unwrap();
    assert_eq!(json_value["method"], "notifications/progress");
    assert_eq!(json_value["params"]["progressToken"], "progress_token_123");
    assert_eq!(json_value["params"]["progress"], 75.0);
    // Note: message field would be included if set via builder
}

#[tokio::test]
async fn test_resource_updated_notification_structure() {
    // Test resource updated notification
    let updated = ResourceUpdatedNotification::new("file:///test/resource.txt".to_string());

    assert_eq!(updated.method, "notifications/resources/updated");
    assert_eq!(updated.params.uri, "file:///test/resource.txt");

    // Test JSON serialization
    let json_value = serde_json::to_value(&updated).unwrap();
    assert_eq!(json_value["method"], "notifications/resources/updated");
    assert_eq!(json_value["params"]["uri"], "file:///test/resource.txt");
}

#[tokio::test]
async fn test_notification_with_meta_structure() {
    // Test notification with meta information
    let mut meta = HashMap::new();
    meta.insert("correlation_id".to_string(), json!("uuid-v7-test"));
    meta.insert("timestamp".to_string(), json!(1234567890));

    let notification = ResourceListChangedNotification::new().with_meta(meta);

    // Verify meta is present in params
    assert!(notification.params.is_some());
    let params = notification.params.as_ref().unwrap();
    assert!(params.meta.is_some());

    // Test JSON serialization includes _meta field
    let json_value = serde_json::to_value(&notification).unwrap();
    assert!(json_value["params"]["_meta"].is_object());
    assert_eq!(
        json_value["params"]["_meta"]["correlation_id"],
        "uuid-v7-test"
    );
    assert_eq!(json_value["params"]["_meta"]["timestamp"], 1234567890);
}

#[tokio::test]
async fn test_json_rpc_wrapper_format() {
    // Test how notification would be wrapped for SSE transport
    let notification = ResourceListChangedNotification::new();

    // This is how it would be wrapped by the transport layer
    let sse_payload = json!({
        "jsonrpc": "2.0",
        "method": notification.method,
        "params": notification.params
    });

    // Verify proper JSON-RPC 2.0 structure
    assert_eq!(sse_payload["jsonrpc"], "2.0");
    assert_eq!(
        sse_payload["method"],
        "notifications/resources/list_changed"
    );
    assert!(sse_payload["params"].is_null()); // No params in basic list change

    // Verify no id field (notifications don't have request IDs)
    assert!(sse_payload.get("id").is_none());
}

#[tokio::test]
async fn test_notification_method_name_constants() {
    // Verify that method names match MCP 2025-11-25 specification exactly
    let expected_methods = vec![
        (
            "resources/list_changed",
            ResourceListChangedNotification::new().method,
        ),
        (
            "tools/list_changed",
            ToolListChangedNotification::new().method,
        ),
        (
            "prompts/list_changed",
            PromptListChangedNotification::new().method,
        ),
        (
            "roots/list_changed",
            RootsListChangedNotification::new().method,
        ),
        (
            "resources/updated",
            ResourceUpdatedNotification::new("test://uri".to_string()).method,
        ),
        (
            "progress",
            ProgressNotification::new("token".to_string(), 0.0).method,
        ),
    ];

    for (suffix, actual_method) in expected_methods {
        let expected_full_method = format!("notifications/{}", suffix);
        assert_eq!(actual_method, expected_full_method);

        // Verify underscore convention for list_changed methods
        if actual_method.contains("list") {
            assert!(actual_method.contains("list_changed"));
            assert!(!actual_method.contains("listChanged"));
        }
    }
}
