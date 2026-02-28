//! Schemars helpers for auto-generating tool schemas
//!
//! This module provides utilities for converting schemars-generated JSON Schemas
//! into MCP ToolSchema format.
//!
//! # Example
//!
//! ```rust
//! use turul_mcp_builders::ToolSchemaExt;
//! use turul_mcp_protocol::ToolSchema;
//! use schemars::{JsonSchema, schema_for};
//! use serde::Serialize;
//!
//! #[derive(Serialize, JsonSchema)]
//! struct CalculatorOutput {
//!     result: f64,
//!     operation: String,
//! }
//!
//! let json_schema = schema_for!(CalculatorOutput);
//! let tool_schema = ToolSchema::from_schemars(json_schema)
//!     .expect("Valid schema");
//! ```

use serde_json::Value;
use std::collections::HashMap;
use turul_mcp_protocol::ToolSchema;
use turul_mcp_protocol::schema::JsonSchema;

/// Convert a serde_json::Value from schemars to MCP's JsonSchema enum
///
/// This is a "lossy but safe" converter that:
/// - Handles basic types: string, number, integer, boolean, object, array
/// - Recursively converts nested properties and array items
/// - Returns generic Object for complex patterns (anyOf, oneOf, etc.)
/// - **Never panics** - always returns a valid JsonSchema
pub fn convert_value_to_json_schema(value: &Value) -> JsonSchema {
    convert_value_to_json_schema_with_defs(value, &HashMap::new())
}

/// Convert a serde_json::Value from schemars to MCP's JsonSchema enum with $ref resolution
///
/// This version accepts a definitions map to resolve $ref references for nested types.
/// Use this when converting a schemars RootSchema that includes definitions.
///
/// # Arguments
///
/// * `value` - The JSON schema value to convert
/// * `definitions` - Map of type names to their schema definitions for $ref resolution
///
/// # Returns
///
/// A converted JsonSchema that:
/// - Handles basic types: string, number, integer, boolean, object, array
/// - Recursively converts nested properties and array items
/// - Resolves $ref references to definitions for nested types
/// - Returns generic Object for unresolvable patterns (anyOf, oneOf, etc.)
/// - **Never panics** - always returns a valid JsonSchema
pub fn convert_value_to_json_schema_with_defs(
    value: &Value,
    definitions: &HashMap<String, Value>,
) -> JsonSchema {
    // Handle boolean schemas (rare, but valid in JSON Schema)
    if let Some(b) = value.as_bool() {
        // true = accept anything, false = accept nothing
        // Both represented as generic objects
        return JsonSchema::Object {
            description: None,
            properties: None,
            required: None,
            additional_properties: Some(b),
        };
    }

    // Must be an object schema
    let obj = match value.as_object() {
        Some(o) => o,
        None => {
            // Not an object or boolean - return generic object
            return JsonSchema::Object {
                description: None,
                properties: None,
                required: None,
                additional_properties: None,
            };
        }
    };

    // Handle $ref - resolve from definitions
    if let Some(ref_path) = obj.get("$ref").and_then(|v| v.as_str()) {
        // Extract definition name from "#/definitions/TypeName" or "#/$defs/TypeName"
        let def_name = ref_path
            .strip_prefix("#/definitions/")
            .or_else(|| ref_path.strip_prefix("#/$defs/"));

        if let Some(name) = def_name
            && let Some(def_schema) = definitions.get(name)
        {
            // Recursively convert the referenced definition
            return convert_value_to_json_schema_with_defs(def_schema, definitions);
        }
        // Couldn't resolve reference - fall back to generic object
        return JsonSchema::Object {
            description: obj
                .get("description")
                .and_then(|v| v.as_str())
                .map(String::from),
            properties: None,
            required: None,
            additional_properties: None,
        };
    }

    // Handle anyOf - common for Option<T> which generates anyOf: [T, null]
    if let Some(any_of) = obj.get("anyOf").and_then(|v| v.as_array()) {
        // Look for the non-null schema in the anyOf array
        for schema in any_of {
            // Skip null schemas
            if let Some(obj) = schema.as_object() {
                if let Some(t) = obj.get("type")
                    && t.as_str() == Some("null")
                {
                    continue; // Skip null type
                }
                // Found non-null schema - convert it
                return convert_value_to_json_schema_with_defs(schema, definitions);
            }
        }
        // All schemas were null or couldn't parse - fall back to generic object
        return JsonSchema::Object {
            description: obj
                .get("description")
                .and_then(|v| v.as_str())
                .map(String::from),
            properties: None,
            required: None,
            additional_properties: None,
        };
    }

    // Get the type field - can be string or array of strings
    let schema_type = obj
        .get("type")
        .and_then(|v| {
            if let Some(s) = v.as_str() {
                // Single type as string
                Some(s.to_string())
            } else if let Some(arr) = v.as_array() {
                // Array of types (e.g., ["string", "null"] for Option<String>)
                // Find the non-null type
                for type_val in arr {
                    if let Some(t) = type_val.as_str()
                        && t != "null"
                    {
                        return Some(t.to_string());
                    }
                }
                None
            } else {
                None
            }
        })
        .or_else(|| {
            // If no type but has properties, assume object
            if obj.contains_key("properties") {
                Some("object".to_string())
            } else {
                None
            }
        });

    let schema_type = schema_type.as_deref();
    // Note: Unknown schema types fall back to generic object

    // Convert based on type
    match schema_type {
        Some("string") => JsonSchema::String {
            description: obj
                .get("description")
                .and_then(|v| v.as_str())
                .map(String::from),
            pattern: obj
                .get("pattern")
                .and_then(|v| v.as_str())
                .map(String::from),
            min_length: obj.get("minLength").and_then(|v| v.as_u64()),
            max_length: obj.get("maxLength").and_then(|v| v.as_u64()),
            enum_values: obj.get("enum").and_then(|v| {
                v.as_array().and_then(|arr| {
                    arr.iter()
                        .map(|v| v.as_str().map(String::from))
                        .collect::<Option<Vec<_>>>()
                })
            }),
        },

        Some("number") => JsonSchema::Number {
            description: obj
                .get("description")
                .and_then(|v| v.as_str())
                .map(String::from),
            minimum: obj.get("minimum").and_then(|v| v.as_f64()),
            maximum: obj.get("maximum").and_then(|v| v.as_f64()),
        },

        Some("integer") => JsonSchema::Integer {
            description: obj
                .get("description")
                .and_then(|v| v.as_str())
                .map(String::from),
            minimum: obj.get("minimum").and_then(|v| v.as_i64()),
            maximum: obj.get("maximum").and_then(|v| v.as_i64()),
        },

        Some("boolean") => JsonSchema::Boolean {
            description: obj
                .get("description")
                .and_then(|v| v.as_str())
                .map(String::from),
        },

        Some("array") => {
            // Recursively convert array items
            let items = obj
                .get("items")
                .map(|v| Box::new(convert_value_to_json_schema_with_defs(v, definitions)));

            JsonSchema::Array {
                description: obj
                    .get("description")
                    .and_then(|v| v.as_str())
                    .map(String::from),
                items,
                min_items: obj.get("minItems").and_then(|v| v.as_u64()),
                max_items: obj.get("maxItems").and_then(|v| v.as_u64()),
            }
        }

        Some("object") => {
            // Recursively convert properties
            let properties = obj
                .get("properties")
                .and_then(|v| v.as_object())
                .map(|props| {
                    props
                        .iter()
                        .map(|(k, v)| {
                            (
                                k.clone(),
                                convert_value_to_json_schema_with_defs(v, definitions),
                            )
                        })
                        .collect::<HashMap<_, _>>()
                });

            // Get required fields
            let required = obj.get("required").and_then(|v| v.as_array()).map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            });

            JsonSchema::Object {
                description: obj
                    .get("description")
                    .and_then(|v| v.as_str())
                    .map(String::from),
                properties,
                required,
                additional_properties: obj.get("additionalProperties").and_then(|v| v.as_bool()),
            }
        }

        _ => {
            // Unknown type, $ref, anyOf, oneOf, allOf, etc.
            // Return generic object (lossy but safe)
            JsonSchema::Object {
                description: obj
                    .get("description")
                    .and_then(|v| v.as_str())
                    .map(String::from),
                properties: None,
                required: None,
                additional_properties: None,
            }
        }
    }
}

/// Extension trait for ToolSchema to support schemars conversion
///
/// This trait is automatically implemented for `ToolSchema`, providing the
/// `from_schemars()` method for converting schemars schemas to MCP format.
pub trait ToolSchemaExt {
    /// Convert a schemars JSON Schema to MCP ToolSchema
    ///
    /// This enables auto-generating tool output schemas from Rust types using the
    /// `schemars` crate's `JsonSchema` derive macro.
    ///
    /// # Arguments
    ///
    /// * `schema` - A schemars Schema generated via `schema_for!()`
    ///
    /// # Returns
    ///
    /// * `Ok(ToolSchema)` - Successfully converted schema
    /// * `Err(String)` - Conversion error message
    ///
    /// # Example
    ///
    /// ```rust
    /// use turul_mcp_builders::ToolSchemaExt;
    /// use turul_mcp_protocol::ToolSchema;
    /// use schemars::{JsonSchema, schema_for};
    /// use serde::Serialize;
    /// use std::sync::OnceLock;
    ///
    /// #[derive(Serialize, JsonSchema)]
    /// struct Output {
    ///     result: f64,
    ///     timestamp: String,
    /// }
    ///
    /// // In your HasOutputSchema implementation:
    /// fn get_output_schema() -> &'static ToolSchema {
    ///     static SCHEMA: OnceLock<ToolSchema> = OnceLock::new();
    ///     SCHEMA.get_or_init(|| {
    ///         let json_schema = schema_for!(Output);
    ///         ToolSchema::from_schemars(json_schema)
    ///             .expect("Valid schema")
    ///     })
    /// }
    /// ```
    fn from_schemars(schema: schemars::Schema) -> Result<Self, String>
    where
        Self: Sized;
}

impl ToolSchemaExt for ToolSchema {
    fn from_schemars(schema: schemars::Schema) -> Result<Self, String> {
        let json_value = serde_json::to_value(schema)
            .map_err(|e| format!("Failed to serialize schemars schema: {}", e))?;

        let obj = json_value
            .as_object()
            .ok_or_else(|| "Schema is not an object".to_string())?;

        // Validate root is an object schema (ToolSchema requires type: "object")
        let is_object = obj.get("type").is_some_and(|v| {
            v.as_str() == Some("object")
                || v.as_array()
                    .is_some_and(|arr| arr.iter().any(|t| t.as_str() == Some("object")))
        }) || obj.contains_key("properties");

        if !is_object {
            return Err("ToolSchema requires an object schema (type: \"object\")".to_string());
        }

        // Extract definitions for $ref resolution — merge both $defs and definitions
        let mut definitions: HashMap<String, Value> = HashMap::new();
        for key in ["$defs", "definitions"] {
            if let Some(defs) = obj.get(key).and_then(|v| v.as_object()) {
                definitions.extend(defs.iter().map(|(k, v)| (k.clone(), v.clone())));
            }
        }

        // Convert each property using the centralized converter
        let properties = obj
            .get("properties")
            .and_then(|v| v.as_object())
            .map(|props| {
                props
                    .iter()
                    .map(|(k, v)| {
                        (
                            k.clone(),
                            convert_value_to_json_schema_with_defs(v, &definitions),
                        )
                    })
                    .collect()
            });

        let required = obj
            .get("required")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            });

        // Preserve remaining top-level fields (description, title, additionalProperties, etc.)
        let reserved = [
            "type",
            "properties",
            "required",
            "$defs",
            "definitions",
            "$schema",
        ];
        let additional: HashMap<String, Value> = obj
            .iter()
            .filter(|(k, _)| !reserved.contains(&k.as_str()))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        Ok(ToolSchema {
            schema_type: "object".to_string(),
            properties,
            required,
            additional,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use schemars::{JsonSchema, schema_for};
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, JsonSchema)]
    struct TestOutput {
        value: i32,
        message: String,
    }

    #[test]
    fn test_from_schemars_basic() {
        let json_schema = schema_for!(TestOutput);
        let result = ToolSchema::from_schemars(json_schema);

        assert!(result.is_ok(), "Schema conversion should succeed");
        let tool_schema = result.unwrap();
        assert_eq!(tool_schema.schema_type, "object");
    }

    #[test]
    fn test_from_schemars_with_optional_field() {
        #[derive(Serialize, Deserialize, JsonSchema)]
        struct OutputWithOptional {
            required_field: String,
            #[serde(skip_serializing_if = "Option::is_none")]
            optional_field: Option<i32>,
        }

        let json_schema = schema_for!(OutputWithOptional);
        let result = ToolSchema::from_schemars(json_schema);

        assert!(
            result.is_ok(),
            "Schema with optional fields should convert successfully"
        );
        let schema = result.unwrap();
        assert_eq!(schema.schema_type, "object");
        assert!(schema.properties.is_some());
        let props = schema.properties.as_ref().unwrap();
        assert!(props.contains_key("required_field"));
        assert!(props.contains_key("optional_field"));
    }

    #[test]
    fn test_from_schemars_anyof_null() {
        #[derive(Serialize, Deserialize, JsonSchema)]
        struct Inner {
            x: i32,
        }

        #[derive(Serialize, Deserialize, JsonSchema)]
        struct WithOptionalNested {
            name: String,
            inner: Option<Inner>,
        }

        let json_schema = schema_for!(WithOptionalNested);
        let result = ToolSchema::from_schemars(json_schema);

        assert!(
            result.is_ok(),
            "Schema with anyOf/null optional nested struct should convert: {:?}",
            result.err()
        );
        let schema = result.unwrap();
        assert_eq!(schema.schema_type, "object");
        let props = schema.properties.as_ref().unwrap();
        assert!(props.contains_key("name"));
        assert!(props.contains_key("inner"));
    }

    #[test]
    fn test_from_schemars_with_nested_ref() {
        #[derive(Serialize, Deserialize, JsonSchema)]
        struct Nested {
            value: f64,
        }

        #[derive(Serialize, Deserialize, JsonSchema)]
        struct WithNested {
            label: String,
            nested: Nested,
        }

        let json_schema = schema_for!(WithNested);
        let result = ToolSchema::from_schemars(json_schema);

        assert!(
            result.is_ok(),
            "Schema with $ref nested struct should convert: {:?}",
            result.err()
        );
        let schema = result.unwrap();
        assert_eq!(schema.schema_type, "object");
        let props = schema.properties.as_ref().unwrap();
        assert!(props.contains_key("label"));
        assert!(props.contains_key("nested"));
    }

    #[test]
    fn test_from_schemars_with_legacy_definitions() {
        // Construct a schema using "definitions" (not "$defs") to test backward compat
        let schema_json = serde_json::json!({
            "type": "object",
            "properties": {
                "item": { "$ref": "#/definitions/Item" }
            },
            "required": ["item"],
            "definitions": {
                "Item": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "integer" }
                    },
                    "required": ["id"]
                }
            }
        });

        let schema: schemars::Schema =
            serde_json::from_value(schema_json).expect("valid schemars schema");
        let result = ToolSchema::from_schemars(schema);

        assert!(
            result.is_ok(),
            "Schema with legacy definitions should convert: {:?}",
            result.err()
        );
        let tool_schema = result.unwrap();
        assert_eq!(tool_schema.schema_type, "object");
        let props = tool_schema.properties.as_ref().unwrap();
        assert!(props.contains_key("item"));
    }

    #[test]
    fn test_from_schemars_rejects_non_object() {
        let json_schema = schema_for!(String);
        let result = ToolSchema::from_schemars(json_schema);

        assert!(
            result.is_err(),
            "Non-object root schema should be rejected"
        );
        assert!(result
            .unwrap_err()
            .contains("ToolSchema requires an object schema"));
    }
}
