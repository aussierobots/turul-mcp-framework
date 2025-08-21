//! MCP Tools Protocol Types
//!
//! This module defines the types used for the MCP tools functionality.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::schema::JsonSchema;

/// A cursor for pagination
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cursor(pub String);

impl Cursor {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// JSON Schema definition for tool input/output
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolSchema {
    /// The schema type (must be "object" for tools)
    #[serde(rename = "type")]
    pub schema_type: String,
    /// Property definitions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<HashMap<String, JsonSchema>>,
    /// Required property names
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
    /// Additional properties
    #[serde(flatten)]
    pub additional: HashMap<String, Value>,
}

impl ToolSchema {
    pub fn object() -> Self {
        Self {
            schema_type: "object".to_string(),
            properties: None,
            required: None,
            additional: HashMap::new(),
        }
    }

    pub fn with_properties(mut self, properties: HashMap<String, JsonSchema>) -> Self {
        self.properties = Some(properties);
        self
    }

    pub fn with_required(mut self, required: Vec<String>) -> Self {
        self.required = Some(required);
        self
    }
}

/// Tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tool {
    /// The tool's name - used as identifier when calling
    pub name: String,
    /// Optional human-readable description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// JSON Schema for input parameters
    pub input_schema: ToolSchema,
    /// Optional annotations for client hints
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Value>,
}

impl Tool {
    pub fn new(name: impl Into<String>, input_schema: ToolSchema) -> Self {
        Self {
            name: name.into(),
            description: None,
            input_schema,
            annotations: None,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_annotations(mut self, annotations: Value) -> Self {
        self.annotations = Some(annotations);
        self
    }
}

/// Parameters for tools/list request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListToolsRequest {
    /// Optional cursor for pagination
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<Cursor>,
}

impl ListToolsRequest {
    pub fn new() -> Self {
        Self { cursor: None }
    }

    pub fn with_cursor(mut self, cursor: Cursor) -> Self {
        self.cursor = Some(cursor);
        self
    }
}

impl Default for ListToolsRequest {
    fn default() -> Self {
        Self::new()
    }
}

/// Response for tools/list
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListToolsResponse {
    /// Available tools
    pub tools: Vec<Tool>,
    /// Optional cursor for next page
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<Cursor>,
}

impl ListToolsResponse {
    pub fn new(tools: Vec<Tool>) -> Self {
        Self {
            tools,
            next_cursor: None,
        }
    }

    pub fn with_next_cursor(mut self, cursor: Cursor) -> Self {
        self.next_cursor = Some(cursor);
        self
    }
}

/// Parameters for tools/call request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallToolRequest {
    /// Name of the tool to call
    pub name: String,
    /// Arguments to pass to the tool
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Value>,
}

impl CallToolRequest {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            arguments: None,
        }
    }

    pub fn with_arguments(mut self, arguments: Value) -> Self {
        self.arguments = Some(arguments);
        self
    }
}

/// Content item types that tools can return
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ToolResult {
    /// Text content
    Text {
        text: String,
    },
    /// Image content
    Image {
        data: String,
        #[serde(rename = "mimeType")]
        mime_type: String,
    },
    /// Audio content
    Audio {
        data: String,
        #[serde(rename = "mimeType")]
        mime_type: String,
    },
    /// Resource reference
    Resource {
        resource: Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        annotations: Option<Value>,
    },
}

impl ToolResult {
    pub fn text(content: impl Into<String>) -> Self {
        Self::Text {
            text: content.into(),
        }
    }

    pub fn image(data: impl Into<String>, mime_type: impl Into<String>) -> Self {
        Self::Image {
            data: data.into(),
            mime_type: mime_type.into(),
        }
    }

    pub fn audio(data: impl Into<String>, mime_type: impl Into<String>) -> Self {
        Self::Audio {
            data: data.into(),
            mime_type: mime_type.into(),
        }
    }

    pub fn resource(resource: Value) -> Self {
        Self::Resource {
            resource,
            annotations: None,
        }
    }

    pub fn resource_with_annotations(resource: Value, annotations: Value) -> Self {
        Self::Resource {
            resource,
            annotations: Some(annotations),
        }
    }
}

/// Response for tools/call
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallToolResponse {
    /// Content returned by the tool
    pub content: Vec<ToolResult>,
    /// Whether the tool call resulted in an error
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

impl CallToolResponse {
    pub fn new(content: Vec<ToolResult>) -> Self {
        Self {
            content,
            is_error: None,
        }
    }

    pub fn success(content: Vec<ToolResult>) -> Self {
        Self {
            content,
            is_error: Some(false),
        }
    }

    pub fn error(content: Vec<ToolResult>) -> Self {
        Self {
            content,
            is_error: Some(true),
        }
    }

    pub fn with_error_flag(mut self, is_error: bool) -> Self {
        self.is_error = Some(is_error);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_tool_creation() {
        let schema = ToolSchema::object()
            .with_properties(HashMap::from([(
                "text".to_string(),
                JsonSchema::string(),
            )]))
            .with_required(vec!["text".to_string()]);

        let tool = Tool::new("test_tool", schema)
            .with_description("A test tool");

        assert_eq!(tool.name, "test_tool");
        assert!(tool.description.is_some());
        assert_eq!(tool.input_schema.schema_type, "object");
    }

    #[test]
    fn test_tool_result_creation() {
        let text_result = ToolResult::text("Hello, world!");
        let image_result = ToolResult::image("base64data", "image/png");
        let resource_result = ToolResult::resource(json!({"key": "value"}));

        assert!(matches!(text_result, ToolResult::Text { .. }));
        assert!(matches!(image_result, ToolResult::Image { .. }));
        assert!(matches!(resource_result, ToolResult::Resource { .. }));
    }

    #[test]
    fn test_call_tool_response() {
        let response = CallToolResponse::success(vec![
            ToolResult::text("Operation completed successfully"),
        ]);

        assert_eq!(response.is_error, Some(false));
        assert_eq!(response.content.len(), 1);
    }

    #[test]
    fn test_serialization() {
        let tool = Tool::new("echo", ToolSchema::object())
            .with_description("Echo tool");

        let json = serde_json::to_string(&tool).unwrap();
        assert!(json.contains("echo"));
        assert!(json.contains("Echo tool"));

        let parsed: Tool = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "echo");
    }
}