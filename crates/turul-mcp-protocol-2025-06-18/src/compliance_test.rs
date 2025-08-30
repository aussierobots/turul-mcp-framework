//! MCP TypeScript Specification Compliance Tests
//!
//! This module contains tests to verify our Rust implementations match the MCP TypeScript schema.

#[cfg(test)]
mod tests {
    use crate::tools::*;
    use crate::notifications::*;
    use serde_json::json;
    use std::collections::HashMap;

    #[test]
    fn test_call_tool_request_matches_typescript_spec() {
        // Test CallToolRequest matches: { method: string, params: { name: string, arguments?: {...}, _meta?: {...} } }
        let mut args = HashMap::new();
        args.insert("text".to_string(), json!("Hello, world!"));

        let mut meta = HashMap::new();
        meta.insert("clientId".to_string(), json!("test-client"));

        let request = CallToolRequest::new("echo")
            .with_arguments_value(json!(args))
            .with_meta(meta);

        // Serialize to JSON to check structure
        let json_value = serde_json::to_value(&request).unwrap();

        assert_eq!(json_value["method"], "tools/call");
        assert!(json_value["params"].is_object());
        assert_eq!(json_value["params"]["name"], "echo");
        assert!(json_value["params"]["arguments"].is_object());
        assert!(json_value["params"]["_meta"].is_object());
        assert_eq!(json_value["params"]["_meta"]["clientId"], "test-client");
    }

    #[test]
    fn test_list_tools_request_matches_typescript_spec() {
        // Test ListToolsRequest matches: { method: string, params?: { cursor?: string, _meta?: {...} } }
        let mut meta = HashMap::new();
        meta.insert("sessionId".to_string(), json!("session-123"));

        let request = ListToolsRequest::new()
            .with_cursor(crate::meta::Cursor::new("cursor-456"))
            .with_meta(meta);

        let json_value = serde_json::to_value(&request).unwrap();

        assert_eq!(json_value["method"], "tools/list");
        assert!(json_value["params"].is_object());
        assert_eq!(json_value["params"]["cursor"], "cursor-456");
        assert_eq!(json_value["params"]["_meta"]["sessionId"], "session-123");
    }

    #[test]
    fn test_call_tool_response_matches_typescript_spec() {
        // Test CallToolResponse has top-level _meta: { content: [...], isError?: boolean, _meta?: {...} }
        let mut meta = HashMap::new();
        meta.insert("executionTime".to_string(), json!(42));

        let response = CallToolResult::success(vec![ToolResult::text("Success!")])
            .with_meta(meta);

        let json_value = serde_json::to_value(&response).unwrap();

        assert!(json_value["content"].is_array());
        assert_eq!(json_value["isError"], false);
        assert!(json_value["_meta"].is_object());
        assert_eq!(json_value["_meta"]["executionTime"], 42);
    }

    #[test]
    fn test_notification_matches_typescript_spec() {
        // Test Notification matches: { method: string, params?: { _meta?: {...}, [key: string]: unknown } }
        let mut meta = HashMap::new();
        meta.insert("timestamp".to_string(), json!("2025-01-01T00:00:00Z"));

        let notification = ResourceListChangedNotification::new()
            .with_meta(meta);

        let json_value = serde_json::to_value(&notification).unwrap();

        assert_eq!(json_value["method"], "notifications/resources/list_changed");
        assert!(json_value["params"].is_object());
        assert_eq!(json_value["params"]["_meta"]["timestamp"], "2025-01-01T00:00:00Z");
    }

    #[test]
    fn test_progress_notification_with_params() {
        // Test notification with specific params + _meta
        let mut meta = HashMap::new();
        meta.insert("requestId".to_string(), json!("req-789"));

        let notification = ProgressNotification::new("token-123", 50)
            .with_total(100)
            .with_message("Processing...")
            .with_meta(meta);

        let json_value = serde_json::to_value(&notification).unwrap();

        assert_eq!(json_value["method"], "notifications/progress");
        assert_eq!(json_value["params"]["progressToken"], "token-123");
        assert_eq!(json_value["params"]["progress"], 50);
        assert_eq!(json_value["params"]["total"], 100);
        assert_eq!(json_value["params"]["message"], "Processing...");
        assert_eq!(json_value["params"]["_meta"]["requestId"], "req-789");
    }

    #[test]
    fn test_resource_updated_notification_with_uri() {
        // Test notification with URI param + _meta
        let mut meta = HashMap::new();
        meta.insert("changeType".to_string(), json!("modified"));

        let notification = ResourceUpdatedNotification::new("file:///config.json")
            .with_meta(meta);

        let json_value = serde_json::to_value(&notification).unwrap();

        assert_eq!(json_value["method"], "notifications/resources/updated");
        assert_eq!(json_value["params"]["uri"], "file:///config.json");
        assert_eq!(json_value["params"]["_meta"]["changeType"], "modified");
    }

    #[test]
    fn test_optional_params_serialization() {
        // Test that empty notifications don't serialize params if None
        let notification = ResourceListChangedNotification::new();
        let json_value = serde_json::to_value(&notification).unwrap();

        assert_eq!(json_value["method"], "notifications/resources/list_changed");
        // params should be null/absent since it's None
        assert!(json_value["params"].is_null() || !json_value.as_object().unwrap().contains_key("params"));
    }

    #[test]
    fn test_optional_meta_serialization() {
        // Test that _meta is properly omitted when None
        let request = CallToolRequest::new("test");
        let json_value = serde_json::to_value(&request).unwrap();

        assert_eq!(json_value["method"], "tools/call");
        assert_eq!(json_value["params"]["name"], "test");
        // _meta should be absent since it's None
        assert!(!json_value["params"].as_object().unwrap().contains_key("_meta"));
    }
}