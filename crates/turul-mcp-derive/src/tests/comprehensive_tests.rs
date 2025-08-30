//! Comprehensive unit tests for all macro types and tool generation
//!
//! This module tests all aspects of the MCP derive framework including:
//! - Derive macro generation (#[derive(McpTool)])
//! - Function attribute macros (#[mcp_tool])
//! - Declarative macros (tool!, resource!, schema_for!)
//! - Parameter extraction and validation
//! - Schema generation for various types
//! - Error handling and edge cases

use crate::utils::{extract_tool_meta, extract_param_meta, type_to_schema, generate_param_extraction};
use syn::{parse_quote, DeriveInput, Type, Ident};

/// Helper function to normalize generated code strings for testing
fn normalize_generated_code(code: &str) -> String {
    code.chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_lowercase()
}

/// Helper function to check if generated code contains patterns (whitespace-insensitive)
fn contains_pattern(code: &str, pattern: &str) -> bool {
    let normalized_code = normalize_generated_code(code);
    let normalized_pattern = normalize_generated_code(pattern);
    normalized_code.contains(&normalized_pattern)
}

/// Test suite for tool metadata extraction
#[cfg(test)]
mod tool_meta_tests {
    use super::*;

    #[test]
    fn test_extract_simple_tool_meta() {
        let input: DeriveInput = parse_quote! {
            #[tool(name = "calculator", description = "A simple calculator")]
            struct Calculator;
        };

        let meta = extract_tool_meta(&input.attrs).unwrap();
        assert_eq!(meta.name, "calculator");
        assert_eq!(meta.description, "A simple calculator");
    }

    #[test]
    fn test_extract_tool_meta_with_complex_description() {
        let input: DeriveInput = parse_quote! {
            #[tool(name = "complex_tool", description = "A tool with a very long description that spans multiple lines and contains special characters like @#$%")]
            struct ComplexTool;
        };

        let meta = extract_tool_meta(&input.attrs).unwrap();
        assert_eq!(meta.name, "complex_tool");
        assert!(meta.description.contains("special characters"));
    }

    #[test]
    fn test_missing_name_attribute() {
        let input: DeriveInput = parse_quote! {
            #[tool(description = "Missing name")]
            struct MissingName;
        };

        let result = extract_tool_meta(&input.attrs);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Missing 'name'"));
    }

    #[test]
    fn test_missing_description_attribute() {
        let input: DeriveInput = parse_quote! {
            #[tool(name = "missing_desc")]
            struct MissingDescription;
        };

        let result = extract_tool_meta(&input.attrs);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Missing 'description'"));
    }

    #[test]
    fn test_no_tool_attribute() {
        let input: DeriveInput = parse_quote! {
            struct NoAttribute;
        };

        let result = extract_tool_meta(&input.attrs);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Missing #[tool("));
    }
}

/// Test suite for parameter metadata extraction
#[cfg(test)]
mod param_meta_tests {
    use super::*;
    use syn::{Field, parse_quote};

    #[test]
    fn test_simple_param_meta() {
        let field: Field = parse_quote! {
            #[param(description = "A simple parameter")]
            value: f64
        };

        let meta = extract_param_meta(&field.attrs).unwrap();
        assert_eq!(meta.description, Some("A simple parameter".to_string()));
        assert!(!meta.optional);
        assert!(meta.min.is_none());
        assert!(meta.max.is_none());
    }

    #[test]
    fn test_optional_param_meta() {
        let field: Field = parse_quote! {
            #[param(description = "Optional parameter", optional)]
            optional_value: Option<String>
        };

        let meta = extract_param_meta(&field.attrs).unwrap();
        assert_eq!(meta.description, Some("Optional parameter".to_string()));
        assert!(meta.optional);
    }

    #[test]
    fn test_param_with_min_max() {
        let field: Field = parse_quote! {
            #[param(description = "Constrained number", min = 0.0, max = 100.0)]
            percentage: f64
        };

        let meta = extract_param_meta(&field.attrs).unwrap();
        assert_eq!(meta.description, Some("Constrained number".to_string()));
        assert_eq!(meta.min, Some(0.0));
        assert_eq!(meta.max, Some(100.0));
    }

    #[test]
    fn test_param_with_all_attributes() {
        let field: Field = parse_quote! {
            #[param(description = "Full param", optional, min = -10.0, max = 10.0)]
            full_param: Option<f64>
        };

        let meta = extract_param_meta(&field.attrs).unwrap();
        assert_eq!(meta.description, Some("Full param".to_string()));
        assert!(meta.optional);
        assert_eq!(meta.min, Some(-10.0));
        assert_eq!(meta.max, Some(10.0));
    }

    #[test]
    fn test_no_param_attributes() {
        let field: Field = parse_quote! {
            bare_field: String
        };

        let meta = extract_param_meta(&field.attrs).unwrap();
        assert!(meta.description.is_none());
        assert!(!meta.optional);
        assert!(meta.min.is_none());
        assert!(meta.max.is_none());
    }
}

/// Test suite for JSON schema generation
#[cfg(test)]
mod schema_generation_tests {
    use super::*;
    use crate::utils::ParamMeta;

    #[test]
    fn test_string_schema_generation() {
        let ty: Type = parse_quote! { String };
        let meta = ParamMeta {
            description: Some("A string parameter".to_string()),
            ..Default::default()
        };

        let schema = type_to_schema(&ty, &meta);
        let schema_str = schema.to_string();
        
        assert!(contains_pattern(&schema_str, "JsonSchema::string"));
        assert!(contains_pattern(&schema_str, "with_description"));
    }

    #[test]
    fn test_number_schema_with_constraints() {
        let ty: Type = parse_quote! { f64 };
        let meta = ParamMeta {
            description: Some("A constrained number".to_string()),
            min: Some(0.0),
            max: Some(100.0),
            ..Default::default()
        };

        let schema = type_to_schema(&ty, &meta);
        let schema_str = schema.to_string();
        
        assert!(contains_pattern(&schema_str, "JsonSchema::number()"));
        assert!(schema_str.contains("with_minimum"));
        assert!(schema_str.contains("with_maximum"));
    }

    #[test]
    fn test_integer_schema_generation() {
        let ty: Type = parse_quote! { i32 };
        let meta = ParamMeta {
            description: Some("An integer".to_string()),
            ..Default::default()
        };

        let schema = type_to_schema(&ty, &meta);
        let schema_str = schema.to_string();
        
        assert!(contains_pattern(&schema_str, "JsonSchema::integer()"));
    }

    #[test]
    fn test_boolean_schema_generation() {
        let ty: Type = parse_quote! { bool };
        let meta = ParamMeta {
            description: Some("A boolean flag".to_string()),
            ..Default::default()
        };

        let schema = type_to_schema(&ty, &meta);
        let schema_str = schema.to_string();
        
        assert!(contains_pattern(&schema_str, "JsonSchema::boolean()"));
    }

    #[test]
    fn test_unknown_type_fallback() {
        let ty: Type = parse_quote! { MyCustomType };
        let meta = ParamMeta::default();

        let schema = type_to_schema(&ty, &meta);
        let schema_str = schema.to_string();
        
        // Should fall back to string schema for unknown types
        assert!(contains_pattern(&schema_str, "JsonSchema::string()"));
    }
}

/// Test suite for parameter extraction code generation
#[cfg(test)]
mod param_extraction_tests {
    use super::*;

    #[test]
    fn test_required_string_extraction() {
        let field_name: Ident = parse_quote! { message };
        let field_type: Type = parse_quote! { String };

        let extraction = generate_param_extraction(&field_name, &field_type, false);
        let extraction_str = extraction.to_string();
        
        assert!(extraction_str.contains("let message"));
        assert!(contains_pattern(&extraction_str, "as_str()"));
        assert!(contains_pattern(&extraction_str, "to_string()"));
        assert!(extraction_str.contains("invalid_param_type"));
    }

    #[test]
    fn test_required_number_extraction() {
        let field_name: Ident = parse_quote! { value };
        let field_type: Type = parse_quote! { f64 };

        let extraction = generate_param_extraction(&field_name, &field_type, false);
        let extraction_str = extraction.to_string();
        
        assert!(extraction_str.contains("let value"));
        assert!(contains_pattern(&extraction_str, "as_f64()"));
        assert!(extraction_str.contains("invalid_param_type"));
    }

    #[test]
    fn test_optional_field_extraction() {
        let field_name: Ident = parse_quote! { optional_value };
        let field_type: Type = parse_quote! { String };

        let extraction = generate_param_extraction(&field_name, &field_type, true);
        let extraction_str = extraction.to_string();
        
        assert!(extraction_str.contains("let optional_value"));
        assert!(contains_pattern(&extraction_str, "Option<String>"));
        assert!(!extraction_str.contains("Missing or invalid parameter"));
    }

    #[test]
    fn test_option_type_extraction() {
        let field_name: Ident = parse_quote! { maybe_value };
        let field_type: Type = parse_quote! { Option<String> };

        let extraction = generate_param_extraction(&field_name, &field_type, false);
        let extraction_str = extraction.to_string();
        
        assert!(extraction_str.contains("let maybe_value"));
        assert!(contains_pattern(&extraction_str, "Option<String>"));
    }
}

/// Test suite for derive macro implementation
#[cfg(test)]
mod derive_macro_tests {
    use super::*;
    use crate::tool_derive::derive_mcp_tool_impl;

    #[test]
    fn test_simple_derive_macro() {
        let input: DeriveInput = parse_quote! {
            #[tool(name = "add", description = "Add two numbers")]
            struct AddTool {
                #[param(description = "First number")]
                a: f64,
                #[param(description = "Second number")]
                b: f64,
            }
        };

        let result = derive_mcp_tool_impl(input);
        assert!(result.is_ok());
        
        let generated = result.unwrap();
        let generated_str = generated.to_string();
        
        // Check that the implementation contains expected elements (use contains_pattern for whitespace-insensitive matching)
        assert!(contains_pattern(&generated_str, "impl turul_mcp_server::McpTool"));
        assert!(contains_pattern(&generated_str, "fn name(&self)"));
        assert!(contains_pattern(&generated_str, "fn description(&self)"));
        assert!(contains_pattern(&generated_str, "fn input_schema(&self)"));
        assert!(contains_pattern(&generated_str, "async fn call"));
    }

    #[test]
    fn test_derive_with_optional_fields() {
        let input: DeriveInput = parse_quote! {
            #[tool(name = "flexible", description = "Tool with optional parameters")]
            struct FlexibleTool {
                #[param(description = "Required parameter")]
                required: String,
                #[param(description = "Optional parameter", optional)]
                optional: f64,
            }
        };

        let result = derive_mcp_tool_impl(input);
        assert!(result.is_ok());
        
        let generated = result.unwrap();
        let generated_str = generated.to_string();
        
        // Should handle both required and optional parameters
        assert!(generated_str.contains("required"));
        assert!(generated_str.contains("optional"));
    }

    #[test]
    fn test_derive_with_constraints() {
        let input: DeriveInput = parse_quote! {
            #[tool(name = "constrained", description = "Tool with parameter constraints")]
            struct ConstrainedTool {
                #[param(description = "Percentage value", min = 0.0, max = 100.0)]
                percentage: f64,
            }
        };

        let result = derive_mcp_tool_impl(input);
        assert!(result.is_ok());
        
        let generated = result.unwrap();
        let generated_str = generated.to_string();
        
        // Should include constraint information in schema
        assert!(generated_str.contains("with_minimum"));
        assert!(generated_str.contains("with_maximum"));
    }

    #[test]
    fn test_derive_with_enum() {
        let input: DeriveInput = parse_quote! {
            #[tool(name = "bad", description = "This should fail")]
            enum BadTool {
                Variant1,
                Variant2,
            }
        };

        let result = derive_mcp_tool_impl(input);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("can only be derived for structs"));
    }

    #[test]
    fn test_derive_with_tuple_struct() {
        let input: DeriveInput = parse_quote! {
            #[tool(name = "tuple", description = "Tuple struct")]
            struct TupleTool(String, f64);
        };

        let result = derive_mcp_tool_impl(input);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("named fields"));
    }
}

/// Test suite for function attribute macro
#[cfg(test)]
mod function_attribute_tests {
    use super::*;
    use crate::tool_attr::mcp_tool_impl;
    use syn::{ItemFn, punctuated::Punctuated, Token, Meta};

    #[test]
    fn test_simple_function_attribute() {
        let args: Punctuated<Meta, Token![,]> = parse_quote! {
            name = "multiply", description = "Multiply two numbers"
        };

        let input: ItemFn = parse_quote! {
            async fn multiply(a: f64, b: f64) -> Result<String, String> {
                Ok(format!("{} × {} = {}", a, b, a * b))
            }
        };

        let result = mcp_tool_impl(args, input);
        assert!(result.is_ok());
        
        let generated = result.unwrap();
        let generated_str = generated.to_string();
        
        // Check for struct and tool implementation (no module structure anymore)
        assert!(generated_str.contains("struct Multiply"));
        assert!(contains_pattern(&generated_str, "impl turul_mcp_server::McpTool"));
        assert!(contains_pattern(&generated_str, "fn name(&self)"));
        assert!(generated_str.contains("multiply"));
    }

    #[test]
    fn test_function_with_parameter_attributes() {
        let args: Punctuated<Meta, Token![,]> = parse_quote! {
            name = "divide", description = "Divide two numbers"
        };

        let input: ItemFn = parse_quote! {
            async fn divide(
                #[param(description = "Dividend")] dividend: f64,
                #[param(description = "Divisor")] divisor: f64
            ) -> Result<String, String> {
                if divisor == 0.0 {
                    Err("Division by zero".to_string())
                } else {
                    Ok(format!("{} ÷ {} = {}", dividend, divisor, dividend / divisor))
                }
            }
        };

        let result = mcp_tool_impl(args, input);
        assert!(result.is_ok());
        
        let generated = result.unwrap();
        let generated_str = generated.to_string();
        
        // Should include parameter descriptions in schema
        assert!(generated_str.contains("Dividend"));
        assert!(generated_str.contains("Divisor"));
    }

    #[test]
    fn test_missing_name_in_function_attribute() {
        let args: Punctuated<Meta, Token![,]> = parse_quote! {
            description = "Missing name"
        };

        let input: ItemFn = parse_quote! {
            async fn test_fn() -> Result<String, String> {
                Ok("test".to_string())
            }
        };

        let result = mcp_tool_impl(args, input);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Missing 'name'"));
    }

    #[test]
    fn test_missing_description_in_function_attribute() {
        let args: Punctuated<Meta, Token![,]> = parse_quote! {
            name = "test"
        };

        let input: ItemFn = parse_quote! {
            async fn test_fn() -> Result<String, String> {
                Ok("test".to_string())
            }
        };

        let result = mcp_tool_impl(args, input);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Missing 'description'"));
    }
}

/// Test suite for complex scenarios and edge cases
#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_very_long_parameter_names() {
        let input: DeriveInput = parse_quote! {
            #[tool(name = "long_params", description = "Tool with very long parameter names")]
            struct LongParamsTool {
                #[param(description = "A parameter with an extremely long name that might cause issues")]
                this_is_a_very_long_parameter_name_that_might_cause_problems_in_code_generation: String,
            }
        };

        let result = crate::tool_derive::derive_mcp_tool_impl(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_special_characters_in_descriptions() {
        let input: DeriveInput = parse_quote! {
            #[tool(name = "special", description = "Tool with special chars: @#$%^&*()[]{}|\\:;\"'<>,.?/")]
            struct SpecialCharsTool {
                #[param(description = "Parameter with unicode: 🚀 🎉 ✨ and newlines\nand tabs\t")]
                special_param: String,
            }
        };

        let result = crate::tool_derive::derive_mcp_tool_impl(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_empty_struct() {
        let input: DeriveInput = parse_quote! {
            #[tool(name = "empty", description = "Tool with no parameters")]
            struct EmptyTool {
            }
        };

        let result = crate::tool_derive::derive_mcp_tool_impl(input);
        assert!(result.is_ok());
        
        let generated = result.unwrap();
        let generated_str = generated.to_string();
        
        // Should still generate valid implementation even with no fields
        assert!(contains_pattern(&generated_str, "impl turul_mcp_server::McpTool"));
    }

    #[test]
    fn test_many_parameters() {
        let input: DeriveInput = parse_quote! {
            #[tool(name = "many_params", description = "Tool with many parameters")]
            struct ManyParamsTool {
                #[param(description = "First parameter")]
                param1: String,
                #[param(description = "Second parameter")]
                param2: f64,
                #[param(description = "Third parameter")]
                param3: i32,
                #[param(description = "Fourth parameter")]
                param4: bool,
                #[param(description = "Fifth parameter", optional)]
                param5: Option<String>,
                #[param(description = "Sixth parameter", min = 0.0, max = 100.0)]
                param6: f64,
                #[param(description = "Seventh parameter")]
                param7: u64,
                #[param(description = "Eighth parameter")]
                param8: i8,
            }
        };

        let result = crate::tool_derive::derive_mcp_tool_impl(input);
        assert!(result.is_ok());
        
        let generated = result.unwrap();
        let generated_str = generated.to_string();
        
        // Should handle all parameters correctly
        for i in 1..=8 {
            assert!(generated_str.contains(&format!("param{}", i)));
        }
    }
}

/// Test utilities and helper functions
#[cfg(test)]
pub mod test_utilities {

    /// Helper function to check if generated code contains expected patterns
    pub fn assert_contains_patterns(code: &str, patterns: &[&str]) {
        for pattern in patterns {
            assert!(
                code.contains(pattern),
                "Generated code does not contain expected pattern: '{}'",
                pattern
            );
        }
    }

    /// Helper function to check if generated code is syntactically valid
    pub fn assert_valid_rust_syntax(code: &str) {
        let result = syn::parse_str::<syn::File>(code);
        assert!(
            result.is_ok(),
            "Generated code is not valid Rust syntax: {:?}",
            result.unwrap_err()
        );
    }

    #[test]
    fn test_helper_functions() {
        let valid_code = "fn test() {}";
        assert_valid_rust_syntax(valid_code);
        
        assert_contains_patterns(valid_code, &["fn", "test"]);
    }
}