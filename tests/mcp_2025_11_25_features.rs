//! Integration tests for MCP 2025-11-25 protocol features.
//!
//! Tests cover: Icons, URL Elicitation, Sampling Tools, Tasks, camelCase compliance,
//! and McpVersion feature detection.

use serde_json::{json, Value};
use std::collections::HashMap;
use turul_mcp_protocol::*;

// ===================================================================
// Icons
// ===================================================================

mod icons {
    use super::*;

    // -- Icon creation & basics --

    #[test]
    fn test_icon_data_uri_creation() {
        let icon = Icon::data_uri("image/png", "iVBORw0KGgo=");
        assert_eq!(
            icon.src,
            "data:image/png;base64,iVBORw0KGgo=",
            "data URI should have correct format"
        );
    }

    #[test]
    fn test_icon_https_creation() {
        let icon = Icon::new("https://example.com/icon.png");
        assert_eq!(icon.src, "https://example.com/icon.png");
    }

    #[test]
    fn test_icon_display_via_src() {
        let icon = Icon::new("https://cdn.example.com/logo.svg");
        assert_eq!(
            icon.src,
            "https://cdn.example.com/logo.svg",
            "src should contain the raw URL"
        );
    }

    // -- Serialization --

    #[test]
    fn test_icon_serializes_as_object_with_src() {
        let icon = Icon::new("https://example.com/icon.png");
        let json_val = serde_json::to_value(&icon).unwrap();
        assert_eq!(
            json_val,
            json!({"src": "https://example.com/icon.png"}),
            "Icon should serialize as an object with src field"
        );
    }

    #[test]
    fn test_icon_data_uri_serialization() {
        let icon = Icon::data_uri("image/svg+xml", "PHN2Zz4=");
        let json_val = serde_json::to_value(&icon).unwrap();
        assert_eq!(json_val["src"], "data:image/svg+xml;base64,PHN2Zz4=");
    }

    // -- Round-trip --

    #[test]
    fn test_icon_roundtrip_https() {
        let original = Icon::new("https://example.com/tool.png");
        let json = serde_json::to_string(&original).unwrap();
        let parsed: Icon = serde_json::from_str(&json).unwrap();
        assert_eq!(original, parsed, "HTTPS icon round-trip failed");
    }

    #[test]
    fn test_icon_roundtrip_data_uri() {
        let original = Icon::data_uri("image/png", "AAAA");
        let json = serde_json::to_string(&original).unwrap();
        let parsed: Icon = serde_json::from_str(&json).unwrap();
        assert_eq!(original, parsed, "data URI icon round-trip failed");
    }

    // -- Icon on Tool --

    #[test]
    fn test_tool_with_icons_serialization() {
        let tool = tools::Tool::new("my-tool", tools::ToolSchema::object())
            .with_icons(vec![Icon::new("https://example.com/tool.png")]);

        let json_val = serde_json::to_value(&tool).unwrap();
        assert_eq!(
            json_val["icons"][0]["src"], "https://example.com/tool.png",
            "Tool.icons[0].src should contain the URL"
        );
    }

    #[test]
    fn test_tool_without_icons_omits_field() {
        let tool = tools::Tool::new("bare-tool", tools::ToolSchema::object());
        let json_val = serde_json::to_value(&tool).unwrap();
        assert!(
            json_val.get("icons").is_none(),
            "Tool without icons should not have icons key in JSON"
        );
    }

    #[test]
    fn test_tool_with_icons_roundtrip() {
        let original = tools::Tool::new("rt-tool", tools::ToolSchema::object())
            .with_description("A tool with icon")
            .with_icons(vec![Icon::data_uri("image/png", "AAAA")]);

        let json = serde_json::to_string(&original).unwrap();
        let parsed: tools::Tool = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.name, "rt-tool");
        assert_eq!(
            parsed.icons.as_ref().unwrap()[0].src,
            "data:image/png;base64,AAAA"
        );
    }

    // -- Icon on Resource --

    #[test]
    fn test_resource_with_icons() {
        let resource = resources::Resource::new("file:///test.txt", "test-res")
            .with_icons(vec![Icon::new("https://example.com/file.png")]);

        let json_val = serde_json::to_value(&resource).unwrap();
        assert_eq!(json_val["icons"][0]["src"], "https://example.com/file.png");
    }

    #[test]
    fn test_resource_without_icons_omits_field() {
        let resource = resources::Resource::new("file:///test.txt", "test-res");
        let json_val = serde_json::to_value(&resource).unwrap();
        assert!(json_val.get("icons").is_none());
    }

    // -- Icon on ResourceTemplate --

    #[test]
    fn test_resource_template_with_icons() {
        let tmpl = resources::ResourceTemplate::new("logs", "file:///logs/{date}.txt")
            .with_icons(vec![Icon::new("https://example.com/log.png")]);

        let json_val = serde_json::to_value(&tmpl).unwrap();
        assert_eq!(json_val["icons"][0]["src"], "https://example.com/log.png");
    }

    // -- Icon on Prompt --

    #[test]
    fn test_prompt_with_icons() {
        let prompt = prompts::Prompt::new("summarize")
            .with_icons(vec![Icon::new("https://example.com/prompt.png")]);

        let json_val = serde_json::to_value(&prompt).unwrap();
        assert_eq!(json_val["icons"][0]["src"], "https://example.com/prompt.png");
    }

    #[test]
    fn test_prompt_without_icons_omits_field() {
        let prompt = prompts::Prompt::new("plain-prompt");
        let json_val = serde_json::to_value(&prompt).unwrap();
        assert!(json_val.get("icons").is_none());
    }

    // -- Icon on Implementation --

    #[test]
    fn test_implementation_with_icons() {
        let impl_info = Implementation::new("my-server", "1.0.0")
            .with_icons(vec![Icon::new("https://example.com/server.png")]);

        let json_val = serde_json::to_value(&impl_info).unwrap();
        assert_eq!(json_val["icons"][0]["src"], "https://example.com/server.png");
    }

    #[test]
    fn test_implementation_without_icons_omits_field() {
        let impl_info = Implementation::new("my-server", "1.0.0");
        let json_val = serde_json::to_value(&impl_info).unwrap();
        assert!(json_val.get("icons").is_none());
    }

    #[test]
    fn test_implementation_with_icons_roundtrip() {
        let original = Implementation::new("srv", "2.0.0")
            .with_title("My Server")
            .with_icons(vec![Icon::data_uri("image/png", "BBBB")]);

        let json = serde_json::to_string(&original).unwrap();
        let parsed: Implementation = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.name, "srv");
        assert_eq!(parsed.version, "2.0.0");
        assert_eq!(parsed.title, Some("My Server".to_string()));
        assert_eq!(
            parsed.icons.as_ref().unwrap()[0].src,
            "data:image/png;base64,BBBB"
        );
    }
}

// ===================================================================
// URL Elicitation
// ===================================================================

mod url_elicitation {
    use super::*;

    #[test]
    fn test_string_format_uri_variant_exists() {
        let format = elicitation::StringFormat::Uri;
        let json_val = serde_json::to_value(&format).unwrap();
        assert_eq!(json_val, json!("uri"), "StringFormat::Uri should serialize as \"uri\"");
    }

    #[test]
    fn test_string_format_uri_roundtrip() {
        let format = elicitation::StringFormat::Uri;
        let json = serde_json::to_string(&format).unwrap();
        let parsed: elicitation::StringFormat = serde_json::from_str(&json).unwrap();
        assert!(
            matches!(parsed, elicitation::StringFormat::Uri),
            "Should deserialize back to Uri"
        );
    }

    #[test]
    fn test_string_schema_url_constructor() {
        let schema = elicitation::StringSchema::url();
        assert_eq!(schema.schema_type, "string");
        assert!(
            matches!(schema.format, Some(elicitation::StringFormat::Uri)),
            "url() should set format to Uri"
        );
    }

    #[test]
    fn test_string_schema_url_serialization() {
        let schema = elicitation::StringSchema::url();
        let json_val = serde_json::to_value(&schema).unwrap();
        assert_eq!(json_val["type"], "string");
        assert_eq!(json_val["format"], "uri");
    }

    #[test]
    fn test_primitive_schema_url_constructor() {
        let psd = PrimitiveSchemaDefinition::url();
        let json_val = serde_json::to_value(&psd).unwrap();
        assert_eq!(json_val["type"], "string");
        assert_eq!(json_val["format"], "uri");
    }

    #[test]
    fn test_primitive_schema_url_with_description() {
        let psd = PrimitiveSchemaDefinition::url_with_description("Enter a URL");
        let json_val = serde_json::to_value(&psd).unwrap();
        assert_eq!(json_val["type"], "string");
        assert_eq!(json_val["format"], "uri");
        assert_eq!(json_val["description"], "Enter a URL");
    }

    #[test]
    fn test_elicitation_builder_url_input() {
        let request = ElicitationBuilder::url_input(
            "Enter your website URL",
            "website",
            "Your website address",
        );

        assert_eq!(request.method, "elicitation/create");
        assert_eq!(request.params.message, "Enter your website URL");

        let props = &request.params.requested_schema.properties;
        assert!(props.contains_key("website"), "Should have 'website' property");

        let required = request.params.requested_schema.required.as_ref().unwrap();
        assert!(required.contains(&"website".to_string()));

        // Verify the property has format: uri
        let json_val = serde_json::to_value(&props["website"]).unwrap();
        assert_eq!(json_val["format"], "uri");
        assert_eq!(json_val["description"], "Your website address");
    }

    #[test]
    fn test_url_elicitation_full_roundtrip() {
        let request = ElicitationBuilder::url_input(
            "Provide callback URL",
            "callbackUrl",
            "The URL for callbacks",
        );

        let json = serde_json::to_string(&request).unwrap();
        let parsed: elicitation::ElicitCreateRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.method, "elicitation/create");
        assert_eq!(parsed.params.message, "Provide callback URL");
        assert!(
            parsed
                .params
                .requested_schema
                .properties
                .contains_key("callbackUrl")
        );
    }
}

// ===================================================================
// Sampling Tools
// ===================================================================

mod sampling_tools {
    use super::*;
    use turul_mcp_protocol::sampling::*;

    #[test]
    fn test_create_message_params_without_tools() {
        let msg = SamplingMessage::user_text("Hello");
        let params = CreateMessageParams::new(vec![msg], 100);

        assert!(params.tools.is_none(), "tools should be None by default");

        let json_val = serde_json::to_value(&params).unwrap();
        assert!(
            json_val.get("tools").is_none(),
            "tools field should be omitted when None"
        );
    }

    #[test]
    fn test_create_message_params_with_tools() {
        let tool = tools::Tool::new("calculator", tools::ToolSchema::object())
            .with_description("A calculator tool");

        let msg = SamplingMessage::user_text("Calculate 2+2");
        let params = CreateMessageParams::new(vec![msg], 200).with_tools(vec![tool]);

        assert!(params.tools.is_some());
        assert_eq!(params.tools.as_ref().unwrap().len(), 1);
        assert_eq!(params.tools.as_ref().unwrap()[0].name, "calculator");
    }

    #[test]
    fn test_create_message_params_with_tools_serialization() {
        let tool = tools::Tool::new("search", tools::ToolSchema::object())
            .with_description("Search the web");

        let msg = SamplingMessage::user_text("Find info");
        let params = CreateMessageParams::new(vec![msg], 500).with_tools(vec![tool]);

        let json_val = serde_json::to_value(&params).unwrap();

        assert!(json_val["tools"].is_array(), "tools should be an array");
        assert_eq!(json_val["tools"].as_array().unwrap().len(), 1);
        assert_eq!(json_val["tools"][0]["name"], "search");
        assert_eq!(json_val["tools"][0]["description"], "Search the web");
    }

    #[test]
    fn test_create_message_params_with_empty_tools() {
        let msg = SamplingMessage::user_text("Hello");
        let params = CreateMessageParams::new(vec![msg], 100).with_tools(vec![]);

        let json_val = serde_json::to_value(&params).unwrap();
        assert!(json_val["tools"].is_array());
        assert_eq!(
            json_val["tools"].as_array().unwrap().len(),
            0,
            "Empty tools array should serialize"
        );
    }

    #[test]
    fn test_create_message_params_with_tools_roundtrip() {
        let tool1 = tools::Tool::new("tool-a", tools::ToolSchema::object())
            .with_description("First tool");
        let tool2 = tools::Tool::new("tool-b", tools::ToolSchema::object())
            .with_description("Second tool");

        let msg = SamplingMessage::user_text("Use tools");
        let params = CreateMessageParams::new(vec![msg], 300).with_tools(vec![tool1, tool2]);

        let json = serde_json::to_string(&params).unwrap();
        let parsed: CreateMessageParams = serde_json::from_str(&json).unwrap();

        let tools = parsed.tools.unwrap();
        assert_eq!(tools.len(), 2);
        assert_eq!(tools[0].name, "tool-a");
        assert_eq!(tools[1].name, "tool-b");
    }

    #[test]
    fn test_create_message_request_with_tools() {
        let tool = tools::Tool::new("echo", tools::ToolSchema::object());
        let msg = SamplingMessage::user_text("Echo this");
        let request =
            CreateMessageRequest::new(vec![msg], 100).with_tools(vec![tool]);

        assert_eq!(request.method, "sampling/createMessage");
        assert!(request.params.tools.is_some());
        assert_eq!(request.params.tools.as_ref().unwrap()[0].name, "echo");
    }

    #[test]
    fn test_sampling_tools_match_regular_tool_schema() {
        // Verify that tools in sampling have the same schema structure as regular tools
        let regular_tool = tools::Tool::new("calc", tools::ToolSchema::object())
            .with_description("Calculator")
            .with_icons(vec![Icon::new("https://example.com/calc.png")]);

        let msg = SamplingMessage::user_text("Calculate");
        let params = CreateMessageParams::new(vec![msg], 100).with_tools(vec![regular_tool]);

        let json_val = serde_json::to_value(&params).unwrap();
        let tool_json = &json_val["tools"][0];

        // Same fields as a regular Tool
        assert_eq!(tool_json["name"], "calc");
        assert_eq!(tool_json["description"], "Calculator");
        assert_eq!(tool_json["icons"][0]["src"], "https://example.com/calc.png");
        assert!(tool_json["inputSchema"].is_object());
    }
}

// ===================================================================
// Tasks
// ===================================================================

mod task_tests {
    use super::*;
    use turul_mcp_protocol::meta::Cursor;

    const TIMESTAMP: &str = "2024-01-01T00:00:00Z";

    // -- TaskStatus enum --

    #[test]
    fn test_task_status_working_serialization() {
        let json_val = serde_json::to_value(TaskStatus::Working).unwrap();
        assert_eq!(json_val, json!("working"));
    }

    #[test]
    fn test_task_status_completed_serialization() {
        let json_val = serde_json::to_value(TaskStatus::Completed).unwrap();
        assert_eq!(json_val, json!("completed"));
    }

    #[test]
    fn test_task_status_failed_serialization() {
        let json_val = serde_json::to_value(TaskStatus::Failed).unwrap();
        assert_eq!(json_val, json!("failed"));
    }

    #[test]
    fn test_task_status_cancelled_serialization() {
        let json_val = serde_json::to_value(TaskStatus::Cancelled).unwrap();
        assert_eq!(json_val, json!("cancelled"));
    }

    #[test]
    fn test_task_status_all_variants_roundtrip() {
        for status in [
            TaskStatus::Working,
            TaskStatus::Completed,
            TaskStatus::Failed,
            TaskStatus::Cancelled,
        ] {
            let json = serde_json::to_string(&status).unwrap();
            let parsed: TaskStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(status, parsed, "TaskStatus round-trip failed for {:?}", status);
        }
    }

    // -- Task (formerly TaskInfo) --

    #[test]
    fn test_task_minimal() {
        let task = Task::new("task-001", TaskStatus::Working, TIMESTAMP, TIMESTAMP);
        assert_eq!(task.task_id, "task-001");
        assert_eq!(task.status, TaskStatus::Working);
        assert!(task.status_message.is_none());
        assert!(task.meta.is_none());
    }

    #[test]
    fn test_task_full() {
        let mut meta = HashMap::new();
        meta.insert("tool".to_string(), json!("calculator"));

        let task = Task::new("task-002", TaskStatus::Completed, TIMESTAMP, TIMESTAMP)
            .with_status_message("Calculation done")
            .with_meta(meta);

        assert_eq!(task.task_id, "task-002");
        assert_eq!(task.status, TaskStatus::Completed);
        assert_eq!(task.status_message, Some("Calculation done".to_string()));
        assert_eq!(
            task.meta.as_ref().unwrap()["tool"],
            json!("calculator")
        );
    }

    #[test]
    fn test_task_roundtrip() {
        let task = Task::new("task-rt", TaskStatus::Failed, TIMESTAMP, TIMESTAMP)
            .with_status_message("Disk full");

        let json = serde_json::to_string(&task).unwrap();
        let parsed: Task = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.task_id, "task-rt");
        assert_eq!(parsed.status, TaskStatus::Failed);
        assert_eq!(parsed.status_message, Some("Disk full".to_string()));
    }

    #[test]
    fn test_task_optional_fields_omitted() {
        let task = Task::new("task-min", TaskStatus::Working, TIMESTAMP, TIMESTAMP);
        let json_val = serde_json::to_value(&task).unwrap();

        assert!(json_val.get("taskId").is_some());
        assert!(json_val.get("status").is_some());
        assert!(json_val.get("createdAt").is_some());
        assert!(json_val.get("lastUpdatedAt").is_some());
        assert!(json_val.get("statusMessage").is_none(), "statusMessage should be omitted");
        assert!(json_val.get("_meta").is_none(), "_meta should be omitted");
    }

    // -- Task lifecycle simulation (get and cancel only - no create) --

    #[test]
    fn test_task_lifecycle_get_cancel() {
        // 1. Get a task
        let get_req = GetTaskRequest::new("task-lifecycle");
        assert_eq!(get_req.method, "tasks/get");
        assert_eq!(get_req.params.task_id, "task-lifecycle");

        let in_progress_task = Task::new("task-lifecycle", TaskStatus::Working, TIMESTAMP, TIMESTAMP)
            .with_status_message("Processing data");
        let get_result = GetTaskResult::new(in_progress_task);
        assert_eq!(get_result.task.task_id, "task-lifecycle");
        assert_eq!(get_result.task.status, TaskStatus::Working);

        // 2. Cancel the task
        let cancel_req = CancelTaskRequest::new("task-lifecycle");
        assert_eq!(cancel_req.method, "tasks/cancel");

        let cancelled_task = Task::new("task-lifecycle", TaskStatus::Cancelled, TIMESTAMP, TIMESTAMP)
            .with_status_message("User cancelled");
        let cancel_result = CancelTaskResult::new(cancelled_task);
        assert_eq!(cancel_result.task.status, TaskStatus::Cancelled);
    }

    // -- tasks/list with pagination --

    #[test]
    fn test_list_tasks_no_pagination() {
        let req = ListTasksRequest::new();
        assert_eq!(req.method, "tasks/list");
        assert!(req.params.cursor.is_none());
        assert!(req.params.limit.is_none());
    }

    #[test]
    fn test_list_tasks_with_pagination() {
        let req = ListTasksRequest::new()
            .with_cursor(Cursor::new("page-2"))
            .with_limit(10);

        let json_val = serde_json::to_value(&req).unwrap();
        assert_eq!(json_val["params"]["cursor"], "page-2");
        assert_eq!(json_val["params"]["limit"], 10);
    }

    #[test]
    fn test_list_tasks_result_with_next_cursor() {
        let tasks = vec![
            Task::new("t1", TaskStatus::Working, TIMESTAMP, TIMESTAMP),
            Task::new("t2", TaskStatus::Completed, TIMESTAMP, TIMESTAMP),
            Task::new("t3", TaskStatus::Failed, TIMESTAMP, TIMESTAMP),
        ];

        let result = ListTasksResult::new(tasks).with_next_cursor(Cursor::new("page-3"));

        assert_eq!(result.tasks.len(), 3);
        assert_eq!(result.next_cursor.as_ref().unwrap().as_str(), "page-3");

        let json_val = serde_json::to_value(&result).unwrap();
        assert_eq!(json_val["nextCursor"], "page-3");
        assert_eq!(json_val["tasks"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn test_list_tasks_result_without_cursor() {
        let result = ListTasksResult::new(vec![
            Task::new("t1", TaskStatus::Completed, TIMESTAMP, TIMESTAMP),
        ]);

        let json_val = serde_json::to_value(&result).unwrap();
        assert!(
            json_val.get("nextCursor").is_none(),
            "nextCursor should be omitted when there is no next page"
        );
    }

    #[test]
    fn test_list_tasks_result_roundtrip() {
        let tasks = vec![
            Task::new("t-a", TaskStatus::Working, TIMESTAMP, TIMESTAMP)
                .with_status_message("Working"),
            Task::new("t-b", TaskStatus::Cancelled, TIMESTAMP, TIMESTAMP),
        ];
        let result = ListTasksResult::new(tasks).with_next_cursor(Cursor::new("cursor-xyz"));

        let json = serde_json::to_string(&result).unwrap();
        let parsed: ListTasksResult = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.tasks.len(), 2);
        assert_eq!(parsed.tasks[0].task_id, "t-a");
        assert_eq!(parsed.tasks[0].status, TaskStatus::Working);
        assert_eq!(parsed.tasks[1].task_id, "t-b");
        assert_eq!(parsed.tasks[1].status, TaskStatus::Cancelled);
        assert_eq!(parsed.next_cursor.as_ref().unwrap().as_str(), "cursor-xyz");
    }

    // -- Request serialization --

    #[test]
    fn test_get_task_request_serialization() {
        let req = GetTaskRequest::new("task-42");
        let json_val = serde_json::to_value(&req).unwrap();

        assert_eq!(json_val["method"], "tasks/get");
        assert_eq!(json_val["params"]["taskId"], "task-42");
    }

    #[test]
    fn test_cancel_task_request_serialization() {
        let req = CancelTaskRequest::new("task-99");
        let json_val = serde_json::to_value(&req).unwrap();

        assert_eq!(json_val["method"], "tasks/cancel");
        assert_eq!(json_val["params"]["taskId"], "task-99");
    }

    #[test]
    fn test_list_tasks_request_serialization() {
        let req = ListTasksRequest::new()
            .with_cursor(Cursor::new("abc"))
            .with_limit(25);

        let json_val = serde_json::to_value(&req).unwrap();
        assert_eq!(json_val["method"], "tasks/list");
        assert_eq!(json_val["params"]["cursor"], "abc");
        assert_eq!(json_val["params"]["limit"], 25);
    }

    // -- Result types --

    #[test]
    fn test_get_task_result_serialization() {
        let task = Task::new("got-task", TaskStatus::Completed, TIMESTAMP, TIMESTAMP)
            .with_status_message("All done");
        let result = GetTaskResult::new(task);

        let json_val = serde_json::to_value(&result).unwrap();
        // GetTaskResult flattens Task, so fields are at top level
        assert_eq!(json_val["taskId"], "got-task");
        assert_eq!(json_val["status"], "completed");
        assert_eq!(json_val["statusMessage"], "All done");
    }

    #[test]
    fn test_cancel_task_result_serialization() {
        let task = Task::new("cancelled-task", TaskStatus::Cancelled, TIMESTAMP, TIMESTAMP);
        let result = CancelTaskResult::new(task);

        let json_val = serde_json::to_value(&result).unwrap();
        // CancelTaskResult flattens Task, so fields are at top level
        assert_eq!(json_val["taskId"], "cancelled-task");
        assert_eq!(json_val["status"], "cancelled");
    }

    // -- Default impls --

    #[test]
    fn test_list_tasks_params_default() {
        let params = ListTasksParams::default();
        assert!(params.cursor.is_none());
        assert!(params.limit.is_none());
    }

    // -- Meta fields on task requests --

    #[test]
    fn test_get_task_request_with_meta() {
        let mut meta = HashMap::new();
        meta.insert("traceId".to_string(), json!("trace-abc"));

        let req = GetTaskRequest::new("task-x").with_meta(meta);
        let json_val = serde_json::to_value(&req).unwrap();
        assert_eq!(json_val["params"]["_meta"]["traceId"], "trace-abc");
    }

    // -- Task with meta --

    #[test]
    fn test_task_with_meta() {
        let mut meta = HashMap::new();
        meta.insert("timing".to_string(), json!(42));

        let task = Task::new("t1", TaskStatus::Completed, TIMESTAMP, TIMESTAMP)
            .with_meta(meta);

        let json_val = serde_json::to_value(&task).unwrap();
        assert_eq!(json_val["_meta"]["timing"], 42);
    }

    // -- ProgressNotification (replaces TaskProgress tests) --

    #[test]
    fn test_progress_notification_basic() {
        let notification = ProgressNotification::new("token-1".to_string(), 42.0);
        let json_val = serde_json::to_value(&notification).unwrap();

        assert_eq!(json_val["params"]["progress"], 42.0);
        assert!(
            json_val["params"].get("total").is_none(),
            "total should be omitted when None"
        );
    }

    #[test]
    fn test_progress_notification_with_total() {
        let notification = ProgressNotification::new("token-2".to_string(), 75.0)
            .with_total(100.0);
        let json_val = serde_json::to_value(&notification).unwrap();

        assert_eq!(json_val["params"]["progress"], 75.0);
        assert_eq!(json_val["params"]["total"], 100.0);
    }

    #[test]
    fn test_progress_notification_roundtrip() {
        let original = ProgressNotification::new("token-3".to_string(), 50.0)
            .with_total(200.0);
        let json = serde_json::to_string(&original).unwrap();
        let parsed: ProgressNotification = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.params.progress, 50.0);
        assert_eq!(parsed.params.total, Some(200.0));
    }
}

// ===================================================================
// camelCase Verification
// ===================================================================

mod camel_case_verification {
    use super::*;
    use turul_mcp_protocol::meta::Cursor;

    const TIMESTAMP: &str = "2024-01-01T00:00:00Z";

    /// Helper to verify no snake_case keys in a JSON object (recursive)
    fn assert_no_snake_case_keys(val: &Value, context: &str) {
        match val {
            Value::Object(map) => {
                for (key, child) in map {
                    // _meta is a special case - it starts with underscore by design
                    if key != "_meta" {
                        assert!(
                            !key.contains('_'),
                            "Found snake_case key '{}' in {}",
                            key,
                            context
                        );
                    }
                    assert_no_snake_case_keys(child, &format!("{}.{}", context, key));
                }
            }
            Value::Array(arr) => {
                for (i, item) in arr.iter().enumerate() {
                    assert_no_snake_case_keys(item, &format!("{}[{}]", context, i));
                }
            }
            _ => {}
        }
    }

    #[test]
    fn test_task_camel_case() {
        let mut meta = HashMap::new();
        meta.insert("key".to_string(), json!("val"));

        let task = Task::new("t1", TaskStatus::Working, TIMESTAMP, TIMESTAMP)
            .with_status_message("Working")
            .with_meta(meta);

        let json_val = serde_json::to_value(&task).unwrap();
        assert_no_snake_case_keys(&json_val, "Task");
    }

    #[test]
    fn test_list_tasks_result_camel_case() {
        let tasks = vec![Task::new("t1", TaskStatus::Completed, TIMESTAMP, TIMESTAMP)];
        let result = ListTasksResult::new(tasks).with_next_cursor(Cursor::new("cursor"));

        let json_val = serde_json::to_value(&result).unwrap();
        assert_no_snake_case_keys(&json_val, "ListTasksResult");

        // Explicitly check the most important camelCase field
        assert!(json_val.get("nextCursor").is_some(), "Should use nextCursor (camelCase)");
        assert!(json_val.get("next_cursor").is_none(), "Should NOT have next_cursor");
    }

    #[test]
    fn test_get_task_request_camel_case() {
        let mut meta = HashMap::new();
        meta.insert("requestId".to_string(), json!("req-1"));

        let req = GetTaskRequest::new("task-1").with_meta(meta);

        let json_val = serde_json::to_value(&req).unwrap();
        assert_no_snake_case_keys(&json_val, "GetTaskRequest");
    }

    #[test]
    fn test_tool_with_icons_camel_case() {
        let tool = tools::Tool::new("my-tool", tools::ToolSchema::object())
            .with_description("A tool")
            .with_icons(vec![Icon::new("https://example.com/icon.png")])
            .with_annotations(
                tools::ToolAnnotations::new()
                    .with_read_only_hint(true)
                    .with_destructive_hint(false)
                    .with_idempotent_hint(true)
                    .with_open_world_hint(false),
            );

        let json_val = serde_json::to_value(&tool).unwrap();
        assert_no_snake_case_keys(&json_val, "Tool");

        // Verify specific annotation fields are camelCase
        let annotations = &json_val["annotations"];
        assert!(annotations.get("readOnlyHint").is_some());
        assert!(annotations.get("destructiveHint").is_some());
        assert!(annotations.get("idempotentHint").is_some());
        assert!(annotations.get("openWorldHint").is_some());
    }

    #[test]
    fn test_create_message_params_with_tools_camel_case() {
        let tool = tools::Tool::new("t", tools::ToolSchema::object());
        let msg = sampling::SamplingMessage::user_text("Hi");
        let params = sampling::CreateMessageParams::new(vec![msg], 100)
            .with_tools(vec![tool])
            .with_system_prompt("Be helpful")
            .with_temperature(0.7);

        let json_val = serde_json::to_value(&params).unwrap();
        assert_no_snake_case_keys(&json_val, "CreateMessageParams");

        // Verify specific camelCase fields
        assert!(json_val.get("maxTokens").is_some());
        assert!(json_val.get("systemPrompt").is_some());
    }

    #[test]
    fn test_elicitation_request_camel_case() {
        let req = ElicitationBuilder::url_input("Enter URL", "url", "A URL");
        let json_val = serde_json::to_value(&req).unwrap();
        assert_no_snake_case_keys(&json_val, "ElicitCreateRequest");

        // Verify requestedSchema is camelCase
        assert!(
            json_val["params"].get("requestedSchema").is_some(),
            "Should use requestedSchema (camelCase)"
        );
    }

    #[test]
    fn test_implementation_camel_case() {
        let impl_info = Implementation::new("srv", "1.0.0")
            .with_title("My Server")
            .with_icons(vec![Icon::new("https://example.com/icon.png")]);

        let json_val = serde_json::to_value(&impl_info).unwrap();
        assert_no_snake_case_keys(&json_val, "Implementation");
    }

    #[test]
    fn test_resource_camel_case() {
        let resource = resources::Resource::new("file:///test.txt", "test")
            .with_icons(vec![Icon::new("https://example.com/icon.png")]);

        let json_val = serde_json::to_value(&resource).unwrap();
        assert_no_snake_case_keys(&json_val, "Resource");
    }

    #[test]
    fn test_prompt_camel_case() {
        let prompt = prompts::Prompt::new("summarize")
            .with_description("Summarize text")
            .with_icons(vec![Icon::new("https://example.com/icon.png")]);

        let json_val = serde_json::to_value(&prompt).unwrap();
        assert_no_snake_case_keys(&json_val, "Prompt");
    }
}

// ===================================================================
// Version Detection
// ===================================================================

mod version_detection {
    use super::*;

    #[test]
    fn test_v2025_11_25_supports_tasks() {
        assert!(
            McpVersion::V2025_11_25.supports_tasks(),
            "2025-11-25 should support tasks"
        );
    }

    #[test]
    fn test_v2025_11_25_supports_icons() {
        assert!(
            McpVersion::V2025_11_25.supports_icons(),
            "2025-11-25 should support icons"
        );
    }

    #[test]
    fn test_v2025_11_25_supports_url_elicitation() {
        assert!(
            McpVersion::V2025_11_25.supports_url_elicitation(),
            "2025-11-25 should support URL elicitation"
        );
    }

    #[test]
    fn test_v2025_11_25_supports_sampling_tools() {
        assert!(
            McpVersion::V2025_11_25.supports_sampling_tools(),
            "2025-11-25 should support sampling tools"
        );
    }

    #[test]
    fn test_v2025_11_25_inherits_previous_features() {
        let v = McpVersion::V2025_11_25;
        assert!(v.supports_streamable_http(), "Should inherit streamable HTTP");
        assert!(v.supports_meta_fields(), "Should inherit _meta fields");
        assert!(
            v.supports_progress_and_cursor(),
            "Should inherit progress and cursor"
        );
        assert!(v.supports_elicitation(), "Should inherit elicitation");
    }

    #[test]
    fn test_older_versions_do_not_support_new_features() {
        for version in [
            McpVersion::V2024_11_05,
            McpVersion::V2025_03_26,
            McpVersion::V2025_06_18,
        ] {
            assert!(
                !version.supports_tasks(),
                "{} should NOT support tasks",
                version
            );
            assert!(
                !version.supports_icons(),
                "{} should NOT support icons",
                version
            );
            assert!(
                !version.supports_url_elicitation(),
                "{} should NOT support URL elicitation",
                version
            );
            assert!(
                !version.supports_sampling_tools(),
                "{} should NOT support sampling tools",
                version
            );
        }
    }

    #[test]
    fn test_supported_features_list() {
        let features = McpVersion::V2025_11_25.supported_features();
        assert!(features.contains(&"tasks"));
        assert!(features.contains(&"icons"));
        assert!(features.contains(&"url-elicitation"));
        assert!(features.contains(&"sampling-tools"));
        assert!(features.contains(&"streamable-http"));
        assert!(features.contains(&"_meta-fields"));
        assert!(features.contains(&"elicitation"));
    }

    #[test]
    fn test_version_serialization() {
        let v = McpVersion::V2025_11_25;
        let json_val = serde_json::to_value(&v).unwrap();
        assert_eq!(json_val, json!("2025-11-25"));
    }

    #[test]
    fn test_version_deserialization() {
        let v: McpVersion = serde_json::from_value(json!("2025-11-25")).unwrap();
        assert_eq!(v, McpVersion::V2025_11_25);
    }

    #[test]
    fn test_version_roundtrip() {
        let original = McpVersion::V2025_11_25;
        let json = serde_json::to_string(&original).unwrap();
        let parsed: McpVersion = serde_json::from_str(&json).unwrap();
        assert_eq!(original, parsed);
    }

    #[test]
    fn test_version_string_parsing() {
        let v: McpVersion = "2025-11-25".parse().unwrap();
        assert_eq!(v, McpVersion::V2025_11_25);
    }

    #[test]
    fn test_unknown_version_parsing_fails() {
        let result: Result<McpVersion, _> = "9999-01-01".parse();
        assert!(result.is_err(), "Unknown version should fail to parse");
    }

    #[test]
    fn test_version_ordering() {
        assert!(McpVersion::V2024_11_05 < McpVersion::V2025_03_26);
        assert!(McpVersion::V2025_03_26 < McpVersion::V2025_06_18);
        assert!(McpVersion::V2025_06_18 < McpVersion::V2025_11_25);
    }

    #[test]
    fn test_latest_and_current() {
        assert_eq!(McpVersion::LATEST, McpVersion::V2025_11_25);
        assert_eq!(McpVersion::CURRENT, McpVersion::V2025_11_25);
    }

    #[test]
    fn test_version_display() {
        assert_eq!(format!("{}", McpVersion::V2025_11_25), "2025-11-25");
    }

    #[test]
    fn test_mcp_version_constant() {
        assert_eq!(MCP_VERSION, "2025-11-25");
    }
}

// ===================================================================
// Cross-feature Integration
// ===================================================================

mod cross_feature {
    use super::*;

    const TIMESTAMP: &str = "2024-01-01T00:00:00Z";

    #[test]
    fn test_tool_with_icons_in_sampling() {
        // Tool with icons used in a sampling request
        let tool = tools::Tool::new("search", tools::ToolSchema::object())
            .with_description("Web search")
            .with_icons(vec![Icon::data_uri("image/png", "XXXX")]);

        let msg = sampling::SamplingMessage::user_text("Search for Rust");
        let params = sampling::CreateMessageParams::new(vec![msg], 500)
            .with_tools(vec![tool]);

        let json_val = serde_json::to_value(&params).unwrap();

        // Tool in sampling should preserve icons
        assert_eq!(json_val["tools"][0]["icons"][0]["src"], "data:image/png;base64,XXXX");
        assert_eq!(json_val["tools"][0]["name"], "search");
    }

    #[test]
    fn test_task_with_meta_and_status_message() {
        let mut meta = HashMap::new();
        meta.insert("progressToken".to_string(), json!("pt-123"));

        let task = Task::new("task-combined", TaskStatus::Working, TIMESTAMP, TIMESTAMP)
            .with_status_message("Searching...")
            .with_meta(meta);

        let json_val = serde_json::to_value(&task).unwrap();

        // All fields present and correctly cased
        assert_eq!(json_val["taskId"], "task-combined");
        assert_eq!(json_val["status"], "working");
        assert_eq!(json_val["statusMessage"], "Searching...");
        assert_eq!(json_val["createdAt"], TIMESTAMP);
        assert_eq!(json_val["lastUpdatedAt"], TIMESTAMP);
        assert_eq!(json_val["_meta"]["progressToken"], "pt-123");
    }

    #[test]
    fn test_deserialize_external_json_task() {
        // Simulate receiving a task from an external source
        let external_json = json!({
            "taskId": "ext-task-1",
            "status": "completed",
            "createdAt": "2024-01-01T00:00:00Z",
            "lastUpdatedAt": "2024-01-01T01:00:00Z",
            "statusMessage": "Data processed"
        });

        let task: Task = serde_json::from_value(external_json).unwrap();
        assert_eq!(task.task_id, "ext-task-1");
        assert_eq!(task.status, TaskStatus::Completed);
        assert_eq!(task.status_message, Some("Data processed".to_string()));
    }

    #[test]
    fn test_deserialize_external_json_tool_with_icons() {
        let external_json = json!({
            "name": "ext-tool",
            "description": "External tool",
            "inputSchema": {
                "type": "object"
            },
            "icons": [{"src": "https://cdn.example.com/tool.svg"}]
        });

        let tool: tools::Tool = serde_json::from_value(external_json).unwrap();
        assert_eq!(tool.name, "ext-tool");
        assert_eq!(
            tool.icons.as_ref().unwrap()[0].src,
            "https://cdn.example.com/tool.svg"
        );
    }
}
