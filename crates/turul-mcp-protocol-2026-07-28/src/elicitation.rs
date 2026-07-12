//! MCP Elicitation Protocol Types
//!
//! This module defines the types used for MCP elicitation functionality,
//! which enables structured user input collection via restricted primitive schemas.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// `StringSchema` — primitive string schema for elicitation requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
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

/// `NumberSchema` — primitive numeric schema; `schema_type` is `"number"` or `"integer"`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
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
#[serde(rename_all = "camelCase", deny_unknown_fields)]
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

/// `LegacyTitledEnumSchema` — the pre-2026 enum shape with the non-standard
/// `enumNames`. Schema: "Use TitledSingleSelectEnumSchema instead. This
/// interface will be removed in a future version."
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyTitledEnumSchema {
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
}

/// `EnumSchema` — the schema's enum union:
/// `SingleSelectEnumSchema | MultiSelectEnumSchema | LegacyTitledEnumSchema`.
///
/// Untagged: the single-select variants are tried first (the untitled one
/// rejects unknown fields, so a legacy payload carrying `enumNames` falls
/// through to [`LegacyTitledEnumSchema`] instead of silently dropping it).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EnumSchema {
    SingleSelect(SingleSelectEnumSchema),
    MultiSelect(MultiSelectEnumSchema),
    LegacyTitled(LegacyTitledEnumSchema),
}

// --- DRAFT-2026-v1 enum schema variants -----------------------------------

/// Single-select enum with no display titles.
///
/// Wire shape: `{type:"string", title?, description?, enum: string[], default?}`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
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
#[serde(rename_all = "camelCase", deny_unknown_fields)]
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
///
/// Schema fixes a literal `type` discriminator per interface (`"string"`,
/// `"number" | "integer"`, `"boolean"`) — `#[serde(untagged)]` alone tries
/// variants in declaration order and can't tell them apart when a payload
/// carries only the discriminator (e.g. `{"type":"integer"}` structurally
/// matches [`StringSchema`] too, since `schema_type` is a bare `String`).
/// `Deserialize` is hand-written to dispatch on `type` (and `enum`
/// presence, to route to [`EnumSchema`]) before ever trying a variant.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum PrimitiveSchemaDefinition {
    String(StringSchema),
    Number(NumberSchema),
    Boolean(BooleanSchema),
    Enum(EnumSchema),
}

impl<'de> Deserialize<'de> for PrimitiveSchemaDefinition {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        let type_str = value.get("type").and_then(Value::as_str);
        // Enum-schema variants are structurally distinguished from
        // StringSchema/BooleanSchema by carrying `enum` (untitled) or
        // `oneOf` (titled, per-option display titles) — StringSchema and
        // BooleanSchema carry neither.
        let is_enum_shaped = value.get("enum").is_some() || value.get("oneOf").is_some();

        let variant = match (type_str, is_enum_shaped) {
            (Some("string"), false) => "String",
            (Some("number") | Some("integer"), _) => "Number",
            (Some("boolean"), false) => "Boolean",
            // `enum`/`oneOf`-bearing string schemas and multi-select `array`
            // schemas all route through the untagged `EnumSchema` union,
            // which discriminates structurally among its own variants.
            (Some("string"), true) | (Some("array"), _) => "Enum",
            _ => {
                return Err(serde::de::Error::custom(format!(
                    "PrimitiveSchemaDefinition: unrecognized or missing `type` \
                     discriminator in {value}"
                )));
            }
        };

        match variant {
            "String" => serde_json::from_value(value)
                .map(PrimitiveSchemaDefinition::String)
                .map_err(serde::de::Error::custom),
            "Number" => serde_json::from_value(value)
                .map(PrimitiveSchemaDefinition::Number)
                .map_err(serde::de::Error::custom),
            "Boolean" => serde_json::from_value(value)
                .map(PrimitiveSchemaDefinition::Boolean)
                .map_err(serde::de::Error::custom),
            _ => serde_json::from_value(value)
                .map(PrimitiveSchemaDefinition::Enum)
                .map_err(serde::de::Error::custom),
        }
    }
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
    /// JSON Schema dialect URI. DRAFT-2026-v1 adopts JSON Schema 2020-12;
    /// clients use this to declare which dialect validated the schema.
    #[serde(rename = "$schema", skip_serializing_if = "Option::is_none")]
    pub schema_dialect: Option<String>,
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
/// selects Form. URL is tried first because its `mode`/`url` fields uniquely
/// identify it.
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
    pub fn new_url(message: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            method: "elicitation/create".to_string(),
            params: ElicitRequestParams::Url(ElicitRequestURLParams {
                mode: UrlModeMarker::Url,
                message: message.into(),
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
    pub fn new(message: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            mode: UrlModeMarker::Url,
            message: message.into(),
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
            schema_dialect: None,
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

    /// Set the JSON Schema dialect URI (e.g. `"https://json-schema.org/draft/2020-12/schema"`).
    pub fn with_schema_dialect(mut self, dialect: impl Into<String>) -> Self {
        self.schema_dialect = Some(dialect.into());
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

/// A single value inside [`ElicitResult`]'s `content` map.
///
/// Schema: `ElicitResult.content?: { [key: string]: string | number | boolean | string[] }`.
/// Untagged — each JSON primitive kind (string, number, bool, array) is
/// structurally distinct, so dispatch is unambiguous: unlike
/// [`PrimitiveSchemaDefinition`], no variant here can structurally accept
/// another variant's wire shape.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ElicitResultValue {
    String(String),
    Number(f64),
    Boolean(bool),
    StringArray(Vec<String>),
}

impl From<String> for ElicitResultValue {
    fn from(s: String) -> Self {
        Self::String(s)
    }
}

impl From<&str> for ElicitResultValue {
    fn from(s: &str) -> Self {
        Self::String(s.to_string())
    }
}

impl From<f64> for ElicitResultValue {
    fn from(n: f64) -> Self {
        Self::Number(n)
    }
}

impl From<bool> for ElicitResultValue {
    fn from(b: bool) -> Self {
        Self::Boolean(b)
    }
}

impl From<Vec<String>> for ElicitResultValue {
    fn from(values: Vec<String>) -> Self {
        Self::StringArray(values)
    }
}

impl ElicitResultValue {
    /// Return the string body if this is a string-form value; `None` otherwise.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    /// Return the numeric body if this is a number-form value; `None` otherwise.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Number(n) => Some(*n),
            _ => None,
        }
    }

    /// Return the boolean body if this is a boolean-form value; `None` otherwise.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Boolean(b) => Some(*b),
            _ => None,
        }
    }

    /// Return the string-array body if this is an array-form value; `None` otherwise.
    pub fn as_string_array(&self) -> Option<&[String]> {
        match self {
            Self::StringArray(values) => Some(values.as_slice()),
            _ => None,
        }
    }
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
    pub content: Option<HashMap<String, ElicitResultValue>>,
}

impl ElicitResult {
    pub fn accept(content: HashMap<String, ElicitResultValue>) -> Self {
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

impl LegacyTitledEnumSchema {
    pub fn new(enum_values: Vec<String>) -> Self {
        Self {
            schema_type: "string".to_string(),
            title: None,
            description: None,
            enum_values,
            enum_names: None,
            default: None,
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

    pub fn with_default(mut self, default: impl Into<String>) -> Self {
        self.default = Some(default.into());
        self
    }
}

impl EnumSchema {
    /// Spec-pure untitled single-select: `{type:"string", enum:[...]}`.
    pub fn new(enum_values: Vec<String>) -> Self {
        Self::SingleSelect(SingleSelectEnumSchema::Untitled(
            UntitledSingleSelectEnumSchema::new(enum_values),
        ))
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        let d = description.into();
        match &mut self {
            Self::SingleSelect(SingleSelectEnumSchema::Untitled(v)) => {
                v.description = Some(d);
            }
            Self::SingleSelect(SingleSelectEnumSchema::Titled(v)) => {
                v.description = Some(d);
            }
            Self::MultiSelect(MultiSelectEnumSchema::Untitled(v)) => {
                v.description = Some(d);
            }
            Self::MultiSelect(MultiSelectEnumSchema::Titled(v)) => {
                v.description = Some(d);
            }
            Self::LegacyTitled(v) => v.description = Some(d),
        }
        self
    }

    /// The set of permitted values, across all union shapes (titled variants
    /// carry them as `oneOf[].const`, multi-selects inside `items`).
    pub fn allowed_values(&self) -> Vec<String> {
        match self {
            Self::SingleSelect(SingleSelectEnumSchema::Untitled(v)) => v.enum_values.clone(),
            Self::SingleSelect(SingleSelectEnumSchema::Titled(v)) => {
                v.one_of.iter().map(|o| o.const_value.clone()).collect()
            }
            Self::MultiSelect(MultiSelectEnumSchema::Untitled(v)) => v.items.enum_values.clone(),
            Self::MultiSelect(MultiSelectEnumSchema::Titled(v)) => v
                .items
                .any_of
                .iter()
                .map(|o| o.const_value.clone())
                .collect(),
            Self::LegacyTitled(v) => v.enum_values.clone(),
        }
    }

    /// True for the multi-select shapes (the submitted value is an array).
    pub fn is_multi_select(&self) -> bool {
        matches!(self, Self::MultiSelect(_))
    }

    /// Per-value display names use the legacy `enumNames` wire shape; this
    /// converts the union to [`LegacyTitledEnumSchema`], preserving any
    /// existing single-select values. (New code wanting titles should build a
    /// [`TitledSingleSelectEnumSchema`] with `oneOf` const/title pairs.)
    pub fn with_enum_names(self, enum_names: Vec<String>) -> Self {
        let legacy = match self {
            Self::LegacyTitled(v) => v,
            Self::SingleSelect(SingleSelectEnumSchema::Untitled(v)) => LegacyTitledEnumSchema {
                schema_type: v.schema_type,
                title: v.title,
                description: v.description,
                enum_values: v.enum_values,
                enum_names: None,
                default: v.default,
            },
            other => {
                // Titled/multi-select shapes have no enumNames equivalent;
                // keep them unchanged rather than destroy their structure.
                return other;
            }
        };
        Self::LegacyTitled(legacy.with_enum_names(enum_names))
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

    /// Create a single-field URL input elicitation (format: `"uri"`).
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
#[allow(deprecated)]
mod tests {
    use super::*;

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
        assert!(schema.schema_dialect.is_none());
    }

    #[test]
    fn test_elicitation_schema_dialect_round_trips() {
        let schema = ElicitationSchema::new()
            .with_schema_dialect("https://json-schema.org/draft/2020-12/schema");
        let json = serde_json::to_value(&schema).unwrap();
        assert_eq!(
            json["$schema"],
            "https://json-schema.org/draft/2020-12/schema"
        );
        // Round-trip.
        let back: ElicitationSchema = serde_json::from_value(json).unwrap();
        assert_eq!(
            back.schema_dialect.as_deref(),
            Some("https://json-schema.org/draft/2020-12/schema")
        );
    }

    #[test]
    fn test_elicitation_schema_omits_dialect_when_none() {
        let schema = ElicitationSchema::new();
        let json = serde_json::to_value(&schema).unwrap();
        assert!(!json.as_object().unwrap().contains_key("$schema"));
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
        content.insert("name".to_string(), ElicitResultValue::from("John"));

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
        content.insert("name".to_string(), ElicitResultValue::from("John Doe"));
        content.insert("age".to_string(), ElicitResultValue::from(30.0));

        let result = ElicitResult::accept(content.clone());
        let json_value = serde_json::to_value(&result).unwrap();

        assert_eq!(json_value["action"], "accept");
        assert!(json_value["content"].is_object());
        assert_eq!(json_value["content"]["name"], "John Doe");
        assert_eq!(json_value["content"]["age"], 30.0);
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

#[cfg(test)]
mod enum_union_fidelity_tests {
    //! Round-trip fidelity through the untagged unions: enum constraints must
    //! survive deserialize → reserialize (they previously collapsed into
    //! `StringSchema`, silently dropping `enum`).
    use super::*;
    use serde_json::json;

    #[test]
    fn plain_string_enum_round_trips_via_primitive_union() {
        let wire = json!({"type": "string", "enum": ["red", "green", "blue"]});
        let parsed: PrimitiveSchemaDefinition = serde_json::from_value(wire.clone()).unwrap();
        assert!(
            matches!(
                &parsed,
                PrimitiveSchemaDefinition::Enum(EnumSchema::SingleSelect(
                    SingleSelectEnumSchema::Untitled(_)
                ))
            ),
            "a {{type, enum}} payload must parse as an untitled single-select, got: {parsed:?}"
        );
        assert_eq!(serde_json::to_value(&parsed).unwrap(), wire);
    }

    #[test]
    fn legacy_enum_names_payload_round_trips_losslessly() {
        let wire = json!({
            "type": "string",
            "enum": ["s", "m", "l"],
            "enumNames": ["Small", "Medium", "Large"],
            "default": "m"
        });
        let parsed: PrimitiveSchemaDefinition = serde_json::from_value(wire.clone()).unwrap();
        assert!(
            matches!(
                &parsed,
                PrimitiveSchemaDefinition::Enum(EnumSchema::LegacyTitled(_))
            ),
            "enumNames must select the legacy shape, got: {parsed:?}"
        );
        assert_eq!(
            serde_json::to_value(&parsed).unwrap(),
            wire,
            "enumNames and default must survive the round trip"
        );
    }

    #[test]
    fn titled_single_select_round_trips_via_elicitation_schema_properties() {
        let wire = json!({
            "type": "object",
            "properties": {
                "color": {
                    "type": "string",
                    "oneOf": [
                        {"const": "r", "title": "Red"},
                        {"const": "g", "title": "Green"}
                    ]
                }
            },
            "required": ["color"]
        });
        let parsed: ElicitationSchema = serde_json::from_value(wire.clone()).unwrap();
        assert!(
            matches!(
                parsed.properties.get("color"),
                Some(PrimitiveSchemaDefinition::Enum(EnumSchema::SingleSelect(
                    SingleSelectEnumSchema::Titled(_)
                )))
            ),
            "oneOf const/title pairs must parse as a titled single-select"
        );
        assert_eq!(serde_json::to_value(&parsed).unwrap(), wire);
    }

    #[test]
    fn untitled_multi_select_round_trips_via_primitive_union() {
        let wire = json!({
            "type": "array",
            "items": {"type": "string", "enum": ["a", "b"]}
        });
        let parsed: PrimitiveSchemaDefinition = serde_json::from_value(wire.clone()).unwrap();
        assert!(
            matches!(
                &parsed,
                PrimitiveSchemaDefinition::Enum(EnumSchema::MultiSelect(
                    MultiSelectEnumSchema::Untitled(_)
                ))
            ),
            "array-of-enum must parse as an untitled multi-select, got: {parsed:?}"
        );
        assert_eq!(serde_json::to_value(&parsed).unwrap(), wire);
    }

    #[test]
    fn allowed_values_spans_all_union_shapes() {
        let single = EnumSchema::new(vec!["x".into(), "y".into()]);
        assert_eq!(single.allowed_values(), vec!["x", "y"]);
        assert!(!single.is_multi_select());

        let legacy: EnumSchema = serde_json::from_value(json!({
            "type": "string", "enum": ["a"], "enumNames": ["A"]
        }))
        .unwrap();
        assert_eq!(legacy.allowed_values(), vec!["a"]);

        let multi: EnumSchema = serde_json::from_value(json!({
            "type": "array", "items": {"type": "string", "enum": ["p", "q"]}
        }))
        .unwrap();
        assert_eq!(multi.allowed_values(), vec!["p", "q"]);
        assert!(multi.is_multi_select());
    }

    #[test]
    fn plain_string_schema_still_parses_as_string() {
        // Control: no enum field → StringSchema, constraints intact.
        let wire = json!({"type": "string", "minLength": 2, "maxLength": 5});
        let parsed: PrimitiveSchemaDefinition = serde_json::from_value(wire.clone()).unwrap();
        assert!(matches!(&parsed, PrimitiveSchemaDefinition::String(_)));
        assert_eq!(serde_json::to_value(&parsed).unwrap(), wire);
    }

    #[test]
    fn number_constraints_survive_the_union() {
        // Pre-fix, {type:"number", minimum} could collapse into StringSchema
        // (unknown fields silently ignored). deny_unknown_fields forbids it.
        let wire = json!({"type": "number", "minimum": 1.5, "maximum": 9.0});
        let parsed: PrimitiveSchemaDefinition = serde_json::from_value(wire.clone()).unwrap();
        assert!(
            matches!(&parsed, PrimitiveSchemaDefinition::Number(_)),
            "numeric constraints must select NumberSchema, got: {parsed:?}"
        );
        assert_eq!(serde_json::to_value(&parsed).unwrap(), wire);
    }

    #[test]
    fn bare_integer_discriminator_selects_number_not_string() {
        // Regression: a bare {"type":"integer"} with no numeric-only field
        // (minimum/maximum/default) used to fall through to StringSchema
        // because untagged dispatch tried the String variant first and
        // schema_type: String accepts any string value.
        let wire = json!({"type": "integer"});
        let parsed: PrimitiveSchemaDefinition = serde_json::from_value(wire.clone()).unwrap();
        assert!(
            matches!(&parsed, PrimitiveSchemaDefinition::Number(_)),
            "bare integer discriminator must select NumberSchema, got: {parsed:?}"
        );
        assert_eq!(serde_json::to_value(&parsed).unwrap(), wire);
    }

    #[test]
    fn bare_boolean_discriminator_selects_boolean_not_string() {
        let wire = json!({"type": "boolean", "title": "x"});
        let parsed: PrimitiveSchemaDefinition = serde_json::from_value(wire.clone()).unwrap();
        assert!(
            matches!(&parsed, PrimitiveSchemaDefinition::Boolean(_)),
            "bare boolean discriminator must select BooleanSchema, got: {parsed:?}"
        );
        assert_eq!(serde_json::to_value(&parsed).unwrap(), wire);
    }

    #[test]
    fn unknown_type_discriminator_is_rejected() {
        // A `type` value that matches none of string/number/integer/boolean
        // and carries no `enum` must fail to deserialize, not silently
        // collapse into StringSchema.
        let wire = json!({"type": "banana"});
        let result: Result<PrimitiveSchemaDefinition, _> = serde_json::from_value(wire);
        assert!(result.is_err(), "unknown type discriminator must be rejected");
    }
}

#[cfg(test)]
mod elicit_result_value_tests {
    //! `ElicitResult.content` values: `string | number | boolean | string[]`.
    //! Each JSON primitive kind is structurally distinct, so the untagged
    //! union dispatches unambiguously — round-trip each arm plus a rejection
    //! test for a shape (object) the union doesn't accept.
    use super::*;
    use serde_json::json;

    #[test]
    fn string_value_round_trips() {
        let wire = json!("hello");
        let parsed: ElicitResultValue = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(parsed, ElicitResultValue::String("hello".to_string()));
        assert_eq!(serde_json::to_value(&parsed).unwrap(), wire);
    }

    #[test]
    fn bare_integer_value_round_trips_as_number() {
        // JSON has one numeric type; a bare integer must select the Number
        // arm, not fall through to String or Boolean.
        let wire = json!(42);
        let parsed: ElicitResultValue = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(parsed, ElicitResultValue::Number(42.0));
        assert_eq!(serde_json::to_value(&parsed).unwrap(), json!(42.0));
    }

    #[test]
    fn float_value_round_trips() {
        let wire = json!(3.5);
        let parsed: ElicitResultValue = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(parsed, ElicitResultValue::Number(3.5));
        assert_eq!(serde_json::to_value(&parsed).unwrap(), wire);
    }

    #[test]
    fn boolean_value_round_trips() {
        let wire = json!(true);
        let parsed: ElicitResultValue = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(parsed, ElicitResultValue::Boolean(true));
        assert_eq!(serde_json::to_value(&parsed).unwrap(), wire);
    }

    #[test]
    fn accessor_methods_only_return_some_for_their_own_variant() {
        let s = ElicitResultValue::String("hi".to_string());
        assert_eq!(s.as_str(), Some("hi"));
        assert_eq!(s.as_f64(), None);
        assert_eq!(s.as_bool(), None);
        assert!(s.as_string_array().is_none());

        let n = ElicitResultValue::Number(2.5);
        assert_eq!(n.as_f64(), Some(2.5));
        assert_eq!(n.as_str(), None);

        let b = ElicitResultValue::Boolean(true);
        assert_eq!(b.as_bool(), Some(true));
        assert_eq!(b.as_str(), None);

        let arr = ElicitResultValue::StringArray(vec!["a".to_string()]);
        assert_eq!(arr.as_string_array(), Some(&["a".to_string()][..]));
        assert_eq!(arr.as_str(), None);
    }

    #[test]
    fn string_array_value_round_trips() {
        let wire = json!(["red", "green", "blue"]);
        let parsed: ElicitResultValue = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(
            parsed,
            ElicitResultValue::StringArray(vec![
                "red".to_string(),
                "green".to_string(),
                "blue".to_string()
            ])
        );
        assert_eq!(serde_json::to_value(&parsed).unwrap(), wire);
    }

    #[test]
    fn object_value_is_rejected() {
        // Schema restricts content values to string | number | boolean |
        // string[] — an object shape must not silently coerce into any arm.
        let wire = json!({"nested": "value"});
        let result: Result<ElicitResultValue, _> = serde_json::from_value(wire);
        assert!(result.is_err(), "object value must be rejected");
    }

    #[test]
    fn elicit_result_content_map_round_trips_mixed_value_kinds() {
        let mut content = HashMap::new();
        content.insert("name".to_string(), ElicitResultValue::from("Ada"));
        content.insert("age".to_string(), ElicitResultValue::from(36.0));
        content.insert("subscribed".to_string(), ElicitResultValue::from(true));
        content.insert(
            "colors".to_string(),
            ElicitResultValue::from(vec!["red".to_string(), "blue".to_string()]),
        );

        let result = ElicitResult::accept(content);
        let v = serde_json::to_value(&result).unwrap();
        assert_eq!(v["content"]["name"], "Ada");
        assert_eq!(v["content"]["age"], 36.0);
        assert_eq!(v["content"]["subscribed"], true);
        assert_eq!(v["content"]["colors"], json!(["red", "blue"]));

        let parsed: ElicitResult = serde_json::from_value(v).unwrap();
        assert_eq!(
            parsed.content.unwrap().get("subscribed"),
            Some(&ElicitResultValue::Boolean(true))
        );
    }
}
