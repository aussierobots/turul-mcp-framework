//! MCP Prompts Notifications Integration Tests
//!
//! Tests for SSE delivery of notifications/prompts/list_changed with underscore method names.
//! Following MCP 2025-11-25 convention for notification method strings.

use serde_json::json;
use turul_mcp_protocol::notifications::*;

#[tokio::test]
async fn test_prompt_list_changed_notification_structure() {
    let notification = PromptListChangedNotification::new();

    // Verify method name uses underscore convention (MCP 2025-11-25 compliance)
    assert_eq!(
        notification.method,
        "notifications/prompts/list_changed"
    );
    assert!(notification.method.contains("list_changed"));

    // Verify notification structure
    assert!(notification.params.is_none()); // Empty params by default
}

#[tokio::test]
async fn test_prompt_list_changed_notification_with_meta() {
    let mut notification = PromptListChangedNotification::new();

    // Add _meta information
    let mut meta_map = std::collections::HashMap::new();
    meta_map.insert("source".to_string(), json!("config_reload"));
    meta_map.insert("timestamp".to_string(), json!("2025-09-11T17:55:00Z"));

    let params = NotificationParams::new().with_meta(meta_map);
    notification.params = Some(params);

    // Verify serialization includes _meta
    let serialized = serde_json::to_string(&notification).unwrap();
    assert!(serialized.contains("\"_meta\""));
    assert!(serialized.contains("\"source\":\"config_reload\""));
    assert!(serialized.contains("\"timestamp\""));
}

#[tokio::test]
async fn test_prompt_list_changed_json_rpc_format() {
    let notification = PromptListChangedNotification::new();

    // Serialize to JSON and verify JSON-RPC 2.0 structure
    let json_value = serde_json::to_value(&notification).unwrap();

    // Verify it has method field with correct underscore convention
    assert!(json_value.get("method").is_some());
    assert_eq!(
        json_value["method"],
        "notifications/prompts/list_changed"
    );

    // Verify params field behavior (can be null/missing for empty notifications)
    if json_value.get("params").is_some() {
        assert!(json_value["params"].is_null() || json_value["params"].is_object());
    }
}

#[tokio::test]
async fn test_prompt_notification_direct_fields() {
    let notification = PromptListChangedNotification::new();

    // Test direct method field access
    assert_eq!(
        notification.method,
        "notifications/prompts/list_changed"
    );

    // Test params field behavior
    assert!(notification.params.is_none());
}

#[tokio::test]
async fn test_prompt_notification_serialization_round_trip() {
    let mut notification = PromptListChangedNotification::new();

    // Add params with meta
    let mut meta_map = std::collections::HashMap::new();
    meta_map.insert("change_type".to_string(), json!("prompt_added"));
    meta_map.insert("prompt_name".to_string(), json!("new_test_prompt"));

    let params = NotificationParams::new().with_meta(meta_map);
    notification.params = Some(params);

    // Serialize and deserialize
    let serialized = serde_json::to_string(&notification).unwrap();
    let deserialized: PromptListChangedNotification = serde_json::from_str(&serialized).unwrap();

    // Verify round-trip integrity
    assert_eq!(deserialized.method, notification.method);
    assert!(deserialized.params.is_some());

    let original_meta = notification.params.as_ref().unwrap().meta.as_ref().unwrap();
    let deserialized_meta = deserialized.params.as_ref().unwrap().meta.as_ref().unwrap();
    assert_eq!(
        deserialized_meta.get("change_type"),
        original_meta.get("change_type")
    );
    assert_eq!(
        deserialized_meta.get("prompt_name"),
        original_meta.get("prompt_name")
    );
}

#[tokio::test]
async fn test_prompt_notification_sse_event_mapping() {
    let notification = PromptListChangedNotification::new();

    // In SSE context, method name becomes event type
    let expected_event_type = "notifications/prompts/list_changed";
    assert_eq!(notification.method, expected_event_type);

    // Verify event type follows MCP 2025-11-25 SSE specification:
    // - Uses underscore convention for method strings
    // - Matches exact method name from JSON-RPC
    assert!(expected_event_type.contains("list_changed"));
}

#[tokio::test]
async fn test_multiple_prompt_notifications_batch() {
    // Test that multiple prompt notifications can be created and serialized
    let notifications = vec![
        PromptListChangedNotification::new(),
        PromptListChangedNotification::new(),
        PromptListChangedNotification::new(),
    ];

    // Verify all have consistent method names
    for notification in &notifications {
        assert_eq!(
            notification.method,
            "notifications/prompts/list_changed"
        );
    }

    // Test batch serialization
    let batch_json = serde_json::to_string(&notifications).unwrap();
    assert!(batch_json.contains("notifications/prompts/list_changed"));

    // Verify deserialization
    let deserialized: Vec<PromptListChangedNotification> =
        serde_json::from_str(&batch_json).unwrap();
    assert_eq!(deserialized.len(), 3);

    for notification in deserialized {
        assert_eq!(
            notification.method,
            "notifications/prompts/list_changed"
        );
    }
}

#[tokio::test]
async fn test_prompt_notification_method_string_compliance() {
    // This test specifically verifies MCP 2025-11-25 underscore method string compliance
    let notification = PromptListChangedNotification::new();

    // Method name MUST use underscore convention as per MCP 2025-11-25 spec
    assert_eq!(
        notification.method,
        "notifications/prompts/list_changed"
    );

    // Test that we're using the correct underscore convention
    let json_str = serde_json::to_string_pretty(&notification).unwrap();
    assert!(json_str.contains("list_changed"));
    assert!(json_str.contains("notifications/prompts/list_changed"));

    // Should NOT contain old camelCase version
    assert!(!json_str.contains("listChanged"));
}
