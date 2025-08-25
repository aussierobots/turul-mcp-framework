//! MCP Elicitation Protocol Types
//!
//! This module defines the types used for MCP elicitation functionality,
//! which enables structured user input collection via restricted primitive schemas.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::meta::Meta;
use std::collections::HashMap;

/// Restricted schema definitions that only allow primitive types
/// without nested objects or arrays (per MCP spec).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum PrimitiveSchemaDefinition {
    String {
        #[serde(skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        min_length: Option<usize>,
        #[serde(skip_serializing_if = "Option::is_none")]
        max_length: Option<usize>,
        #[serde(skip_serializing_if = "Option::is_none")]
        format: Option<StringFormat>,
    },
    Number {
        #[serde(skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        minimum: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        maximum: Option<f64>,
    },
    Integer {
        #[serde(skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        minimum: Option<i64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        maximum: Option<i64>,
    },
    Boolean {
        #[serde(skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        default: Option<bool>,
    },
}

/// String format constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StringFormat {
    Email,
    Uri,
    Date,
    DateTime,
}

/// String enum schema (special case of string type)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumSchema {
    pub r#type: String, // Always "string"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub r#enum: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_names: Option<Vec<String>>, // Display names for enum values
}

/// Restricted schema for elicitation (only primitive types, no nesting)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElicitationSchema {
    pub r#type: String, // Always "object"
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
            },
        }
    }
}

impl ElicitCreateParams {
    pub fn new(message: impl Into<String>, requested_schema: ElicitationSchema) -> Self {
        Self {
            message: message.into(),
            requested_schema,
        }
    }
}

// Trait implementations for protocol compliance
use crate::traits::Params;
impl Params for ElicitCreateParams {}

impl ElicitationSchema {
    pub fn new() -> Self {
        Self {
            r#type: "object".to_string(),
            properties: HashMap::new(),
            required: None,
        }
    }

    pub fn with_property(mut self, name: impl Into<String>, schema: PrimitiveSchemaDefinition) -> Self {
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
    pub meta: Option<Meta>,
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

    pub fn with_meta(mut self, meta: Meta) -> Self {
        self.meta = Some(meta);
        self
    }
}

// Convenience constructors for PrimitiveSchemaDefinition
impl PrimitiveSchemaDefinition {
    pub fn string() -> Self {
        Self::String {
            title: None,
            description: None,
            min_length: None,
            max_length: None,
            format: None,
        }
    }

    pub fn string_with_description(description: impl Into<String>) -> Self {
        Self::String {
            title: None,
            description: Some(description.into()),
            min_length: None,
            max_length: None,
            format: None,
        }
    }

    pub fn number() -> Self {
        Self::Number {
            title: None,
            description: None,
            minimum: None,
            maximum: None,
        }
    }

    pub fn integer() -> Self {
        Self::Integer {
            title: None,
            description: None,
            minimum: None,
            maximum: None,
        }
    }

    pub fn boolean() -> Self {
        Self::Boolean {
            title: None,
            description: None,
            default: None,
        }
    }
}

/// Builder for creating common elicitation patterns
pub struct ElicitationBuilder;

impl ElicitationBuilder {
    /// Create a simple text input elicitation (MCP spec compliant)
    pub fn text_input(
        message: impl Into<String>, 
        field_name: impl Into<String>, 
        field_description: impl Into<String>
    ) -> ElicitCreateRequest {
        let field_name = field_name.into();
        let schema = ElicitationSchema::new()
            .with_property(field_name.clone(), PrimitiveSchemaDefinition::string_with_description(field_description))
            .with_required(vec![field_name]);
        
        ElicitCreateRequest::new(message, schema)
    }

    /// Create a number input elicitation (MCP spec compliant)
    pub fn number_input(
        message: impl Into<String>, 
        field_name: impl Into<String>, 
        field_description: impl Into<String>,
        min: Option<f64>,
        max: Option<f64>
    ) -> ElicitCreateRequest {
        let field_name = field_name.into();
        let number_schema = PrimitiveSchemaDefinition::Number {
            title: None,
            description: Some(field_description.into()),
            minimum: min,
            maximum: max,
        };
        
        let schema = ElicitationSchema::new()
            .with_property(field_name.clone(), number_schema)
            .with_required(vec![field_name]);
        
        ElicitCreateRequest::new(message, schema)
    }

    /// Create a boolean confirmation elicitation (MCP spec compliant)
    pub fn confirm(message: impl Into<String>) -> ElicitCreateRequest {
        let schema = ElicitationSchema::new()
            .with_property("confirmed".to_string(), PrimitiveSchemaDefinition::boolean())
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
    fn process_content(&self, content: HashMap<String, Value>) -> Result<HashMap<String, Value>, String> {
        Ok(content)
    }
}

/// Composed elicitation definition trait (automatically implemented via blanket impl)
pub trait ElicitationDefinition: 
    HasElicitationMetadata + 
    HasElicitationSchema + 
    HasElicitationHandling 
{
    /// Convert this elicitation definition to a protocol ElicitCreateRequest
    fn to_create_request(&self) -> ElicitCreateRequest {
        ElicitCreateRequest::new(self.message(), self.requested_schema().clone())
    }
}

// Blanket implementation: any type implementing the fine-grained traits automatically gets ElicitationDefinition
impl<T> ElicitationDefinition for T 
where 
    T: HasElicitationMetadata + HasElicitationSchema + HasElicitationHandling 
{}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_primitive_schema_creation() {
        let string_schema = PrimitiveSchemaDefinition::string_with_description("Enter your name");
        let number_schema = PrimitiveSchemaDefinition::number();
        let boolean_schema = PrimitiveSchemaDefinition::boolean();
        
        assert!(matches!(string_schema, PrimitiveSchemaDefinition::String { .. }));
        assert!(matches!(number_schema, PrimitiveSchemaDefinition::Number { .. }));
        assert!(matches!(boolean_schema, PrimitiveSchemaDefinition::Boolean { .. }));
    }

    #[test]
    fn test_elicitation_schema() {
        let schema = ElicitationSchema::new()
            .with_property("name".to_string(), PrimitiveSchemaDefinition::string_with_description("Your name"))
            .with_property("age".to_string(), PrimitiveSchemaDefinition::integer())
            .with_required(vec!["name".to_string()]);
        
        assert_eq!(schema.r#type, "object");
        assert_eq!(schema.properties.len(), 2);
        assert_eq!(schema.required, Some(vec!["name".to_string()]));
    }

    #[test]
    fn test_elicit_create_request() {
        let schema = ElicitationSchema::new()
            .with_property("username".to_string(), PrimitiveSchemaDefinition::string_with_description("Username"));
        
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
        let text_request = ElicitationBuilder::text_input(
            "Enter your name",
            "name", 
            "Your full name"
        );
        
        let confirm_request = ElicitationBuilder::confirm("Do you agree?");
        
        assert_eq!(text_request.method, "elicitation/create");
        assert!(text_request.params.requested_schema.properties.contains_key("name"));
        
        assert_eq!(confirm_request.method, "elicitation/create");
        assert!(confirm_request.params.requested_schema.properties.contains_key("confirmed"));
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
}
