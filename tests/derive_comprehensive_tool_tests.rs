//! Comprehensive tests for MCP derive macros covering all data types and scenarios

use serde_json::json;
use turul_mcp_builders::prelude::*; // HasBaseMetadata, HasOutputSchema, etc.
use turul_mcp_derive::{McpTool, mcp_tool, tool};
use turul_mcp_server::{McpResult, McpTool as McpToolTrait, SessionContext};

/// Test tool returning f64 (number)
#[derive(McpTool)]
struct NumberTool {
    #[param(description = "Input value")]
    value: f64,
}

impl NumberTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<f64> {
        Ok(self.value * 2.0)
    }
}

/// Test tool returning String
#[derive(McpTool)]
struct StringTool {
    #[param(description = "Input text")]
    text: String,
}

impl StringTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<String> {
        Ok(format!("Processed: {}", self.text))
    }
}

/// Test tool returning bool
#[derive(McpTool)]
struct BooleanTool {
    #[param(description = "Input number")]
    number: i32,
}

impl BooleanTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<bool> {
        Ok(self.number > 0)
    }
}

/// Test tool returning Vec<i32> (array)
#[derive(McpTool)]
struct ArrayTool {
    #[param(description = "Array size")]
    size: u32,
}

impl ArrayTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<Vec<i32>> {
        Ok((0..self.size as i32).collect())
    }
}

/// Test tool returning custom struct (object)
#[derive(serde::Serialize)]
struct CustomResult {
    message: String,
    count: u32,
    success: bool,
}

#[derive(McpTool)]
struct ObjectTool {
    #[param(description = "Message text")]
    message: String,
    #[param(description = "Count value")]
    count: u32,
}

impl ObjectTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<CustomResult> {
        Ok(CustomResult {
            message: self.message.clone(),
            count: self.count,
            success: true,
        })
    }
}

#[tokio::test]
async fn test_number_tool_execution_and_schema() {
    let tool = NumberTool { value: 5.0 };

    // Test execution
    let result = tool.call(json!({"value": 5.0}), None).await;
    assert!(result.is_ok());

    let response = result.unwrap();

    // Should have text content
    assert!(!response.content.is_empty());

    // Should have structured content in MCP object format {"output": value} for primitives
    assert!(response.structured_content.is_some());
    if let Some(structured) = response.structured_content {
        let obj = structured.as_object().unwrap();
        assert!(
            obj.contains_key("output"),
            "Expected 'output' field for primitive type, got keys: {:?}",
            obj.keys().collect::<Vec<_>>()
        );
        assert_eq!(obj["output"], json!(10.0));
    }

    // Should have MCP-compliant object output schema after execution
    assert!(tool.output_schema().is_some());
    let schema = tool.output_schema().unwrap();
    assert_eq!(
        schema.schema_type, "object",
        "MCP requires all output schemas to be objects"
    );
}

#[tokio::test]
async fn test_string_tool_execution_and_schema() {
    let tool = StringTool {
        text: "test".to_string(),
    };

    // Test execution
    let result = tool.call(json!({"text": "test"}), None).await;
    assert!(result.is_ok());

    let response = result.unwrap();

    // Should have text content
    assert!(!response.content.is_empty());

    // Should have structured content in MCP object format {"output": value} for primitives
    assert!(response.structured_content.is_some());
    if let Some(structured) = response.structured_content {
        let obj = structured.as_object().unwrap();
        assert!(
            obj.contains_key("output"),
            "Expected 'output' field for primitive type, got keys: {:?}",
            obj.keys().collect::<Vec<_>>()
        );
        assert_eq!(obj["output"], json!("Processed: test"));
    }

    // Should have MCP-compliant object output schema after execution
    assert!(tool.output_schema().is_some());
    let schema = tool.output_schema().unwrap();
    assert_eq!(
        schema.schema_type, "object",
        "MCP requires all output schemas to be objects"
    );
}

#[tokio::test]
async fn test_boolean_tool_execution_and_schema() {
    let tool = BooleanTool { number: 5 };

    // Test execution
    let result = tool.call(json!({"number": 5}), None).await;
    assert!(result.is_ok());

    let response = result.unwrap();

    // Should have structured content in MCP object format {"output": value} for primitives
    assert!(response.structured_content.is_some());
    if let Some(structured) = response.structured_content {
        let obj = structured.as_object().unwrap();
        assert!(
            obj.contains_key("output"),
            "Expected 'output' field for primitive type, got keys: {:?}",
            obj.keys().collect::<Vec<_>>()
        );
        assert_eq!(obj["output"], json!(true));
    }

    // Should have MCP-compliant object output schema after execution
    assert!(tool.output_schema().is_some());
    let schema = tool.output_schema().unwrap();
    assert_eq!(
        schema.schema_type, "object",
        "MCP requires all output schemas to be objects"
    );
}

#[tokio::test]
async fn test_array_tool_execution_and_schema() {
    let tool = ArrayTool { size: 3 };

    // Test execution
    let result = tool.call(json!({"size": 3}), None).await;
    assert!(result.is_ok());

    let response = result.unwrap();

    // Should have structured content in MCP object format {"output": value} for primitives
    assert!(response.structured_content.is_some());
    if let Some(structured) = response.structured_content {
        let obj = structured.as_object().unwrap();
        assert!(
            obj.contains_key("output"),
            "Expected 'output' field for primitive type, got keys: {:?}",
            obj.keys().collect::<Vec<_>>()
        );
        assert_eq!(obj["output"], json!([0, 1, 2]));
    }

    // Should have MCP-compliant object output schema after execution
    assert!(tool.output_schema().is_some());
    let schema = tool.output_schema().unwrap();
    assert_eq!(
        schema.schema_type, "object",
        "MCP requires all output schemas to be objects"
    );
}

#[tokio::test]
async fn test_object_tool_execution_and_schema() {
    let tool = ObjectTool {
        message: "test".to_string(),
        count: 42,
    };

    // Test execution
    let result = tool
        .call(json!({"message": "test", "count": 42}), None)
        .await;
    assert!(result.is_ok());

    let response = result.unwrap();

    // Should have structured content in MCP object format with default "output" field
    assert!(response.structured_content.is_some());
    if let Some(structured) = response.structured_content {
        let obj = structured.as_object().unwrap();
        // Zero-config struct outputs use "output" as default field name
        assert!(obj.contains_key("output"));
        let expected_result = json!({
            "message": "test",
            "count": 42,
            "success": true
        });
        assert_eq!(obj["output"], expected_result);
    }

    // Should have MCP-compliant object output schema after execution
    assert!(tool.output_schema().is_some());
    let schema = tool.output_schema().unwrap();
    assert_eq!(
        schema.schema_type, "object",
        "MCP requires all output schemas to be objects"
    );
}

#[tokio::test]
async fn test_all_tools_have_schemas_after_execution() {
    // Create instances of all tool types
    let number_tool = NumberTool { value: 1.0 };
    let string_tool = StringTool {
        text: "test".to_string(),
    };
    let boolean_tool = BooleanTool { number: 1 };
    let array_tool = ArrayTool { size: 1 };
    let object_tool = ObjectTool {
        message: "test".to_string(),
        count: 1,
    };

    // Execute all tools
    let _ = number_tool.call(json!({"value": 1.0}), None).await;
    let _ = string_tool.call(json!({"text": "test"}), None).await;
    let _ = boolean_tool.call(json!({"number": 1}), None).await;
    let _ = array_tool.call(json!({"size": 1}), None).await;
    let _ = object_tool
        .call(json!({"message": "test", "count": 1}), None)
        .await;

    // All should have output schemas
    assert!(
        number_tool.output_schema().is_some(),
        "NumberTool should have output schema"
    );
    assert!(
        string_tool.output_schema().is_some(),
        "StringTool should have output schema"
    );
    assert!(
        boolean_tool.output_schema().is_some(),
        "BooleanTool should have output schema"
    );
    assert!(
        array_tool.output_schema().is_some(),
        "ArrayTool should have output schema"
    );
    assert!(
        object_tool.output_schema().is_some(),
        "ObjectTool should have output schema"
    );

    // Verify all schema types are MCP-compliant objects
    assert_eq!(
        number_tool.output_schema().unwrap().schema_type,
        "object",
        "All MCP schemas must be objects"
    );
    assert_eq!(
        string_tool.output_schema().unwrap().schema_type,
        "object",
        "All MCP schemas must be objects"
    );
    assert_eq!(
        boolean_tool.output_schema().unwrap().schema_type,
        "object",
        "All MCP schemas must be objects"
    );
    assert_eq!(
        array_tool.output_schema().unwrap().schema_type,
        "object",
        "All MCP schemas must be objects"
    );
    assert_eq!(
        object_tool.output_schema().unwrap().schema_type,
        "object",
        "All MCP schemas must be objects"
    );
}

#[tokio::test]
async fn test_struct_output_uses_struct_name_as_field() {
    let tool = ObjectTool {
        message: "test".to_string(),
        count: 42,
    };

    // Test execution
    let result = tool
        .call(json!({"message": "test", "count": 42}), None)
        .await;
    assert!(result.is_ok());

    let response = result.unwrap();

    // Should have structured content with default "output" field name
    assert!(response.structured_content.is_some());
    if let Some(structured) = response.structured_content {
        let obj = structured.as_object().unwrap();

        // Zero-config struct outputs use "output" as default field name
        assert!(
            obj.contains_key("output"),
            "Expected 'output' field for zero-config struct output, got keys: {:?}",
            obj.keys().collect::<Vec<_>>()
        );

        let expected_result = json!({
            "message": "test",
            "count": 42,
            "success": true
        });
        assert_eq!(obj["output"], expected_result);
    }
}

#[test]
fn test_tools_have_output_schemas_before_execution() {
    // Test that zero-config tools have output schemas even before first execution
    let number_tool = NumberTool { value: 0.0 };
    let string_tool = StringTool {
        text: String::new(),
    };
    let boolean_tool = BooleanTool { number: 0 };
    let array_tool = ArrayTool { size: 0 };
    let object_tool = ObjectTool {
        message: String::new(),
        count: 0,
    };

    // All tools should have output schemas for tools/list even before execution
    assert!(
        number_tool.output_schema().is_some(),
        "NumberTool should have output schema before execution"
    );
    assert!(
        string_tool.output_schema().is_some(),
        "StringTool should have output schema before execution"
    );
    assert!(
        boolean_tool.output_schema().is_some(),
        "BooleanTool should have output schema before execution"
    );
    assert!(
        array_tool.output_schema().is_some(),
        "ArrayTool should have output schema before execution"
    );
    assert!(
        object_tool.output_schema().is_some(),
        "ObjectTool should have output schema before execution"
    );

    // All schemas should be MCP-compliant objects
    assert_eq!(number_tool.output_schema().unwrap().schema_type, "object");
    assert_eq!(string_tool.output_schema().unwrap().schema_type, "object");
    assert_eq!(boolean_tool.output_schema().unwrap().schema_type, "object");
    assert_eq!(array_tool.output_schema().unwrap().schema_type, "object");
    assert_eq!(object_tool.output_schema().unwrap().schema_type, "object");
}

#[tokio::test]
async fn test_output_schema_upgrades_after_execution() {
    // Test that generic schema gets upgraded to specific schema after execution
    let tool = NumberTool { value: 5.0 };

    // Before execution: should have generic schema
    let schema_before = tool.output_schema().unwrap();
    assert_eq!(schema_before.schema_type, "object");

    // Execute tool
    let _result = tool.call(json!({"value": 5.0}), None).await.unwrap();

    // After execution: should have specific schema (still object, but potentially with different properties)
    let schema_after = tool.output_schema().unwrap();
    assert_eq!(schema_after.schema_type, "object");

    // Both should be valid MCP object schemas
    // Generic schema before execution may have no specific required fields (flexible schema)
    // After execution: should have specific required field based on output type
    if let Some(required_after) = &schema_after.required {
        assert!(
            !required_after.is_empty(),
            "Schema after execution should have required fields"
        );
        // Should have either "output" (primitive) or struct name (struct outputs)
        assert!(required_after.contains(&"output".to_string()) || required_after.len() == 1);
    }
}

#[test]
fn test_tool_names_auto_determined() {
    // Test zero-configuration tool name determination
    let number_tool = NumberTool { value: 0.0 };
    let string_tool = StringTool {
        text: String::new(),
    };
    let boolean_tool = BooleanTool { number: 0 };
    let array_tool = ArrayTool { size: 0 };
    let object_tool = ObjectTool {
        message: String::new(),
        count: 0,
    };

    // Names should be auto-determined from struct names
    assert_eq!(number_tool.name(), "number");
    assert_eq!(string_tool.name(), "string");
    assert_eq!(boolean_tool.name(), "boolean");
    assert_eq!(array_tool.name(), "array");
    assert_eq!(object_tool.name(), "object");
}

// =============================================================================
// ToolAnnotations macro support tests
// =============================================================================

/// Derive macro: tool with all annotations set
#[derive(McpTool)]
#[tool(name = "delete_file", description = "Delete a file",
       title = "File Deleter",
       annotation_title = "File Deletion",
       read_only = false, destructive = true,
       idempotent = true, open_world = false)]
struct DeleteFileTool {
    #[param(description = "File path")]
    path: String,
}

impl DeleteFileTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<String> {
        Ok(format!("Deleted: {}", self.path))
    }
}

/// Derive macro: tool with no annotations (backward compat)
#[derive(McpTool)]
#[tool(name = "plain_tool", description = "A plain tool")]
struct PlainTool {
    #[param(description = "Input")]
    value: String,
}

impl PlainTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<String> {
        Ok(self.value.clone())
    }
}

/// Derive macro: tool with partial annotations
#[derive(McpTool)]
#[tool(name = "reader", description = "Read data", read_only = true)]
struct ReaderTool {
    #[param(description = "Key")]
    key: String,
}

impl ReaderTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<String> {
        Ok(format!("Read: {}", self.key))
    }
}

/// Function macro: tool with annotations
#[mcp_tool(name = "web_search", description = "Search the web",
           title = "Web Search",
           read_only = true, open_world = true)]
async fn web_search(query: String) -> McpResult<String> {
    Ok(format!("Results for: {}", query))
}

/// Function macro: tool with no annotations
#[mcp_tool(name = "echo", description = "Echo input")]
async fn echo(text: String) -> McpResult<String> {
    Ok(text)
}

#[test]
fn test_derive_annotations_appear_in_to_tool() {
    let tool = DeleteFileTool {
        path: String::new(),
    };
    let tool_def = tool.to_tool();

    // title = "File Deleter" → Tool.title (HasBaseMetadata)
    assert_eq!(tool_def.title.as_deref(), Some("File Deleter"));

    // annotations should be present
    let annotations = tool_def.annotations.as_ref().expect("annotations should be Some");

    // annotation_title = "File Deletion" → ToolAnnotations.title
    assert_eq!(annotations.title.as_deref(), Some("File Deletion"));
    assert_eq!(annotations.read_only_hint, Some(false));
    assert_eq!(annotations.destructive_hint, Some(true));
    assert_eq!(annotations.idempotent_hint, Some(true));
    assert_eq!(annotations.open_world_hint, Some(false));

    // Verify camelCase serialization
    let json = serde_json::to_value(&tool_def).unwrap();
    let ann = json["annotations"].as_object().expect("annotations in JSON");
    assert_eq!(ann.get("readOnlyHint"), Some(&json!(false)));
    assert_eq!(ann.get("destructiveHint"), Some(&json!(true)));
    assert_eq!(ann.get("idempotentHint"), Some(&json!(true)));
    assert_eq!(ann.get("openWorldHint"), Some(&json!(false)));
    assert_eq!(ann.get("title"), Some(&json!("File Deletion")));
}

#[test]
fn test_derive_no_annotations_returns_none() {
    let tool = PlainTool {
        value: String::new(),
    };
    let tool_def = tool.to_tool();

    assert!(tool_def.annotations.is_none());
    assert!(tool_def.title.is_none());

    // JSON should not have annotations key (skip_serializing_if)
    let json = serde_json::to_value(&tool_def).unwrap();
    assert!(json.get("annotations").is_none());
}

#[test]
fn test_derive_partial_annotations() {
    let tool = ReaderTool {
        key: String::new(),
    };
    let tool_def = tool.to_tool();

    let annotations = tool_def.annotations.as_ref().expect("annotations should be Some");
    assert_eq!(annotations.read_only_hint, Some(true));
    // Unset fields should be None
    assert!(annotations.destructive_hint.is_none());
    assert!(annotations.idempotent_hint.is_none());
    assert!(annotations.open_world_hint.is_none());
    assert!(annotations.title.is_none());
}

#[test]
fn test_function_macro_annotations_in_to_tool() {
    let tool = web_search();
    let tool_def = tool.to_tool();

    // title = "Web Search" → Tool.title
    assert_eq!(tool_def.title.as_deref(), Some("Web Search"));

    // annotations should be present
    let annotations = tool_def.annotations.as_ref().expect("annotations should be Some");
    assert_eq!(annotations.read_only_hint, Some(true));
    assert_eq!(annotations.open_world_hint, Some(true));
    // Unset fields should be None
    assert!(annotations.destructive_hint.is_none());
    assert!(annotations.idempotent_hint.is_none());

    // Verify camelCase serialization
    let json = serde_json::to_value(&tool_def).unwrap();
    let ann = json["annotations"].as_object().expect("annotations in JSON");
    assert_eq!(ann.get("readOnlyHint"), Some(&json!(true)));
    assert_eq!(ann.get("openWorldHint"), Some(&json!(true)));
}

#[test]
fn test_function_macro_no_annotations() {
    let tool = echo();
    let tool_def = tool.to_tool();

    assert!(tool_def.annotations.is_none());
    assert!(tool_def.title.is_none());
}

#[test]
fn test_tool_macro_annotations_in_to_tool() {
    let tool = tool! {
        name: "lookup",
        description: "Lookup a value",
        read_only: true,
        idempotent: true,
        annotation_title: "Value Lookup",
        params: {
            key: String => "The key to look up",
        },
        execute: |key: String| async move {
            Ok::<_, &str>(format!("value for {}", key))
        }
    };

    let tool_def = tool.to_tool();

    let annotations = tool_def.annotations.as_ref().expect("annotations should be Some");
    assert_eq!(annotations.read_only_hint, Some(true));
    assert_eq!(annotations.idempotent_hint, Some(true));
    assert_eq!(annotations.title.as_deref(), Some("Value Lookup"));
    // Unset fields should be None
    assert!(annotations.destructive_hint.is_none());
    assert!(annotations.open_world_hint.is_none());
}

#[test]
fn test_tool_macro_with_title() {
    let tool = tool! {
        name: "named_lookup",
        description: "Lookup with title",
        title: "Named Lookup Tool",
        read_only: true,
        params: {
            key: String => "The key to look up",
        },
        execute: |key: String| async move {
            Ok::<_, &str>(key)
        }
    };

    let tool_def = tool.to_tool();
    assert_eq!(tool_def.title.as_deref(), Some("Named Lookup Tool"));

    let annotations = tool_def.annotations.as_ref().expect("annotations should be Some");
    assert_eq!(annotations.read_only_hint, Some(true));
}

#[test]
fn test_tool_macro_no_annotations() {
    let tool = tool! {
        name: "simple",
        description: "Simple tool",
        params: {
            value: String => "Input value",
        },
        execute: |value: String| async move {
            Ok::<_, &str>(value)
        }
    };

    let tool_def = tool.to_tool();
    assert!(tool_def.annotations.is_none());
}

#[test]
fn test_title_routes_only_to_base_metadata() {
    // title = "X" should set Tool.title via HasBaseMetadata, NOT ToolAnnotations.title
    let tool = DeleteFileTool {
        path: String::new(),
    };

    // HasBaseMetadata::title()
    assert_eq!(tool.title(), Some("File Deleter"));

    // ToolAnnotations.title is set separately via annotation_title
    let ann = tool.annotations().unwrap();
    assert_eq!(ann.title.as_deref(), Some("File Deletion"));
    // They're independent
    assert_ne!(tool.title(), ann.title.as_deref());
}

#[tokio::test]
async fn test_server_tools_list_handler_includes_annotations() {
    // Exercise the actual ListToolsHandler (JSON-RPC dispatcher path)
    use std::collections::HashMap;
    use std::sync::Arc;
    use turul_mcp_json_rpc_server::JsonRpcHandler;
    use turul_mcp_protocol::tools::ListToolsResult;
    use turul_mcp_server::ListToolsHandler;

    let mut tools: HashMap<String, Arc<dyn McpToolTrait>> = HashMap::new();
    tools.insert(
        "delete_file".to_string(),
        Arc::new(DeleteFileTool {
            path: String::new(),
        }),
    );
    tools.insert(
        "plain_tool".to_string(),
        Arc::new(PlainTool {
            value: String::new(),
        }),
    );

    let handler = ListToolsHandler::new(tools, false);
    let result_value = handler
        .handle("tools/list", None, None)
        .await
        .expect("tools/list handler should succeed");

    // Parse the JSON-RPC response payload
    let response: ListToolsResult =
        serde_json::from_value(result_value.clone()).expect("should parse as ListToolsResult");
    assert_eq!(response.tools.len(), 2);

    // Find delete_file tool in response
    let delete_tool = response
        .tools
        .iter()
        .find(|t| t.name == "delete_file")
        .expect("delete_file tool should exist in tools/list response");

    // Verify annotations through the raw JSON (actual wire format)
    let raw_json = serde_json::to_value(delete_tool).unwrap();
    assert_eq!(raw_json["title"], json!("File Deleter"));
    let ann = raw_json["annotations"]
        .as_object()
        .expect("annotations present in JSON-RPC response");
    assert_eq!(ann.get("readOnlyHint"), Some(&json!(false)));
    assert_eq!(ann.get("destructiveHint"), Some(&json!(true)));
    assert_eq!(ann.get("idempotentHint"), Some(&json!(true)));
    assert_eq!(ann.get("openWorldHint"), Some(&json!(false)));
    assert_eq!(ann.get("title"), Some(&json!("File Deletion")));
    // No snake_case keys
    assert!(ann.get("read_only_hint").is_none());
    assert!(ann.get("destructive_hint").is_none());

    // Find plain_tool — should NOT have annotations
    let plain_tool = response
        .tools
        .iter()
        .find(|t| t.name == "plain_tool")
        .expect("plain_tool should exist in tools/list response");
    let plain_json = serde_json::to_value(plain_tool).unwrap();
    assert!(plain_json.get("annotations").is_none());
    assert!(plain_json.get("title").is_none());
}
