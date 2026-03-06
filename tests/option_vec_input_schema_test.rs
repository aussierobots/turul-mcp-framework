//! Regression tests: `Option<T>` and `Vec<T>` parameters must generate correct
//! JSON Schema types in `tools/list` input schemas.
//!
//! Before the fix, `Option<bool>`, `Option<u32>`, `Vec<String>`, etc. all
//! fell through to the `"type": "string"` fallback because `get_ident()`
//! returns `None` for types with angle-bracketed generic arguments.

use serde_json::Value;
use turul_mcp_builders::prelude::ToolDefinition;
use turul_mcp_derive::{McpTool, mcp_tool};
use turul_mcp_server::{McpResult, SessionContext};

// ─── Derive-macro tool ───────────────────────────────────────────────────────

#[derive(McpTool, Clone)]
#[tool(
    name = "derive_mixed_params",
    description = "Tool with mixed param types"
)]
#[allow(dead_code)]
struct DeriveMixedParams {
    #[param(description = "A required string")]
    name: String,

    #[param(description = "Optional boolean flag")]
    active: Option<bool>,

    #[param(description = "Optional integer count")]
    count: Option<u32>,

    #[param(description = "Optional float score")]
    score: Option<f64>,

    #[param(description = "Required list of tags")]
    tags: Vec<String>,

    #[param(description = "Optional list of labels")]
    labels: Option<Vec<String>>,
}

impl DeriveMixedParams {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<String> {
        Ok("ok".into())
    }
}

// ─── Function-attribute tool ─────────────────────────────────────────────────

#[mcp_tool(
    name = "attr_mixed_params",
    description = "Attr tool with mixed param types"
)]
async fn attr_mixed_params(
    name: String,
    active: Option<bool>,
    count: Option<u32>,
    score: Option<f64>,
    tags: Vec<String>,
    labels: Option<Vec<String>>,
) -> McpResult<String> {
    let _ = (name, active, count, score, tags, labels);
    Ok("ok".into())
}

// ─── Qualified-path tool ─────────────────────────────────────────────────────

#[derive(McpTool, Clone)]
#[tool(
    name = "qualified_path_params",
    description = "Tool with fully-qualified Option and Vec paths"
)]
#[allow(dead_code)]
struct QualifiedPathParams {
    #[param(description = "Qualified option bool")]
    flag: std::option::Option<bool>,

    #[param(description = "Qualified vec")]
    items: std::vec::Vec<String>,

    #[param(description = "Qualified option of qualified vec")]
    maybe_items: std::option::Option<Vec<String>>,
}

impl QualifiedPathParams {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<String> {
        Ok("ok".into())
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Extract the `"type"` value for a named property from a tool's input schema.
fn property_type(tool: &impl ToolDefinition, prop: &str) -> String {
    let schema: Value = serde_json::to_value(tool.input_schema()).unwrap();
    schema["properties"][prop]["type"]
        .as_str()
        .unwrap_or_else(|| panic!("no type for property '{prop}' in schema: {schema:#}"))
        .to_string()
}

/// Check whether `prop` appears in the schema's `required` array.
fn is_required(tool: &impl ToolDefinition, prop: &str) -> bool {
    let schema: Value = serde_json::to_value(tool.input_schema()).unwrap();
    schema["required"]
        .as_array()
        .is_some_and(|arr| arr.iter().any(|v| v.as_str() == Some(prop)))
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[test]
fn derive_tool_schema_types() {
    let tool = DeriveMixedParams {
        name: String::new(),
        active: None,
        count: None,
        score: None,
        tags: vec![],
        labels: None,
    };

    assert_eq!(property_type(&tool, "name"), "string");
    assert_eq!(property_type(&tool, "active"), "boolean");
    assert_eq!(property_type(&tool, "count"), "integer");
    assert_eq!(property_type(&tool, "score"), "number");
    assert_eq!(property_type(&tool, "tags"), "array");
    assert_eq!(property_type(&tool, "labels"), "array");
}

#[test]
fn derive_tool_required_fields() {
    let tool = DeriveMixedParams {
        name: String::new(),
        active: None,
        count: None,
        score: None,
        tags: vec![],
        labels: None,
    };

    assert!(is_required(&tool, "name"), "String should be required");
    assert!(
        !is_required(&tool, "active"),
        "Option<bool> should not be required"
    );
    assert!(
        !is_required(&tool, "count"),
        "Option<u32> should not be required"
    );
    assert!(
        !is_required(&tool, "score"),
        "Option<f64> should not be required"
    );
    assert!(is_required(&tool, "tags"), "Vec<String> should be required");
    assert!(
        !is_required(&tool, "labels"),
        "Option<Vec<String>> should not be required"
    );
}

#[test]
fn attr_tool_schema_types() {
    assert_eq!(property_type(&AttrMixedParamsToolImpl, "name"), "string");
    assert_eq!(property_type(&AttrMixedParamsToolImpl, "active"), "boolean");
    assert_eq!(property_type(&AttrMixedParamsToolImpl, "count"), "integer");
    assert_eq!(property_type(&AttrMixedParamsToolImpl, "score"), "number");
    assert_eq!(property_type(&AttrMixedParamsToolImpl, "tags"), "array");
    assert_eq!(property_type(&AttrMixedParamsToolImpl, "labels"), "array");
}

#[test]
fn attr_tool_required_fields() {
    assert!(is_required(&AttrMixedParamsToolImpl, "name"));
    assert!(!is_required(&AttrMixedParamsToolImpl, "active"));
    assert!(!is_required(&AttrMixedParamsToolImpl, "count"));
    assert!(!is_required(&AttrMixedParamsToolImpl, "score"));
    assert!(is_required(&AttrMixedParamsToolImpl, "tags"));
    assert!(!is_required(&AttrMixedParamsToolImpl, "labels"));
}

#[test]
fn qualified_path_schema_types() {
    let tool = QualifiedPathParams {
        flag: None,
        items: vec![],
        maybe_items: None,
    };

    assert_eq!(property_type(&tool, "flag"), "boolean");
    assert_eq!(property_type(&tool, "items"), "array");
    assert_eq!(property_type(&tool, "maybe_items"), "array");
}

#[test]
fn qualified_path_required_fields() {
    let tool = QualifiedPathParams {
        flag: None,
        items: vec![],
        maybe_items: None,
    };

    assert!(
        !is_required(&tool, "flag"),
        "std::option::Option<bool> should not be required"
    );
    assert!(
        is_required(&tool, "items"),
        "std::vec::Vec<String> should be required"
    );
    assert!(
        !is_required(&tool, "maybe_items"),
        "std::option::Option<Vec<String>> should not be required"
    );
}
