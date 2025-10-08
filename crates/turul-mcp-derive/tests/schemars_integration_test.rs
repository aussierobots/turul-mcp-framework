//! Test schemars integration with automatic schema generation
//!
//! This test verifies that:
//! 1. Schemars JsonSchema derive works with MCP tools
//! 2. Generated schemas have DETAILED properties (not just generic "Object")
//! 3. Schema includes field names, types, and required fields

mod schemars_tests {
    use schemars::JsonSchema;
    use serde::{Deserialize, Serialize};
    use turul_mcp_derive::McpTool;
    use turul_mcp_protocol::McpResult;
    use turul_mcp_server::SessionContext;
    use turul_mcp_builders::traits::*;

    /// Output type with JsonSchema derive
    #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
    pub struct CalculationOutput {
        /// The calculated result
        pub result: f64,
        /// The operation performed
        pub operation: String,
        /// Timestamp of calculation
        pub timestamp: String,
    }

    /// Test tool using schemars for output schema
    #[derive(McpTool, Default)]
    #[tool(
        name = "test_calculator",
        description = "Test calculator with schemars output schema",
        output = CalculationOutput
    )]
    pub struct TestCalculatorTool {
        #[param(description = "First number")]
        pub a: f64,
        #[param(description = "Second number")]
        pub b: f64,
    }

    impl TestCalculatorTool {
        async fn execute(&self, _session: Option<SessionContext>) -> McpResult<CalculationOutput> {
            Ok(CalculationOutput {
                result: self.a + self.b,
                operation: "add".to_string(),
                timestamp: "2024-01-01T00:00:00Z".to_string(),
            })
        }
    }

    #[tokio::test]
    async fn test_schemars_tool_compiles() {
        // Just verify it compiles
        let tool = TestCalculatorTool { a: 5.0, b: 3.0 };
        let result = tool.execute(None).await.unwrap();
        assert_eq!(result.result, 8.0);
    }

    #[tokio::test]
    async fn test_schemars_generates_detailed_schema() {
        let tool = TestCalculatorTool::default();

        // Get the output schema
        let schema = tool.output_schema();
        assert!(schema.is_some(), "Tool must have output schema");

        let schema = schema.unwrap();

        // CRITICAL: Verify schema has DETAILED properties, not just "Object"
        assert!(
            schema.properties.is_some(),
            "Schema must have properties defined (not just generic Object)"
        );

        let properties = schema.properties.as_ref().unwrap();
        assert!(
            !properties.is_empty(),
            "Schema must list actual field names and types"
        );

        // Verify schema has required fields list
        assert!(
            schema.required.is_some(),
            "Schema must specify which fields are required"
        );

        // The schema should wrap the output in a field (default "result")
        // So we expect ONE property that contains the actual CalculationOutput schema
        assert_eq!(
            properties.len(),
            1,
            "Output schema should wrap result in a single field"
        );

        // Get the wrapped schema
        let wrapped_schema = properties.values().next().unwrap();

        // Verify the wrapped schema is an Object with the actual fields
        match wrapped_schema {
            turul_mcp_protocol::schema::JsonSchema::Object { properties: inner_props, required: inner_req, .. } => {
                assert!(inner_props.is_some(), "Inner schema must have properties");
                let inner_props = inner_props.as_ref().unwrap();

                // Verify actual fields from CalculationOutput are present
                assert!(
                    inner_props.contains_key("result"),
                    "Schema must include 'result' field from CalculationOutput"
                );
                assert!(
                    inner_props.contains_key("operation"),
                    "Schema must include 'operation' field from CalculationOutput"
                );
                assert!(
                    inner_props.contains_key("timestamp"),
                    "Schema must include 'timestamp' field from CalculationOutput"
                );

                // Verify field types are detailed, not just Object
                if let turul_mcp_protocol::schema::JsonSchema::Number { .. } = &inner_props["result"] {
                    // Correct - result is a number
                } else {
                    panic!("'result' field should be Number type, got: {:?}", inner_props["result"]);
                }

                if let turul_mcp_protocol::schema::JsonSchema::String { .. } = &inner_props["operation"] {
                    // Correct - operation is a string
                } else {
                    panic!("'operation' field should be String type, got: {:?}", inner_props["operation"]);
                }

                // Verify required fields
                assert!(inner_req.is_some(), "Inner schema must specify required fields");
                let required = inner_req.as_ref().unwrap();
                assert!(required.contains(&"result".to_string()), "result should be required");
                assert!(required.contains(&"operation".to_string()), "operation should be required");
            }
            other => panic!("Expected Object schema, got: {:?}", other),
        }

        println!("✓ Schema has detailed field definitions!");
        println!("✓ Schema includes field names: result, operation, timestamp");
        println!("✓ Schema includes field types: Number, String, String");
        println!("✓ Schema specifies required fields");
        println!("✓ NOT just a generic Object type!");
    }

    // ===== NESTED STRUCTURE TEST =====

    /// Nested struct for testing - like AccuracyStatistics
    #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
    pub struct NestedStats {
        pub min_value: f32,
        pub max_value: f32,
        pub avg_value: f32,
    }

    /// Array item struct for testing - like AccuracyRecord
    #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
    pub struct DataPoint {
        pub timestamp: String,
        pub value: f32,
    }

    /// Complex nested output - like AccuracyHistoryOutput
    #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
    pub struct NestedOutput {
        pub device_id: String,
        pub stats: NestedStats,             // Nested object
        pub data_points: Vec<DataPoint>,    // Array of objects
    }

    #[derive(McpTool, Default)]
    #[tool(
        name = "test_nested",
        description = "Test nested schema generation",
        output = NestedOutput
    )]
    pub struct NestedTool {
        #[param(description = "Device ID")]
        pub id: String,
    }

    impl NestedTool {
        async fn execute(&self, _session: Option<SessionContext>) -> McpResult<NestedOutput> {
            Ok(NestedOutput {
                device_id: self.id.clone(),
                stats: NestedStats {
                    min_value: 1.0,
                    max_value: 10.0,
                    avg_value: 5.5,
                },
                data_points: vec![
                    DataPoint { timestamp: "2024-01-01".to_string(), value: 1.0 },
                    DataPoint { timestamp: "2024-01-02".to_string(), value: 2.0 },
                ],
            })
        }
    }

    #[tokio::test]
    async fn test_nested_structure_schema() {
        let tool = NestedTool::default();

        // Get the output schema
        let schema = tool.output_schema();
        assert!(schema.is_some(), "Tool must have output schema");

        let schema = schema.unwrap();
        let properties = schema.properties.as_ref().unwrap();

        // Get the wrapped output schema
        let output_schema = properties.values().next().unwrap();

        match output_schema {
            turul_mcp_protocol::schema::JsonSchema::Object { properties: Some(props), .. } => {
                println!("✓ Top-level schema has {} properties", props.len());

                // Verify top-level fields
                assert!(props.contains_key("device_id"), "Missing device_id");
                assert!(props.contains_key("stats"), "Missing stats");
                assert!(props.contains_key("data_points"), "Missing data_points");

                // ===== CRITICAL: Verify nested object (stats) has detailed schema =====
                match &props["stats"] {
                    turul_mcp_protocol::schema::JsonSchema::Object { properties: Some(stats_props), .. } => {
                        println!("✓ Nested 'stats' object has {} properties", stats_props.len());
                        assert!(stats_props.contains_key("min_value"), "stats missing min_value");
                        assert!(stats_props.contains_key("max_value"), "stats missing max_value");
                        assert!(stats_props.contains_key("avg_value"), "stats missing avg_value");
                        println!("✓ Nested object has detailed fields!");
                    },
                    other => panic!("'stats' should be detailed Object with properties, got: {:?}", other),
                }

                // ===== CRITICAL: Verify array items have detailed schema =====
                match &props["data_points"] {
                    turul_mcp_protocol::schema::JsonSchema::Array { items: Some(items), .. } => {
                        println!("✓ Array 'data_points' has item schema");
                        match items.as_ref() {
                            turul_mcp_protocol::schema::JsonSchema::Object { properties: Some(item_props), .. } => {
                                println!("✓ Array items have {} properties", item_props.len());
                                assert!(item_props.contains_key("timestamp"), "items missing timestamp");
                                assert!(item_props.contains_key("value"), "items missing value");
                                println!("✓ Array items have detailed schema!");
                            },
                            other => panic!("Array items should be detailed objects, got: {:?}", other),
                        }
                    },
                    other => panic!("'data_points' should be Array with items, got: {:?}", other),
                }

                println!("✅ NESTED SCHEMA TEST PASSED!");
                println!("   - Top-level fields: device_id, stats, data_points");
                println!("   - Nested object (stats): 3 detailed fields");
                println!("   - Array items (data_points): 2 detailed fields each");
            },
            other => panic!("Expected detailed Object schema, got: {:?}", other),
        }
    }

    // ===== OPTIONAL FIELDS TEST =====

    /// Output with optional fields - tests anyOf handling
    #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
    pub struct OutputWithOptionals {
        pub required_field: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub optional_string: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub optional_number: Option<f32>,
    }

    #[derive(McpTool, Default)]
    #[tool(
        name = "test_optional_fields",
        description = "Test optional field schema generation",
        output = OutputWithOptionals
    )]
    pub struct OptionalFieldsTool {
        #[param(description = "Test ID")]
        pub id: String,
    }

    impl OptionalFieldsTool {
        async fn execute(&self, _session: Option<SessionContext>) -> McpResult<OutputWithOptionals> {
            Ok(OutputWithOptionals {
                required_field: self.id.clone(),
                optional_string: Some("test".to_string()),
                optional_number: Some(42.5),
            })
        }
    }

    #[tokio::test]
    async fn test_optional_fields_schema() {
        let tool = OptionalFieldsTool::default();

        // Get the output schema
        let schema = tool.output_schema();
        assert!(schema.is_some(), "Tool must have output schema");

        let schema = schema.unwrap();
        let properties = schema.properties.as_ref().unwrap();

        // Get the wrapped output schema
        let output_schema = properties.values().next().unwrap();

        match output_schema {
            turul_mcp_protocol::schema::JsonSchema::Object { properties: Some(props), required, .. } => {
                println!("✓ Top-level schema has {} properties", props.len());

                // Verify all fields are present
                assert!(props.contains_key("required_field"), "Missing required_field");
                assert!(props.contains_key("optional_string"), "Missing optional_string");
                assert!(props.contains_key("optional_number"), "Missing optional_number");

                // Verify optional fields have proper types (not generic objects)
                match &props["optional_string"] {
                    turul_mcp_protocol::schema::JsonSchema::String { .. } => {
                        println!("✓ optional_string is String type (anyOf resolved correctly)");
                    },
                    other => panic!("optional_string should be String, got: {:?}", other),
                }

                match &props["optional_number"] {
                    turul_mcp_protocol::schema::JsonSchema::Number { .. } => {
                        println!("✓ optional_number is Number type (anyOf resolved correctly)");
                    },
                    other => panic!("optional_number should be Number, got: {:?}", other),
                }

                // Verify required fields - only required_field should be required
                assert!(required.is_some(), "Schema must specify required fields");
                let required = required.as_ref().unwrap();
                assert!(required.contains(&"required_field".to_string()), "required_field should be required");
                assert!(!required.contains(&"optional_string".to_string()), "optional_string should NOT be required");
                assert!(!required.contains(&"optional_number".to_string()), "optional_number should NOT be required");

                println!("✅ OPTIONAL FIELDS TEST PASSED!");
                println!("   - Optional fields have proper types (not generic objects)");
                println!("   - anyOf nullable patterns resolved correctly");
            },
            other => panic!("Expected detailed Object schema, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_optional_fields_serialization() {
        // This test verifies that None values don't serialize as null
        // which would fail MCP validation if schema says type is "string"

        let output_with_values = OutputWithOptionals {
            required_field: "test".to_string(),
            optional_string: Some("value".to_string()),
            optional_number: Some(42.5),
        };

        let output_with_nulls = OutputWithOptionals {
            required_field: "test".to_string(),
            optional_string: None,
            optional_number: None,
        };

        let json_with_values = serde_json::to_value(&output_with_values).unwrap();
        let json_with_nulls = serde_json::to_value(&output_with_nulls).unwrap();

        println!("\n=== Output with Some values ===");
        println!("{}", serde_json::to_string_pretty(&json_with_values).unwrap());

        println!("\n=== Output with None values ===");
        println!("{}", serde_json::to_string_pretty(&json_with_nulls).unwrap());

        // Verify Some values are present
        assert!(json_with_values.get("optional_string").is_some());
        assert!(json_with_values.get("optional_number").is_some());

        // CRITICAL: Verify None values are OMITTED, not serialized as null
        assert!(json_with_nulls.get("optional_string").is_none(),
            "optional_string should be omitted when None, not serialized as null");
        assert!(json_with_nulls.get("optional_number").is_none(),
            "optional_number should be omitted when None, not serialized as null");

        println!("✅ OPTIONAL SERIALIZATION TEST PASSED!");
        println!("   - None values are omitted (not serialized as null)");
        println!("   - Some values are properly serialized");
        println!("   - Output will pass MCP validation even with string-typed optional fields");
    }
}
