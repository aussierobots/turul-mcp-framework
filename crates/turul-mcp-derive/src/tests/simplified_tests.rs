//! Simplified unit tests for core macro functionality
//!
//! This module focuses on testing the core functionality of the MCP derive framework
//! with robust assertions that work reliably across different formatting.

use crate::utils::{ParamMeta, extract_param_meta, extract_tool_meta};
use syn::{DeriveInput, parse_quote};

/// Test suite for basic functionality that we know works
#[cfg(test)]
mod basic_tests {
    use super::*;

    #[test]
    fn test_tool_meta_extraction_works() {
        let input: DeriveInput = parse_quote! {
            #[tool(name = "test_tool", description = "A test tool")]
            struct TestTool;
        };

        let result = extract_tool_meta(&input.attrs);
        assert!(result.is_ok());

        let meta = result.unwrap();
        assert_eq!(meta.name, "test_tool");
        assert_eq!(meta.description, "A test tool");
    }

    #[test]
    fn test_param_meta_extraction_works() {
        let field: syn::Field = parse_quote! {
            #[param(description = "Test parameter")]
            test_field: String
        };

        let result = extract_param_meta(&field.attrs);
        assert!(result.is_ok());

        let meta = result.unwrap();
        assert_eq!(meta.description, Some("Test parameter".to_string()));
        assert!(!meta.optional);
    }

    #[test]
    fn test_param_meta_with_optional() {
        let field: syn::Field = parse_quote! {
            #[param(description = "Optional parameter", optional)]
            optional_field: Option<String>
        };

        let result = extract_param_meta(&field.attrs);
        assert!(result.is_ok());

        let meta = result.unwrap();
        assert_eq!(meta.description, Some("Optional parameter".to_string()));
        assert!(meta.optional);
    }

    #[test]
    fn test_param_meta_with_constraints() {
        let field: syn::Field = parse_quote! {
            #[param(description = "Constrained parameter", min = 0.0, max = 100.0)]
            constrained_field: f64
        };

        let result = extract_param_meta(&field.attrs);
        assert!(result.is_ok());

        let meta = result.unwrap();
        assert_eq!(meta.description, Some("Constrained parameter".to_string()));
        assert_eq!(meta.min, Some(0.0));
        assert_eq!(meta.max, Some(100.0));
    }

    #[test]
    fn test_derive_macro_compiles() {
        let input: DeriveInput = parse_quote! {
            #[tool(name = "calculator", description = "Simple calculator")]
            struct Calculator {
                #[param(description = "First number")]
                a: f64,
                #[param(description = "Second number")]
                b: f64,
            }
        };

        let result = crate::tool_derive::derive_mcp_tool_impl(input);
        assert!(result.is_ok(), "Derive macro should compile successfully");
    }

    #[test]
    fn test_function_attribute_compiles() {
        let args: syn::punctuated::Punctuated<syn::Meta, syn::Token![,]> = parse_quote! {
            name = "test_func", description = "Test function"
        };

        let input: syn::ItemFn = parse_quote! {
            async fn test_func(input: String) -> Result<String, String> {
                Ok(format!("Processed: {}", input))
            }
        };

        let result = crate::tool_attr::mcp_tool_impl(args, input);
        assert!(
            result.is_ok(),
            "Function attribute macro should compile successfully"
        );
    }

    #[test]
    fn test_error_handling_missing_name() {
        let input: DeriveInput = parse_quote! {
            #[tool(description = "Missing name")]
            struct MissingName;
        };

        let result = extract_tool_meta(&input.attrs);
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.to_lowercase().contains("name"));
    }

    #[test]
    fn test_error_handling_missing_description() {
        let input: DeriveInput = parse_quote! {
            #[tool(name = "missing_desc")]
            struct MissingDescription;
        };

        let result = extract_tool_meta(&input.attrs);
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.to_lowercase().contains("description"));
    }

    #[test]
    fn test_error_handling_no_tool_attribute() {
        let input: DeriveInput = parse_quote! {
            struct NoAttribute;
        };

        let result = extract_tool_meta(&input.attrs);
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.to_lowercase().contains("tool"));
    }

    #[test]
    fn test_derive_macro_rejects_enum() {
        let input: DeriveInput = parse_quote! {
            #[tool(name = "bad", description = "Should fail")]
            enum BadTool {
                Variant1,
                Variant2,
            }
        };

        let result = crate::tool_derive::derive_mcp_tool_impl(input);
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.to_lowercase().contains("struct"));
    }

    #[test]
    fn test_derive_macro_rejects_tuple_struct() {
        let input: DeriveInput = parse_quote! {
            #[tool(name = "tuple", description = "Tuple struct")]
            struct TupleTool(String, f64);
        };

        let result = crate::tool_derive::derive_mcp_tool_impl(input);
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.to_lowercase().contains("named"));
    }
}

/// Tests that verify generated code produces valid TokenStreams
#[cfg(test)]
mod generation_tests {
    use super::*;
    use crate::utils::{generate_param_extraction, type_to_schema};
    use syn::{Ident, Type};

    #[test]
    fn test_type_to_schema_generates_valid_tokens() {
        let types: Vec<Type> = vec![
            parse_quote! { String },
            parse_quote! { f64 },
            parse_quote! { i32 },
            parse_quote! { bool },
            parse_quote! { Option<String> },
        ];

        for ty in types {
            let meta = ParamMeta::default();
            let schema = type_to_schema(&ty, &meta);

            // The main test is that this doesn't panic and produces valid tokens
            let token_string = schema.to_string();
            assert!(
                !token_string.is_empty(),
                "Schema generation should produce non-empty output"
            );
        }
    }

    #[test]
    fn test_param_extraction_generates_valid_tokens() {
        let field_name: Ident = parse_quote! { test_field };
        let field_types: Vec<Type> = vec![
            parse_quote! { String },
            parse_quote! { f64 },
            parse_quote! { i32 },
            parse_quote! { bool },
            parse_quote! { Option<String> },
        ];

        for ty in field_types {
            for optional in [true, false] {
                let extraction = generate_param_extraction(&field_name, &ty, optional);

                // The main test is that this doesn't panic and produces valid tokens
                let token_string = extraction.to_string();
                assert!(
                    !token_string.is_empty(),
                    "Parameter extraction should produce non-empty output"
                );
            }
        }
    }

    #[test]
    fn test_complex_derive_macro_scenario() {
        let input: DeriveInput = parse_quote! {
            #[tool(name = "complex_tool", description = "A complex tool with many parameter types")]
            struct ComplexTool {
                #[param(description = "String parameter")]
                text: String,
                #[param(description = "Number with constraints", min = 0.0, max = 100.0)]
                percentage: f64,
                #[param(description = "Integer parameter")]
                count: i32,
                #[param(description = "Boolean flag")]
                enabled: bool,
                #[param(description = "Optional string", optional)]
                optional_text: Option<String>,
                #[param(description = "Optional number", optional)]
                optional_value: Option<f64>,
            }
        };

        let result = crate::tool_derive::derive_mcp_tool_impl(input);
        assert!(
            result.is_ok(),
            "Complex derive macro should compile successfully"
        );

        let generated = result.unwrap();
        let generated_str = generated.to_string();

        // Basic sanity checks on the generated code
        assert!(!generated_str.is_empty());
        assert!(generated_str.contains("impl"));
        assert!(generated_str.contains("McpTool"));
    }
}

/// Integration tests that verify macros work together
#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_end_to_end_simple_tool() {
        // Test that we can parse tool metadata and generate a derive macro for the same tool
        let input: DeriveInput = parse_quote! {
            #[tool(name = "add_numbers", description = "Add two numbers together")]
            struct AddTool {
                #[param(description = "First number to add")]
                a: f64,
                #[param(description = "Second number to add")]
                b: f64,
            }
        };

        // Extract metadata
        let meta_result = extract_tool_meta(&input.attrs);
        assert!(meta_result.is_ok());
        let meta = meta_result.unwrap();
        assert_eq!(meta.name, "add_numbers");
        assert_eq!(meta.description, "Add two numbers together");

        // Generate derive macro
        let derive_result = crate::tool_derive::derive_mcp_tool_impl(input);
        assert!(derive_result.is_ok());

        let generated = derive_result.unwrap();
        let generated_str = generated.to_string();

        // Verify the generated code contains expected elements
        assert!(generated_str.contains("add_numbers"));
        assert!(generated_str.contains("Add two numbers together"));
    }

    #[test]
    fn test_end_to_end_function_tool() {
        let args: syn::punctuated::Punctuated<syn::Meta, syn::Token![,]> = parse_quote! {
            name = "multiply", description = "Multiply two numbers"
        };

        let input: syn::ItemFn = parse_quote! {
            async fn multiply(
                #[param(description = "First multiplicand")] a: f64,
                #[param(description = "Second multiplicand")] b: f64
            ) -> Result<String, String> {
                Ok(format!("{} × {} = {}", a, b, a * b))
            }
        };

        let result = crate::tool_attr::mcp_tool_impl(args, input);
        assert!(result.is_ok());

        let generated = result.unwrap();
        let generated_str = generated.to_string();

        // Verify basic structure is present
        assert!(generated_str.contains("multiply"));
        assert!(generated_str.contains("McpTool"));
    }
}
