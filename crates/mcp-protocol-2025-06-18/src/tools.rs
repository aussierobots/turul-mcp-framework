//! MCP Tools Protocol Types
//!
//! This module defines the types used for the MCP tools functionality.

use crate::meta::Cursor;
use crate::schema::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

// ===========================================
// === Tool Definition Trait Hierarchy ===
// ===========================================

/// Base metadata trait - matches TypeScript BaseMetadata interface
pub trait HasBaseMetadata {
    /// Programmatic identifier (fallback display name)
    fn name(&self) -> &str;
    
    /// Human-readable display name (UI contexts)
    fn title(&self) -> Option<&str> { None }
}

/// Tool description trait
pub trait HasDescription {
    fn description(&self) -> Option<&str> { None }
}

/// Input schema trait
pub trait HasInputSchema {
    fn input_schema(&self) -> &ToolSchema;
}

/// Output schema trait  
pub trait HasOutputSchema {
    fn output_schema(&self) -> Option<&ToolSchema> { None }
}

/// Annotations trait
pub trait HasAnnotations {
    fn annotations(&self) -> Option<&ToolAnnotations> { None }
}

/// Tool-specific meta trait (separate from RPC _meta)
pub trait HasToolMeta {
    fn tool_meta(&self) -> Option<&HashMap<String, Value>> { None }
}

/// Complete tool definition - composed from fine-grained traits
pub trait ToolDefinition: 
    HasBaseMetadata +           // name, title
    HasDescription +            // description  
    HasInputSchema +            // inputSchema
    HasOutputSchema +           // outputSchema
    HasAnnotations +            // annotations
    HasToolMeta +               // _meta (tool-specific)
    Send + 
    Sync 
{
    /// Display name precedence: title > annotations.title > name (matches TypeScript spec)
    fn display_name(&self) -> &str {
        if let Some(title) = self.title() {
            title
        } else if let Some(annotations) = self.annotations() {
            if let Some(title) = &annotations.title {
                title
            } else {
                self.name()
            }
        } else {
            self.name()
        }
    }
    
    /// Convert to concrete Tool struct for protocol serialization
    fn to_tool(&self) -> Tool {
        Tool {
            name: self.name().to_string(),
            title: self.title().map(String::from),
            description: self.description().map(String::from),
            input_schema: self.input_schema().clone(),
            output_schema: self.output_schema().cloned(),
            annotations: self.annotations().cloned(),
            meta: self.tool_meta().cloned(),
        }
    }
}

/// Tool annotations structure (matches TypeScript ToolAnnotations)
/// NOTE: all properties in ToolAnnotations are **hints**.
/// They are not guaranteed to provide a faithful description of tool behavior.
/// Clients should never make tool use decisions based on ToolAnnotations from untrusted servers.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolAnnotations {
    /// A human-readable title for the tool
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// If true, the tool does not modify its environment. Default: false
    #[serde(rename = "readOnlyHint", skip_serializing_if = "Option::is_none")]
    pub read_only_hint: Option<bool>,
    /// If true, the tool may perform destructive updates to its environment.
    /// If false, the tool performs only additive updates.
    /// (This property is meaningful only when `readOnlyHint == false`) Default: true
    #[serde(rename = "destructiveHint", skip_serializing_if = "Option::is_none")]
    pub destructive_hint: Option<bool>,
    /// If true, calling the tool repeatedly with the same arguments
    /// will have no additional effect on its environment.
    /// (This property is meaningful only when `readOnlyHint == false`) Default: false
    #[serde(rename = "idempotentHint", skip_serializing_if = "Option::is_none")]
    pub idempotent_hint: Option<bool>,
    /// If true, this tool may interact with an "open world" of external entities.
    /// If false, the tool's domain of interaction is closed.
    /// For example, the world of a web search tool is open, whereas that of a memory tool is not.
    /// Default: true
    #[serde(rename = "openWorldHint", skip_serializing_if = "Option::is_none")]
    pub open_world_hint: Option<bool>,
}

impl ToolAnnotations {
    pub fn new() -> Self {
        Self { 
            title: None,
            read_only_hint: None,
            destructive_hint: None,
            idempotent_hint: None,
            open_world_hint: None,
        }
    }
    
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }
    
    pub fn with_read_only_hint(mut self, read_only: bool) -> Self {
        self.read_only_hint = Some(read_only);
        self
    }
    
    pub fn with_destructive_hint(mut self, destructive: bool) -> Self {
        self.destructive_hint = Some(destructive);
        self
    }
    
    pub fn with_idempotent_hint(mut self, idempotent: bool) -> Self {
        self.idempotent_hint = Some(idempotent);
        self
    }
    
    pub fn with_open_world_hint(mut self, open_world: bool) -> Self {
        self.open_world_hint = Some(open_world);
        self
    }
}

// ===========================================
// === Protocol Types ===
// ===========================================

/// JSON Schema definition for tool input/output (matches TypeScript spec exactly)
/// Must be an object with type: "object", properties, and required fields
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// Additional schema properties
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
    /// Intended for UI and end-user contexts — optimized to be human-readable
    /// and easily understood, even by those unfamiliar with domain-specific terminology.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Optional human-readable description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// JSON Schema for input parameters
    pub input_schema: ToolSchema,
    /// Optional JSON Schema for output results
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<ToolSchema>,
    /// Optional annotations for client hints
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<ToolAnnotations>,

    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        alias = "_meta",
        rename = "_meta"
    )]
    pub meta: Option<HashMap<String, Value>>,
}

impl Tool {
    pub fn new(name: impl Into<String>, input_schema: ToolSchema) -> Self {
        Self {
            name: name.into(),
            title: None,
            description: None,
            input_schema,
            output_schema: None,
            annotations: None,
            meta: None,
        }
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_output_schema(mut self, output_schema: ToolSchema) -> Self {
        self.output_schema = Some(output_schema);
        self
    }

    pub fn with_annotations(mut self, annotations: ToolAnnotations) -> Self {
        self.annotations = Some(annotations);
        self
    }

    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.meta = Some(meta);
        self
    }
}

// ===========================================
// === Tool Implements ToolDefinition ===
// ===========================================

impl HasBaseMetadata for Tool {
    fn name(&self) -> &str { &self.name }
    fn title(&self) -> Option<&str> { self.title.as_deref() }
}

impl HasDescription for Tool {
    fn description(&self) -> Option<&str> { self.description.as_deref() }
}

impl HasInputSchema for Tool {
    fn input_schema(&self) -> &ToolSchema { &self.input_schema }
}

impl HasOutputSchema for Tool {
    fn output_schema(&self) -> Option<&ToolSchema> { self.output_schema.as_ref() }
}

impl HasAnnotations for Tool {
    fn annotations(&self) -> Option<&ToolAnnotations> { self.annotations.as_ref() }
}

impl HasToolMeta for Tool {
    fn tool_meta(&self) -> Option<&HashMap<String, Value>> { self.meta.as_ref() }
}

// Blanket implementation: any type that implements all component traits automatically implements ToolDefinition
impl<T> ToolDefinition for T 
where 
    T: HasBaseMetadata + HasDescription + HasInputSchema + HasOutputSchema + HasAnnotations + HasToolMeta + Send + Sync,
{
    // Default implementations are provided by the trait definition
}

/// Parameters for tools/list request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListToolsParams {
    /// Optional cursor for pagination
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<Cursor>,
    /// Meta information (optional _meta field inside params)
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<HashMap<String, Value>>,
}

impl ListToolsParams {
    pub fn new() -> Self {
        Self {
            cursor: None,
            meta: None,
        }
    }

    pub fn with_cursor(mut self, cursor: Cursor) -> Self {
        self.cursor = Some(cursor);
        self
    }

    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.meta = Some(meta);
        self
    }
}

impl Default for ListToolsParams {
    fn default() -> Self {
        Self::new()
    }
}

/// Complete tools/list request (matches TypeScript ListToolsRequest interface)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListToolsRequest {
    /// Method name (always "tools/list")
    pub method: String,
    /// Request parameters
    pub params: ListToolsParams,
}

impl ListToolsRequest {
    pub fn new() -> Self {
        Self {
            method: "tools/list".to_string(),
            params: ListToolsParams::new(),
        }
    }

    pub fn with_cursor(mut self, cursor: Cursor) -> Self {
        self.params = self.params.with_cursor(cursor);
        self
    }

    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.params = self.params.with_meta(meta);
        self
    }
}

/// Result for tools/list (per MCP spec) - extends PaginatedResult
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListToolsResult {
    /// Available tools
    pub tools: Vec<Tool>,
    /// Optional cursor for next page
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<Cursor>,
    /// Meta information (from PaginatedResult)
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<HashMap<String, Value>>,
}

impl ListToolsResult {
    pub fn new(tools: Vec<Tool>) -> Self {
        Self {
            tools,
            next_cursor: None,
            meta: None,
        }
    }

    pub fn with_next_cursor(mut self, cursor: Cursor) -> Self {
        self.next_cursor = Some(cursor);
        self
    }
    
    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.meta = Some(meta);
        self
    }
}

/// Parameters for tools/call request (matches TypeScript CallToolRequest.params)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallToolParams {
    /// Name of the tool to call
    pub name: String,
    /// Arguments to pass to the tool - matches TypeScript { [key: string]: unknown }
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<HashMap<String, Value>>,
    /// Meta information (optional _meta field inside params)
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<HashMap<String, Value>>,
}

impl CallToolParams {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            arguments: None,
            meta: None,
        }
    }

    pub fn with_arguments(mut self, arguments: HashMap<String, Value>) -> Self {
        self.arguments = Some(arguments);
        self
    }
    
    pub fn with_arguments_value(mut self, arguments: Value) -> Self {
        // Helper for backward compatibility - convert Value to HashMap if it's an object
        if let Value::Object(map) = arguments {
            self.arguments = Some(map.into_iter().collect());
        }
        self
    }

    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.meta = Some(meta);
        self
    }
}

/// Complete tools/call request (matches TypeScript CallToolRequest interface)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallToolRequest {
    /// Method name (always "tools/call")
    pub method: String,
    /// Request parameters
    pub params: CallToolParams,
}

impl CallToolRequest {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            method: "tools/call".to_string(),
            params: CallToolParams::new(name),
        }
    }

    pub fn with_arguments(mut self, arguments: HashMap<String, Value>) -> Self {
        self.params = self.params.with_arguments(arguments);
        self
    }
    
    pub fn with_arguments_value(mut self, arguments: Value) -> Self {
        self.params = self.params.with_arguments_value(arguments);
        self
    }

    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.params = self.params.with_meta(meta);
        self
    }
}

/// Content item types that tools can return
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ToolResult {
    /// Text content
    Text { text: String },
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

/// Result for tools/call (per MCP spec)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallToolResult {
    /// Content returned by the tool
    pub content: Vec<ToolResult>,
    /// Whether the tool call resulted in an error
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
    /// Structured content that matches the tool's output schema (MCP 2025-06-18)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structured_content: Option<Value>,
    /// Meta information (follows MCP Result interface)
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        alias = "_meta",
        rename = "_meta"
    )]
    pub meta: Option<HashMap<String, Value>>,
}

impl CallToolResult {
    pub fn new(content: Vec<ToolResult>) -> Self {
        Self {
            content,
            is_error: None,
            structured_content: None,
            meta: None,
        }
    }

    pub fn success(content: Vec<ToolResult>) -> Self {
        Self {
            content,
            is_error: Some(false),
            structured_content: None,
            meta: None,
        }
    }

    pub fn error(content: Vec<ToolResult>) -> Self {
        Self {
            content,
            is_error: Some(true),
            structured_content: None,
            meta: None,
        }
    }

    pub fn with_error_flag(mut self, is_error: bool) -> Self {
        self.is_error = Some(is_error);
        self
    }

    pub fn with_structured_content(mut self, structured_content: Value) -> Self {
        self.structured_content = Some(structured_content);
        self
    }

    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.meta = Some(meta);
        self
    }
    
    // ===========================================
    // === Smart Response Builders ===
    // ===========================================
    
    /// Create response from serializable result with automatic structured content based on ToolDefinition
    pub fn from_result_with_tool<T: serde::Serialize>(
        result: &T,
        tool: &dyn ToolDefinition
    ) -> Result<Self, crate::McpError> {
        let text_content = serde_json::to_string(result)
            .map_err(|e| crate::McpError::tool_execution(&format!("Serialization error: {}", e)))?;
        
        let response = Self::success(vec![ToolResult::text(text_content)]);
        
        // Auto-add structured content if tool has output schema
        if let Some(_) = tool.output_schema() {
            let structured = serde_json::to_value(result)
                .map_err(|e| crate::McpError::tool_execution(&format!("Structured content error: {}", e)))?;
            Ok(response.with_structured_content(structured))
        } else {
            Ok(response)
        }
    }
    
    /// Create response from serializable result with automatic structured content based on schema
    pub fn from_result_with_schema<T: serde::Serialize>(
        result: &T,
        schema: Option<&ToolSchema>
    ) -> Result<Self, crate::McpError> {
        let text_content = serde_json::to_string(result)
            .map_err(|e| crate::McpError::tool_execution(&format!("Serialization error: {}", e)))?;
        
        let response = Self::success(vec![ToolResult::text(text_content)]);
        
        // Auto-add structured content if schema exists
        if schema.is_some() {
            let structured = serde_json::to_value(result)
                .map_err(|e| crate::McpError::tool_execution(&format!("Structured content error: {}", e)))?;
            Ok(response.with_structured_content(structured))
        } else {
            Ok(response)
        }
    }
    
    /// Create response with automatic structured content for primitives (zero-config)
    pub fn from_result_auto<T: serde::Serialize>(
        result: &T,
        schema: Option<&ToolSchema>
    ) -> Result<Self, crate::McpError> {
        let text_content = serde_json::to_string(result)
            .map_err(|e| crate::McpError::tool_execution(&format!("Serialization error: {}", e)))?;
        
        let response = Self::success(vec![ToolResult::text(text_content)]);
        
        // Auto-detect structured content for common types
        let structured = serde_json::to_value(result)
            .map_err(|e| crate::McpError::tool_execution(&format!("Structured content error: {}", e)))?;
        
        let should_add_structured = schema.is_some() || match &structured {
            // Auto-add structured content for primitive types (zero-config)
            Value::Number(_) | Value::Bool(_) => true,
            // Auto-add for arrays and objects (structured data)
            Value::Array(_) | Value::Object(_) => true,
            // Skip for plain strings (text is sufficient)
            Value::String(_) => false,
            Value::Null => false,
        };
        
        if should_add_structured {
            Ok(response.with_structured_content(structured))
        } else {
            Ok(response)
        }
    }

    /// Create response from JSON value with automatic structured content
    pub fn from_json_with_schema(
        json_result: Value,
        schema: Option<&ToolSchema>
    ) -> Self {
        let text_content = json_result.to_string();
        let response = Self::success(vec![ToolResult::text(text_content)]);
        
        if schema.is_some() {
            response.with_structured_content(json_result)
        } else {
            response
        }
    }
}

// Trait implementations for CallToolResult

use crate::traits::*;

impl HasData for CallToolResult {
    fn data(&self) -> HashMap<String, Value> {
        let mut data = HashMap::new();
        data.insert(
            "content".to_string(),
            serde_json::to_value(&self.content).unwrap_or(Value::Null),
        );
        if let Some(is_error) = self.is_error {
            data.insert("isError".to_string(), Value::Bool(is_error));
        }
        if let Some(ref structured_content) = self.structured_content {
            data.insert("structuredContent".to_string(), structured_content.clone());
        }
        data
    }
}

impl HasMeta for CallToolResult {
    fn meta(&self) -> Option<HashMap<String, Value>> {
        self.meta.clone()
    }
}

impl RpcResult for CallToolResult {}

impl crate::traits::CallToolResult for CallToolResult {
    fn content(&self) -> &Vec<ToolResult> {
        &self.content
    }

    fn is_error(&self) -> Option<bool> {
        self.is_error
    }

    fn structured_content(&self) -> Option<&Value> {
        self.structured_content.as_ref()
    }
}

// Trait implementations for ListToolsParams
impl Params for ListToolsParams {}

impl HasListToolsParams for ListToolsParams {
    fn cursor(&self) -> Option<&Cursor> {
        self.cursor.as_ref()
    }
}

impl HasMetaParam for ListToolsParams {
    fn meta(&self) -> Option<&HashMap<String, Value>> {
        self.meta.as_ref()
    }
}

// Trait implementations for ListToolsRequest
impl HasMethod for ListToolsRequest {
    fn method(&self) -> &str {
        &self.method
    }
}

impl HasParams for ListToolsRequest {
    fn params(&self) -> Option<&dyn Params> {
        Some(&self.params)
    }
}

// Trait implementations for ListToolsResult
impl HasData for ListToolsResult {
    fn data(&self) -> HashMap<String, Value> {
        let mut data = HashMap::new();
        data.insert(
            "tools".to_string(),
            serde_json::to_value(&self.tools).unwrap_or(Value::Null),
        );
        if let Some(ref next_cursor) = self.next_cursor {
            data.insert(
                "nextCursor".to_string(),
                Value::String(next_cursor.as_str().to_string()),
            );
        }
        data
    }
}

impl HasMeta for ListToolsResult {
    fn meta(&self) -> Option<HashMap<String, Value>> {
        self.meta.clone()
    }
}

impl RpcResult for ListToolsResult {}

impl crate::traits::ListToolsResult for ListToolsResult {
    fn tools(&self) -> &Vec<Tool> {
        &self.tools
    }

    fn next_cursor(&self) -> Option<&Cursor> {
        self.next_cursor.as_ref()
    }
}

// Trait implementations for CallToolParams
impl Params for CallToolParams {}

impl HasCallToolParams for CallToolParams {
    fn name(&self) -> &String {
        &self.name
    }

    fn arguments(&self) -> Option<&Value> {
        // This is a temporary workaround for trait compatibility
        // The trait expects &Value but we store HashMap<String, Value>
        // TODO: Fix trait definition to use proper HashMap type
        self.arguments.as_ref().and_then(|_| None) // Return None for now
    }

    fn meta(&self) -> Option<&HashMap<String, Value>> {
        self.meta.as_ref()
    }
}

// Trait implementations for CallToolRequest
impl HasMethod for CallToolRequest {
    fn method(&self) -> &str {
        &self.method
    }
}

impl HasParams for CallToolRequest {
    fn params(&self) -> Option<&dyn Params> {
        Some(&self.params)
    }
}

pub mod builder;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_tool_creation() {
        let schema = ToolSchema::object()
            .with_properties(HashMap::from([("text".to_string(), serde_json::json!({"type": "string"}))]))
            .with_required(vec!["text".to_string()]);

        let tool = Tool::new("test_tool", schema).with_description("A test tool");

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
        let response =
            CallToolResult::success(vec![ToolResult::text("Operation completed successfully")]);

        assert_eq!(response.is_error, Some(false));
        assert_eq!(response.content.len(), 1);
        assert!(response.structured_content.is_none());
    }

    #[test]
    fn test_call_tool_response_with_structured_content() {
        let structured_data = serde_json::json!({
            "result": "success",
            "value": 42
        });

        let response =
            CallToolResult::success(vec![ToolResult::text("Operation completed successfully")])
                .with_structured_content(structured_data.clone());

        assert_eq!(response.is_error, Some(false));
        assert_eq!(response.content.len(), 1);
        assert_eq!(response.structured_content, Some(structured_data));
    }

    #[test]
    fn test_serialization() {
        let tool = Tool::new("echo", ToolSchema::object()).with_description("Echo tool");

        let json = serde_json::to_string(&tool).unwrap();
        assert!(json.contains("echo"));
        assert!(json.contains("Echo tool"));

        let parsed: Tool = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "echo");
    }
}
