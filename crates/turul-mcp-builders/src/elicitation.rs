//! Elicitation Builder for Runtime User Input Collection
//!
//! This module provides a builder pattern for creating elicitation requests at runtime
//! for collecting structured user input via restricted primitive schemas.

use std::collections::HashMap;
use serde_json::Value;

// Import from protocol via alias
use turul_mcp_protocol::elicitation::{
    ElicitCreateRequest, ElicitationSchema, PrimitiveSchemaDefinition,
    StringSchema, NumberSchema, BooleanSchema, EnumSchema, StringFormat, ElicitResult,
    HasElicitationMetadata, HasElicitationSchema, HasElicitationHandling
};

/// Builder for creating elicitation requests at runtime
pub struct ElicitationBuilder {
    message: String,
    title: Option<String>,
    schema: ElicitationSchema,
    meta: Option<HashMap<String, Value>>,
}

impl ElicitationBuilder {
    /// Create a new elicitation builder with the given message
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            title: None,
            schema: ElicitationSchema::new(),
            meta: None,
        }
    }

    /// Set the title for the elicitation dialog
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set meta information
    pub fn meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.meta = Some(meta);
        self
    }

    /// Add a meta key-value pair
    pub fn meta_value(mut self, key: impl Into<String>, value: Value) -> Self {
        if self.meta.is_none() {
            self.meta = Some(HashMap::new());
        }
        self.meta.as_mut().unwrap().insert(key.into(), value);
        self
    }

    /// Add a string field to the schema
    pub fn string_field(
        mut self,
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        let schema = PrimitiveSchemaDefinition::String(
            StringSchema::new().with_description(description)
        );
        self.schema = self.schema.with_property(name, schema);
        self
    }

    /// Add a string field with length constraints
    pub fn string_field_with_length(
        mut self,
        name: impl Into<String>,
        description: impl Into<String>,
        min_length: Option<usize>,
        max_length: Option<usize>,
    ) -> Self {
        let mut string_schema = StringSchema::new().with_description(description);
        string_schema.min_length = min_length;
        string_schema.max_length = max_length;
        let schema = PrimitiveSchemaDefinition::String(string_schema);
        self.schema = self.schema.with_property(name, schema);
        self
    }

    /// Add a string field with format constraint
    pub fn string_field_with_format(
        mut self,
        name: impl Into<String>,
        description: impl Into<String>,
        format: StringFormat,
    ) -> Self {
        let mut string_schema = StringSchema::new().with_description(description);
        string_schema.format = Some(format);
        let schema = PrimitiveSchemaDefinition::String(string_schema);
        self.schema = self.schema.with_property(name, schema);
        self
    }

    /// Add a number field to the schema
    pub fn number_field(
        mut self,
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        let schema = PrimitiveSchemaDefinition::Number(
            NumberSchema::new().with_description(description)
        );
        self.schema = self.schema.with_property(name, schema);
        self
    }

    /// Add a number field with range constraints
    pub fn number_field_with_range(
        mut self,
        name: impl Into<String>,
        description: impl Into<String>,
        minimum: Option<f64>,
        maximum: Option<f64>,
    ) -> Self {
        let mut number_schema = NumberSchema::new().with_description(description);
        number_schema.minimum = minimum;
        number_schema.maximum = maximum;
        let schema = PrimitiveSchemaDefinition::Number(number_schema);
        self.schema = self.schema.with_property(name, schema);
        self
    }

    /// Add an integer field to the schema
    pub fn integer_field(
        mut self,
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        let schema = PrimitiveSchemaDefinition::Number(
            NumberSchema::integer().with_description(description)
        );
        self.schema = self.schema.with_property(name, schema);
        self
    }

    /// Add an integer field with range constraints
    pub fn integer_field_with_range(
        mut self,
        name: impl Into<String>,
        description: impl Into<String>,
        minimum: Option<f64>,
        maximum: Option<f64>,
    ) -> Self {
        let mut integer_schema = NumberSchema::integer().with_description(description);
        integer_schema.minimum = minimum;
        integer_schema.maximum = maximum;
        let schema = PrimitiveSchemaDefinition::Number(integer_schema);
        self.schema = self.schema.with_property(name, schema);
        self
    }

    /// Add a boolean field to the schema
    pub fn boolean_field(
        mut self,
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        let schema = PrimitiveSchemaDefinition::Boolean(
            BooleanSchema::new().with_description(description)
        );
        self.schema = self.schema.with_property(name, schema);
        self
    }

    /// Add a boolean field with default value
    pub fn boolean_field_with_default(
        mut self,
        name: impl Into<String>,
        description: impl Into<String>,
        default: bool,
    ) -> Self {
        let mut boolean_schema = BooleanSchema::new().with_description(description);
        boolean_schema.default = Some(default);
        let schema = PrimitiveSchemaDefinition::Boolean(boolean_schema);
        self.schema = self.schema.with_property(name, schema);
        self
    }

    /// Add an enum field (string with predefined values)
    pub fn enum_field(
        mut self,
        name: impl Into<String>,
        description: impl Into<String>,
        values: Vec<String>,
    ) -> Self {
        let schema = PrimitiveSchemaDefinition::Enum(
            EnumSchema::new(values).with_description(description)
        );
        self.schema = self.schema.with_property(name, schema);
        self
    }

    /// Add an enum field with display names
    pub fn enum_field_with_names(
        mut self,
        name: impl Into<String>,
        description: impl Into<String>,
        values: Vec<String>,
        display_names: Vec<String>,
    ) -> Self {
        let schema = PrimitiveSchemaDefinition::Enum(
            EnumSchema::new(values)
                .with_description(description)
                .with_enum_names(display_names)
        );
        self.schema = self.schema.with_property(name, schema);
        self
    }

    /// Mark a field as required
    pub fn require_field(mut self, field_name: impl Into<String>) -> Self {
        let field_name = field_name.into();
        if let Some(ref mut required) = self.schema.required {
            if !required.contains(&field_name) {
                required.push(field_name);
            }
        } else {
            self.schema.required = Some(vec![field_name]);
        }
        self
    }

    /// Set multiple fields as required
    pub fn require_fields(mut self, field_names: Vec<String>) -> Self {
        self.schema = self.schema.with_required(field_names);
        self
    }

    /// Build the elicitation request
    pub fn build(self) -> ElicitCreateRequest {
        let mut request = ElicitCreateRequest::new(self.message, self.schema);
        if let Some(meta) = self.meta {
            request = request.with_meta(meta);
        }
        request
    }

    /// Build a dynamic elicitation that implements the definition traits
    pub fn build_dynamic(self) -> DynamicElicitation {
        DynamicElicitation {
            message: self.message,
            title: self.title,
            schema: self.schema,
            meta: self.meta,
        }
    }
}

/// Dynamic elicitation created by ElicitationBuilder
#[derive(Debug)]
pub struct DynamicElicitation {
    message: String,
    title: Option<String>,
    schema: ElicitationSchema,
    #[allow(dead_code)]
    meta: Option<HashMap<String, Value>>,
}

// Implement all fine-grained traits for DynamicElicitation
impl HasElicitationMetadata for DynamicElicitation {
    fn message(&self) -> &str {
        &self.message
    }

    fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }
}

impl HasElicitationSchema for DynamicElicitation {
    fn requested_schema(&self) -> &ElicitationSchema {
        &self.schema
    }
}

impl HasElicitationHandling for DynamicElicitation {
    fn validate_content(&self, content: &HashMap<String, Value>) -> Result<(), String> {
        // Validate required fields are present
        if let Some(ref required_fields) = self.schema.required {
            for field in required_fields {
                if !content.contains_key(field) {
                    return Err(format!("Required field '{}' is missing", field));
                }
            }
        }

        // Validate field types match schema
        for (field_name, value) in content {
            if let Some(field_schema) = self.schema.properties.get(field_name) {
                match field_schema {
                    PrimitiveSchemaDefinition::String(_) => {
                        if !value.is_string() {
                            return Err(format!("Field '{}' must be a string", field_name));
                        }
                    }
                    PrimitiveSchemaDefinition::Number(num_schema) => {
                        if !value.is_number() {
                            return Err(format!("Field '{}' must be a number", field_name));
                        }
                        if num_schema.schema_type == "integer" && !value.as_i64().is_some() {
                            return Err(format!("Field '{}' must be an integer", field_name));
                        }
                    }
                    PrimitiveSchemaDefinition::Boolean(_) => {
                        if !value.is_boolean() {
                            return Err(format!("Field '{}' must be a boolean", field_name));
                        }
                    }
                    PrimitiveSchemaDefinition::Enum(enum_schema) => {
                        if let Some(str_value) = value.as_str() {
                            if !enum_schema.enum_values.contains(&str_value.to_string()) {
                                return Err(format!(
                                    "Field '{}' must be one of: {}",
                                    field_name,
                                    enum_schema.enum_values.join(", ")
                                ));
                            }
                        } else {
                            return Err(format!("Field '{}' must be a string from enum values", field_name));
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn process_content(&self, content: HashMap<String, Value>) -> Result<HashMap<String, Value>, String> {
        // Validate first
        self.validate_content(&content)?;
        
        // Process and normalize content
        let mut processed = HashMap::new();
        
        for (field_name, value) in content {
            if let Some(field_schema) = self.schema.properties.get(&field_name) {
                match field_schema {
                    PrimitiveSchemaDefinition::String(string_schema) => {
                        if let Some(str_value) = value.as_str() {
                            // Apply length constraints if defined
                            if let Some(min_len) = string_schema.min_length {
                                if str_value.len() < min_len {
                                    return Err(format!(
                                        "Field '{}' must be at least {} characters long",
                                        field_name, min_len
                                    ));
                                }
                            }
                            if let Some(max_len) = string_schema.max_length {
                                if str_value.len() > max_len {
                                    return Err(format!(
                                        "Field '{}' must be at most {} characters long",
                                        field_name, max_len
                                    ));
                                }
                            }
                        }
                        processed.insert(field_name, value);
                    }
                    PrimitiveSchemaDefinition::Number(number_schema) => {
                        if let Some(num_value) = value.as_f64() {
                            // Apply range constraints if defined
                            if let Some(min) = number_schema.minimum {
                                if num_value < min {
                                    return Err(format!(
                                        "Field '{}' must be at least {}",
                                        field_name, min
                                    ));
                                }
                            }
                            if let Some(max) = number_schema.maximum {
                                if num_value > max {
                                    return Err(format!(
                                        "Field '{}' must be at most {}",
                                        field_name, max
                                    ));
                                }
                            }
                        }
                        processed.insert(field_name, value);
                    }
                    _ => {
                        processed.insert(field_name, value);
                    }
                }
            } else {
                // Unknown field - include as-is but could warn
                processed.insert(field_name, value);
            }
        }

        Ok(processed)
    }
}

// ElicitationDefinition is automatically implemented via blanket impl!

/// Convenience methods for common elicitation patterns
impl ElicitationBuilder {
    /// Create a simple text input elicitation
    pub fn text_input(
        message: impl Into<String>,
        field_name: impl Into<String>,
        field_description: impl Into<String>,
    ) -> Self {
        let field_name = field_name.into();
        Self::new(message)
            .string_field(&field_name, field_description)
            .require_field(field_name)
    }

    /// Create a number input elicitation
    pub fn number_input(
        message: impl Into<String>,
        field_name: impl Into<String>,
        field_description: impl Into<String>,
        min: Option<f64>,
        max: Option<f64>,
    ) -> Self {
        let field_name = field_name.into();
        Self::new(message)
            .number_field_with_range(&field_name, field_description, min, max)
            .require_field(field_name)
    }

    /// Create an integer input elicitation
    pub fn integer_input(
        message: impl Into<String>,
        field_name: impl Into<String>,
        field_description: impl Into<String>,
        min: Option<f64>,
        max: Option<f64>,
    ) -> Self {
        let field_name = field_name.into();
        Self::new(message)
            .integer_field_with_range(&field_name, field_description, min, max)
            .require_field(field_name)
    }

    /// Create a boolean confirmation elicitation
    pub fn confirm(message: impl Into<String>) -> Self {
        Self::new(message)
            .boolean_field("confirmed", "Confirmation")
            .require_field("confirmed")
    }

    /// Create a boolean confirmation with custom field name
    pub fn confirm_with_field(
        message: impl Into<String>,
        field_name: impl Into<String>,
        field_description: impl Into<String>,
    ) -> Self {
        let field_name = field_name.into();
        Self::new(message)
            .boolean_field(&field_name, field_description)
            .require_field(field_name)
    }

    /// Create a choice elicitation (enum)
    pub fn choice(
        message: impl Into<String>,
        field_name: impl Into<String>,
        field_description: impl Into<String>,
        choices: Vec<String>,
    ) -> Self {
        let field_name = field_name.into();
        Self::new(message)
            .enum_field(&field_name, field_description, choices)
            .require_field(field_name)
    }

    /// Create an email input elicitation
    pub fn email_input(
        message: impl Into<String>,
        field_name: impl Into<String>,
        field_description: impl Into<String>,
    ) -> Self {
        let field_name = field_name.into();
        Self::new(message)
            .string_field_with_format(&field_name, field_description, StringFormat::Email)
            .require_field(field_name)
    }

    /// Create a URL input elicitation
    pub fn url_input(
        message: impl Into<String>,
        field_name: impl Into<String>,
        field_description: impl Into<String>,
    ) -> Self {
        let field_name = field_name.into();
        Self::new(message)
            .string_field_with_format(&field_name, field_description, StringFormat::Uri)
            .require_field(field_name)
    }

    /// Create a complex form with multiple fields
    pub fn form(message: impl Into<String>) -> Self {
        Self::new(message)
    }
}

/// Result builder for creating elicitation responses
pub struct ElicitResultBuilder;

impl ElicitResultBuilder {
    /// Create an accept result with content
    pub fn accept(content: HashMap<String, Value>) -> ElicitResult {
        ElicitResult::accept(content)
    }

    /// Create a decline result
    pub fn decline() -> ElicitResult {
        ElicitResult::decline()
    }

    /// Create a cancel result
    pub fn cancel() -> ElicitResult {
        ElicitResult::cancel()
    }

    /// Create an accept result with a single field
    pub fn accept_single(field_name: impl Into<String>, value: Value) -> ElicitResult {
        let mut content = HashMap::new();
        content.insert(field_name.into(), value);
        ElicitResult::accept(content)
    }

    /// Create an accept result with multiple fields from key-value pairs
    pub fn accept_fields(fields: Vec<(String, Value)>) -> ElicitResult {
        let content = fields.into_iter().collect();
        ElicitResult::accept(content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use turul_mcp_protocol::elicitation::{ElicitAction, ElicitationDefinition};

    #[test]
    fn test_elicitation_builder_basic() {
        let request = ElicitationBuilder::new("Enter your details")
            .title("User Information")
            .string_field("name", "Your full name")
            .integer_field("age", "Your age")
            .require_field("name")
            .build();

        assert_eq!(request.method, "elicitation/create");
        assert_eq!(request.params.message, "Enter your details");
        assert_eq!(request.params.requested_schema.properties.len(), 2);
        assert!(request.params.requested_schema.properties.contains_key("name"));
        assert!(request.params.requested_schema.properties.contains_key("age"));
        assert_eq!(request.params.requested_schema.required, Some(vec!["name".to_string()]));
    }

    #[test]
    fn test_elicitation_builder_with_constraints() {
        let request = ElicitationBuilder::new("Create account")
            .string_field_with_length("username", "Username", Some(3), Some(20))
            .number_field_with_range("score", "Score", Some(0.0), Some(100.0))
            .boolean_field_with_default("newsletter", "Subscribe to newsletter", false)
            .require_fields(vec!["username".to_string(), "score".to_string()])
            .build();

        assert_eq!(request.params.requested_schema.properties.len(), 3);
        assert_eq!(request.params.requested_schema.required.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn test_elicitation_builder_enum_field() {
        let choices = vec!["red".to_string(), "green".to_string(), "blue".to_string()];
        let display_names = vec!["Red".to_string(), "Green".to_string(), "Blue".to_string()];

        let request = ElicitationBuilder::new("Choose a color")
            .enum_field_with_names("color", "Your favorite color", choices.clone(), display_names)
            .require_field("color")
            .build();

        assert_eq!(request.params.requested_schema.properties.len(), 1);
        
        if let Some(PrimitiveSchemaDefinition::Enum(enum_schema)) = 
            request.params.requested_schema.properties.get("color") {
            assert_eq!(enum_schema.enum_values, choices);
            assert!(enum_schema.enum_names.is_some());
        } else {
            panic!("Expected enum schema for color field");
        }
    }

    #[test]
    fn test_elicitation_builder_meta() {
        let request = ElicitationBuilder::new("Test")
            .meta_value("request_id", json!("req-123"))
            .meta_value("priority", json!(1))
            .build();

        let params = request.params;
        assert!(params.meta.is_some());
        let meta = params.meta.unwrap();
        assert_eq!(meta.get("request_id"), Some(&json!("req-123")));
        assert_eq!(meta.get("priority"), Some(&json!(1)));
    }

    #[test]
    fn test_convenience_builders() {
        // Text input
        let text_request = ElicitationBuilder::text_input("Enter name", "name", "Your name")
            .build();
        assert!(text_request.params.requested_schema.properties.contains_key("name"));

        // Number input
        let number_request = ElicitationBuilder::number_input(
            "Enter score", "score", "Your score", Some(0.0), Some(100.0)
        ).build();
        assert!(number_request.params.requested_schema.properties.contains_key("score"));

        // Confirmation
        let confirm_request = ElicitationBuilder::confirm("Do you agree?").build();
        assert!(confirm_request.params.requested_schema.properties.contains_key("confirmed"));

        // Choice
        let choice_request = ElicitationBuilder::choice(
            "Select option",
            "option",
            "Choose an option",
            vec!["a".to_string(), "b".to_string(), "c".to_string()]
        ).build();
        assert!(choice_request.params.requested_schema.properties.contains_key("option"));

        // Email
        let email_request = ElicitationBuilder::email_input("Enter email", "email", "Email address")
            .build();
        assert!(email_request.params.requested_schema.properties.contains_key("email"));
        
        if let Some(PrimitiveSchemaDefinition::String(string_schema)) = 
            email_request.params.requested_schema.properties.get("email") {
            assert!(string_schema.format.is_some());
            // Note: StringFormat doesn't implement PartialEq, so we can't compare directly
        } else {
            panic!("Expected string schema with email format");
        }
    }

    #[test]
    fn test_dynamic_elicitation_validation() {
        let elicitation = ElicitationBuilder::new("Test form")
            .string_field("name", "Name")
            .integer_field_with_range("age", "Age", Some(0.0), Some(120.0))
            .boolean_field("active", "Active")
            .enum_field("status", "Status", vec!["new".to_string(), "active".to_string()])
            .require_fields(vec!["name".to_string(), "age".to_string()])
            .build_dynamic();

        // Valid content
        let mut valid_content = HashMap::new();
        valid_content.insert("name".to_string(), json!("John"));
        valid_content.insert("age".to_string(), json!(30));
        valid_content.insert("active".to_string(), json!(true));
        valid_content.insert("status".to_string(), json!("active"));

        assert!(elicitation.validate_content(&valid_content).is_ok());

        // Missing required field
        let mut missing_required = HashMap::new();
        missing_required.insert("name".to_string(), json!("John"));
        // age is missing
        assert!(elicitation.validate_content(&missing_required).is_err());

        // Wrong type
        let mut wrong_type = HashMap::new();
        wrong_type.insert("name".to_string(), json!("John"));
        wrong_type.insert("age".to_string(), json!("thirty")); // Should be number
        assert!(elicitation.validate_content(&wrong_type).is_err());

        // Invalid enum value
        let mut invalid_enum = HashMap::new();
        invalid_enum.insert("name".to_string(), json!("John"));
        invalid_enum.insert("age".to_string(), json!(30));
        invalid_enum.insert("status".to_string(), json!("invalid")); // Not in enum values
        assert!(elicitation.validate_content(&invalid_enum).is_err());
    }

    #[test]
    fn test_dynamic_elicitation_processing() {
        let elicitation = ElicitationBuilder::new("Test form")
            .string_field_with_length("name", "Name", Some(2), Some(50))
            .number_field_with_range("score", "Score", Some(0.0), Some(100.0))
            .build_dynamic();

        // Valid content
        let mut valid_content = HashMap::new();
        valid_content.insert("name".to_string(), json!("John"));
        valid_content.insert("score".to_string(), json!(85.5));

        let processed = elicitation.process_content(valid_content.clone());
        assert!(processed.is_ok());
        assert_eq!(processed.unwrap(), valid_content);

        // String too short
        let mut short_string = HashMap::new();
        short_string.insert("name".to_string(), json!("J")); // Too short
        short_string.insert("score".to_string(), json!(50.0));

        assert!(elicitation.process_content(short_string).is_err());

        // Number out of range
        let mut out_of_range = HashMap::new();
        out_of_range.insert("name".to_string(), json!("John"));
        out_of_range.insert("score".to_string(), json!(150.0)); // Too high

        assert!(elicitation.process_content(out_of_range).is_err());
    }

    #[test]
    fn test_elicit_result_builder() {
        // Accept with single field
        let single_result = ElicitResultBuilder::accept_single("name", json!("John"));
        assert!(matches!(single_result.action, ElicitAction::Accept));
        assert_eq!(single_result.content.as_ref().unwrap().get("name"), Some(&json!("John")));

        // Accept with multiple fields
        let multi_result = ElicitResultBuilder::accept_fields(vec![
            ("name".to_string(), json!("Alice")),
            ("age".to_string(), json!(25)),
        ]);
        assert!(matches!(multi_result.action, ElicitAction::Accept));
        assert_eq!(multi_result.content.as_ref().unwrap().len(), 2);

        // Decline
        let decline_result = ElicitResultBuilder::decline();
        assert!(matches!(decline_result.action, ElicitAction::Decline));
        assert!(decline_result.content.is_none());

        // Cancel
        let cancel_result = ElicitResultBuilder::cancel();
        assert!(matches!(cancel_result.action, ElicitAction::Cancel));
        assert!(cancel_result.content.is_none());
    }

    #[test]
    fn test_trait_implementations() {
        let elicitation = ElicitationBuilder::new("Test message")
            .title("Test Title")
            .string_field("field1", "Field 1")
            .build_dynamic();

        // Test HasElicitationMetadata
        assert_eq!(elicitation.message(), "Test message");
        assert_eq!(elicitation.title(), Some("Test Title"));

        // Test HasElicitationSchema
        assert_eq!(elicitation.requested_schema().schema_type, "object");
        assert!(elicitation.requested_schema().properties.contains_key("field1"));
        
        // Test validation (should pass for well-formed schema)
        assert!(elicitation.validate_schema().is_ok());

        // Test ElicitationDefinition (auto-implemented)
        let request = elicitation.to_create_request();
        assert_eq!(request.method, "elicitation/create");
        assert_eq!(request.params.message, "Test message");
    }
}