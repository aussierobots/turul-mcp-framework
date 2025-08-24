//! JSON Schema Support for MCP
//!
//! This module provides JSON Schema types used throughout the MCP protocol.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Trait for generating JSON schemas from Rust types
pub trait JsonSchemaGenerator {
    /// Generate a ToolSchema for this type
    fn json_schema() -> crate::tools::ToolSchema;
}

/// A JSON Schema definition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum JsonSchema {
    /// String type
    String {
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pattern: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        min_length: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        max_length: Option<u64>,
        #[serde(rename = "enum", skip_serializing_if = "Option::is_none")]
        enum_values: Option<Vec<String>>,
    },
    /// Number type
    Number {
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        minimum: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        maximum: Option<f64>,
    },
    /// Integer type
    Integer {
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        minimum: Option<i64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        maximum: Option<i64>,
    },
    /// Boolean type
    Boolean {
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
    },
    /// Array type
    Array {
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        items: Option<Box<JsonSchema>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        min_items: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        max_items: Option<u64>,
    },
    /// Object type
    Object {
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        properties: Option<HashMap<String, JsonSchema>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        required: Option<Vec<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        additional_properties: Option<bool>,
    },
}

impl JsonSchema {
    /// Create a string schema
    pub fn string() -> Self {
        Self::String {
            description: None,
            pattern: None,
            min_length: None,
            max_length: None,
            enum_values: None,
        }
    }

    /// Create a string schema with description
    pub fn string_with_description(description: impl Into<String>) -> Self {
        Self::String {
            description: Some(description.into()),
            pattern: None,
            min_length: None,
            max_length: None,
            enum_values: None,
        }
    }

    /// Create a string enum schema
    pub fn string_enum(values: Vec<String>) -> Self {
        Self::String {
            description: None,
            pattern: None,
            min_length: None,
            max_length: None,
            enum_values: Some(values),
        }
    }

    /// Create a number schema
    pub fn number() -> Self {
        Self::Number {
            description: None,
            minimum: None,
            maximum: None,
        }
    }

    /// Create a number schema with description
    pub fn number_with_description(description: impl Into<String>) -> Self {
        Self::Number {
            description: Some(description.into()),
            minimum: None,
            maximum: None,
        }
    }

    /// Create an integer schema
    pub fn integer() -> Self {
        Self::Integer {
            description: None,
            minimum: None,
            maximum: None,
        }
    }

    /// Create an integer schema with description
    pub fn integer_with_description(description: impl Into<String>) -> Self {
        Self::Integer {
            description: Some(description.into()),
            minimum: None,
            maximum: None,
        }
    }

    /// Create a boolean schema
    pub fn boolean() -> Self {
        Self::Boolean {
            description: None,
        }
    }

    /// Create a boolean schema with description
    pub fn boolean_with_description(description: impl Into<String>) -> Self {
        Self::Boolean {
            description: Some(description.into()),
        }
    }

    /// Create an array schema
    pub fn array(items: JsonSchema) -> Self {
        Self::Array {
            description: None,
            items: Some(Box::new(items)),
            min_items: None,
            max_items: None,
        }
    }

    /// Create an array schema with description
    pub fn array_with_description(items: JsonSchema, description: impl Into<String>) -> Self {
        Self::Array {
            description: Some(description.into()),
            items: Some(Box::new(items)),
            min_items: None,
            max_items: None,
        }
    }

    /// Create an object schema
    pub fn object() -> Self {
        Self::Object {
            description: None,
            properties: None,
            required: None,
            additional_properties: None,
        }
    }

    /// Create an object schema with properties
    pub fn object_with_properties(properties: HashMap<String, JsonSchema>) -> Self {
        Self::Object {
            description: None,
            properties: Some(properties),
            required: None,
            additional_properties: None,
        }
    }

    /// Create an object schema with properties and required fields
    pub fn object_with_required(
        properties: HashMap<String, JsonSchema>,
        required: Vec<String>,
    ) -> Self {
        Self::Object {
            description: None,
            properties: Some(properties),
            required: Some(required),
            additional_properties: None,
        }
    }

    /// Add description to any schema
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        match &mut self {
            JsonSchema::String { description: d, .. } => *d = Some(description.into()),
            JsonSchema::Number { description: d, .. } => *d = Some(description.into()),
            JsonSchema::Integer { description: d, .. } => *d = Some(description.into()),
            JsonSchema::Boolean { description: d, .. } => *d = Some(description.into()),
            JsonSchema::Array { description: d, .. } => *d = Some(description.into()),
            JsonSchema::Object { description: d, .. } => *d = Some(description.into()),
        }
        self
    }

    /// Add minimum constraint to number schema
    pub fn with_minimum(mut self, minimum: f64) -> Self {
        match &mut self {
            JsonSchema::Number { minimum: m, .. } => *m = Some(minimum),
            JsonSchema::Integer { minimum: m, .. } => *m = Some(minimum as i64),
            _ => {}, // Ignore for non-numeric types
        }
        self
    }

    /// Add maximum constraint to number schema
    pub fn with_maximum(mut self, maximum: f64) -> Self {
        match &mut self {
            JsonSchema::Number { maximum: m, .. } => *m = Some(maximum),
            JsonSchema::Integer { maximum: m, .. } => *m = Some(maximum as i64),
            _ => {}, // Ignore for non-numeric types
        }
        self
    }

    /// Add properties to object schema
    pub fn with_properties(mut self, properties: HashMap<String, JsonSchema>) -> Self {
        match &mut self {
            JsonSchema::Object { properties: p, .. } => *p = Some(properties),
            _ => {}, // Ignore for non-object types
        }
        self
    }

    /// Add required fields to object schema
    pub fn with_required(mut self, required: Vec<String>) -> Self {
        match &mut self {
            JsonSchema::Object { required: r, .. } => *r = Some(required),
            _ => {}, // Ignore for non-object types
        }
        self
    }
}

/// Converts common Rust types to JsonSchema
pub trait ToJsonSchema {
    fn to_json_schema() -> JsonSchema;
}

impl ToJsonSchema for String {
    fn to_json_schema() -> JsonSchema {
        JsonSchema::string()
    }
}

impl ToJsonSchema for &str {
    fn to_json_schema() -> JsonSchema {
        JsonSchema::string()
    }
}

impl ToJsonSchema for i32 {
    fn to_json_schema() -> JsonSchema {
        JsonSchema::integer()
    }
}

impl ToJsonSchema for i64 {
    fn to_json_schema() -> JsonSchema {
        JsonSchema::integer()
    }
}

impl ToJsonSchema for f32 {
    fn to_json_schema() -> JsonSchema {
        JsonSchema::number()
    }
}

impl ToJsonSchema for f64 {
    fn to_json_schema() -> JsonSchema {
        JsonSchema::number()
    }
}

impl ToJsonSchema for bool {
    fn to_json_schema() -> JsonSchema {
        JsonSchema::boolean()
    }
}

impl<T: ToJsonSchema> ToJsonSchema for Vec<T> {
    fn to_json_schema() -> JsonSchema {
        JsonSchema::array(T::to_json_schema())
    }
}

impl<T: ToJsonSchema> ToJsonSchema for Option<T> {
    fn to_json_schema() -> JsonSchema {
        T::to_json_schema()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_schema() {
        let schema = JsonSchema::string_with_description("A test string");
        let json = serde_json::to_string(&schema).unwrap();
        assert!(json.contains("string"));
        assert!(json.contains("A test string"));
    }

    #[test]
    fn test_object_schema() {
        let mut properties = HashMap::new();
        properties.insert("name".to_string(), JsonSchema::string());
        properties.insert("age".to_string(), JsonSchema::integer());

        let schema = JsonSchema::object_with_required(
            properties,
            vec!["name".to_string()],
        );

        let json = serde_json::to_string(&schema).unwrap();
        assert!(json.contains("object"));
        assert!(json.contains("name"));
        assert!(json.contains("age"));
    }

    #[test]
    fn test_array_schema() {
        let schema = JsonSchema::array(JsonSchema::string());
        let json = serde_json::to_string(&schema).unwrap();
        assert!(json.contains("array"));
    }

    #[test]
    fn test_enum_schema() {
        let schema = JsonSchema::string_enum(vec![
            "option1".to_string(),
            "option2".to_string(),
        ]);
        let json = serde_json::to_string(&schema).unwrap();
        assert!(json.contains("option1"));
        assert!(json.contains("option2"));
    }

    #[test]
    fn test_to_json_schema_trait() {
        assert!(matches!(String::to_json_schema(), JsonSchema::String { .. }));
        assert!(matches!(i32::to_json_schema(), JsonSchema::Integer { .. }));
        assert!(matches!(f64::to_json_schema(), JsonSchema::Number { .. }));
        assert!(matches!(bool::to_json_schema(), JsonSchema::Boolean { .. }));
    }
}