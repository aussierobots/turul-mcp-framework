//! Tool Builder for Runtime Tool Construction
//!
//! This module provides a builder pattern for creating tools at runtime
//! without requiring procedural macros. This is Level 3 of the tool creation spectrum.

use serde_json::Value;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

// Import traits from local traits module
use crate::traits::{
    HasAnnotations, HasBaseMetadata, HasDescription, HasInputSchema, HasOutputSchema, HasToolMeta,
};
// Import protocol types
use turul_mcp_protocol::schema::JsonSchema;
use turul_mcp_protocol::tools::{ToolAnnotations, ToolSchema};

/// Type alias for dynamic tool execution function
pub type DynamicToolFn =
    Box<dyn Fn(Value) -> Pin<Box<dyn Future<Output = Result<Value, String>> + Send>> + Send + Sync>;

/// Builder for creating tools at runtime
pub struct ToolBuilder {
    name: String,
    title: Option<String>,
    description: Option<String>,
    input_schema: ToolSchema,
    output_schema: Option<ToolSchema>,
    annotations: Option<ToolAnnotations>,
    meta: Option<HashMap<String, Value>>,
    execute_fn: Option<DynamicToolFn>,
}

impl ToolBuilder {
    /// Create a new tool builder with the given name
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            title: None,
            description: None,
            input_schema: ToolSchema::object(),
            output_schema: None,
            annotations: None,
            meta: None,
            execute_fn: None,
        }
    }

    /// Set the tool title (display name)
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the tool description
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Add a parameter to the input schema
    pub fn param<T: Into<String>>(mut self, name: T, schema: JsonSchema) -> Self {
        let param_name = name.into();
        if let Some(properties) = &mut self.input_schema.properties {
            // Store JsonSchema directly
            properties.insert(param_name, schema);
        } else {
            // Store JsonSchema directly
            self.input_schema.properties = Some(HashMap::from([(param_name, schema)]));
        }
        self
    }

    /// Add a required parameter
    pub fn required_param<T: Into<String>>(mut self, name: T, schema: JsonSchema) -> Self {
        let param_name = name.into();
        self = self.param(&param_name, schema);
        if let Some(required) = &mut self.input_schema.required {
            required.push(param_name);
        } else {
            self.input_schema.required = Some(vec![param_name]);
        }
        self
    }

    /// Add a string parameter
    pub fn string_param(self, name: impl Into<String>, description: impl Into<String>) -> Self {
        self.required_param(name, JsonSchema::string().with_description(description))
    }

    /// Add a number parameter
    pub fn number_param(self, name: impl Into<String>, description: impl Into<String>) -> Self {
        self.required_param(name, JsonSchema::number().with_description(description))
    }

    /// Add an integer parameter
    pub fn integer_param(self, name: impl Into<String>, description: impl Into<String>) -> Self {
        self.required_param(name, JsonSchema::integer().with_description(description))
    }

    /// Add a boolean parameter
    pub fn boolean_param(self, name: impl Into<String>, description: impl Into<String>) -> Self {
        self.required_param(name, JsonSchema::boolean().with_description(description))
    }

    /// Set the output schema
    pub fn output_schema(mut self, schema: ToolSchema) -> Self {
        self.output_schema = Some(schema);
        self
    }

    /// Set the output schema to expect a number result
    pub fn number_output(mut self) -> Self {
        self.output_schema = Some(
            ToolSchema::object()
                .with_properties(HashMap::from([(
                    "result".to_string(),
                    JsonSchema::number(),
                )]))
                .with_required(vec!["result".to_string()]),
        );
        self
    }

    /// Set the output schema to expect a string result
    pub fn string_output(mut self) -> Self {
        self.output_schema = Some(
            ToolSchema::object()
                .with_properties(HashMap::from([(
                    "result".to_string(),
                    JsonSchema::string(),
                )]))
                .with_required(vec!["result".to_string()]),
        );
        self
    }

    /// Set annotations
    pub fn annotations(mut self, annotations: ToolAnnotations) -> Self {
        self.annotations = Some(annotations);
        self
    }

    /// Set meta information
    pub fn meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.meta = Some(meta);
        self
    }

    /// Set the execution function
    pub fn execute<F, Fut>(mut self, f: F) -> Self
    where
        F: Fn(Value) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Value, String>> + Send + 'static,
    {
        self.execute_fn = Some(Box::new(move |args| Box::pin(f(args))));
        self
    }

    /// Build the dynamic tool
    pub fn build(self) -> Result<DynamicTool, String> {
        let execute_fn = self.execute_fn.ok_or("Execution function is required")?;

        Ok(DynamicTool {
            name: self.name,
            title: self.title,
            description: self.description,
            input_schema: self.input_schema,
            output_schema: self.output_schema,
            annotations: self.annotations,
            meta: self.meta,
            execute_fn,
        })
    }
}

/// Dynamic tool created by ToolBuilder
pub struct DynamicTool {
    name: String,
    title: Option<String>,
    description: Option<String>,
    input_schema: ToolSchema,
    output_schema: Option<ToolSchema>,
    annotations: Option<ToolAnnotations>,
    meta: Option<HashMap<String, Value>>,
    execute_fn: DynamicToolFn,
}

impl DynamicTool {
    /// Execute the tool with the given arguments
    pub async fn execute(&self, args: Value) -> Result<Value, String> {
        (self.execute_fn)(args).await
    }
}

// Implement all fine-grained traits for DynamicTool
/// Implements HasBaseMetadata for DynamicTool providing name and title access
impl HasBaseMetadata for DynamicTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }
}

/// Implements HasDescription for DynamicTool providing description text access
impl HasDescription for DynamicTool {
    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}

/// Implements HasInputSchema for DynamicTool providing parameter schema access
impl HasInputSchema for DynamicTool {
    fn input_schema(&self) -> &ToolSchema {
        &self.input_schema
    }
}

/// Implements HasOutputSchema for DynamicTool providing result schema access
impl HasOutputSchema for DynamicTool {
    fn output_schema(&self) -> Option<&ToolSchema> {
        self.output_schema.as_ref()
    }
}

/// Implements HasAnnotations for DynamicTool providing metadata annotations
impl HasAnnotations for DynamicTool {
    fn annotations(&self) -> Option<&ToolAnnotations> {
        self.annotations.as_ref()
    }
}

/// Implements HasToolMeta for DynamicTool providing additional metadata fields
impl HasToolMeta for DynamicTool {
    fn tool_meta(&self) -> Option<&HashMap<String, Value>> {
        self.meta.as_ref()
    }
}

// ToolDefinition is automatically implemented via blanket impl!

// Note: McpTool implementation will be provided by the turul-mcp-server crate
// since it depends on types from that crate (SessionContext, etc.)

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_tool_builder_basic() {
        let tool = ToolBuilder::new("test_tool")
            .description("A test tool")
            .string_param("input", "Test input parameter")
            .execute(|args| async move {
                let input = args
                    .get("input")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing input parameter")?;
                Ok(json!({"result": format!("Hello, {}", input)}))
            })
            .build()
            .expect("Failed to build tool");

        assert_eq!(tool.name(), "test_tool");
        assert_eq!(tool.description(), Some("A test tool"));

        let result = tool
            .execute(json!({"input": "World"}))
            .await
            .expect("Tool execution failed");

        assert_eq!(result, json!({"result": "Hello, World"}));
    }

    #[test]
    fn test_tool_builder_schema_generation() {
        let tool = ToolBuilder::new("calculator")
            .description("Simple calculator")
            .number_param("a", "First number")
            .number_param("b", "Second number")
            .number_output()
            .execute(|args| async move {
                let a = args.get("a").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let b = args.get("b").and_then(|v| v.as_f64()).unwrap_or(0.0);
                Ok(json!({"result": a + b}))
            })
            .build()
            .expect("Failed to build calculator tool");

        // Verify schema was generated correctly
        let input_schema = tool.input_schema();
        assert!(input_schema.properties.is_some());
        assert_eq!(input_schema.required.as_ref().unwrap().len(), 2);

        let output_schema = tool.output_schema();
        assert!(output_schema.is_some());
    }
}
