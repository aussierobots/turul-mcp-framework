//! Implementation of #[derive(McpTool)]

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, Result};

use crate::utils::{
    determine_output_field_name, extract_param_meta, extract_tool_meta,
    generate_output_schema_auto, generate_param_extraction, type_to_schema,
};

/// Auto-determine tool name from struct name (ZERO CONFIGURATION!)
/// Examples:
/// - `CalculatorTool` → `"calculator"`
/// - `FileReaderTool` → `"file_reader"`
/// - `Calculator` → `"calculator"`
fn auto_determine_tool_name(struct_name: String) -> String {
    // Remove "Tool" suffix if present
    let base_name = if struct_name.ends_with("Tool") {
        &struct_name[..struct_name.len() - 4] // Remove "Tool"
    } else {
        &struct_name
    };

    // Convert CamelCase to snake_case
    camel_to_snake_case(base_name)
}

/// Convert CamelCase to snake_case
fn camel_to_snake_case(input: &str) -> String {
    let mut result = String::new();
    for (i, ch) in input.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            result.push('_');
        }
        result.push(ch.to_lowercase().next().unwrap());
    }
    result
}

/// Convert CamelCase to readable words with spaces
fn camel_to_readable(input: &str) -> String {
    let mut result = String::new();
    for (i, ch) in input.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            result.push(' ');
        }
        result.push(ch);
    }
    result
}

pub fn derive_mcp_tool_impl(input: DeriveInput) -> Result<TokenStream> {
    let name = &input.ident;

    // AUTO-DETERMINE tool name from struct name (ZERO CONFIGURATION!)
    let auto_name = auto_determine_tool_name(name.to_string());

    // Try to extract attributes, but use auto-determined values as defaults
    let tool_meta = match extract_tool_meta(&input.attrs) {
        Ok(meta) => meta,
        Err(_) => {
            // No attributes found - use zero-configuration defaults
            crate::utils::ToolMeta {
                name: auto_name,
                description: camel_to_readable(&name.to_string()),
                output_type: None,
                output_field: None,
            }
        }
    };

    // Only support named structs for now
    let fields = match &input.data {
        Data::Struct(data_struct) => match &data_struct.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return Err(syn::Error::new_spanned(
                    name,
                    "McpTool can only be derived for structs with named fields",
                ));
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                name,
                "McpTool can only be derived for structs",
            ));
        }
    };

    // Process each field to build schema and parameter extraction
    let mut schema_properties = Vec::new();
    let mut required_fields = Vec::new();
    let mut param_extractions = Vec::new();
    let mut field_assignments = Vec::new();

    for field in fields {
        let field_name = field.ident.as_ref().unwrap();
        let field_type = &field.ty;
        let param_meta = extract_param_meta(&field.attrs)?;

        // Generate schema for this field
        let field_name_str = field_name.to_string();
        let schema = type_to_schema(field_type, &param_meta);

        schema_properties.push(quote! {
            (#field_name_str.to_string(), #schema)
        });

        // Check if field type is Option<T>
        let is_option_type = if let syn::Type::Path(type_path) = field_type {
            type_path.path.segments.len() == 1 && type_path.path.segments[0].ident == "Option"
        } else {
            false
        };

        // Only add to required fields if not explicitly optional and not Option<T>
        if !param_meta.optional && !is_option_type {
            required_fields.push(quote! {
                #field_name_str.to_string()
            });
        }

        // Generate parameter extraction code
        let extraction = generate_param_extraction(
            field_name,
            field_type,
            param_meta.optional || is_option_type,
        );
        param_extractions.push(extraction);

        if param_meta.optional && !is_option_type {
            // Field is marked optional but type is T (not Option<T>)
            // The extraction generates Option<T>, so we need to unwrap with a default
            field_assignments.push(quote! {
                #field_name: #field_name.unwrap_or_default()
            });
        } else {
            field_assignments.push(quote! {
                #field_name
            });
        }
    }

    let tool_name = &tool_meta.name;
    let tool_description = &tool_meta.description;

    // Determine the output field name consistently for both schema and runtime
    let runtime_field_name = if let Some(ref output_type) = tool_meta.output_type {
        // User specified output type - use same logic as schema generation
        determine_output_field_name(output_type, tool_meta.output_field.as_ref())
    } else {
        // Zero-config case - use custom field or default to "output"
        tool_meta
            .output_field
            .clone()
            .unwrap_or_else(|| "output".to_string())
    };

    let custom_field_name_tokens = quote! { Some(#runtime_field_name) };

    // Generate output schema with automatic schemars detection
    let output_schema_tokens = if let Some(ref output_type) = tool_meta.output_type {
        // User specified output type - use auto-detection (schemars if available, else introspection)
        generate_output_schema_auto(output_type, &runtime_field_name, Some(&input))
    } else {
        // Zero-config case - treat as Self for introspection
        // This allows tools to return Self and get detailed schemas automatically
        let self_type: syn::Type = syn::parse_quote!(Self);
        generate_output_schema_auto(&self_type, &runtime_field_name, Some(&input))
    };

    let expanded = quote! {
        #[automatically_derived]
        // Generate fine-grained trait implementations
        impl turul_mcp_builders::traits::HasBaseMetadata for #name {
            fn name(&self) -> &str {
                #tool_name
            }

            fn title(&self) -> Option<&str> {
                // TODO: Extract from tool attributes when available
                None
            }
        }

        impl turul_mcp_builders::traits::HasDescription for #name {
            fn description(&self) -> Option<&str> {
                Some(#tool_description)
            }
        }

        impl turul_mcp_builders::traits::HasInputSchema for #name {
            fn input_schema(&self) -> &turul_mcp_protocol::tools::ToolSchema {
                // Generate static schema at compile time
                static INPUT_SCHEMA: std::sync::OnceLock<turul_mcp_protocol::tools::ToolSchema> = std::sync::OnceLock::new();
                INPUT_SCHEMA.get_or_init(|| {
                    use std::collections::HashMap;
                    turul_mcp_protocol::tools::ToolSchema::object()
                        .with_properties(HashMap::from([
                            #(#schema_properties),*
                        ]))
                        .with_required(vec![
                            #(#required_fields),*
                        ])
                })
            }
        }

        impl turul_mcp_builders::traits::HasOutputSchema for #name {
            #output_schema_tokens
        }

        // Schema generation for zero-config derive macros
        impl #name {
            /// Generate schema using heuristics from struct name and common patterns
            /// Note: Zero-config mode has limitations for struct outputs - use #[tool(output = Type)] for exact schemas
            fn generate_heuristic_schema(field_name: &str, struct_name: &str) -> turul_mcp_protocol::tools::ToolSchema {
                use std::collections::HashMap;
                use turul_mcp_protocol::schema::JsonSchema;

                // For struct outputs in zero-config mode, we can't predict the exact field name
                // since it depends on the actual return type. Use a flexible schema instead.
                if struct_name.contains("Struct") || struct_name.contains("Complex") || struct_name.contains("Result") {
                    // Generate flexible object schema that accepts any properties
                    let mut schema = turul_mcp_protocol::tools::ToolSchema::object();
                    schema.additional.insert("additionalProperties".to_string(), serde_json::json!(true));
                    schema.additional.insert("description".to_string(), serde_json::json!("Complex object output (zero-config heuristic)"));
                    return schema;
                }

                // For primitive types, use specific schemas with provided field name (or "output" as default)
                let field_schema = if struct_name.contains("String") || struct_name.contains("Text") || struct_name.contains("Message") || struct_name.contains("Log") || struct_name.contains("Progress") {
                    JsonSchema::string()
                } else if struct_name.contains("Boolean") || struct_name.contains("Check") || struct_name.contains("Verify") {
                    JsonSchema::boolean()
                } else if struct_name.contains("List") || struct_name.contains("Array") || struct_name.contains("Search") || struct_name.contains("Query") || struct_name.contains("Find") || struct_name.contains("Batch") {
                    JsonSchema::Array {
                        description: Some("Array output (heuristic: tool name suggests collection)".to_string()),
                        items: None,
                        min_items: None,
                        max_items: None,
                    }
                } else if struct_name.contains("Calculator") || struct_name.contains("Counter") || struct_name.contains("Math") {
                    JsonSchema::number()
                } else {
                    // Default to flexible object schema since we can't reliably determine the type
                    // from struct names alone. This is safer than defaulting to string.
                    JsonSchema::Object {
                        description: Some("JSON object or value (zero-config fallback)".to_string()),
                        properties: None,
                        required: None,
                        additional_properties: Some(true),
                    }
                };

                turul_mcp_protocol::tools::ToolSchema::object()
                    .with_properties(HashMap::from([
                        (field_name.to_string(), field_schema)
                    ]))
                    .with_required(vec![field_name.to_string()])
            }

            /// Update schema based on actual return value (called during execution)
            fn update_schema_from_value(field_name: &str, value: &serde_json::Value) -> turul_mcp_protocol::tools::ToolSchema {
                use std::collections::HashMap;
                use turul_mcp_protocol::schema::JsonSchema;

                fn infer_schema(value: &serde_json::Value) -> JsonSchema {
                    match value {
                        serde_json::Value::Number(n) if n.is_f64() => JsonSchema::number(),
                        serde_json::Value::Number(_) => JsonSchema::integer(),
                        serde_json::Value::String(_) => JsonSchema::string(),
                        serde_json::Value::Bool(_) => JsonSchema::boolean(),
                        serde_json::Value::Array(arr) => {
                            let item_schema = arr.first().map(|item| Box::new(infer_schema(item)));
                            JsonSchema::Array {
                                description: Some("Array output".to_string()),
                                items: item_schema,
                                min_items: None,
                                max_items: None,
                            }
                        }
                        serde_json::Value::Object(obj) => {
                            let mut properties = HashMap::new();
                            let mut required = Vec::new();
                            for (key, value) in obj.iter() {
                                properties.insert(key.clone(), infer_schema(value));
                                required.push(key.clone());
                            }
                            JsonSchema::Object {
                                description: Some("Generated from runtime object structure".to_string()),
                                properties: Some(properties),
                                required: Some(required),
                                additional_properties: Some(false),
                            }
                        }
                        serde_json::Value::Null => JsonSchema::Object {
                            description: Some("JSON value".to_string()),
                            properties: None,
                            required: None,
                            additional_properties: Some(true),
                        },
                    }
                }

                let field_schema = infer_schema(value);

                turul_mcp_protocol::tools::ToolSchema::object()
                    .with_properties(HashMap::from([
                        (field_name.to_string(), field_schema)
                    ]))
                    .with_required(vec![field_name.to_string()])
            }
        }

        impl turul_mcp_builders::traits::HasAnnotations for #name {
            fn annotations(&self) -> Option<&turul_mcp_protocol::tools::ToolAnnotations> {
                // TODO: Extract from tool attributes when available
                None
            }
        }

        impl turul_mcp_builders::traits::HasToolMeta for #name {
            fn tool_meta(&self) -> Option<&std::collections::HashMap<String, serde_json::Value>> {
                None
            }
        }

        // ToolDefinition automatically implemented via trait composition!

        #[automatically_derived]
        #[async_trait::async_trait]
        impl turul_mcp_server::McpTool for #name {
            async fn call(&self, args: serde_json::Value, session: Option<turul_mcp_server::SessionContext>) -> turul_mcp_server::McpResult<turul_mcp_protocol::tools::CallToolResult> {
                use serde_json::Value;
                use turul_mcp_builders::traits::HasOutputSchema;

                // Extract parameters
                #(#param_extractions)*

                // Create instance with extracted parameters
                let instance = #name {
                    #(#field_assignments),*
                };

                // Execute with session - user's execute method now receives session
                match instance.execute(session).await {
                    Ok(result) => {
                        // Serialize result for output
                        let result_value = serde_json::to_value(&result)
                            .map_err(|e| turul_mcp_protocol::McpError::tool_execution(&format!("Serialization error: {}", e)))?;

                        // Determine field name using enhanced logic with custom field support
                        let field_name = {
                            // Use custom field name if specified during macro expansion
                            let compile_time_custom_field: Option<&str> = #custom_field_name_tokens;
                            if let Some(custom_name) = compile_time_custom_field {
                                custom_name.to_string()
                            } else {
                                // Zero-config tools ALWAYS use "output" for consistency with schema
                                // This ensures tools/list and tools/call responses match
                                "output".to_string()
                            }
                        };

                        // Wrap result to match MCP object schema format
                        let wrapped_result = serde_json::json!({&field_name: result_value});

                        // Correct schema if needed (zero-config type mismatch fix)
                        let corrected_schema = if let Some(schema) = self.output_schema() {
                            // Check if the schema needs correction based on actual return value
                            let needs_correction = if let Some(props) = &schema.properties {
                                if let Some(output_schema) = props.get(&field_name) {
                                    // Check if schema says "object" but value is array
                                    use turul_mcp_protocol::schema::JsonSchema;
                                    matches!(output_schema, JsonSchema::Object { .. }) && result_value.is_array()
                                } else {
                                    false
                                }
                            } else {
                                false
                            };

                            if needs_correction {
                                // Generate corrected schema with array type
                                use std::collections::HashMap;
                                use turul_mcp_protocol::schema::JsonSchema;
                                let array_schema = JsonSchema::Array {
                                    description: Some("Array of items".to_string()),
                                    items: None,
                                    min_items: None,
                                    max_items: None,
                                };
                                Some(turul_mcp_protocol::tools::ToolSchema::object()
                                    .with_properties(HashMap::from([(field_name.clone(), array_schema)]))
                                    .with_required(vec![field_name.clone()]))
                            } else {
                                Some(schema.clone())
                            }
                        } else {
                            None
                        };

                        // Return structured result with corrected schema
                        turul_mcp_protocol::tools::CallToolResult::from_result_auto(&wrapped_result, corrected_schema.as_ref())
                    }
                    Err(e) => Err(e)
                }
            }
        }
    };

    Ok(expanded)
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_simple_tool_derive() {
        let input: DeriveInput = parse_quote! {
            #[tool(name = "test", description = "A test tool")]
            struct TestTool {
                #[param(description = "A message")]
                message: String,
                #[param(description = "A number")]
                value: f64,
            }
        };

        let result = derive_mcp_tool_impl(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_zero_config_tool() {
        // ✅ ZERO CONFIGURATION - No attributes needed!
        let input: DeriveInput = parse_quote! {
            #[derive(McpTool)]
            struct CalculatorTool {
                #[param(description = "First number")]
                a: f64,
                #[param(description = "Second number")]
                b: f64,
            }
        };

        let result = derive_mcp_tool_impl(input);
        assert!(result.is_ok());

        // Framework auto-determines name as "calculator"
    }

    #[test]
    fn test_zero_config_without_tool_suffix() {
        // ✅ ZERO CONFIGURATION - Works without "Tool" suffix
        let input: DeriveInput = parse_quote! {
            #[derive(McpTool)]
            struct Calculator {
                value: f64,
            }
        };

        let result = derive_mcp_tool_impl(input);
        assert!(result.is_ok());

        // Framework auto-determines name as "calculator"
    }
}
