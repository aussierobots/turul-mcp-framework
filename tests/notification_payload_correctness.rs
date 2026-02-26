//! Comprehensive notification payload correctness tests
//!
//! Tests that all notification types properly serialize their payloads,
//! preserving all fields including _meta data.
//!
//! This test suite validates the fix for the critical regression where
//! all notification payloads were returning None.

use serde_json::{Value, json};
use std::collections::HashMap;
use turul_mcp_builders::traits::{HasNotificationPayload, NotificationDefinition};
use turul_mcp_protocol::RequestId;
use turul_mcp_protocol::notifications::*;

#[test]
fn test_base_notification_with_params() {
    // Base Notification with params.other and _meta
    let mut params = NotificationParams::new();
    params
        .other
        .insert("customField".to_string(), json!("customValue"));

    let mut meta = HashMap::new();
    meta.insert("sessionId".to_string(), json!("test-session-123"));
    params.meta = Some(meta);

    let notification = Notification::new("notifications/test").with_params(params);

    let payload = notification.payload();
    assert!(
        payload.is_some(),
        "Base notification with params should return payload"
    );

    let payload_obj = payload.unwrap();
    assert_eq!(
        payload_obj.get("customField").and_then(|v| v.as_str()),
        Some("customValue"),
        "Custom field should be preserved in payload"
    );

    let meta_value = payload_obj.get("_meta");
    assert!(meta_value.is_some(), "_meta should be present in payload");
    assert_eq!(
        meta_value
            .unwrap()
            .get("sessionId")
            .and_then(|v| v.as_str()),
        Some("test-session-123"),
        "_meta.sessionId should be preserved"
    );
}

#[test]
fn test_base_notification_without_params() {
    let notification = Notification::new("notifications/test");

    let payload = notification.payload();
    assert!(
        payload.is_none(),
        "Base notification without params should return None"
    );
}

#[test]
fn test_progress_notification_full_payload() {
    let mut params = ProgressNotificationParams {
        progress_token: "test-token-456".to_string().into(),
        progress: 50.0,
        total: Some(100.0),
        message: Some("Processing items...".to_string()),
        meta: None,
    };

    let mut meta = HashMap::new();
    meta.insert("operationId".to_string(), json!("op-789"));
    params.meta = Some(meta);

    let notification = ProgressNotification {
        method: "notifications/progress".to_string(),
        params,
    };

    let payload = notification.payload();
    assert!(
        payload.is_some(),
        "ProgressNotification should return payload"
    );

    let payload_obj = payload.unwrap();
    assert_eq!(
        payload_obj.get("progressToken").and_then(|v| v.as_str()),
        Some("test-token-456"),
        "progressToken should be preserved"
    );
    assert_eq!(
        payload_obj.get("progress").and_then(|v| v.as_f64()),
        Some(50.0),
        "progress should be preserved"
    );
    assert_eq!(
        payload_obj.get("total").and_then(|v| v.as_f64()),
        Some(100.0),
        "total should be preserved"
    );
    assert_eq!(
        payload_obj.get("message").and_then(|v| v.as_str()),
        Some("Processing items..."),
        "message should be preserved"
    );

    let meta_value = payload_obj.get("_meta");
    assert!(
        meta_value.is_some(),
        "_meta should be present in ProgressNotification payload"
    );
    assert_eq!(
        meta_value
            .unwrap()
            .get("operationId")
            .and_then(|v| v.as_str()),
        Some("op-789"),
        "_meta.operationId should be preserved"
    );
}

#[test]
fn test_resource_updated_notification_payload() {
    let mut params = ResourceUpdatedNotificationParams {
        uri: "file:///test/resource.txt".to_string(),
        meta: None,
    };

    let mut meta = HashMap::new();
    meta.insert("changeType".to_string(), json!("modified"));
    params.meta = Some(meta);

    let notification = ResourceUpdatedNotification {
        method: "notifications/resources/updated".to_string(),
        params,
    };

    let payload = notification.payload();
    assert!(
        payload.is_some(),
        "ResourceUpdatedNotification should return payload"
    );

    let payload_obj = payload.unwrap();
    assert_eq!(
        payload_obj.get("uri").and_then(|v| v.as_str()),
        Some("file:///test/resource.txt"),
        "uri should be preserved"
    );

    let meta_value = payload_obj.get("_meta");
    assert!(
        meta_value.is_some(),
        "_meta should be present in ResourceUpdatedNotification payload"
    );
    assert_eq!(
        meta_value
            .unwrap()
            .get("changeType")
            .and_then(|v| v.as_str()),
        Some("modified"),
        "_meta.changeType should be preserved"
    );
}

#[test]
fn test_cancelled_notification_full_payload() {
    let mut params = CancelledNotificationParams {
        request_id: RequestId::String("req-123".to_string()),
        reason: Some("User cancelled operation".to_string()),
        meta: None,
    };

    let mut meta = HashMap::new();
    meta.insert("timestamp".to_string(), json!("2024-01-01T12:00:00Z"));
    params.meta = Some(meta);

    let notification = CancelledNotification {
        method: "notifications/cancelled".to_string(),
        params,
    };

    let payload = notification.payload();
    assert!(
        payload.is_some(),
        "CancelledNotification should return payload"
    );

    let payload_obj = payload.unwrap();
    assert_eq!(
        payload_obj.get("requestId").and_then(|v| v.as_str()),
        Some("req-123"),
        "requestId should be preserved"
    );
    assert_eq!(
        payload_obj.get("reason").and_then(|v| v.as_str()),
        Some("User cancelled operation"),
        "reason should be preserved"
    );

    let meta_value = payload_obj.get("_meta");
    assert!(
        meta_value.is_some(),
        "_meta should be present in CancelledNotification payload"
    );
    assert_eq!(
        meta_value
            .unwrap()
            .get("timestamp")
            .and_then(|v| v.as_str()),
        Some("2024-01-01T12:00:00Z"),
        "_meta.timestamp should be preserved"
    );
}

#[test]
fn test_resource_list_changed_with_params() {
    let mut params = NotificationParams::new();
    let mut meta = HashMap::new();
    meta.insert("source".to_string(), json!("file-watcher"));
    params.meta = Some(meta);

    let notification = ResourceListChangedNotification {
        method: "notifications/resources/list_changed".to_string(),
        params: Some(params),
    };

    let payload = notification.payload();
    assert!(
        payload.is_some(),
        "ResourceListChangedNotification with params should return payload"
    );

    let payload_obj = payload.unwrap();
    let meta_value = payload_obj.get("_meta");
    assert!(meta_value.is_some(), "_meta should be present");
    assert_eq!(
        meta_value.unwrap().get("source").and_then(|v| v.as_str()),
        Some("file-watcher"),
        "_meta.source should be preserved"
    );
}

#[test]
fn test_resource_list_changed_without_params() {
    let notification = ResourceListChangedNotification {
        method: "notifications/resources/list_changed".to_string(),
        params: None,
    };

    let payload = notification.payload();
    assert!(
        payload.is_none(),
        "ResourceListChangedNotification without params should return None"
    );
}

#[test]
fn test_tool_list_changed_with_meta() {
    let mut params = NotificationParams::new();
    let mut meta = HashMap::new();
    meta.insert("pluginLoaded".to_string(), json!("calculator-plugin"));
    params.meta = Some(meta);

    let notification = ToolListChangedNotification {
        method: "notifications/tools/list_changed".to_string(),
        params: Some(params),
    };

    let payload = notification.payload();
    assert!(
        payload.is_some(),
        "ToolListChangedNotification with params should return payload"
    );

    let payload_obj = payload.unwrap();
    let meta_value = payload_obj.get("_meta");
    assert!(meta_value.is_some(), "_meta should be present");
    assert_eq!(
        meta_value
            .unwrap()
            .get("pluginLoaded")
            .and_then(|v| v.as_str()),
        Some("calculator-plugin"),
        "_meta.pluginLoaded should be preserved"
    );
}

#[test]
fn test_prompt_list_changed_with_meta() {
    let mut params = NotificationParams::new();
    let mut meta = HashMap::new();
    meta.insert("reason".to_string(), json!("template-updated"));
    params.meta = Some(meta);

    let notification = PromptListChangedNotification {
        method: "notifications/prompts/list_changed".to_string(),
        params: Some(params),
    };

    let payload = notification.payload();
    assert!(
        payload.is_some(),
        "PromptListChangedNotification with params should return payload"
    );

    let payload_obj = payload.unwrap();
    let meta_value = payload_obj.get("_meta");
    assert!(meta_value.is_some(), "_meta should be present");
    assert_eq!(
        meta_value.unwrap().get("reason").and_then(|v| v.as_str()),
        Some("template-updated"),
        "_meta.reason should be preserved"
    );
}

#[test]
fn test_roots_list_changed_with_meta() {
    let mut params = NotificationParams::new();
    let mut meta = HashMap::new();
    meta.insert("mountPoint".to_string(), json!("/workspace"));
    params.meta = Some(meta);

    let notification = RootsListChangedNotification {
        method: "notifications/roots/list_changed".to_string(),
        params: Some(params),
    };

    let payload = notification.payload();
    assert!(
        payload.is_some(),
        "RootsListChangedNotification with params should return payload"
    );

    let payload_obj = payload.unwrap();
    let meta_value = payload_obj.get("_meta");
    assert!(meta_value.is_some(), "_meta should be present");
    assert_eq!(
        meta_value
            .unwrap()
            .get("mountPoint")
            .and_then(|v| v.as_str()),
        Some("/workspace"),
        "_meta.mountPoint should be preserved"
    );
}

#[test]
fn test_initialized_notification_with_meta() {
    let mut params = NotificationParams::new();
    let mut meta = HashMap::new();
    meta.insert("serverVersion".to_string(), json!("1.0.0"));
    params.meta = Some(meta);

    let notification = InitializedNotification {
        method: "notifications/initialized".to_string(),
        params: Some(params),
    };

    let payload = notification.payload();
    assert!(
        payload.is_some(),
        "InitializedNotification with params should return payload"
    );

    let payload_obj = payload.unwrap();
    let meta_value = payload_obj.get("_meta");
    assert!(meta_value.is_some(), "_meta should be present");
    assert_eq!(
        meta_value
            .unwrap()
            .get("serverVersion")
            .and_then(|v| v.as_str()),
        Some("1.0.0"),
        "_meta.serverVersion should be preserved"
    );
}

#[test]
fn test_initialized_notification_without_params() {
    let notification = InitializedNotification {
        method: "notifications/initialized".to_string(),
        params: None,
    };

    let payload = notification.payload();
    assert!(
        payload.is_none(),
        "InitializedNotification without params should return None"
    );
}

#[test]
fn test_notification_definition_to_notification() {
    // Test that NotificationDefinition trait works correctly
    let mut params = ProgressNotificationParams {
        progress_token: "token-999".to_string().into(),
        progress: 75.0,
        total: Some(100.0),
        message: None,
        meta: None,
    };
    let mut meta = HashMap::new();
    meta.insert("test".to_string(), json!(true));
    params.meta = Some(meta);

    let notification = ProgressNotification {
        method: "notifications/progress".to_string(),
        params,
    };

    // to_notification() should convert to base Notification
    let base_notification = notification.to_notification();
    assert_eq!(base_notification.method, "notifications/progress");
    assert!(
        base_notification.params.is_some(),
        "Converted notification should have params"
    );
}

#[test]
fn test_serialize_payload_produces_valid_json() {
    let params = ProgressNotificationParams {
        progress_token: "token-serialize".to_string().into(),
        progress: 25.0,
        total: Some(50.0),
        message: Some("Serialization test".to_string()),
        meta: None,
    };

    let notification = ProgressNotification {
        method: "notifications/progress".to_string(),
        params,
    };

    let serialized = notification.serialize_payload();
    assert!(serialized.is_ok(), "Serialization should succeed");

    let json_str = serialized.unwrap();
    let parsed: Result<Value, _> = serde_json::from_str(&json_str);
    assert!(parsed.is_ok(), "Serialized payload should be valid JSON");

    let parsed_obj = parsed.unwrap();
    assert_eq!(
        parsed_obj.get("progressToken").and_then(|v| v.as_str()),
        Some("token-serialize"),
        "Deserialized payload should contain correct data"
    );
}

#[test]
fn test_empty_params_serialization() {
    // Test notification with empty params (no other fields, no meta)
    let params = NotificationParams::new();
    let notification = Notification::new("notifications/test").with_params(params);

    let payload = notification.payload();
    assert!(
        payload.is_some(),
        "Empty params should still produce payload"
    );

    let payload_obj = payload.unwrap();
    assert!(payload_obj.is_object(), "Payload should be an object");

    // Empty params should produce empty object (no _meta if meta is None)
    let obj_map = payload_obj.as_object().unwrap();
    assert!(
        obj_map.is_empty() || !obj_map.contains_key("_meta"),
        "Empty params should not have _meta field if meta is None"
    );
}

#[test]
fn test_progress_notification_priority() {
    let params = ProgressNotificationParams {
        progress_token: "token-priority".to_string().into(),
        progress: 10.0,
        total: Some(20.0),
        message: None,
        meta: None,
    };
    let notification = ProgressNotification {
        method: "notifications/progress".to_string(),
        params,
    };

    // Verify HasNotificationRules implementation
    use turul_mcp_builders::traits::HasNotificationRules;
    assert_eq!(
        notification.priority(),
        2,
        "ProgressNotification should have priority 2"
    );
}

#[test]
fn test_cancelled_notification_priority() {
    let params = CancelledNotificationParams {
        request_id: RequestId::String("req-cancel".to_string()),
        reason: None,
        meta: None,
    };
    let notification = CancelledNotification {
        method: "notifications/cancelled".to_string(),
        params,
    };

    // Verify HasNotificationRules implementation
    use turul_mcp_builders::traits::HasNotificationRules;
    assert_eq!(
        notification.priority(),
        3,
        "CancelledNotification should have priority 3"
    );
}

#[test]
fn test_initialized_notification_priority() {
    let notification = InitializedNotification {
        method: "notifications/initialized".to_string(),
        params: None,
    };

    // Verify HasNotificationRules implementation
    use turul_mcp_builders::traits::HasNotificationRules;
    assert_eq!(
        notification.priority(),
        3,
        "InitializedNotification should have priority 3"
    );
}
