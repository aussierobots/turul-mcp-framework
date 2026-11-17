//! JSON Schema Support for MCP
//!
//! This module provides JSON Schema types used throughout the MCP protocol.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A JSON Schema definition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum JsonSchema {
    /// String type
    String {
        /// Human-readable description of the value.
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        /// Regular expression the string must match.
        #[serde(skip_serializing_if = "Option::is_none")]
        pattern: Option<String>,
        /// Minimum string length.
        #[serde(rename = "minLength", skip_serializing_if = "Option::is_none")]
        min_length: Option<u64>,
        /// Maximum string length.
        #[serde(rename = "maxLength", skip_serializing_if = "Option::is_none")]
        max_length: Option<u64>,
        /// Closed set of permitted string values (`enum`).
        #[serde(rename = "enum", skip_serializing_if = "Option::is_none")]
        enum_values: Option<Vec<String>>,
    },
    /// Number type
    Number {
        /// Human-readable description of the value.
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        /// Inclusive lower bound.
        #[serde(skip_serializing_if = "Option::is_none")]
        minimum: Option<f64>,
        /// Inclusive upper bound.
        #[serde(skip_serializing_if = "Option::is_none")]
        maximum: Option<f64>,
    },
    /// Integer type
    Integer {
        /// Human-readable description of the value.
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        /// Inclusive lower bound.
        #[serde(skip_serializing_if = "Option::is_none")]
        minimum: Option<i64>,
        /// Inclusive upper bound.
        #[serde(skip_serializing_if = "Option::is_none")]
        maximum: Option<i64>,
    },
    /// Boolean type
    Boolean {
        /// Human-readable description of the value.
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
    },
    /// Array type
    Array {
        /// Human-readable description of the value.
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        /// Schema every array element must satisfy.
        #[serde(skip_serializing_if = "Option::is_none")]
        items: Option<Box<JsonSchema>>,
        /// Minimum number of items.
        #[serde(rename = "minItems", skip_serializing_if = "Option::is_none")]
        min_items: Option<u64>,
        /// Maximum number of items.
        #[serde(rename = "maxItems", skip_serializing_if = "Option::is_none")]
        max_items: Option<u64>,
    },
    /// Object type
    Object {
        /// Human-readable description of the value.
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        /// Schemas of the object's properties, keyed by property name.
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(serialize_with = "crate::serde_maps::sorted_opt")]
        properties: Option<HashMap<String, JsonSchema>>,
        /// Names of the properties that must be present.
        #[serde(skip_serializing_if = "Option::is_none")]
        required: Option<Vec<String>>,
        /// Whether properties not listed in `properties` are permitted.
        #[serde(
            rename = "additionalProperties",
            skip_serializing_if = "Option::is_none"
        )]
        additional_properties: Option<bool>,
    },

    /// Verbatim JSON Schema 2020-12 subschema, serialized as-is (untagged).
    ///
    /// Escape hatch for schema shapes the structured variants cannot express
    /// (`oneOf`/`anyOf`/`allOf`, `const`, nested `$defs`-resolved object
    /// trees, conditionals, …). The 2026 wire bindings accept arbitrary
    /// 2020-12 values, so this variant round-trips without downgrading.
    /// On deserialize it is the last-tried fallback, capturing any subschema
    /// the structured variants reject.
    #[serde(untagged)]
    Raw(serde_json::Value),
}

impl JsonSchema {
    /// Creates a string schema with no constraints.
    pub fn string() -> Self {
        Self::String {
            description: None,
            pattern: None,
            min_length: None,
            max_length: None,
            enum_values: None,
        }
    }

    /// Creates a string schema with the given description.
    pub fn string_with_description(description: impl Into<String>) -> Self {
        Self::String {
            description: Some(description.into()),
            pattern: None,
            min_length: None,
            max_length: None,
            enum_values: None,
        }
    }

    /// Creates a string schema restricted to the given `enum` values.
    pub fn string_enum(values: Vec<String>) -> Self {
        Self::String {
            description: None,
            pattern: None,
            min_length: None,
            max_length: None,
            enum_values: Some(values),
        }
    }

    /// Creates a number schema with no bounds.
    pub fn number() -> Self {
        Self::Number {
            description: None,
            minimum: None,
            maximum: None,
        }
    }

    /// Creates a number schema with the given description.
    pub fn number_with_description(description: impl Into<String>) -> Self {
        Self::Number {
            description: Some(description.into()),
            minimum: None,
            maximum: None,
        }
    }

    /// Creates an integer schema with no bounds.
    pub fn integer() -> Self {
        Self::Integer {
            description: None,
            minimum: None,
            maximum: None,
        }
    }

    /// Creates an integer schema with the given description.
    pub fn integer_with_description(description: impl Into<String>) -> Self {
        Self::Integer {
            description: Some(description.into()),
            minimum: None,
            maximum: None,
        }
    }

    /// Creates a boolean schema.
    pub fn boolean() -> Self {
        Self::Boolean { description: None }
    }

    /// Creates a boolean schema with the given description.
    pub fn boolean_with_description(description: impl Into<String>) -> Self {
        Self::Boolean {
            description: Some(description.into()),
        }
    }

    /// Creates an array schema whose elements satisfy `items`.
    pub fn array(items: JsonSchema) -> Self {
        Self::Array {
            description: None,
            items: Some(Box::new(items)),
            min_items: None,
            max_items: None,
        }
    }

    /// Creates an array schema whose elements satisfy `items`, with the given description.
    pub fn array_with_description(items: JsonSchema, description: impl Into<String>) -> Self {
        Self::Array {
            description: Some(description.into()),
            items: Some(Box::new(items)),
            min_items: None,
            max_items: None,
        }
    }

    /// Creates an object schema with no properties.
    pub fn object() -> Self {
        Self::Object {
            description: None,
            properties: None,
            required: None,
            additional_properties: None,
        }
    }

    /// Creates an object schema with the given properties and none required.
    pub fn object_with_properties(properties: HashMap<String, JsonSchema>) -> Self {
        Self::Object {
            description: None,
            properties: Some(properties),
            required: None,
            additional_properties: None,
        }
    }

    /// Creates an object schema with the given properties and required property names.
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

    /// Sets the description on any typed variant; a `Raw` object schema gets a `description` key.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        match &mut self {
            JsonSchema::String { description: d, .. } => *d = Some(description.into()),
            JsonSchema::Number { description: d, .. } => *d = Some(description.into()),
            JsonSchema::Integer { description: d, .. } => *d = Some(description.into()),
            JsonSchema::Boolean { description: d, .. } => *d = Some(description.into()),
            JsonSchema::Array { description: d, .. } => *d = Some(description.into()),
            JsonSchema::Object { description: d, .. } => *d = Some(description.into()),
            JsonSchema::Raw(value) => {
                if let Some(obj) = value.as_object_mut() {
                    obj.insert(
                        "description".to_string(),
                        serde_json::Value::String(description.into()),
                    );
                }
            }
        }
        self
    }

    /// Verbatim 2020-12 subschema (see [`JsonSchema::Raw`]).
    pub fn raw(value: serde_json::Value) -> Self {
        JsonSchema::Raw(value)
    }

    /// Sets the inclusive `minimum` on a number or integer schema (truncated toward zero for integers); other variants are returned unchanged.
    pub fn with_minimum(mut self, minimum: f64) -> Self {
        match &mut self {
            JsonSchema::Number { minimum: m, .. } => *m = Some(minimum),
            JsonSchema::Integer { minimum: m, .. } => *m = Some(minimum as i64),
            _ => {} // Ignore for non-numeric types
        }
        self
    }

    /// Sets the inclusive `maximum` on a number or integer schema (truncated toward zero for integers); other variants are returned unchanged.
    pub fn with_maximum(mut self, maximum: f64) -> Self {
        match &mut self {
            JsonSchema::Number { maximum: m, .. } => *m = Some(maximum),
            JsonSchema::Integer { maximum: m, .. } => *m = Some(maximum as i64),
            _ => {} // Ignore for non-numeric types
        }
        self
    }

    /// Replaces the properties of an object schema; other variants are returned unchanged.
    pub fn with_properties(mut self, properties: HashMap<String, JsonSchema>) -> Self {
        if let JsonSchema::Object { properties: p, .. } = &mut self {
            *p = Some(properties);
        }
        self
    }

    /// Replaces the required property names of an object schema; other variants are returned unchanged.
    pub fn with_required(mut self, required: Vec<String>) -> Self {
        if let JsonSchema::Object { required: r, .. } = &mut self {
            *r = Some(required);
        }
        self
    }

    /// Sets `minItems` on an array schema; other variants are returned unchanged.
    pub fn with_min_items(mut self, min: u64) -> Self {
        if let JsonSchema::Array { min_items, .. } = &mut self {
            *min_items = Some(min);
        }
        self
    }

    /// Sets `maxItems` on an array schema; other variants are returned unchanged.
    pub fn with_max_items(mut self, max: u64) -> Self {
        if let JsonSchema::Array { max_items, .. } = &mut self {
            *max_items = Some(max);
        }
        self
    }
}

/// Converts common Rust types to JsonSchema
pub trait ToJsonSchema {
    /// Returns the JSON Schema describing this Rust type.
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

        let schema = JsonSchema::object_with_required(properties, vec!["name".to_string()]);

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
    fn test_fixed_length_array_schema() {
        let schema = JsonSchema::array(JsonSchema::number())
            .with_min_items(3)
            .with_max_items(3);
        let json = serde_json::to_string(&schema).unwrap();
        assert!(json.contains(r#""type":"array""#), "Must be array type");
        assert!(
            json.contains(r#""minItems":3"#),
            "Must have minItems: {json}"
        );
        assert!(
            json.contains(r#""maxItems":3"#),
            "Must have maxItems: {json}"
        );
        assert!(
            json.contains(r#""type":"number""#),
            "Items must be number: {json}"
        );
    }

    #[test]
    fn test_min_max_items_ignored_on_non_array() {
        // with_min_items/with_max_items should be no-ops on non-array schemas
        let schema = JsonSchema::string().with_min_items(3).with_max_items(5);
        let json = serde_json::to_string(&schema).unwrap();
        assert!(
            !json.contains("minItems"),
            "String should not have minItems"
        );
        assert!(
            !json.contains("maxItems"),
            "String should not have maxItems"
        );
    }

    #[test]
    fn test_enum_schema() {
        let schema = JsonSchema::string_enum(vec!["option1".to_string(), "option2".to_string()]);
        let json = serde_json::to_string(&schema).unwrap();
        assert!(json.contains("option1"));
        assert!(json.contains("option2"));
    }

    #[test]
    fn test_to_json_schema_trait() {
        assert!(matches!(
            String::to_json_schema(),
            JsonSchema::String { .. }
        ));
        assert!(matches!(i32::to_json_schema(), JsonSchema::Integer { .. }));
        assert!(matches!(f64::to_json_schema(), JsonSchema::Number { .. }));
        assert!(matches!(bool::to_json_schema(), JsonSchema::Boolean { .. }));
    }
}
