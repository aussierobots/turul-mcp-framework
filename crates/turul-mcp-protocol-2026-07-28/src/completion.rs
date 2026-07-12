//! MCP Completion Protocol Types
//!
//! This module defines types for completion requests in MCP.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// A reference to a resource or resource template definition (per MCP spec)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceTemplateReference {
    #[serde(rename = "type")]
    pub ref_type: String, // "ref/resource"
    /// The URI or URI template of the resource
    #[serde(rename = "uri")]
    pub uri: String,
}

/// Identifies a prompt (per MCP spec) - extends BaseMetadata
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptReference {
    #[serde(rename = "type")]
    pub ref_type: String, // "ref/prompt"
    /// The name of the prompt (BaseMetadata).
    pub name: String,
    /// Display title (BaseMetadata).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

/// Union type for completion references — `ref/resource` or `ref/prompt`.
///
/// Schema fixes a literal `type` discriminator per interface
/// (`ResourceTemplateReference.type = "ref/resource"`,
/// `PromptReference.type = "ref/prompt"`) — `#[serde(untagged)]` alone tries
/// variants in declaration order and can't reject a `type` value that
/// doesn't match either literal but is otherwise structurally compatible
/// (e.g. `{"type":"bogus","uri":"x"}` still matches [`ResourceTemplateReference`]
/// since `ref_type` is a bare `String`). `Deserialize` is hand-written to
/// dispatch on `type` before trying a variant, mirroring
/// `PrimitiveSchemaDefinition` in `elicitation.rs`.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum CompletionReference {
    ResourceTemplate(ResourceTemplateReference),
    Prompt(PromptReference),
}

impl<'de> Deserialize<'de> for CompletionReference {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        match value.get("type").and_then(Value::as_str) {
            Some("ref/resource") => serde_json::from_value(value)
                .map(CompletionReference::ResourceTemplate)
                .map_err(serde::de::Error::custom),
            Some("ref/prompt") => serde_json::from_value(value)
                .map(CompletionReference::Prompt)
                .map_err(serde::de::Error::custom),
            other => Err(serde::de::Error::custom(format!(
                "CompletionReference: unrecognized or missing `type` discriminator {other:?} \
                 (expected \"ref/resource\" or \"ref/prompt\")"
            ))),
        }
    }
}

/// Completion context (per MCP spec)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompletionContext {
    /// Arguments context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<HashMap<String, String>>,
}

/// Argument being completed (per MCP spec)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompleteArgument {
    /// Name of the argument
    pub name: String,
    /// Current value being completed
    pub value: String,
}

/// Completion request parameters (per MCP spec)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompleteRequestParams {
    /// Reference to the prompt or resource being completed
    #[serde(rename = "ref")]
    pub reference: CompletionReference,
    /// The argument being completed
    pub argument: CompleteArgument,
    /// Optional context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<CompletionContext>,
    /// Schema-typed `_meta` per `RequestMetaObject`. Required per schema
    /// (`CompleteRequestParams extends RequestParams`, and `RequestParams._meta`
    /// is required in DRAFT-2026-v1 stateless core).
    #[serde(rename = "_meta")]
    pub meta: crate::meta::RequestMetaObject,
}

impl CompleteRequestParams {
    /// Construct with the required `_meta`, reference, and argument.
    pub fn new(
        reference: CompletionReference,
        argument: CompleteArgument,
        meta: crate::meta::RequestMetaObject,
    ) -> Self {
        Self {
            reference,
            argument,
            context: None,
            meta,
        }
    }

    pub fn with_context(mut self, context: CompletionContext) -> Self {
        self.context = Some(context);
        self
    }

    pub fn with_meta(mut self, meta: crate::meta::RequestMetaObject) -> Self {
        self.meta = meta;
        self
    }
}

/// Complete completion/complete request (matches TypeScript CompleteRequest interface)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompleteRequest {
    /// Method name (always "completion/complete")
    pub method: String,
    /// Request parameters
    pub params: CompleteRequestParams,
}

impl CompleteRequest {
    /// Construct with the required `_meta`, reference, and argument.
    pub fn new(
        reference: CompletionReference,
        argument: CompleteArgument,
        meta: crate::meta::RequestMetaObject,
    ) -> Self {
        Self {
            method: "completion/complete".to_string(),
            params: CompleteRequestParams::new(reference, argument, meta),
        }
    }

    /// Attach a fully-constructed params struct.
    pub fn with_params(mut self, params: CompleteRequestParams) -> Self {
        self.params = params;
        self
    }

    pub fn with_context(mut self, context: CompletionContext) -> Self {
        self.params = self.params.with_context(context);
        self
    }

    pub fn with_meta(mut self, meta: crate::meta::RequestMetaObject) -> Self {
        self.params = self.params.with_meta(meta);
        self
    }
}

/// Completion result (per MCP spec)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompletionResult {
    /// The completion values.
    ///
    /// Schema: `@maxItems 100` — "Must not exceed 100 items." This
    /// constructor does not truncate: the server dispatch layer
    /// (`CompletionHandler` in `turul-mcp-server`) is the single
    /// authoritative enforcement point, since it truncates `values` to 100
    /// while capturing the pre-truncation length into `total` — a
    /// constructor-level truncation here would discard that count before
    /// the dispatcher ever sees it.
    pub values: Vec<String>,
    /// Optional total number of possible completions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<f64>,
    /// Whether there are more completions available
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_more: Option<bool>,
}

/// Complete completion/complete response — extends `Result`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompleteResult {
    /// Discriminator per `Result.resultType`.
    #[serde(default)]
    pub result_type: crate::result_type::ResultType,

    /// The completion result.
    pub completion: CompletionResult,
    /// Meta information (follows MCP Result interface).
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        alias = "_meta",
        rename = "_meta"
    )]
    pub meta: Option<HashMap<String, Value>>,
}

impl CompletionResult {
    pub fn new(values: Vec<String>) -> Self {
        Self {
            values,
            total: None,
            has_more: None,
        }
    }

    pub fn with_total(mut self, total: f64) -> Self {
        self.total = Some(total);
        self
    }

    pub fn with_has_more(mut self, has_more: bool) -> Self {
        self.has_more = Some(has_more);
        self
    }
}

impl CompleteResult {
    pub fn new(completion: CompletionResult) -> Self {
        Self {
            result_type: crate::result_type::ResultType::Complete,
            completion,
            meta: None,
        }
    }

    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.meta = Some(meta);
        self
    }
}

/// Convenience constructors
impl ResourceTemplateReference {
    pub fn new(uri: impl Into<String>) -> Self {
        Self {
            ref_type: "ref/resource".to_string(),
            uri: uri.into(),
        }
    }
}

impl PromptReference {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            ref_type: "ref/prompt".to_string(),
            name: name.into(),
            title: None,
        }
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }
}

impl CompletionReference {
    pub fn resource(uri: impl Into<String>) -> Self {
        Self::ResourceTemplate(ResourceTemplateReference::new(uri))
    }

    pub fn prompt(name: impl Into<String>) -> Self {
        Self::Prompt(PromptReference::new(name))
    }
}

impl CompleteArgument {
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
        }
    }
}

impl Default for CompletionContext {
    fn default() -> Self {
        Self::new()
    }
}

impl CompletionContext {
    pub fn new() -> Self {
        Self { arguments: None }
    }

    pub fn with_arguments(mut self, arguments: HashMap<String, String>) -> Self {
        self.arguments = Some(arguments);
        self
    }
}

// Trait implementations for protocol compliance
use crate::traits::*;

impl Params for CompleteRequestParams {}

impl HasMetaParam for CompleteRequestParams {
    fn meta(&self) -> Option<&HashMap<String, Value>> {
        // Surface namespaced `extra` keys from the required typed `RequestMetaObject`.
        Some(&self.meta.extra)
    }
}

impl HasMethod for CompleteRequest {
    fn method(&self) -> &str {
        &self.method
    }
}

impl HasParams for CompleteRequest {
    fn params(&self) -> Option<&dyn Params> {
        Some(&self.params)
    }
}

impl HasData for CompleteResult {
    fn data(&self) -> HashMap<String, Value> {
        let mut data = HashMap::new();
        data.insert(
            "completion".to_string(),
            serde_json::to_value(&self.completion).unwrap_or(Value::Null),
        );
        data
    }
}

impl HasMeta for CompleteResult {
    fn meta(&self) -> Option<&crate::meta::MetaObject> {
        self.meta.as_ref()
    }
}

impl HasResultType for CompleteResult {
    fn result_type(&self) -> crate::result_type::ResultType {
        self.result_type.clone()
    }
}

impl RpcResult for CompleteResult {}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_resource_template_reference() {
        let ref_obj = ResourceTemplateReference::new("file:///test/{name}.txt");

        assert_eq!(ref_obj.ref_type, "ref/resource");
        assert_eq!(ref_obj.uri, "file:///test/{name}.txt");

        let json_value = serde_json::to_value(&ref_obj).unwrap();
        assert_eq!(json_value["type"], "ref/resource");
        assert_eq!(json_value["uri"], "file:///test/{name}.txt");
    }

    #[test]
    fn test_prompt_reference() {
        let ref_obj = PromptReference::new("test_prompt").with_title("A Test Prompt");

        assert_eq!(ref_obj.ref_type, "ref/prompt");
        assert_eq!(ref_obj.name, "test_prompt");
        assert_eq!(ref_obj.title, Some("A Test Prompt".to_string()));

        let json_value = serde_json::to_value(&ref_obj).unwrap();
        assert_eq!(json_value["type"], "ref/prompt");
        assert_eq!(json_value["name"], "test_prompt");
        assert_eq!(json_value["title"], "A Test Prompt");
    }

    #[test]
    fn test_completion_reference_union() {
        let resource_ref = CompletionReference::resource("file:///test.txt");
        let prompt_ref = CompletionReference::prompt("my_prompt");

        // Test serialization
        let resource_json = serde_json::to_value(&resource_ref).unwrap();
        let prompt_json = serde_json::to_value(&prompt_ref).unwrap();

        assert_eq!(resource_json["type"], "ref/resource");
        assert_eq!(resource_json["uri"], "file:///test.txt");

        assert_eq!(prompt_json["type"], "ref/prompt");
        assert_eq!(prompt_json["name"], "my_prompt");
    }

    #[test]
    fn test_complete_request_matches_typescript_spec() {
        // Test CompleteRequest matches: { method: string, params: { ref: ..., argument: ..., context?: ..., _meta?: ... } }
        let meta = crate::meta::RequestMetaObject::new(
            "2026-07-28",
            crate::initialize::Implementation::new("test-client", "1.0.0"),
            crate::initialize::ClientCapabilities::default(),
        )
        .with_extra("requestId", json!("req-123"));

        let mut context_args = HashMap::new();
        context_args.insert("userId".to_string(), "123".to_string());

        let context = CompletionContext::new().with_arguments(context_args);

        let request = CompleteRequest::new(
            CompletionReference::prompt("test_prompt"),
            CompleteArgument::new("arg_name", "partial_value"),
            meta,
        )
        .with_context(context);

        let json_value = serde_json::to_value(&request).unwrap();

        assert_eq!(json_value["method"], "completion/complete");
        assert!(json_value["params"].is_object());
        assert!(json_value["params"]["ref"].is_object());
        assert_eq!(json_value["params"]["ref"]["type"], "ref/prompt");
        assert_eq!(json_value["params"]["ref"]["name"], "test_prompt");
        assert_eq!(json_value["params"]["argument"]["name"], "arg_name");
        assert_eq!(json_value["params"]["argument"]["value"], "partial_value");
        assert!(json_value["params"]["context"].is_object());
        assert_eq!(
            json_value["params"]["context"]["arguments"]["userId"],
            "123"
        );
        assert_eq!(json_value["params"]["_meta"]["requestId"], "req-123");
    }

    #[test]
    fn test_complete_result_matches_typescript_spec() {
        // Test CompleteResult matches: { completion: { values: string[], total?: number, hasMore?: boolean }, _meta?: ... }
        let mut meta = HashMap::new();
        meta.insert("executionTime".to_string(), json!(42));

        let completion = CompletionResult::new(vec![
            "option1".to_string(),
            "option2".to_string(),
            "option3".to_string(),
        ])
        .with_total(100.0)
        .with_has_more(true);

        let result = CompleteResult::new(completion).with_meta(meta);

        let json_value = serde_json::to_value(&result).unwrap();

        assert!(json_value["completion"].is_object());
        assert!(json_value["completion"]["values"].is_array());
        assert_eq!(
            json_value["completion"]["values"].as_array().unwrap().len(),
            3
        );
        assert_eq!(json_value["completion"]["values"][0], "option1");
        assert_eq!(json_value["completion"]["total"], 100.0);
        assert_eq!(json_value["completion"]["hasMore"], true);
        assert_eq!(json_value["_meta"]["executionTime"], 42);
    }

    #[test]
    fn test_serialization() {
        let meta = crate::meta::RequestMetaObject::new(
            "2026-07-28",
            crate::initialize::Implementation::new("test-client", "1.0.0"),
            crate::initialize::ClientCapabilities::default(),
        );
        let request = CompleteRequest::new(
            CompletionReference::resource("file:///test/{id}.txt"),
            CompleteArgument::new("id", "test"),
            meta,
        );

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("completion/complete"));
        assert!(json.contains("ref/resource"));
        assert!(json.contains("file:///test/{id}.txt"));

        let parsed: CompleteRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.method, "completion/complete");
    }

    #[test]
    fn completion_reference_round_trips_resource_and_prompt() {
        let resource = CompletionReference::resource("file:///a/{id}.txt");
        let v = serde_json::to_value(&resource).unwrap();
        let parsed: CompletionReference = serde_json::from_value(v).unwrap();
        match parsed {
            CompletionReference::ResourceTemplate(r) => {
                assert_eq!(r.ref_type, "ref/resource");
                assert_eq!(r.uri, "file:///a/{id}.txt");
            }
            CompletionReference::Prompt(_) => panic!("expected ResourceTemplate variant"),
        }

        let prompt = CompletionReference::Prompt(PromptReference::new("greet").with_title("Greet"));
        let v = serde_json::to_value(&prompt).unwrap();
        let parsed: CompletionReference = serde_json::from_value(v).unwrap();
        match parsed {
            CompletionReference::Prompt(p) => {
                assert_eq!(p.ref_type, "ref/prompt");
                assert_eq!(p.name, "greet");
                assert_eq!(p.title, Some("Greet".to_string()));
            }
            CompletionReference::ResourceTemplate(_) => panic!("expected Prompt variant"),
        }
    }

    #[test]
    fn completion_reference_rejects_unknown_type() {
        let wire = json!({"type": "ref/bogus", "uri": "x"});
        let result: Result<CompletionReference, _> = serde_json::from_value(wire);
        assert!(
            result.is_err(),
            "an unrecognized `type` discriminator must be rejected, not silently matched \
             structurally against ResourceTemplateReference"
        );
    }

    #[test]
    fn completion_reference_rejects_missing_type() {
        let wire = json!({"uri": "x"});
        let result: Result<CompletionReference, _> = serde_json::from_value(wire);
        assert!(
            result.is_err(),
            "a missing `type` discriminator must be rejected"
        );
    }

    #[test]
    fn completion_result_new_does_not_truncate() {
        // Truncation is the server dispatch layer's job (`CompletionHandler` in
        // turul-mcp-server), which needs the untruncated length to populate
        // `total` before capping `values` — this constructor must not
        // pre-empt that by truncating first.
        let values: Vec<String> = (0..150).map(|i| format!("v{i}")).collect();
        let result = CompletionResult::new(values);
        assert_eq!(result.values.len(), 150);
        assert_eq!(result.has_more, None);
    }
}
