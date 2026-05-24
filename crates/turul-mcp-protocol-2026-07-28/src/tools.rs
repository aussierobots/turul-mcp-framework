//! MCP Tools Protocol Types
//!
//! This module defines the types used for the MCP tools functionality.

use crate::meta::Cursor;
// `JsonSchema` no longer imported at top-level: properties values are now
// `Value` per DRAFT-2026-v1 spec's `[k]: unknown`. Tests that exercise
// `JsonSchema` import it locally via `crate::schema::JsonSchema`.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

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

impl Default for ToolAnnotations {
    fn default() -> Self {
        Self::new()
    }
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

// `TaskSupport` and `ToolExecution` removed: tasks moved to extension (SEP-2663);
// DRAFT-2026-v1 `Tool` schema has no `execution` field. Task-aware tool
// invocation belongs in the tasks extension crate (when SEP-2663 is finalized).

// === Protocol Types ===

/// JSON Schema for `Tool.inputSchema` per DRAFT-2026-v1:
///
/// ```text
/// inputSchema: { $schema?: string; type: "object"; [key: string]: unknown }
/// ```
///
/// Root `type` MUST be `"object"`. All other keys (including `$schema`,
/// `properties`, `required`, `oneOf`, `anyOf`, `allOf`, `$ref`, `$defs`,
/// conditionals, etc.) are accepted as arbitrary JSON via `[key: string]: unknown`.
/// `properties` values can be any JSON Schema 2020-12 shape — modeled as
/// `Value` to honor the schema's `unknown` typing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSchema {
    /// The schema type — always `"object"` for inputSchema.
    #[serde(rename = "type")]
    pub schema_type: String,
    /// Property definitions — keys are property names, values are arbitrary
    /// JSON Schema 2020-12 shapes (`unknown` per schema). Accept `Value` rather
    /// than our structured `JsonSchema` enum so `$ref`, `oneOf` inside properties,
    /// and any other 2020-12 keyword work without conversion.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<HashMap<String, Value>>,
    /// Required property names.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
    /// Additional schema properties (`$schema`, `oneOf`, `$defs`, conditionals, etc.)
    /// per the schema's `[key: string]: unknown` clause.
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

    /// Attach properties. Values are arbitrary JSON Schema 2020-12 shapes.
    /// To convert from a structured [`JsonSchema`]: `serde_json::to_value(schema).unwrap()`.
    pub fn with_properties(mut self, properties: HashMap<String, Value>) -> Self {
        self.properties = Some(properties);
        self
    }

    pub fn with_required(mut self, required: Vec<String>) -> Self {
        self.required = Some(required);
        self
    }
}

/// JSON Schema for `Tool.outputSchema` per DRAFT-2026-v1:
///
/// ```text
/// outputSchema?: { $schema?: string; [key: string]: unknown }
/// ```
///
/// Unlike [`ToolSchema`] (inputSchema), `outputSchema` has **no root type
/// constraint** — it may describe any JSON value (object, array, string,
/// number, boolean, null). Use [`Self::any`] for an empty/unrestricted schema,
/// or attach arbitrary 2020-12 keywords via `additional`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolOutputSchema {
    /// All schema fields (no required `type` field — output may be any JSON
    /// per schema). Use this for `$schema`, `type` (if you want object root),
    /// `oneOf`, `anyOf`, `$ref`, `$defs`, etc.
    #[serde(flatten)]
    pub additional: HashMap<String, Value>,
}

impl ToolOutputSchema {
    /// Empty schema — accepts any JSON value as output.
    pub fn any() -> Self {
        Self::default()
    }

    /// Attach an arbitrary 2020-12 keyword.
    pub fn with(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.additional.insert(key.into(), value.into());
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
    /// JSON Schema for input parameters (root `type: "object"`).
    pub input_schema: ToolSchema,
    /// Optional JSON Schema for output results (any 2020-12 root).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<ToolOutputSchema>,
    /// Optional annotations for client hints
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<ToolAnnotations>,
    /// Optional icons for display. Most implementations do not need icons.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icons: Option<Vec<crate::icons::Icon>>,

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
            icons: None,
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

    pub fn with_output_schema(mut self, output_schema: ToolOutputSchema) -> Self {
        self.output_schema = Some(output_schema);
        self
    }

    pub fn with_annotations(mut self, annotations: ToolAnnotations) -> Self {
        self.annotations = Some(annotations);
        self
    }

    pub fn with_icons(mut self, icons: Vec<crate::icons::Icon>) -> Self {
        self.icons = Some(icons);
        self
    }

    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.meta = Some(meta);
        self
    }
}

/// Complete tools/list request — `ListToolsRequest extends PaginatedRequest
/// { method: "tools/list" }`.
///
/// `params` is the shared [`crate::json_rpc::PaginatedRequestParams`] —
/// no `ListToolsParams` shape exists in the spec.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListToolsRequest {
    /// Method name (always `"tools/list"`).
    pub method: String,
    /// Pagination params (shared with all `PaginatedRequest` interfaces).
    pub params: crate::json_rpc::PaginatedRequestParams,
}

impl Default for ListToolsRequest {
    fn default() -> Self {
        Self::new()
    }
}

impl ListToolsRequest {
    pub fn new() -> Self {
        Self {
            method: "tools/list".to_string(),
            params: crate::json_rpc::PaginatedRequestParams::new(),
        }
    }

    /// Attach a fully-constructed params struct.
    pub fn with_params(mut self, params: crate::json_rpc::PaginatedRequestParams) -> Self {
        self.params = params;
        self
    }

    pub fn with_cursor(mut self, cursor: Cursor) -> Self {
        self.params = self.params.with_cursor(cursor);
        self
    }

    pub fn with_meta(mut self, meta: crate::meta::RequestMetaObject) -> Self {
        self.params = self.params.with_meta(meta);
        self
    }
}

/// Result for tools/list — extends `PaginatedResult` and `CacheableResult`.
///
/// `ttl_ms` and `cache_scope` are REQUIRED on the wire per schema (CacheableResult mixin).
/// `new()` defaults them to `(0, Public)` (immediately-stale public response); callers
/// should override via [`Self::with_cache`] with realistic hints.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListToolsResult {
    /// Discriminator per `Result.resultType`.
    /// Defaults to `Complete` on deserialization (backward-compat clause).
    #[serde(default)]
    pub result_type: crate::result_type::ResultType,

    /// Available tools.
    pub tools: Vec<Tool>,
    /// Optional cursor for next page (`PaginatedResult.nextCursor`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<Cursor>,

    /// `CacheableResult.ttlMs` — required by schema.
    pub ttl_ms: u64,
    /// `CacheableResult.cacheScope` — required by schema.
    pub cache_scope: crate::caching::CacheScope,

    /// Meta information.
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<HashMap<String, Value>>,
}

impl ListToolsResult {
    /// Construct with required CacheableResult defaults (`ttl_ms=0`, `cache_scope=Public`).
    /// Override via [`Self::with_cache`].
    pub fn new(tools: Vec<Tool>) -> Self {
        Self {
            result_type: crate::result_type::ResultType::Complete,
            tools,
            next_cursor: None,
            ttl_ms: 0,
            cache_scope: crate::caching::CacheScope::Public,
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

    /// Attach cache hints.
    pub fn with_cache(mut self, cache: crate::caching::CacheableResult) -> Self {
        self.ttl_ms = cache.ttl_ms;
        self.cache_scope = cache.cache_scope;
        self
    }
}

/// Parameters for tools/call request — `CallToolRequestParams extends
/// InputResponseRequestParams`. Carries the SEP-2322 multi-round-trip mixin
/// (`inputResponses?`, `requestState?`) in addition to the tool-call-specific
/// `name` and `arguments`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallToolRequestParams {
    /// Name of the tool to call.
    pub name: String,
    /// Arguments to pass to the tool — `{ [key: string]: unknown }`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<HashMap<String, Value>>,

    /// Responses to a prior `InputRequiredResult` (mixin from
    /// `InputResponseRequestParams`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_responses: Option<crate::input_required::InputResponses>,
    /// Verbatim echo of `InputRequiredResult.requestState` (mixin).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_state: Option<String>,

    /// Schema-typed `_meta` per `RequestMetaObject`.
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<crate::meta::RequestMetaObject>,
}

impl CallToolRequestParams {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            arguments: None,
            input_responses: None,
            request_state: None,
            meta: None,
        }
    }

    /// Attach prior `InputRequiredResult` responses + verbatim state echo.
    pub fn with_input_responses(
        mut self,
        responses: crate::input_required::InputResponses,
        request_state: impl Into<String>,
    ) -> Self {
        self.input_responses = Some(responses);
        self.request_state = Some(request_state.into());
        self
    }

    /// Get arguments as HashMap - CRITICAL: Use this instead of the trait method
    /// The trait method has limitations due to lifetime issues with HashMap->Value conversion
    pub fn get_arguments(&self) -> Option<&HashMap<String, Value>> {
        self.arguments.as_ref()
    }

    /// Get arguments as Value (converted from HashMap)
    pub fn get_arguments_as_value(&self) -> Option<Value> {
        self.arguments
            .as_ref()
            .map(|map| Value::Object(map.clone().into_iter().collect()))
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

    pub fn with_meta(mut self, meta: crate::meta::RequestMetaObject) -> Self {
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
    pub params: CallToolRequestParams,
}

impl CallToolRequest {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            method: "tools/call".to_string(),
            params: CallToolRequestParams::new(name),
        }
    }

    /// Attach a fully-constructed params struct.
    pub fn with_params(mut self, params: CallToolRequestParams) -> Self {
        self.params = params;
        self
    }

    pub fn with_arguments(mut self, arguments: HashMap<String, Value>) -> Self {
        self.params = self.params.with_arguments(arguments);
        self
    }

    pub fn with_arguments_value(mut self, arguments: Value) -> Self {
        self.params = self.params.with_arguments_value(arguments);
        self
    }

    pub fn with_meta(mut self, meta: crate::meta::RequestMetaObject) -> Self {
        self.params = self.params.with_meta(meta);
        self
    }
}

/// Tool result type - an alias for ContentBlock to maintain backward compatibility
/// while ensuring MCP 2025-11-25 specification compliance
pub type ToolResult = crate::content::ContentBlock;

/// Result for tools/call — extends `Result`.
///
/// Note on `structuredContent`: DRAFT-2026-v1 widens this from `object`-only
/// (2025-11-25) to any JSON value (`unknown`). The field is typed `Option<Value>`
/// here — arrays, scalars, and null are now accepted.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallToolResult {
    /// Discriminator per `Result.resultType`.
    /// `Complete` for normal results; clients receiving an `input_required` value
    /// should parse the message as `InputRequiredResult` instead via the
    /// `CallToolResultResponse.result` union.
    #[serde(default)]
    pub result_type: crate::result_type::ResultType,

    /// Content returned by the tool.
    pub content: Vec<ToolResult>,
    /// Whether the tool call resulted in an error.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
    /// Structured content (any JSON value per DRAFT-2026-v1; was object-only in 2025-11-25).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structured_content: Option<Value>,
    /// Meta information (follows MCP Result interface).
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
            result_type: crate::result_type::ResultType::Complete,
            content,
            is_error: None,
            structured_content: None,
            meta: None,
        }
    }

    pub fn success(content: Vec<ToolResult>) -> Self {
        Self {
            result_type: crate::result_type::ResultType::Complete,
            content,
            is_error: Some(false),
            structured_content: None,
            meta: None,
        }
    }

    pub fn error(content: Vec<ToolResult>) -> Self {
        Self {
            result_type: crate::result_type::ResultType::Complete,
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

    /// Create response from serializable result with optional structured content
    pub fn from_result<T: serde::Serialize>(result: &T) -> Result<Self, crate::McpError> {
        let text_content = serde_json::to_string(result)
            .map_err(|e| crate::McpError::tool_execution(&format!("Serialization error: {}", e)))?;

        Ok(Self::success(vec![ToolResult::text(text_content)]))
    }

    /// Create response with both text and structured content
    pub fn from_result_with_structured<T: serde::Serialize>(
        result: &T,
    ) -> Result<Self, crate::McpError> {
        let text_content = serde_json::to_string(result)
            .map_err(|e| crate::McpError::tool_execution(&format!("Serialization error: {}", e)))?;

        let structured = serde_json::to_value(result).map_err(|e| {
            crate::McpError::tool_execution(&format!("Structured content error: {}", e))
        })?;

        Ok(Self::success(vec![ToolResult::text(text_content)]).with_structured_content(structured))
    }

    /// Create response from serializable result with automatic structured content based on schema
    pub fn from_result_with_schema<T: serde::Serialize>(
        result: &T,
        schema: Option<&ToolSchema>,
    ) -> Result<Self, crate::McpError> {
        let text_content = serde_json::to_string(result)
            .map_err(|e| crate::McpError::tool_execution(&format!("Serialization error: {}", e)))?;

        let response = Self::success(vec![ToolResult::text(text_content)]);

        // Auto-add structured content if schema exists
        if schema.is_some() {
            let structured = serde_json::to_value(result).map_err(|e| {
                crate::McpError::tool_execution(&format!("Structured content error: {}", e))
            })?;
            Ok(response.with_structured_content(structured))
        } else {
            Ok(response)
        }
    }

    /// Create response with automatic structured content for primitives (zero-config)
    pub fn from_result_auto<T: serde::Serialize>(
        result: &T,
        schema: Option<&ToolSchema>,
    ) -> Result<Self, crate::McpError> {
        let text_content = serde_json::to_string(result)
            .map_err(|e| crate::McpError::tool_execution(&format!("Serialization error: {}", e)))?;

        let response = Self::success(vec![ToolResult::text(text_content)]);

        // Auto-detect structured content for common types
        let structured = serde_json::to_value(result).map_err(|e| {
            crate::McpError::tool_execution(&format!("Structured content error: {}", e))
        })?;

        let should_add_structured = schema.is_some()
            || match &structured {
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
    pub fn from_json_with_schema(json_result: Value, schema: Option<&ToolSchema>) -> Self {
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

// `HasListToolsParams` is implemented for the shared `PaginatedRequestParams`
// type (one impl serves all `PaginatedRequest` extenders — see ADR notes on
// the `ListToolsParams` → `PaginatedRequestParams` collapse).
impl HasListToolsParams for crate::json_rpc::PaginatedRequestParams {
    fn cursor(&self) -> Option<&Cursor> {
        self.cursor.as_ref()
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

// Trait implementations for CallToolRequestParams
impl Params for CallToolRequestParams {}

impl HasCallToolRequestParams for CallToolRequestParams {
    fn name(&self) -> &String {
        &self.name
    }

    fn arguments(&self) -> Option<&Value> {
        // Note: This trait method has limitations due to HashMap<String, Value> -> Value conversion
        // The conversion creates a temporary Value that can't be borrowed for the required lifetime.
        //
        // For now, use CallToolRequestParams::get_arguments() for HashMap access or
        // get_arguments_as_value() for owned Value access in downstream code.
        //
        // The direct .arguments field access works fine and is used by the framework.
        None
    }

    fn meta(&self) -> Option<&HashMap<String, Value>> {
        self.meta.as_ref().map(|m| &m.extra)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::ResourceContents;
    use crate::schema::JsonSchema;
    use serde_json::json;

    #[test]
    fn test_tool_creation() {
        // `properties` now accepts any JSON value per DRAFT-2026-v1 spec
        // (schema's `[k]: unknown` clause); convert structured JsonSchema via to_value.
        let schema = ToolSchema::object()
            .with_properties(HashMap::from([(
                "text".to_string(),
                serde_json::to_value(JsonSchema::string()).unwrap(),
            )]))
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
        let resource_result = ToolResult::resource(ResourceContents::text(
            "file:///test/resource.json",
            serde_json::to_string(&json!({"key": "value"})).unwrap(),
        ));

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

    #[test]
    fn test_tool_with_icons() {
        use crate::icons::Icon;
        let tool = Tool::new("test", ToolSchema::object())
            .with_icons(vec![Icon::new("https://example.com/tool.png")]);

        let json = serde_json::to_value(&tool).unwrap();
        assert!(json.get("icons").is_some(), "should have icons field");
        let icons = json["icons"].as_array().unwrap();
        assert_eq!(icons.len(), 1);
        assert_eq!(icons[0]["src"], "https://example.com/tool.png");

        // Verify singular "icon" field is NOT present
        assert!(
            json.get("icon").is_none(),
            "should NOT have singular icon field"
        );

        let parsed: Tool = serde_json::from_str(&serde_json::to_string(&tool).unwrap()).unwrap();
        assert_eq!(parsed.icons.unwrap().len(), 1);
    }

    #[test]
    fn test_tool_without_icons_omits_field() {
        let tool = Tool::new("test", ToolSchema::object());
        let json = serde_json::to_string(&tool).unwrap();
        assert!(!json.contains("icons"));
    }
}
