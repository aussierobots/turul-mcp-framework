//! MCP Elicitation Protocol Types
//!
//! This module defines the types used for MCP elicitation functionality,
//! which enables structured user input collection via restricted primitive schemas.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// StringSchema (per MCP spec)
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
    pub min_length: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_length: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<StringFormat>,
}

/// NumberSchema (per MCP spec) - handles both "number" and "integer"
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

/// EnumSchema (per MCP spec) - string type with enum values
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

/// Parameters for elicitation/create request (per MCP spec)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElicitCreateParams {
    /// The message to present to the user
    pub message: String,
    /// A restricted subset of JSON Schema - only top-level properties, no nesting
    pub requested_schema: ElicitationSchema,
    /// Meta information (optional _meta field inside params)
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<HashMap<String, Value>>,
}

/// Complete elicitation/create request (matches TypeScript ElicitRequest interface)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElicitCreateRequest {
    /// Method name (always "elicitation/create")
    pub method: String,
    /// Request parameters
    pub params: ElicitCreateParams,
}

impl ElicitCreateRequest {
    pub fn new(message: impl Into<String>, requested_schema: ElicitationSchema) -> Self {
        Self {
            method: "elicitation/create".to_string(),
            params: ElicitCreateParams {
                message: message.into(),
                requested_schema,
                meta: None,
            },
        }
    }

    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.params.meta = Some(meta);
        self
    }
}

impl ElicitCreateParams {
    pub fn new(message: impl Into<String>, requested_schema: ElicitationSchema) -> Self {
        Self {
            message: message.into(),
            requested_schema,
            meta: None,
        }
    }

    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.meta = Some(meta);
        self
    }
}

// Trait implementations for protocol compliance
use crate::traits::*;

impl Params for ElicitCreateParams {}

impl HasMetaParam for ElicitCreateParams {
    fn meta(&self) -> Option<&HashMap<String, Value>> {
        self.meta.as_ref()
    }
}

impl HasMethod for ElicitCreateRequest {
    fn method(&self) -> &str {
        &self.method
    }
}

impl HasParams for ElicitCreateRequest {
    fn params(&self) -> Option<&dyn Params> {
        Some(&self.params)
    }
}

impl HasData for ElicitResult {
    fn data(&self) -> HashMap<String, Value> {
        let mut data = HashMap::new();
        data.insert(
            "action".to_string(),
            serde_json::to_value(self.action).unwrap_or(Value::String("cancel".to_string())),
        );
        if let Some(ref content) = self.content {
            data.insert(
                "content".to_string(),
                serde_json::to_value(content).unwrap_or(Value::Null),
            );
        }
        data
    }
}

impl HasMeta for ElicitResult {
    fn meta(&self) -> Option<HashMap<String, Value>> {
        self.meta.clone()
    }
}

impl RpcResult for ElicitResult {}

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

/// The client's response to an elicitation request (per MCP spec)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElicitResult {
    /// The user action in response to the elicitation
    pub action: ElicitAction,
    /// The submitted form data, only present when action is "accept"
    /// Contains values matching the requested schema
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<HashMap<String, Value>>,
    /// Optional metadata
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<HashMap<String, Value>>,
}

impl ElicitResult {
    pub fn accept(content: HashMap<String, Value>) -> Self {
        Self {
            action: ElicitAction::Accept,
            content: Some(content),
            meta: None,
        }
    }

    pub fn decline() -> Self {
        Self {
            action: ElicitAction::Decline,
            content: None,
            meta: None,
        }
    }

    pub fn cancel() -> Self {
        Self {
            action: ElicitAction::Cancel,
            content: None,
            meta: None,
        }
    }

    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.meta = Some(meta);
        self
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
            min_length: None,
            max_length: None,
            format: None,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
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
            minimum: None,
            maximum: None,
        }
    }

    pub fn integer() -> Self {
        Self {
            schema_type: "integer".to_string(),
            title: None,
            description: None,
            minimum: None,
            maximum: None,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
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
    ) -> ElicitCreateRequest {
        let field_name = field_name.into();
        let schema = ElicitationSchema::new()
            .with_property(
                field_name.clone(),
                PrimitiveSchemaDefinition::string_with_description(field_description),
            )
            .with_required(vec![field_name]);

        ElicitCreateRequest::new(message, schema)
    }

    /// Create a number input elicitation (MCP spec compliant)
    pub fn number_input(
        message: impl Into<String>,
        field_name: impl Into<String>,
        field_description: impl Into<String>,
        min: Option<f64>,
        max: Option<f64>,
    ) -> ElicitCreateRequest {
        let field_name = field_name.into();
        let mut number_schema = NumberSchema::new().with_description(field_description);
        number_schema.minimum = min;
        number_schema.maximum = max;
        let number_schema = PrimitiveSchemaDefinition::Number(number_schema);

        let schema = ElicitationSchema::new()
            .with_property(field_name.clone(), number_schema)
            .with_required(vec![field_name]);

        ElicitCreateRequest::new(message, schema)
    }

    /// Create a boolean confirmation elicitation (MCP spec compliant)
    pub fn confirm(message: impl Into<String>) -> ElicitCreateRequest {
        let schema = ElicitationSchema::new()
            .with_property(
                "confirmed".to_string(),
                PrimitiveSchemaDefinition::boolean(),
            )
            .with_required(vec!["confirmed".to_string()]);

        ElicitCreateRequest::new(message, schema)
    }
}

// ===========================================
// === Fine-Grained Elicitation Traits ===
// ===========================================

/// Trait for elicitation metadata (message, title)
pub trait HasElicitationMetadata {
    /// The message to present to the user
    fn message(&self) -> &str;

    /// Optional title for the elicitation dialog
    fn title(&self) -> Option<&str> {
        None
    }
}

/// Trait for elicitation schema definition (restricted to primitive types per MCP spec)
pub trait HasElicitationSchema {
    /// Restricted schema defining structure of input to collect (primitives only)
    fn requested_schema(&self) -> &ElicitationSchema;

    /// Validate that schema only contains primitive types (per MCP spec)
    fn validate_schema(&self) -> Result<(), String> {
        // All schemas in ElicitationSchema are already primitive-only by design
        Ok(())
    }
}

/// Trait for elicitation validation and handling
pub trait HasElicitationHandling {
    /// Validate submitted content against the schema
    fn validate_content(&self, _content: &HashMap<String, Value>) -> Result<(), String> {
        // Basic validation - can be extended
        Ok(())
    }

    /// Process accepted content (transform, normalize, etc.)
    fn process_content(
        &self,
        content: HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, String> {
        Ok(content)
    }
}

/// **Complete MCP Elicitation Creation** - Build schema-validated user input collection systems.
///
/// This trait represents a **complete, working MCP elicitation** that can prompt users
/// for structured input and validate their responses against JSON schemas. When you implement
/// the required metadata traits, you automatically get `ElicitationDefinition` for free
/// via blanket implementation.
///
/// # What You're Building
///
/// An elicitation is a sophisticated user input system that:
/// - Presents a clear message/prompt to the user
/// - Defines a JSON schema for expected input structure
/// - Validates user responses against that schema
/// - Processes the validated data for your application
///
/// # How to Create an Elicitation
///
/// Implement these three traits on your struct:
///
/// ```rust
/// # use turul_mcp_protocol::prelude::*;
/// # use serde_json::Value;
/// # use std::collections::HashMap;
///
/// // This struct will automatically implement ElicitationDefinition!
/// struct UserPreferencesForm {
///     context: String,
/// }
///
/// impl HasElicitationMetadata for UserPreferencesForm {
///     fn message(&self) -> &str {
///         "Please configure your preferences for this project"
///     }
/// }
///
/// impl HasElicitationSchema for UserPreferencesForm {
///     fn requested_schema(&self) -> &JsonSchema {
///         &JsonSchema::Object {
///             properties: [
///                 ("theme".to_string(), JsonSchema::String { enum_values: Some(vec!["dark".to_string(), "light".to_string()]) }),
///                 ("notifications".to_string(), JsonSchema::Boolean),
///                 ("max_items".to_string(), JsonSchema::Number { minimum: Some(1.0), maximum: Some(100.0) })
///             ].into_iter().collect(),
///             required: vec!["theme".to_string()],
///             additional_properties: false,
///         }
///     }
/// }
///
/// impl HasElicitationHandling for UserPreferencesForm {
///     async fn handle_response(&self, content: HashMap<String, Value>) -> Result<HashMap<String, Value>, String> {
///         // Validate that theme is acceptable
///         if let Some(theme) = content.get("theme") {
///             if !["dark", "light"].contains(&theme.as_str().unwrap_or("")) {
///                 return Err("Theme must be 'dark' or 'light'".to_string());
///             }
///         }
///
///         // Process and potentially transform the data
///         let mut processed = content.clone();
///         processed.insert("processed_at".to_string(), Value::String(chrono::Utc::now().to_rfc3339()));
///         Ok(processed)
///     }
/// }
///
/// // Now you can use it with the server:
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let form = UserPreferencesForm { context: "project-setup".to_string() };
///
/// // The elicitation automatically implements ElicitationDefinition
/// let create_request = form.to_create_request();
/// # Ok(())
/// # }
/// ```
///
/// # Key Benefits
///
/// - **Type Safety**: Schema validation happens at the protocol level
/// - **Automatic Implementation**: Just implement the three component traits
/// - **Flexible Processing**: Handle and transform user input as needed
/// - **MCP Compliant**: Fully compatible with MCP 2025-06-18 specification
///
/// # Common Use Cases
///
/// - Configuration forms with validation
/// - User preference collection
/// - Survey and feedback systems
/// - Structured data entry workflows
/// - Multi-step input wizards
pub trait ElicitationDefinition:
    HasElicitationMetadata + HasElicitationSchema + HasElicitationHandling
{
    /// Convert this elicitation definition to a protocol ElicitCreateRequest
    fn to_create_request(&self) -> ElicitCreateRequest {
        ElicitCreateRequest::new(self.message(), self.requested_schema().clone())
    }
}

// Blanket implementation: any type implementing the fine-grained traits automatically gets ElicitationDefinition
impl<T> ElicitationDefinition for T where
    T: HasElicitationMetadata + HasElicitationSchema + HasElicitationHandling
{
}

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

        let request = ElicitCreateRequest::new("Please enter your username", schema);

        assert_eq!(request.method, "elicitation/create");
        assert_eq!(request.params.message, "Please enter your username");
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
        assert!(
            text_request
                .params
                .requested_schema
                .properties
                .contains_key("name")
        );

        assert_eq!(confirm_request.method, "elicitation/create");
        assert!(
            confirm_request
                .params
                .requested_schema
                .properties
                .contains_key("confirmed")
        );
    }

    #[test]
    fn test_serialization() {
        let schema = ElicitationSchema::new()
            .with_property("test".to_string(), PrimitiveSchemaDefinition::string());
        let request = ElicitCreateRequest::new("Test message", schema);

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("elicitation/create"));
        assert!(json.contains("Test message"));

        let parsed: ElicitCreateRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.method, "elicitation/create");
        assert_eq!(parsed.params.message, "Test message");
    }

    #[test]
    fn test_elicit_request_matches_typescript_spec() {
        // Test ElicitRequest matches: { method: string, params: { message: string, requestedSchema: {...}, _meta?: {...} } }
        let mut meta = HashMap::new();
        meta.insert("requestId".to_string(), json!("req-123"));

        let schema = ElicitationSchema::new()
            .with_property(
                "name".to_string(),
                PrimitiveSchemaDefinition::string_with_description("Your name"),
            )
            .with_property("age".to_string(), PrimitiveSchemaDefinition::integer())
            .with_required(vec!["name".to_string()]);

        let request =
            ElicitCreateRequest::new("Please provide your details", schema).with_meta(meta);

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
        assert_eq!(json_value["params"]["_meta"]["requestId"], "req-123");
    }

    #[test]
    fn test_elicit_result_matches_typescript_spec() {
        // Test ElicitResult matches: { action: "accept" | "decline" | "cancel", content?: {...}, _meta?: {...} }
        let mut meta = HashMap::new();
        meta.insert("responseTime".to_string(), json!(1234));

        let mut content = HashMap::new();
        content.insert("name".to_string(), json!("John Doe"));
        content.insert("age".to_string(), json!(30));

        let result = ElicitResult::accept(content.clone()).with_meta(meta);

        let json_value = serde_json::to_value(&result).unwrap();

        assert_eq!(json_value["action"], "accept");
        assert!(json_value["content"].is_object());
        assert_eq!(json_value["content"]["name"], "John Doe");
        assert_eq!(json_value["content"]["age"], 30);
        assert_eq!(json_value["_meta"]["responseTime"], 1234);

        // Test decline without content
        let decline_result = ElicitResult::decline();
        let decline_json = serde_json::to_value(&decline_result).unwrap();

        assert_eq!(decline_json["action"], "decline");
        assert!(
            decline_json["content"].is_null()
                || !decline_json.as_object().unwrap().contains_key("content")
        );
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
