//! MCP Tool Trait
//!
//! This module defines the high-level trait for implementing MCP tools.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use turul_mcp_builders::prelude::*;
use turul_mcp_protocol::{CallToolResult, McpResult};

use crate::session::SessionContext;

/// High-level trait for implementing MCP tools
///
/// McpTool extends ToolDefinition with execution capabilities.
/// All metadata is provided by the ToolDefinition trait, ensuring
/// consistency between concrete Tool structs and dynamic implementations.
#[async_trait]
pub trait McpTool: ToolDefinition {
    /// Execute the tool with full session support
    ///
    /// This is the primary execution method that tools should implement.
    /// Returns a complete CallToolResponse with both content and structured data.
    async fn call(&self, args: Value, session: Option<SessionContext>)
    -> McpResult<CallToolResult>;
}

/// Converts an McpTool trait object to a protocol Tool descriptor
///
/// This is now a thin wrapper around the ToolDefinition::to_tool() method
/// for backward compatibility. New code should use tool.to_tool() directly.
pub fn tool_to_descriptor(tool: &dyn McpTool) -> turul_mcp_protocol::Tool {
    tool.to_tool()
}

/// Compute a stable fingerprint of the tool set for session versioning.
///
/// Produces a deterministic 16-char hex string from the full serialized `Tool`
/// descriptors (as returned by `tools/list`). Any change to any advertised field
/// — name, description, inputSchema, outputSchema, annotations, title, icons,
/// execution — produces a different fingerprint.
///
/// Uses FNV-1a (a fixed, well-known algorithm) instead of `DefaultHasher` which
/// is not guaranteed stable across Rust versions or builds.
///
/// HashMap fields in Tool descriptors (e.g., `ToolSchema.properties`,
/// `ToolSchema.additional`, nested `JsonSchema.properties`) have non-deterministic
/// iteration order. The serialized JSON is canonicalized (all object keys sorted
/// recursively) before hashing to ensure stability across processes and instances.
pub fn compute_tool_fingerprint(tools: &HashMap<String, Arc<dyn McpTool>>) -> String {
    let mut tool_names: Vec<&String> = tools.keys().collect();
    tool_names.sort();
    let mut canonical_parts: Vec<String> = Vec::with_capacity(tool_names.len());
    for name in &tool_names {
        let tool = &tools[*name];
        let descriptor = tool_to_descriptor(tool.as_ref());
        // Tool descriptors derive Serialize — failure here indicates a framework bug
        // that would also break tools/list. Fail closed, not open.
        let value = serde_json::to_value(&descriptor).unwrap_or_else(|e| {
            panic!(
                "Tool '{}' descriptor failed to serialize for fingerprint: {}",
                name, e
            )
        });
        let canonical_value = canonicalize_json(value);
        let json = serde_json::to_string(&canonical_value).unwrap_or_else(|e| {
            panic!(
                "Tool '{}' canonical descriptor failed to re-serialize: {}",
                name, e
            )
        });
        canonical_parts.push(json);
    }
    let canonical = canonical_parts.join("\n");
    // FNV-1a: stable, fixed algorithm — no external dependency
    let mut hash: u64 = 0xcbf29ce484222325; // offset basis
    for byte in canonical.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3); // prime
    }
    format!("{:016x}", hash)
}

/// Recursively sort all object keys in a JSON value for deterministic serialization.
/// HashMap iteration order is non-deterministic in Rust; this ensures identical
/// logical structures always produce identical JSON strings.
fn canonicalize_json(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let sorted: serde_json::Map<String, serde_json::Value> = map
                .into_iter()
                .map(|(k, v)| (k, canonicalize_json(v)))
                .collect::<std::collections::BTreeMap<_, _>>()
                .into_iter()
                .collect();
            serde_json::Value::Object(sorted)
        }
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(arr.into_iter().map(canonicalize_json).collect())
        }
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use turul_mcp_protocol::schema::JsonSchema;
    use turul_mcp_protocol::tools::{CallToolResult, ToolAnnotations, ToolResult, ToolSchema};
    // Framework traits already imported via prelude at module level

    struct TestTool {
        input_schema: ToolSchema,
    }

    impl TestTool {
        fn new() -> Self {
            let input_schema = ToolSchema::object()
                .with_properties(HashMap::from([(
                    "message".to_string(),
                    JsonSchema::string(),
                )]))
                .with_required(vec!["message".to_string()]);
            Self { input_schema }
        }
    }

    // Implement all the fine-grained traits
    impl HasBaseMetadata for TestTool {
        fn name(&self) -> &str {
            "test"
        }
        fn title(&self) -> Option<&str> {
            None
        }
    }

    impl HasDescription for TestTool {
        fn description(&self) -> Option<&str> {
            Some("A test tool")
        }
    }

    impl HasInputSchema for TestTool {
        fn input_schema(&self) -> &ToolSchema {
            &self.input_schema
        }
    }

    impl HasOutputSchema for TestTool {
        fn output_schema(&self) -> Option<&ToolSchema> {
            None
        }
    }

    impl HasAnnotations for TestTool {
        fn annotations(&self) -> Option<&ToolAnnotations> {
            None
        }
    }

    impl HasToolMeta for TestTool {
        fn tool_meta(&self) -> Option<&HashMap<String, Value>> {
            None
        }
    }

    impl HasIcons for TestTool {}
    impl HasExecution for TestTool {}

    // ToolDefinition is automatically implemented via blanket impl!

    #[async_trait]
    impl McpTool for TestTool {
        async fn call(
            &self,
            args: Value,
            _session: Option<SessionContext>,
        ) -> McpResult<CallToolResult> {
            let message = args
                .get("message")
                .and_then(|v| v.as_str())
                .ok_or_else(|| turul_mcp_protocol::McpError::missing_param("message"))?;

            let result = format!("Test: {}", message);
            Ok(CallToolResult::success(vec![ToolResult::text(result)]))
        }
    }

    #[test]
    fn test_tool_trait() {
        let tool = TestTool::new();
        assert_eq!(tool.name(), "test");
        assert_eq!(tool.description(), Some("A test tool"));
        assert!(tool.annotations().is_none());
    }

    #[test]
    fn test_tool_conversion() {
        let tool = TestTool::new();
        let mcp_tool = tool_to_descriptor(&tool);

        assert_eq!(mcp_tool.name, "test");
        assert_eq!(mcp_tool.description, Some("A test tool".to_string()));
        // ToolSchema doesn't have schema_type field anymore, check structure instead
        assert!(mcp_tool.input_schema.properties.is_some());
    }

    #[tokio::test]
    async fn test_tool_call() {
        let tool = TestTool::new();
        let args = serde_json::json!({"message": "hello"});

        let result = tool.call(args, None).await.unwrap();
        assert!(!result.content.is_empty());

        let ToolResult::Text { text, .. } = &result.content[0] else {
            panic!("Expected text result, got: {:?}", result.content[0]);
        };
        assert_eq!(text, "Test: hello");
    }

    #[tokio::test]
    async fn test_tool_call_error() {
        let tool = TestTool::new();
        let args = serde_json::json!({"wrong": "parameter"});

        let result = tool.call(args, None).await;
        assert!(result.is_err());

        let error = result.unwrap_err();
        let turul_mcp_protocol::McpError::MissingParameter(param) = error else {
            panic!("Expected MissingParameter error, got: {:?}", error);
        };
        assert_eq!(param, "message");
    }

    /// Regression test: verify that HasExecution is wired through to_tool() → Tool → JSON.
    /// A tool with task_support=optional must serialize `execution.taskSupport = "optional"`.
    /// A tool without execution must omit the field entirely.
    #[test]
    fn test_execution_wiring_in_descriptor() {
        use turul_mcp_protocol::tools::{TaskSupport, ToolExecution};

        // --- Tool with execution ---
        struct TaskAwareTool {
            input_schema: ToolSchema,
        }
        impl HasBaseMetadata for TaskAwareTool {
            fn name(&self) -> &str {
                "task_aware"
            }
        }
        impl HasDescription for TaskAwareTool {
            fn description(&self) -> Option<&str> {
                Some("Has execution")
            }
        }
        impl HasInputSchema for TaskAwareTool {
            fn input_schema(&self) -> &ToolSchema {
                &self.input_schema
            }
        }
        impl HasOutputSchema for TaskAwareTool {
            fn output_schema(&self) -> Option<&ToolSchema> {
                None
            }
        }
        impl HasAnnotations for TaskAwareTool {
            fn annotations(&self) -> Option<&ToolAnnotations> {
                None
            }
        }
        impl HasToolMeta for TaskAwareTool {
            fn tool_meta(&self) -> Option<&HashMap<String, serde_json::Value>> {
                None
            }
        }
        impl HasIcons for TaskAwareTool {}
        impl HasExecution for TaskAwareTool {
            fn execution(&self) -> Option<ToolExecution> {
                Some(ToolExecution {
                    task_support: Some(TaskSupport::Optional),
                })
            }
        }
        #[async_trait]
        impl McpTool for TaskAwareTool {
            async fn call(
                &self,
                _args: serde_json::Value,
                _session: Option<SessionContext>,
            ) -> McpResult<CallToolResult> {
                Ok(CallToolResult::success(vec![ToolResult::text("ok")]))
            }
        }

        let tool = TaskAwareTool {
            input_schema: ToolSchema::object(),
        };
        let descriptor = tool_to_descriptor(&tool);

        // Verify the execution field is populated
        assert!(
            descriptor.execution.is_some(),
            "execution should be Some for task-aware tool"
        );
        let exec = descriptor.execution.clone().unwrap();
        assert_eq!(exec.task_support, Some(TaskSupport::Optional));

        // Verify JSON serialization produces the expected field
        let json = serde_json::to_value(&descriptor).unwrap();
        assert_eq!(json["execution"]["taskSupport"], "optional");

        // --- Tool without execution ---
        let plain_tool = TestTool::new();
        let plain_descriptor = tool_to_descriptor(&plain_tool);
        assert!(
            plain_descriptor.execution.is_none(),
            "execution should be None for plain tool"
        );

        let plain_json = serde_json::to_value(&plain_descriptor).unwrap();
        assert!(
            plain_json.get("execution").is_none(),
            "execution key should be absent in JSON"
        );
    }

    // ======================================================================
    // Tool fingerprint tests
    // ======================================================================

    /// Helper: build a tool map from a slice of McpTool trait objects
    fn tool_map(tools: Vec<Arc<dyn McpTool>>) -> HashMap<String, Arc<dyn McpTool>> {
        tools
            .into_iter()
            .map(|t| (t.name().to_string(), t))
            .collect()
    }

    /// A second test tool with different metadata for fingerprint comparison
    struct AnotherTool;
    impl HasBaseMetadata for AnotherTool {
        fn name(&self) -> &str {
            "another"
        }
    }
    impl HasDescription for AnotherTool {
        fn description(&self) -> Option<&str> {
            Some("Another tool")
        }
    }
    impl HasInputSchema for AnotherTool {
        fn input_schema(&self) -> &ToolSchema {
            static SCHEMA: std::sync::OnceLock<ToolSchema> = std::sync::OnceLock::new();
            SCHEMA.get_or_init(|| ToolSchema::object())
        }
    }
    impl HasOutputSchema for AnotherTool {
        fn output_schema(&self) -> Option<&ToolSchema> {
            None
        }
    }
    impl HasAnnotations for AnotherTool {
        fn annotations(&self) -> Option<&ToolAnnotations> {
            None
        }
    }
    impl HasToolMeta for AnotherTool {
        fn tool_meta(&self) -> Option<&HashMap<String, Value>> {
            None
        }
    }
    impl HasIcons for AnotherTool {}
    impl HasExecution for AnotherTool {}
    #[async_trait]
    impl McpTool for AnotherTool {
        async fn call(
            &self,
            _args: Value,
            _session: Option<SessionContext>,
        ) -> McpResult<CallToolResult> {
            Ok(CallToolResult::success(vec![ToolResult::text("ok")]))
        }
    }

    #[test]
    fn test_fingerprint_deterministic() {
        let tools = tool_map(vec![Arc::new(TestTool::new()) as Arc<dyn McpTool>]);
        let fp1 = compute_tool_fingerprint(&tools);
        let fp2 = compute_tool_fingerprint(&tools);
        assert_eq!(fp1, fp2, "Same tools must produce same fingerprint");
        assert_eq!(fp1.len(), 16, "Fingerprint should be 16 hex chars");
    }

    #[test]
    fn test_fingerprint_order_independent() {
        // Insert in different orders — HashMap ordering varies but fingerprint should be stable
        let mut tools_a = HashMap::new();
        tools_a.insert(
            "test".to_string(),
            Arc::new(TestTool::new()) as Arc<dyn McpTool>,
        );
        tools_a.insert(
            "another".to_string(),
            Arc::new(AnotherTool) as Arc<dyn McpTool>,
        );

        let mut tools_b = HashMap::new();
        tools_b.insert(
            "another".to_string(),
            Arc::new(AnotherTool) as Arc<dyn McpTool>,
        );
        tools_b.insert(
            "test".to_string(),
            Arc::new(TestTool::new()) as Arc<dyn McpTool>,
        );

        let fp_a = compute_tool_fingerprint(&tools_a);
        let fp_b = compute_tool_fingerprint(&tools_b);
        assert_eq!(
            fp_a, fp_b,
            "Tool insertion order must not affect fingerprint"
        );
    }

    #[test]
    fn test_fingerprint_changes_with_different_tools() {
        let tools_1 = tool_map(vec![Arc::new(TestTool::new()) as Arc<dyn McpTool>]);
        let tools_2 = tool_map(vec![Arc::new(AnotherTool) as Arc<dyn McpTool>]);
        let fp_1 = compute_tool_fingerprint(&tools_1);
        let fp_2 = compute_tool_fingerprint(&tools_2);
        assert_ne!(
            fp_1, fp_2,
            "Different tools must produce different fingerprints"
        );
    }

    #[test]
    fn test_fingerprint_changes_with_added_tool() {
        let tools_1 = tool_map(vec![Arc::new(TestTool::new()) as Arc<dyn McpTool>]);
        let tools_2 = tool_map(vec![
            Arc::new(TestTool::new()) as Arc<dyn McpTool>,
            Arc::new(AnotherTool) as Arc<dyn McpTool>,
        ]);
        let fp_1 = compute_tool_fingerprint(&tools_1);
        let fp_2 = compute_tool_fingerprint(&tools_2);
        assert_ne!(fp_1, fp_2, "Adding a tool must change fingerprint");
    }

    #[test]
    fn test_fingerprint_empty_tools() {
        let tools = HashMap::new();
        let fp = compute_tool_fingerprint(&tools);
        assert_eq!(
            fp.len(),
            16,
            "Empty tool set should still produce valid fingerprint"
        );
    }

    /// Mandatory canonicalization test: same tool definition built via different paths
    /// must produce the same fingerprint, validating that serde_json serialization
    /// is deterministic for our struct types.
    #[test]
    fn test_fingerprint_canonicalization_stability() {
        // Build TestTool twice via independent code paths
        let tool_a = TestTool::new();
        let tool_b = TestTool::new();

        // Serialize independently and compare
        let desc_a = tool_to_descriptor(&tool_a);
        let desc_b = tool_to_descriptor(&tool_b);
        let json_a = serde_json::to_string(&desc_a).unwrap();
        let json_b = serde_json::to_string(&desc_b).unwrap();
        assert_eq!(
            json_a, json_b,
            "Same tool built independently must serialize identically"
        );

        // Full fingerprint comparison
        let tools_a = tool_map(vec![Arc::new(tool_a) as Arc<dyn McpTool>]);
        let tools_b = tool_map(vec![Arc::new(tool_b) as Arc<dyn McpTool>]);
        assert_eq!(
            compute_tool_fingerprint(&tools_a),
            compute_tool_fingerprint(&tools_b),
            "Independently constructed identical tools must produce the same fingerprint"
        );
    }

    /// Regression: top-level properties in different HashMap insertion order must
    /// produce identical fingerprints (canonicalization sorts object keys).
    #[test]
    fn test_fingerprint_stable_across_property_order() {
        // Build two tools with the same properties inserted in different order
        struct OrderTestTool {
            input_schema: ToolSchema,
        }
        impl HasBaseMetadata for OrderTestTool {
            fn name(&self) -> &str {
                "order_test"
            }
        }
        impl HasDescription for OrderTestTool {
            fn description(&self) -> Option<&str> {
                Some("test")
            }
        }
        impl HasInputSchema for OrderTestTool {
            fn input_schema(&self) -> &ToolSchema {
                &self.input_schema
            }
        }
        impl HasOutputSchema for OrderTestTool {
            fn output_schema(&self) -> Option<&ToolSchema> {
                None
            }
        }
        impl HasAnnotations for OrderTestTool {
            fn annotations(&self) -> Option<&ToolAnnotations> {
                None
            }
        }
        impl HasToolMeta for OrderTestTool {
            fn tool_meta(&self) -> Option<&HashMap<String, Value>> {
                None
            }
        }
        impl HasIcons for OrderTestTool {}
        impl HasExecution for OrderTestTool {}
        #[async_trait]
        impl McpTool for OrderTestTool {
            async fn call(
                &self,
                _args: Value,
                _session: Option<SessionContext>,
            ) -> McpResult<CallToolResult> {
                Ok(CallToolResult::success(vec![ToolResult::text("ok")]))
            }
        }

        // Order A: a, b, c
        let mut props_a = HashMap::new();
        props_a.insert("alpha".to_string(), JsonSchema::string());
        props_a.insert("beta".to_string(), JsonSchema::number());
        props_a.insert("gamma".to_string(), JsonSchema::boolean());
        let tool_a = Arc::new(OrderTestTool {
            input_schema: ToolSchema::object().with_properties(props_a),
        }) as Arc<dyn McpTool>;

        // Order B: c, a, b (different insertion order)
        let mut props_b = HashMap::new();
        props_b.insert("gamma".to_string(), JsonSchema::boolean());
        props_b.insert("alpha".to_string(), JsonSchema::string());
        props_b.insert("beta".to_string(), JsonSchema::number());
        let tool_b = Arc::new(OrderTestTool {
            input_schema: ToolSchema::object().with_properties(props_b),
        }) as Arc<dyn McpTool>;

        let fp_a = compute_tool_fingerprint(&tool_map(vec![tool_a]));
        let fp_b = compute_tool_fingerprint(&tool_map(vec![tool_b]));
        assert_eq!(
            fp_a, fp_b,
            "Property insertion order must not affect fingerprint"
        );
    }

    /// Regression: nested objects and additional fields with different HashMap
    /// insertion orders must produce identical fingerprints.
    #[test]
    fn test_fingerprint_stable_across_nested_property_order() {
        struct NestedTestTool {
            input_schema: ToolSchema,
        }
        impl HasBaseMetadata for NestedTestTool {
            fn name(&self) -> &str {
                "nested_test"
            }
        }
        impl HasDescription for NestedTestTool {
            fn description(&self) -> Option<&str> {
                Some("test")
            }
        }
        impl HasInputSchema for NestedTestTool {
            fn input_schema(&self) -> &ToolSchema {
                &self.input_schema
            }
        }
        impl HasOutputSchema for NestedTestTool {
            fn output_schema(&self) -> Option<&ToolSchema> {
                None
            }
        }
        impl HasAnnotations for NestedTestTool {
            fn annotations(&self) -> Option<&ToolAnnotations> {
                None
            }
        }
        impl HasToolMeta for NestedTestTool {
            fn tool_meta(&self) -> Option<&HashMap<String, Value>> {
                None
            }
        }
        impl HasIcons for NestedTestTool {}
        impl HasExecution for NestedTestTool {}
        #[async_trait]
        impl McpTool for NestedTestTool {
            async fn call(
                &self,
                _args: Value,
                _session: Option<SessionContext>,
            ) -> McpResult<CallToolResult> {
                Ok(CallToolResult::success(vec![ToolResult::text("ok")]))
            }
        }

        // Helper: build a schema with same logical content but varied insertion order
        fn build_schema(
            inner_order: &[(&str, JsonSchema)],
            outer_order: &[(&str, JsonSchema)],
            additional_order: &[(&str, serde_json::Value)],
        ) -> ToolSchema {
            let mut outer_props = HashMap::new();
            for (k, v) in outer_order {
                outer_props.insert(k.to_string(), v.clone());
            }
            let mut schema = ToolSchema::object().with_properties(outer_props);
            let mut additional = HashMap::new();
            for (k, v) in additional_order {
                additional.insert(k.to_string(), v.clone());
            }
            schema.additional = additional;
            let _ = inner_order; // inner order is baked into the nested JsonSchema below
            schema
        }

        // Inner object with sub-properties
        let mut inner_a = HashMap::new();
        inner_a.insert("host".to_string(), JsonSchema::string());
        inner_a.insert("port".to_string(), JsonSchema::number());
        let nested_a = JsonSchema::object_with_properties(inner_a);

        let mut inner_b = HashMap::new();
        inner_b.insert("port".to_string(), JsonSchema::number());
        inner_b.insert("host".to_string(), JsonSchema::string());
        let nested_b = JsonSchema::object_with_properties(inner_b);

        // Order A: outer=[config, enabled], additional=[x, y]
        let tool_a = Arc::new(NestedTestTool {
            input_schema: build_schema(
                &[],
                &[("config", nested_a), ("enabled", JsonSchema::boolean())],
                &[
                    ("x", serde_json::json!("extra_x")),
                    ("y", serde_json::json!("extra_y")),
                ],
            ),
        }) as Arc<dyn McpTool>;

        // Order B: outer=[enabled, config], additional=[y, x] (reversed at all levels)
        let tool_b = Arc::new(NestedTestTool {
            input_schema: build_schema(
                &[],
                &[("enabled", JsonSchema::boolean()), ("config", nested_b)],
                &[
                    ("y", serde_json::json!("extra_y")),
                    ("x", serde_json::json!("extra_x")),
                ],
            ),
        }) as Arc<dyn McpTool>;

        let fp_a = compute_tool_fingerprint(&tool_map(vec![tool_a]));
        let fp_b = compute_tool_fingerprint(&tool_map(vec![tool_b]));
        assert_eq!(
            fp_a, fp_b,
            "Nested property and additional field order must not affect fingerprint"
        );
    }

    /// Test that annotation changes affect the fingerprint
    #[test]
    fn test_fingerprint_changes_with_annotations() {
        use turul_mcp_protocol::tools::ToolAnnotations;

        struct AnnotatedTool {
            annotations: Option<ToolAnnotations>,
        }
        impl HasBaseMetadata for AnnotatedTool {
            fn name(&self) -> &str {
                "annotated"
            }
        }
        impl HasDescription for AnnotatedTool {
            fn description(&self) -> Option<&str> {
                Some("test")
            }
        }
        impl HasInputSchema for AnnotatedTool {
            fn input_schema(&self) -> &ToolSchema {
                static SCHEMA: std::sync::OnceLock<ToolSchema> = std::sync::OnceLock::new();
                SCHEMA.get_or_init(|| ToolSchema::object())
            }
        }
        impl HasOutputSchema for AnnotatedTool {
            fn output_schema(&self) -> Option<&ToolSchema> {
                None
            }
        }
        impl HasAnnotations for AnnotatedTool {
            fn annotations(&self) -> Option<&ToolAnnotations> {
                self.annotations.as_ref()
            }
        }
        impl HasToolMeta for AnnotatedTool {
            fn tool_meta(&self) -> Option<&HashMap<String, Value>> {
                None
            }
        }
        impl HasIcons for AnnotatedTool {}
        impl HasExecution for AnnotatedTool {}
        #[async_trait]
        impl McpTool for AnnotatedTool {
            async fn call(
                &self,
                _args: Value,
                _session: Option<SessionContext>,
            ) -> McpResult<CallToolResult> {
                Ok(CallToolResult::success(vec![ToolResult::text("ok")]))
            }
        }

        let tool_plain = Arc::new(AnnotatedTool { annotations: None }) as Arc<dyn McpTool>;
        let tool_annotated = Arc::new(AnnotatedTool {
            annotations: Some(
                ToolAnnotations::new()
                    .with_read_only_hint(true)
                    .with_destructive_hint(false),
            ),
        }) as Arc<dyn McpTool>;

        let fp_plain = compute_tool_fingerprint(&tool_map(vec![tool_plain]));
        let fp_annotated = compute_tool_fingerprint(&tool_map(vec![tool_annotated]));
        assert_ne!(
            fp_plain, fp_annotated,
            "Annotation changes must affect fingerprint"
        );
    }
}
