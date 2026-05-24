//! MCP Elicitation Protocol Types
//!
//! This module defines the types used for MCP elicitation functionality,
//! which enables structured user input collection via restricted primitive schemas.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// StringSchema (per MCP 2025-11-25 spec)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StringSchema {
    #[serde(rename = "type")]
    pub schema_type: String, // "string"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_length: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_length: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<StringFormat>,
}

/// NumberSchema (per MCP 2025-11-25 spec) - handles both "number" and "integer"
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NumberSchema {
    #[serde(rename = "type")]
    pub schema_type: String, // "number" or "integer"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum: Option<f64>,
}

/// BooleanSchema (per MCP spec)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BooleanSchema {
    #[serde(rename = "type")]
    pub schema_type: String, // "boolean"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<bool>,
}

/// EnumSchema (legacy 2025-11-25 shape, kept for backward compat).
///
/// New code should use [`SingleSelectEnumSchema`] (untitled or titled) or
/// [`MultiSelectEnumSchema`] instead — they're spec-pure JSON Schema 2020-12
/// shapes whereas `enumNames` is explicitly flagged non-standard in the spec.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnumSchema {
    #[serde(rename = "type")]
    pub schema_type: String, // "string"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "enum")]
    pub enum_values: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_names: Option<Vec<String>>, // Display names for enum values
}

// --- DRAFT-2026-v1 enum schema variants -----------------------------------

/// Single-select enum with no display titles.
///
/// Wire shape: `{type:"string", title?, description?, enum: string[], default?}`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UntitledSingleSelectEnumSchema {
    #[serde(rename = "type")]
    pub schema_type: String, // "string"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "enum")]
    pub enum_values: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
}

impl UntitledSingleSelectEnumSchema {
    pub fn new(enum_values: Vec<String>) -> Self {
        Self {
            schema_type: "string".to_string(),
            title: None,
            description: None,
            enum_values,
            default: None,
        }
    }
}

/// Single-select enum with per-option display titles via `oneOf: [{const, title}]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TitledSingleSelectEnumSchema {
    #[serde(rename = "type")]
    pub schema_type: String, // "string"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Each option is `{const: <value>, title: <display>}`.
    #[serde(rename = "oneOf")]
    pub one_of: Vec<TitledEnumOption>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
}

/// `{const: string, title: string}` shape used inside titled enum schemas.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TitledEnumOption {
    #[serde(rename = "const")]
    pub const_value: String,
    pub title: String,
}

impl TitledEnumOption {
    pub fn new(const_value: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            const_value: const_value.into(),
            title: title.into(),
        }
    }
}

impl TitledSingleSelectEnumSchema {
    pub fn new(one_of: Vec<TitledEnumOption>) -> Self {
        Self {
            schema_type: "string".to_string(),
            title: None,
            description: None,
            one_of,
            default: None,
        }
    }
}

/// Single-select union: `UntitledSingleSelectEnumSchema |
/// TitledSingleSelectEnumSchema`. Untagged — discriminated by presence of
/// `enum` vs `oneOf` at the wire level.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SingleSelectEnumSchema {
    Untitled(UntitledSingleSelectEnumSchema),
    Titled(TitledSingleSelectEnumSchema),
}

/// Multi-select enum with no per-option titles.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UntitledMultiSelectEnumSchema {
    #[serde(rename = "type")]
    pub schema_type: String, // "array"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_items: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_items: Option<u32>,
    /// `{type: "string", enum: string[]}`.
    pub items: UntitledMultiSelectItems,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UntitledMultiSelectItems {
    #[serde(rename = "type")]
    pub schema_type: String, // "string"
    #[serde(rename = "enum")]
    pub enum_values: Vec<String>,
}

impl UntitledMultiSelectEnumSchema {
    pub fn new(enum_values: Vec<String>) -> Self {
        Self {
            schema_type: "array".to_string(),
            title: None,
            description: None,
            min_items: None,
            max_items: None,
            items: UntitledMultiSelectItems {
                schema_type: "string".to_string(),
                enum_values,
            },
            default: None,
        }
    }
}

/// Multi-select enum with per-option titles via `items.anyOf`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TitledMultiSelectEnumSchema {
    #[serde(rename = "type")]
    pub schema_type: String, // "array"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_items: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_items: Option<u32>,
    /// `{anyOf: [{const, title}]}`.
    pub items: TitledMultiSelectItems,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TitledMultiSelectItems {
    #[serde(rename = "anyOf")]
    pub any_of: Vec<TitledEnumOption>,
}

impl TitledMultiSelectEnumSchema {
    pub fn new(any_of: Vec<TitledEnumOption>) -> Self {
        Self {
            schema_type: "array".to_string(),
            title: None,
            description: None,
            min_items: None,
            max_items: None,
            items: TitledMultiSelectItems { any_of },
            default: None,
        }
    }
}

/// Multi-select union.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MultiSelectEnumSchema {
    Untitled(UntitledMultiSelectEnumSchema),
    Titled(TitledMultiSelectEnumSchema),
}

/// Restricted schema definitions that only allow primitive types
/// without nested objects or arrays (per MCP spec).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PrimitiveSchemaDefinition {
    String(StringSchema),
    Number(NumberSchema),
    Boolean(BooleanSchema),
    Enum(EnumSchema),
}

/// String format constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StringFormat {
    Email,
    Uri,
    Date,
    #[serde(rename = "date-time")]
    DateTime,
}

/// Restricted schema for elicitation (only primitive types, no nesting) - per MCP spec
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElicitationSchema {
    #[serde(rename = "type")]
    pub schema_type: String, // Always "object"
    pub properties: HashMap<String, PrimitiveSchemaDefinition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
}

/// Form-mode elicitation params. Per schema this interface does NOT extend
/// `RequestParams` — no `_meta` field.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElicitRequestFormParams {
    /// Elicitation mode marker. Schema: `mode?: "form"` (optional; default is form).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<FormModeMarker>,

    /// The message to present to the user.
    pub message: String,

    /// A restricted subset of JSON Schema — only top-level primitive properties.
    pub requested_schema: ElicitationSchema,
}

/// URL-mode elicitation params. Per schema this interface does NOT extend
/// `RequestParams` — no `_meta` field.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElicitRequestURLParams {
    /// Elicitation mode marker. Schema: `mode: "url"` (required).
    pub mode: UrlModeMarker,

    /// The message to present to the user explaining why the interaction is needed.
    pub message: String,

    /// The ID of the elicitation, unique within the context of the server.
    /// The client MUST treat this as an opaque value.
    pub elicitation_id: String,

    /// The URL the user should navigate to.
    pub url: String,
}

/// Fixed-value mode discriminator for [`ElicitRequestFormParams`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FormModeMarker {
    Form,
}

/// Fixed-value mode discriminator for [`ElicitRequestURLParams`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum UrlModeMarker {
    Url,
}

/// Discriminated union of elicitation params:
/// `ElicitRequestParams = ElicitRequestFormParams | ElicitRequestURLParams`.
///
/// Wire discrimination: `mode: "url"` selects URL; otherwise (absent or `"form"`)
/// selects Form. URL is tried first because its `mode`/`elicitationId`/`url`
/// fields uniquely identify it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ElicitRequestParams {
    /// URL mode — opens a URL in the client for out-of-band user interaction.
    Url(ElicitRequestURLParams),
    /// Form mode — collects structured input via the client's UI.
    Form(ElicitRequestFormParams),
}

/// `elicitation/create` request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElicitRequest {
    /// Method name (always "elicitation/create").
    pub method: String,
    /// Request parameters — form or URL mode.
    pub params: ElicitRequestParams,
}

impl ElicitRequest {
    /// Construct a form-mode elicitation request.
    pub fn new_form(message: impl Into<String>, requested_schema: ElicitationSchema) -> Self {
        Self {
            method: "elicitation/create".to_string(),
            params: ElicitRequestParams::Form(ElicitRequestFormParams {
                mode: None,
                message: message.into(),
                requested_schema,
            }),
        }
    }

    /// Construct a URL-mode elicitation request.
    pub fn new_url(
        message: impl Into<String>,
        elicitation_id: impl Into<String>,
        url: impl Into<String>,
    ) -> Self {
        Self {
            method: "elicitation/create".to_string(),
            params: ElicitRequestParams::Url(ElicitRequestURLParams {
                mode: UrlModeMarker::Url,
                message: message.into(),
                elicitation_id: elicitation_id.into(),
                url: url.into(),
            }),
        }
    }

    /// Attach a fully-constructed params struct.
    pub fn with_params(mut self, params: ElicitRequestParams) -> Self {
        self.params = params;
        self
    }
}

impl ElicitRequestFormParams {
    pub fn new(message: impl Into<String>, requested_schema: ElicitationSchema) -> Self {
        Self {
            mode: None,
            message: message.into(),
            requested_schema,
        }
    }
}

impl ElicitRequestURLParams {
    pub fn new(
        message: impl Into<String>,
        elicitation_id: impl Into<String>,
        url: impl Into<String>,
    ) -> Self {
        Self {
            mode: UrlModeMarker::Url,
            message: message.into(),
            elicitation_id: elicitation_id.into(),
            url: url.into(),
        }
    }
}

// Trait implementations for protocol compliance.
use crate::traits::*;

impl Params for ElicitRequestFormParams {}
impl Params for ElicitRequestURLParams {}

// `HasMetaParam` intentionally NOT implemented — per schema neither
// ElicitRequestFormParams nor ElicitRequestURLParams extends RequestParams,
// so they have no `_meta` field on the wire.

impl HasMethod for ElicitRequest {
    fn method(&self) -> &str {
        &self.method
    }
}

// HasParams requires `&dyn Params` — the params field is an enum, not a Params impl.
// Consumers should match on `self.params` directly rather than going through HasParams.

// `ElicitResult` does not implement `HasMeta`, `HasData`, or `RpcResult` —
// the schema defines it as `{ action, content? }` only.

impl Default for ElicitationSchema {
    fn default() -> Self {
        Self::new()
    }
}

impl ElicitationSchema {
    pub fn new() -> Self {
        Self {
            schema_type: "object".to_string(),
            properties: HashMap::new(),
            required: None,
        }
    }

    pub fn with_property(
        mut self,
        name: impl Into<String>,
        schema: PrimitiveSchemaDefinition,
    ) -> Self {
        self.properties.insert(name.into(), schema);
        self
    }

    pub fn with_required(mut self, required: Vec<String>) -> Self {
        self.required = Some(required);
        self
    }
}

/// User action in response to elicitation
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ElicitAction {
    /// User submitted the form/confirmed the action
    Accept,
    /// User explicitly declined the action
    Decline,
    /// User dismissed without making an explicit choice
    Cancel,
}

/// The client's response to an elicitation request:
/// `{ action: "accept"|"decline"|"cancel", content?: {[k]: string|number|boolean|string[]} }`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElicitResult {
    /// The user action in response to the elicitation.
    pub action: ElicitAction,
    /// The submitted form data, only present when action is `"accept"` and mode was `"form"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<HashMap<String, Value>>,
}

impl ElicitResult {
    pub fn accept(content: HashMap<String, Value>) -> Self {
        Self {
            action: ElicitAction::Accept,
            content: Some(content),
        }
    }

    pub fn decline() -> Self {
        Self {
            action: ElicitAction::Decline,
            content: None,
        }
    }

    pub fn cancel() -> Self {
        Self {
            action: ElicitAction::Cancel,
            content: None,
        }
    }
}

// Convenience constructors for schema types
impl Default for StringSchema {
    fn default() -> Self {
        Self::new()
    }
}

impl StringSchema {
    pub fn new() -> Self {
        Self {
            schema_type: "string".to_string(),
            title: None,
            description: None,
            default: None,
            min_length: None,
            max_length: None,
            format: None,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_default(mut self, default: impl Into<String>) -> Self {
        self.default = Some(default.into());
        self
    }

    /// Create a URL string schema with format: "uri"
    pub fn url() -> Self {
        Self {
            schema_type: "string".to_string(),
            title: None,
            description: None,
            default: None,
            min_length: None,
            max_length: None,
            format: Some(StringFormat::Uri),
        }
    }
}

impl Default for NumberSchema {
    fn default() -> Self {
        Self::new()
    }
}

impl NumberSchema {
    pub fn new() -> Self {
        Self {
            schema_type: "number".to_string(),
            title: None,
            description: None,
            default: None,
            minimum: None,
            maximum: None,
        }
    }

    pub fn integer() -> Self {
        Self {
            schema_type: "integer".to_string(),
            title: None,
            description: None,
            default: None,
            minimum: None,
            maximum: None,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_default(mut self, default: f64) -> Self {
        self.default = Some(default);
        self
    }
}

impl Default for BooleanSchema {
    fn default() -> Self {
        Self::new()
    }
}

impl BooleanSchema {
    pub fn new() -> Self {
        Self {
            schema_type: "boolean".to_string(),
            title: None,
            description: None,
            default: None,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

impl EnumSchema {
    pub fn new(enum_values: Vec<String>) -> Self {
        Self {
            schema_type: "string".to_string(),
            title: None,
            description: None,
            enum_values,
            enum_names: None,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_enum_names(mut self, enum_names: Vec<String>) -> Self {
        self.enum_names = Some(enum_names);
        self
    }
}

// Convenience constructors for PrimitiveSchemaDefinition
impl PrimitiveSchemaDefinition {
    pub fn string() -> Self {
        Self::String(StringSchema::new())
    }

    pub fn string_with_description(description: impl Into<String>) -> Self {
        Self::String(StringSchema::new().with_description(description))
    }

    /// Create a URL string schema with format: "uri"
    pub fn url() -> Self {
        Self::String(StringSchema::url())
    }

    /// Create a URL string schema with description and format: "uri"
    pub fn url_with_description(description: impl Into<String>) -> Self {
        Self::String(StringSchema::url().with_description(description))
    }

    pub fn number() -> Self {
        Self::Number(NumberSchema::new())
    }

    pub fn integer() -> Self {
        Self::Number(NumberSchema::integer())
    }

    pub fn boolean() -> Self {
        Self::Boolean(BooleanSchema::new())
    }

    pub fn enum_values(values: Vec<String>) -> Self {
        Self::Enum(EnumSchema::new(values))
    }
}

/// Builder for creating common elicitation patterns
pub struct ElicitationBuilder;

impl ElicitationBuilder {
    /// Create a simple text input elicitation (MCP spec compliant)
    pub fn text_input(
        message: impl Into<String>,
        field_name: impl Into<String>,
        field_description: impl Into<String>,
    ) -> ElicitRequest {
        let field_name = field_name.into();
        let schema = ElicitationSchema::new()
            .with_property(
                field_name.clone(),
                PrimitiveSchemaDefinition::string_with_description(field_description),
            )
            .with_required(vec![field_name]);

        ElicitRequest::new_form(message, schema)
    }

    /// Create a number input elicitation (MCP spec compliant)
    pub fn number_input(
        message: impl Into<String>,
        field_name: impl Into<String>,
        field_description: impl Into<String>,
        min: Option<f64>,
        max: Option<f64>,
    ) -> ElicitRequest {
        let field_name = field_name.into();
        let mut number_schema = NumberSchema::new().with_description(field_description);
        number_schema.minimum = min;
        number_schema.maximum = max;
        let number_schema = PrimitiveSchemaDefinition::Number(number_schema);

        let schema = ElicitationSchema::new()
            .with_property(field_name.clone(), number_schema)
            .with_required(vec![field_name]);

        ElicitRequest::new_form(message, schema)
    }

    /// Create a URL input elicitation with format: "uri" (MCP 2025-11-25)
    pub fn url_input(
        message: impl Into<String>,
        field_name: impl Into<String>,
        field_description: impl Into<String>,
    ) -> ElicitRequest {
        let field_name = field_name.into();
        let schema = ElicitationSchema::new()
            .with_property(
                field_name.clone(),
                PrimitiveSchemaDefinition::url_with_description(field_description),
            )
            .with_required(vec![field_name]);

        ElicitRequest::new_form(message, schema)
    }

    /// Create a boolean confirmation elicitation (MCP spec compliant)
    pub fn confirm(message: impl Into<String>) -> ElicitRequest {
        let schema = ElicitationSchema::new()
            .with_property(
                "confirmed".to_string(),
                PrimitiveSchemaDefinition::boolean(),
            )
            .with_required(vec!["confirmed".to_string()]);

        ElicitRequest::new_form(message, schema)
    }
}

// ===========================================
// === Fine-Grained Elicitation Traits ===
// ===========================================

/// Trait for elicitation metadata (message, title)
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_primitive_schema_creation() {
        let string_schema = PrimitiveSchemaDefinition::string_with_description("Enter your name");
        let number_schema = PrimitiveSchemaDefinition::number();
        let boolean_schema = PrimitiveSchemaDefinition::boolean();

        assert!(matches!(
            string_schema,
            PrimitiveSchemaDefinition::String { .. }
        ));
        assert!(matches!(
            number_schema,
            PrimitiveSchemaDefinition::Number { .. }
        ));
        assert!(matches!(
            boolean_schema,
            PrimitiveSchemaDefinition::Boolean { .. }
        ));
    }

    #[test]
    fn test_elicitation_schema() {
        let schema = ElicitationSchema::new()
            .with_property(
                "name".to_string(),
                PrimitiveSchemaDefinition::string_with_description("Your name"),
            )
            .with_property("age".to_string(), PrimitiveSchemaDefinition::integer())
            .with_required(vec!["name".to_string()]);

        assert_eq!(schema.schema_type, "object");
        assert_eq!(schema.properties.len(), 2);
        assert_eq!(schema.required, Some(vec!["name".to_string()]));
    }

    #[test]
    fn test_elicit_create_request() {
        let schema = ElicitationSchema::new().with_property(
            "username".to_string(),
            PrimitiveSchemaDefinition::string_with_description("Username"),
        );

        let request = ElicitRequest::new_form("Please enter your username", schema);

        assert_eq!(request.method, "elicitation/create");
        match &request.params {
            ElicitRequestParams::Form(form) => {
                assert_eq!(form.message, "Please enter your username");
            }
            ElicitRequestParams::Url(_) => panic!("expected form variant"),
        }
    }

    #[test]
    fn test_elicit_result() {
        let mut content = HashMap::new();
        content.insert("name".to_string(), json!("John"));

        let accept_result = ElicitResult::accept(content);
        let decline_result = ElicitResult::decline();
        let cancel_result = ElicitResult::cancel();

        assert!(matches!(accept_result.action, ElicitAction::Accept));
        assert!(accept_result.content.is_some());

        assert!(matches!(decline_result.action, ElicitAction::Decline));
        assert!(decline_result.content.is_none());

        assert!(matches!(cancel_result.action, ElicitAction::Cancel));
        assert!(cancel_result.content.is_none());
    }

    #[test]
    fn test_elicitation_builder() {
        let text_request =
            ElicitationBuilder::text_input("Enter your name", "name", "Your full name");

        let confirm_request = ElicitationBuilder::confirm("Do you agree?");

        assert_eq!(text_request.method, "elicitation/create");
        if let ElicitRequestParams::Form(form) = &text_request.params {
            assert!(form.requested_schema.properties.contains_key("name"));
        } else {
            panic!("text builder must produce Form variant");
        }

        assert_eq!(confirm_request.method, "elicitation/create");
        if let ElicitRequestParams::Form(form) = &confirm_request.params {
            assert!(form.requested_schema.properties.contains_key("confirmed"));
        } else {
            panic!("confirm builder must produce Form variant");
        }
    }

    #[test]
    fn test_serialization() {
        let schema = ElicitationSchema::new()
            .with_property("test".to_string(), PrimitiveSchemaDefinition::string());
        let request = ElicitRequest::new_form("Test message", schema);

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("elicitation/create"));
        assert!(json.contains("Test message"));

        let parsed: ElicitRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.method, "elicitation/create");
        match parsed.params {
            ElicitRequestParams::Form(form) => assert_eq!(form.message, "Test message"),
            ElicitRequestParams::Url(_) => panic!("expected form variant"),
        }
    }

    #[test]
    fn test_elicit_request_matches_typescript_spec() {
        // Per schema: ElicitRequestFormParams = { mode?, message, requestedSchema }.
        // Does NOT extend RequestParams — no `_meta` field.
        let schema = ElicitationSchema::new()
            .with_property(
                "name".to_string(),
                PrimitiveSchemaDefinition::string_with_description("Your name"),
            )
            .with_property("age".to_string(), PrimitiveSchemaDefinition::integer())
            .with_required(vec!["name".to_string()]);

        let request = ElicitRequest::new_form("Please provide your details", schema);

        let json_value = serde_json::to_value(&request).unwrap();

        assert_eq!(json_value["method"], "elicitation/create");
        assert!(json_value["params"].is_object());
        assert_eq!(
            json_value["params"]["message"],
            "Please provide your details"
        );
        assert!(json_value["params"]["requestedSchema"].is_object());
        assert_eq!(json_value["params"]["requestedSchema"]["type"], "object");
        assert!(json_value["params"]["requestedSchema"]["properties"].is_object());
        // Spec compliance: ElicitRequestFormParams does NOT carry `_meta`.
        assert!(
            !json_value["params"]
                .as_object()
                .unwrap()
                .contains_key("_meta"),
            "ElicitRequestFormParams MUST NOT serialize a _meta field per schema"
        );
    }

    #[test]
    fn test_elicit_result_matches_typescript_spec() {
        // `ElicitResult { action, content? }`. No _meta, no extends Result.
        let mut content = HashMap::new();
        content.insert("name".to_string(), json!("John Doe"));
        content.insert("age".to_string(), json!(30));

        let result = ElicitResult::accept(content.clone());
        let json_value = serde_json::to_value(&result).unwrap();

        assert_eq!(json_value["action"], "accept");
        assert!(json_value["content"].is_object());
        assert_eq!(json_value["content"]["name"], "John Doe");
        assert_eq!(json_value["content"]["age"], 30);
        let obj = json_value.as_object().unwrap();
        assert!(!obj.contains_key("_meta"));
        assert!(!obj.contains_key("resultType"));

        let decline_result = ElicitResult::decline();
        let decline_json = serde_json::to_value(&decline_result).unwrap();
        assert_eq!(decline_json["action"], "decline");
        assert!(!decline_json.as_object().unwrap().contains_key("content"));
    }

    #[test]
    fn test_primitive_schema_definitions_match_typescript() {
        // Test StringSchema
        let string_schema = PrimitiveSchemaDefinition::string_with_description("Enter text");
        let string_json = serde_json::to_value(&string_schema).unwrap();
        assert_eq!(string_json["type"], "string");
        assert_eq!(string_json["description"], "Enter text");

        // Test NumberSchema
        let number_schema = PrimitiveSchemaDefinition::number();
        let number_json = serde_json::to_value(&number_schema).unwrap();
        assert_eq!(number_json["type"], "number");

        // Test IntegerSchema
        let integer_schema = PrimitiveSchemaDefinition::integer();
        let integer_json = serde_json::to_value(&integer_schema).unwrap();
        assert_eq!(integer_json["type"], "integer");

        // Test BooleanSchema
        let boolean_schema = PrimitiveSchemaDefinition::boolean();
        let boolean_json = serde_json::to_value(&boolean_schema).unwrap();
        assert_eq!(boolean_json["type"], "boolean");

        // Test EnumSchema
        let enum_schema = PrimitiveSchemaDefinition::enum_values(vec![
            "red".to_string(),
            "green".to_string(),
            "blue".to_string(),
        ]);
        let enum_json = serde_json::to_value(&enum_schema).unwrap();
        assert_eq!(enum_json["type"], "string");
        assert!(enum_json["enum"].is_array());
        assert_eq!(enum_json["enum"].as_array().unwrap().len(), 3);
    }
}
