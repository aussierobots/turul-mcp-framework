//! MCP Tool Output Compliance Tests
//!
//! These tests validate that tool outputs strictly comply with the MCP 2025-06-18 specification:
//! 1. Tools with outputSchema MUST provide structuredContent
//! 2. structuredContent MUST match outputSchema structure exactly
//! 3. Field names must match schema requirements
//!
//! CRITICAL: These tests validate the MCP specification, not current implementation.
//! If tests fail, fix the code - NEVER change tests to match broken code.

#[cfg(test)]
mod tests {

    use serde_json::{Value, json};
    use turul_mcp_derive::mcp_tool;
    use turul_mcp_protocol::{
        McpResult, ToolResult,
        tools::CallToolResult,
    };
    use turul_mcp_builders::prelude::{HasOutputSchema, ToolDefinition, ToolSchema};
    use turul_mcp_server::{McpTool, SessionContext};

    /// Test tool that claims to have an output schema but doesn't provide structured content
    /// This SHOULD fail compliance tests with current macro implementation
    #[derive(Clone)]
    #[allow(dead_code)]
    struct NonCompliantCountTool;

    #[async_trait::async_trait]
    impl McpTool for NonCompliantCountTool {
        async fn call(
            &self,
            _args: Value,
            _session: Option<SessionContext>,
        ) -> McpResult<CallToolResult> {
            // Current macro generates this - it's NON-COMPLIANT
            // Only provides content, no structuredContent despite having outputSchema
            Ok(CallToolResult::success(vec![ToolResult::text(
                json!({"count": 622}).to_string(),
            )]))
        }
    }

    impl turul_mcp_builders::prelude::HasBaseMetadata for NonCompliantCountTool {
        fn name(&self) -> &str {
            "count_words"
        }
        fn title(&self) -> Option<&str> {
            Some("Word Counter")
        }
    }

    impl turul_mcp_builders::prelude::HasDescription for NonCompliantCountTool {
        fn description(&self) -> Option<&str> {
            Some("Count words in text")
        }
    }

    impl turul_mcp_builders::prelude::HasInputSchema for NonCompliantCountTool {
        fn input_schema(&self) -> &ToolSchema {
            // Simple input schema - not the issue
            static SCHEMA: std::sync::LazyLock<ToolSchema> = std::sync::LazyLock::new(|| {
                ToolSchema::object()
                    .with_properties(std::collections::HashMap::from([(
                        "text".to_string(),
                        turul_mcp_protocol::JsonSchema::string(),
                    )]))
                    .with_required(vec!["text".to_string()])
            });
            &SCHEMA
        }
    }

    impl HasOutputSchema for NonCompliantCountTool {
        fn output_schema(&self) -> Option<&ToolSchema> {
            // This tool CLAIMS to have an output schema
            static SCHEMA: std::sync::LazyLock<ToolSchema> = std::sync::LazyLock::new(|| {
                ToolSchema::object()
                    .with_properties(std::collections::HashMap::from([(
                        "countResult".to_string(),
                        turul_mcp_protocol::JsonSchema::object()
                            .with_properties(std::collections::HashMap::from([(
                                "count".to_string(),
                                turul_mcp_protocol::JsonSchema::number(),
                            )]))
                            .with_required(vec!["count".to_string()]),
                    )]))
                    .with_required(vec!["countResult".to_string()])
            });
            Some(&SCHEMA)
        }
    }

    impl turul_mcp_builders::prelude::HasAnnotations for NonCompliantCountTool {
        fn annotations(&self) -> Option<&turul_mcp_protocol::tools::ToolAnnotations> {
            None
        }
    }

    impl turul_mcp_builders::prelude::HasToolMeta for NonCompliantCountTool {
        fn tool_meta(&self) -> Option<&std::collections::HashMap<String, Value>> {
            None
        }
    }

    impl turul_mcp_builders::prelude::HasIcons for NonCompliantCountTool {}

    /// Test that demonstrates what a COMPLIANT tool should look like
    #[derive(Clone)]
    struct CompliantCountTool;

    #[async_trait::async_trait]
    impl McpTool for CompliantCountTool {
        async fn call(
            &self,
            _args: Value,
            _session: Option<SessionContext>,
        ) -> McpResult<CallToolResult> {
            // COMPLIANT: Uses schema to generate structured content
            let result = json!({"count": 622});

            // Wrap result according to outputSchema structure
            let structured_content = json!({
                "countResult": result
            });

            Ok(
                CallToolResult::success(vec![ToolResult::text(result.to_string())])
                    .with_structured_content(structured_content),
            )
        }
    }

    impl turul_mcp_builders::prelude::HasBaseMetadata for CompliantCountTool {
        fn name(&self) -> &str {
            "compliant_count_words"
        }
        fn title(&self) -> Option<&str> {
            Some("Compliant Word Counter")
        }
    }

    impl turul_mcp_builders::prelude::HasDescription for CompliantCountTool {
        fn description(&self) -> Option<&str> {
            Some("Count words in text (MCP compliant)")
        }
    }

    impl turul_mcp_builders::prelude::HasInputSchema for CompliantCountTool {
        fn input_schema(&self) -> &ToolSchema {
            static SCHEMA: std::sync::LazyLock<ToolSchema> = std::sync::LazyLock::new(|| {
                ToolSchema::object()
                    .with_properties(std::collections::HashMap::from([(
                        "text".to_string(),
                        turul_mcp_protocol::JsonSchema::string(),
                    )]))
                    .with_required(vec!["text".to_string()])
            });
            &SCHEMA
        }
    }

    impl HasOutputSchema for CompliantCountTool {
        fn output_schema(&self) -> Option<&ToolSchema> {
            static SCHEMA: std::sync::LazyLock<ToolSchema> = std::sync::LazyLock::new(|| {
                ToolSchema::object()
                    .with_properties(std::collections::HashMap::from([(
                        "countResult".to_string(),
                        turul_mcp_protocol::JsonSchema::object()
                            .with_properties(std::collections::HashMap::from([(
                                "count".to_string(),
                                turul_mcp_protocol::JsonSchema::number(),
                            )]))
                            .with_required(vec!["count".to_string()]),
                    )]))
                    .with_required(vec!["countResult".to_string()])
            });
            Some(&SCHEMA)
        }
    }

    impl turul_mcp_builders::prelude::HasAnnotations for CompliantCountTool {
        fn annotations(&self) -> Option<&turul_mcp_protocol::tools::ToolAnnotations> {
            None
        }
    }

    impl turul_mcp_builders::prelude::HasToolMeta for CompliantCountTool {
        fn tool_meta(&self) -> Option<&std::collections::HashMap<String, Value>> {
            None
        }
    }

    impl turul_mcp_builders::prelude::HasIcons for CompliantCountTool {}

    #[tokio::test]
    async fn test_tool_with_output_schema_must_have_structured_content() {
        let tool = CompliantCountTool;
        let result = tool
            .call(json!({"text": "hello world"}), None)
            .await
            .unwrap();

        // CRITICAL MCP COMPLIANCE RULE:
        // If a tool defines outputSchema, it MUST provide structuredContent
        if tool.output_schema().is_some() {
            assert!(
                result.structured_content.is_some(),
                "Tool with outputSchema MUST provide structuredContent (MCP 2025-06-18 spec violation)"
            );

            let structured = result.structured_content.unwrap();

            // Validate structure matches outputSchema exactly
            assert!(
                structured.get("countResult").is_some(),
                "structuredContent must match outputSchema field names exactly"
            );

            let count_result = structured["countResult"].as_object().unwrap();
            assert!(
                count_result.get("count").is_some(),
                "structuredContent nested fields must match outputSchema"
            );

            assert!(
                count_result["count"].is_number(),
                "structuredContent field types must match outputSchema"
            );
        }
    }

    #[tokio::test]
    async fn test_compliant_tool_passes_validation() {
        let tool = CompliantCountTool;
        let result = tool
            .call(json!({"text": "hello world"}), None)
            .await
            .unwrap();

        // This tool should pass all compliance checks
        if tool.output_schema().is_some() {
            assert!(
                result.structured_content.is_some(),
                "Compliant tool must provide structuredContent"
            );

            let structured = result.structured_content.unwrap();

            // Validate structure matches outputSchema exactly
            assert!(
                structured.get("countResult").is_some(),
                "Compliant tool structuredContent matches schema"
            );

            let count_result = structured["countResult"].as_object().unwrap();
            assert!(
                count_result.get("count").is_some(),
                "Compliant tool nested fields match schema"
            );
        }
    }

    #[tokio::test]
    async fn test_tool_without_output_schema_doesnt_need_structured_content() {
        // Tool with no outputSchema - structuredContent is optional
        #[derive(Clone)]
        struct SimpleTextTool;

        #[async_trait::async_trait]
        impl McpTool for SimpleTextTool {
            async fn call(
                &self,
                _args: Value,
                _session: Option<SessionContext>,
            ) -> McpResult<CallToolResult> {
                Ok(CallToolResult::success(vec![ToolResult::text("Hello")]))
            }
        }

        impl turul_mcp_builders::prelude::HasBaseMetadata for SimpleTextTool {
            fn name(&self) -> &str {
                "simple_text"
            }
            fn title(&self) -> Option<&str> {
                None
            }
        }

        impl turul_mcp_builders::prelude::HasDescription for SimpleTextTool {
            fn description(&self) -> Option<&str> {
                Some("Simple text tool")
            }
        }

        impl turul_mcp_builders::prelude::HasInputSchema for SimpleTextTool {
            fn input_schema(&self) -> &ToolSchema {
                static SCHEMA: std::sync::LazyLock<ToolSchema> =
                    std::sync::LazyLock::new(ToolSchema::object);
                &SCHEMA
            }
        }

        impl HasOutputSchema for SimpleTextTool {
            fn output_schema(&self) -> Option<&ToolSchema> {
                None // No output schema
            }
        }

        impl turul_mcp_builders::prelude::HasAnnotations for SimpleTextTool {
            fn annotations(&self) -> Option<&turul_mcp_protocol::tools::ToolAnnotations> {
                None
            }
        }

        impl turul_mcp_builders::prelude::HasToolMeta for SimpleTextTool {
            fn tool_meta(&self) -> Option<&std::collections::HashMap<String, Value>> {
                None
            }
        }

        impl turul_mcp_builders::prelude::HasIcons for SimpleTextTool {}

        let tool = SimpleTextTool;
        let result = tool.call(json!({}), None).await.unwrap();

        // No outputSchema = structuredContent is optional
        // This should pass regardless of whether structuredContent is present
        assert!(!result.content.is_empty(), "Tool must provide content");
        // structured_content can be None or Some - both are valid
    }

    /// Test what happens when macro generates tools
    /// This will fail until we fix the mcp_tool macro
    #[tokio::test]
    async fn test_mcp_tool_macro_compliance() {
        // This will be a simple mcp_tool macro usage
        // Now it should work correctly with structured content

        #[mcp_tool(name = "add", description = "Add two numbers")]
        async fn add_numbers(a: f64, b: f64) -> McpResult<f64> {
            Ok(a + b)
        }

        let tool = add_numbers();
        let result = tool.call(json!({"a": 5.0, "b": 3.0}), None).await.unwrap();
        assert!(
            !result.content.is_empty(),
            "Macro-generated tool must provide content"
        );

        // Check that function tools now have outputSchema and structuredContent
        if let Some(schema) = tool.output_schema() {
            // Tool has schema, so must have structured content
            assert!(
                result.structured_content.is_some(),
                "Tool with outputSchema MUST provide structuredContent"
            );

            if let Some(structured) = &result.structured_content {
                // Verify structured content matches schema structure
                if let Some(properties) = &schema.properties {
                    for schema_field in properties.keys() {
                        assert!(
                            structured.as_object().unwrap().contains_key(schema_field),
                            "Schema field '{}' not found in structured content",
                            schema_field
                        );
                    }
                }
            }
        }
    }

    #[tokio::test]
    async fn test_mcp_tool_custom_output_field_compliance() {
        // Test mcp_tool with custom output field
        #[mcp_tool(
            name = "count_words_custom",
            description = "Count words with custom output field",
            output_field = "wordCount"
        )]
        async fn count_words_custom(text: String) -> McpResult<u32> {
            Ok(text.split_whitespace().count() as u32)
        }

        let tool = count_words_custom();
        let result = tool
            .call(json!({"text": "hello world test"}), None)
            .await
            .unwrap();

        // Check schema uses custom field name
        if let Some(schema) = tool.output_schema()
            && let Some(properties) = &schema.properties
        {
            assert!(
                properties.contains_key("wordCount"),
                "Schema should contain 'wordCount' field, got: {:?}",
                properties.keys().collect::<Vec<_>>()
            );
            assert!(
                !properties.contains_key("result"),
                "Schema should not contain 'result' field when custom field specified"
            );
        }

        // Check runtime output uses custom field name
        assert!(result.structured_content.is_some());
        if let Some(structured) = &result.structured_content {
            assert!(
                structured.as_object().unwrap().contains_key("wordCount"),
                "Runtime output must use 'wordCount' field, got keys: {:?}",
                structured.as_object().unwrap().keys().collect::<Vec<_>>()
            );
            assert!(
                !structured.as_object().unwrap().contains_key("result"),
                "Runtime should not have 'result' field when custom field specified"
            );

            assert_eq!(structured["wordCount"].as_u64().unwrap(), 3);
        }
    }

    #[tokio::test]
    async fn test_derive_macro_compliance() {
        use turul_mcp_derive::McpTool;

        // Test derive macro with custom output field
        use serde::{Deserialize, Serialize};
        use schemars::JsonSchema;

        #[derive(Clone, Serialize, Deserialize, JsonSchema)]
        struct CountResult {
            word_count: u32,
            message: String,
        }

        #[derive(McpTool, Clone)]
        #[tool(
        name = "derive_count_test",
        description = "Test derive macro with custom output field",
        output = CountResult,
        output_field = "countResult"
    )]
        struct DeriveCountTool {
            #[param(description = "Text to analyze")]
            text: String,
        }

        impl DeriveCountTool {
            async fn execute(&self, _session: Option<SessionContext>) -> McpResult<CountResult> {
                let count = self.text.split_whitespace().count() as u32;
                Ok(CountResult {
                    word_count: count,
                    message: format!("Found {} words", count),
                })
            }
        }

        let tool = DeriveCountTool {
            text: "hello world test derive".to_string(),
        };

        // Check schema uses custom field name
        if let Some(schema) = tool.output_schema()
            && let Some(properties) = &schema.properties
        {
            assert!(
                properties.contains_key("countResult"),
                "Derive macro schema should contain 'countResult' field, got: {:?}",
                properties.keys().collect::<Vec<_>>()
            );
        }

        // Check runtime output uses custom field name
        let result = tool
            .call(json!({"text": "hello world test derive"}), None)
            .await
            .unwrap();
        assert!(result.structured_content.is_some());
        if let Some(structured) = &result.structured_content {
            assert!(
                structured.as_object().unwrap().contains_key("countResult"),
                "Derive macro runtime must use 'countResult' field, got keys: {:?}",
                structured.as_object().unwrap().keys().collect::<Vec<_>>()
            );

            let count_result = structured["countResult"].as_object().unwrap();
            assert_eq!(count_result["word_count"].as_u64().unwrap(), 4);
            assert!(
                count_result["message"]
                    .as_str()
                    .unwrap()
                    .contains("4 words")
            );
        }
    }

    #[tokio::test]
    async fn test_tools_list_schema_matches_runtime_output() {
        // This test validates that tools/list schema matches tools/call output structure

        #[mcp_tool(
            name = "schema_validation_test",
            description = "Test schema/runtime consistency",
            output_field = "validationResult"
        )]
        async fn schema_validation_test(input: String) -> McpResult<String> {
            Ok(format!("Validated: {}", input))
        }

        let tool = schema_validation_test();

        // Get what tools/list would return (schema)
        let tool_definition = tool.to_tool();
        let output_schema = tool_definition.output_schema.as_ref().unwrap();

        // Get what tools/call returns (runtime output)
        let call_result = tool.call(json!({"input": "test"}), None).await.unwrap();
        let structured_content = call_result.structured_content.as_ref().unwrap();

        // Validate they match exactly
        if let Some(schema_properties) = &output_schema.properties {
            let content_obj = structured_content.as_object().unwrap();

            // Every field in schema should exist in runtime output
            for schema_field in schema_properties.keys() {
                assert!(
                    content_obj.contains_key(schema_field),
                    "Schema field '{}' missing from runtime output. Schema: {:?}, Runtime: {:?}",
                    schema_field,
                    schema_properties.keys().collect::<Vec<_>>(),
                    content_obj.keys().collect::<Vec<_>>()
                );
            }

            // Every field in runtime output should be in schema
            for content_field in content_obj.keys() {
                assert!(
                    schema_properties.contains_key(content_field),
                    "Runtime field '{}' missing from schema. Runtime: {:?}, Schema: {:?}",
                    content_field,
                    content_obj.keys().collect::<Vec<_>>(),
                    schema_properties.keys().collect::<Vec<_>>()
                );
            }
        }
    }

    #[test]
    fn test_mcp_tool_compliance_documentation() {
        // This test exists to document the MCP compliance requirements
        // It should always pass but serves as living documentation

        println!("MCP Tool Output Compliance Requirements (2025-06-18):");
        println!("1. If tool defines outputSchema, structuredContent MUST be provided");
        println!("2. structuredContent MUST match outputSchema structure exactly");
        println!("3. Field names in structuredContent MUST match outputSchema");
        println!("4. Field types in structuredContent MUST match outputSchema");
        println!("5. Required fields in outputSchema MUST be present in structuredContent");
        println!();
        println!("CRITICAL: Never change tests to match code. Fix code to match MCP spec.");
    }
} // mod tests
