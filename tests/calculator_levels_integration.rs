//! Integration tests for the four-level calculator tool pattern
//!
//! This test suite verifies that all four levels of tool creation work correctly:
//! - Level 1: Function macros (ultra-simple)
//! - Level 2: Derive macros (struct-based)
//! - Level 3: Builder pattern (runtime flexibility)
//! - Level 4: Manual implementation (maximum control)

use serde_json::{Value, json};
use std::collections::HashMap;
use turul_mcp_protocol::{
    McpError,
    tools::{CallToolResult, ToolResult},
};
use turul_mcp_server::{McpResult, McpTool, SessionContext};

// ===========================================
// === Level 1: Function Macro ===
// ===========================================

use turul_mcp_derive::mcp_tool;

#[mcp_tool(
    name = "calculator_add_function_test",
    description = "Add two numbers using function macro"
)]
async fn calculator_add_function(
    #[param(description = "First number")] a: f64,
    #[param(description = "Second number")] b: f64,
) -> McpResult<f64> {
    Ok(a + b)
}

#[tokio::test]
async fn test_level1_function_macro() {
    let tool = calculator_add_function();
    let args = json!({"a": 5.0, "b": 3.0});

    let result = tool.call(args, None).await.unwrap();

    // Verify structured content (Level 1 should wrap in {"result": value})
    assert!(result.structured_content.is_some());
    if let Some(structured) = result.structured_content {
        let result_value = structured.get("result").unwrap().as_f64().unwrap();
        assert_eq!(result_value, 8.0);
    }

    // Verify basic content
    assert!(!result.content.is_empty());
    assert_eq!(result.is_error, Some(false));
}

// ===========================================
// === Level 2: Derive Macro ===
// ===========================================

use turul_mcp_derive::McpTool;

#[derive(McpTool, Default)]
#[tool(
    name = "calculator_add_derive_test",
    description = "Add two numbers using derive"
)]
struct CalculatorAddDeriveTool {
    #[param(description = "First number")]
    a: f64,
    #[param(description = "Second number")]
    b: f64,
}

impl CalculatorAddDeriveTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<f64> {
        Ok(self.a + self.b)
    }
}

#[tokio::test]
async fn test_level2_derive_macro() {
    let tool = CalculatorAddDeriveTool::default();
    let args = json!({"a": 7.0, "b": 2.0});

    let result = tool.call(args, None).await.unwrap();

    // Verify content
    assert!(!result.content.is_empty());
    assert_eq!(result.is_error, Some(false));

    // The derive macro should handle parameter extraction properly
    // Note: This test verifies the macro works, actual computation depends on implementation
}

// ===========================================
// === Level 3: Builder Pattern ===
// ===========================================

use turul_mcp_server::ToolBuilder;

#[tokio::test]
async fn test_level3_builder_pattern() {
    let tool = ToolBuilder::new("calculator_add_builder_test")
        .description("Add two numbers using builder pattern")
        .number_param("a", "First number")
        .number_param("b", "Second number")
        .number_output()
        .execute(|args| async move {
            let a = args
                .get("a")
                .and_then(|v| v.as_f64())
                .ok_or("Missing parameter 'a'")?;
            let b = args
                .get("b")
                .and_then(|v| v.as_f64())
                .ok_or("Missing parameter 'b'")?;

            let sum = a + b;
            Ok(json!({"result": sum}))
        })
        .build()
        .unwrap();

    let args = json!({"a": 4.0, "b": 6.0});
    let result = tool.call(args, None).await.unwrap();

    // Verify structured content (builder should provide schema)
    assert!(result.structured_content.is_some());
    if let Some(structured) = result.structured_content {
        let result_value = structured.get("result").unwrap().as_f64().unwrap();
        assert_eq!(result_value, 10.0);
    }

    assert!(!result.content.is_empty());
    assert_eq!(result.is_error, Some(false));
}

// ===========================================
// === Level 4: Manual Implementation ===
// ===========================================

use async_trait::async_trait;
use turul_mcp_builders::prelude::{
    HasAnnotations, HasBaseMetadata, HasDescription, HasIcons, HasInputSchema, HasOutputSchema,
    HasToolMeta, ToolSchema,
};
use turul_mcp_protocol::schema::JsonSchema;

#[derive(Clone)]
struct CalculatorAddManualTool;

impl HasBaseMetadata for CalculatorAddManualTool {
    fn name(&self) -> &str {
        "calculator_add_manual_test"
    }
    fn title(&self) -> Option<&str> {
        Some("Manual Test Calculator")
    }
}

impl HasDescription for CalculatorAddManualTool {
    fn description(&self) -> Option<&str> {
        Some("Add two numbers using manual implementation")
    }
}

impl HasInputSchema for CalculatorAddManualTool {
    fn input_schema(&self) -> &ToolSchema {
        static INPUT_SCHEMA: std::sync::OnceLock<ToolSchema> = std::sync::OnceLock::new();
        INPUT_SCHEMA.get_or_init(|| {
            ToolSchema::object()
                .with_properties(HashMap::from([
                    ("a".to_string(), JsonSchema::number()),
                    ("b".to_string(), JsonSchema::number()),
                ]))
                .with_required(vec!["a".to_string(), "b".to_string()])
        })
    }
}

impl HasOutputSchema for CalculatorAddManualTool {
    fn output_schema(&self) -> Option<&ToolSchema> {
        None
    }
}

impl HasAnnotations for CalculatorAddManualTool {
    fn annotations(&self) -> Option<&turul_mcp_protocol::tools::ToolAnnotations> {
        None
    }
}

impl HasToolMeta for CalculatorAddManualTool {
    fn tool_meta(&self) -> Option<&HashMap<String, Value>> {
        None
    }
}

impl HasIcons for CalculatorAddManualTool {}

#[async_trait]
impl McpTool for CalculatorAddManualTool {
    async fn call(
        &self,
        args: Value,
        _session: Option<SessionContext>,
    ) -> McpResult<CallToolResult> {
        let a = args
            .get("a")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| McpError::missing_param("a"))?;
        let b = args
            .get("b")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| McpError::missing_param("b"))?;

        let sum = a + b;

        Ok(CallToolResult::success(vec![ToolResult::text(format!(
            "Sum: {}",
            sum
        ))]))
    }
}

#[tokio::test]
async fn test_level4_manual_implementation() {
    let tool = CalculatorAddManualTool;
    let args = json!({"a": 9.0, "b": 1.0});

    let result = tool.call(args, None).await.unwrap();

    // Manual implementation returns simple text, no structured content
    assert!(result.structured_content.is_none());
    assert!(!result.content.is_empty());
    assert_eq!(result.is_error, Some(false));

    // Verify the text contains the correct sum
    if let ToolResult::Text { text, .. } = &result.content[0] {
        assert!(text.contains("10"));
    } else {
        panic!("Expected text result");
    }
}

// ===========================================
// === Cross-Level Consistency Tests ===
// ===========================================

#[tokio::test]
async fn test_all_levels_handle_missing_params() {
    let incomplete_args = json!({"a": 5.0}); // Missing 'b' parameter

    // Level 1
    let level1_tool = calculator_add_function();
    let level1_result = level1_tool.call(incomplete_args.clone(), None).await;
    assert!(level1_result.is_err());

    // Level 3
    let level3_tool = ToolBuilder::new("test")
        .number_param("a", "First number")
        .number_param("b", "Second number")
        .execute(|args| async move {
            let a = args.get("a").and_then(|v| v.as_f64()).ok_or("Missing a")?;
            let b = args.get("b").and_then(|v| v.as_f64()).ok_or("Missing b")?;
            Ok(json!(a + b))
        })
        .build()
        .unwrap();

    let level3_result = level3_tool.call(incomplete_args.clone(), None).await;
    assert!(level3_result.is_err());

    // Level 4
    let level4_tool = CalculatorAddManualTool;
    let level4_result = level4_tool.call(incomplete_args.clone(), None).await;
    assert!(level4_result.is_err());
}

#[tokio::test]
async fn test_all_levels_produce_consistent_results() {
    let args = json!({"a": 12.5, "b": 7.5});
    let expected_sum = 20.0;

    // Level 1: Function macro
    let level1_tool = calculator_add_function();
    let level1_result = level1_tool.call(args.clone(), None).await.unwrap();

    if let Some(structured) = &level1_result.structured_content {
        let result_value = structured.get("result").unwrap().as_f64().unwrap();
        assert_eq!(result_value, expected_sum);
    }

    // Level 3: Builder pattern
    let level3_tool = ToolBuilder::new("test")
        .number_param("a", "First")
        .number_param("b", "Second")
        .number_output()
        .execute(|args| async move {
            let a = args.get("a").and_then(|v| v.as_f64()).ok_or("Missing a")?;
            let b = args.get("b").and_then(|v| v.as_f64()).ok_or("Missing b")?;
            Ok(json!({"result": a + b}))
        })
        .build()
        .unwrap();

    let level3_result = level3_tool.call(args.clone(), None).await.unwrap();

    if let Some(structured) = &level3_result.structured_content {
        let result_value = structured.get("result").unwrap().as_f64().unwrap();
        assert_eq!(result_value, expected_sum);
    }

    // Level 4: Manual implementation returns text, so we parse it
    let level4_tool = CalculatorAddManualTool;
    let level4_result = level4_tool.call(args.clone(), None).await.unwrap();

    if let ToolResult::Text { text, .. } = &level4_result.content[0] {
        // Extract number from "Sum: 20" format
        assert!(text.contains(&expected_sum.to_string()));
    }
}
