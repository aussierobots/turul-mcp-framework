//! Implementation of #[derive(McpTool)]

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, Result};

use crate::utils::{
    extract_tool_meta, extract_param_meta, type_to_schema, generate_param_extraction, 
    determine_output_field_name, generate_enhanced_output_schema
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
            _ => return Err(syn::Error::new_spanned(
                name,
                "McpTool can only be derived for structs with named fields"
            )),
        },
        _ => return Err(syn::Error::new_spanned(
            name,
            "McpTool can only be derived for structs"
        )),
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

        if !param_meta.optional {
            required_fields.push(quote! {
                #field_name_str.to_string()
            });
        }

        // Generate parameter extraction code
        let extraction = generate_param_extraction(field_name, field_type, param_meta.optional);
        param_extractions.push(extraction);

        // Generate field assignment for struct construction
        field_assignments.push(quote! {
            #field_name
        });
    }

    let tool_name = &tool_meta.name;
    let tool_description = &tool_meta.description;
    let custom_field_name_tokens = match &tool_meta.output_field {
        Some(field_name) => quote! { Some(#field_name) },
        None => quote! { None },
    };
    
    // Generate enhanced output schema with struct property introspection
    let output_schema_tokens = if let Some(ref output_type) = tool_meta.output_type {
        // User specified output type - determine field name and generate enhanced schema
        let field_name = determine_output_field_name(output_type, tool_meta.output_field.as_ref());
        generate_enhanced_output_schema(output_type, &field_name, Some(&input))
    } else {
        // Zero-config case - generate type-aware schema using heuristics
        let field_name = tool_meta.output_field.as_deref().unwrap_or("output");
        let struct_name_str = name.to_string();
        quote! {
            fn output_schema(&self) -> Option<&turul_mcp_protocol::tools::ToolSchema> {
                static OUTPUT_SCHEMA: std::sync::OnceLock<turul_mcp_protocol::tools::ToolSchema> = std::sync::OnceLock::new();
                Some(OUTPUT_SCHEMA.get_or_init(|| {
                    Self::generate_heuristic_schema(#field_name, #struct_name_str)
                }))
            }
        }
    };

    let expanded = quote! {
        #[automatically_derived]
        // Generate fine-grained trait implementations
        impl turul_mcp_protocol::tools::HasBaseMetadata for #name {
            fn name(&self) -> &str {
                #tool_name
            }
            
            fn title(&self) -> Option<&str> {
                // TODO: Extract from tool attributes when available
                None
            }
        }

        impl turul_mcp_protocol::tools::HasDescription for #name {
            fn description(&self) -> Option<&str> {
                Some(#tool_description)
            }
        }

        impl turul_mcp_protocol::tools::HasInputSchema for #name {
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

        impl turul_mcp_protocol::tools::HasOutputSchema for #name {
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
                } else if struct_name.contains("List") || struct_name.contains("Array") {
                    JsonSchema::Array {
                        description: Some("Array output".to_string()),
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
                
                let field_schema = match value {
                    serde_json::Value::Number(n) if n.is_f64() => JsonSchema::number(),
                    serde_json::Value::Number(_) => JsonSchema::integer(),
                    serde_json::Value::String(_) => JsonSchema::string(),
                    serde_json::Value::Bool(_) => JsonSchema::boolean(),
                    serde_json::Value::Array(_) => JsonSchema::Array {
                        description: Some("Array output".to_string()),
                        items: None,
                        min_items: None,
                        max_items: None,
                    },
                    serde_json::Value::Object(obj) => {
                        // Generate detailed schema from object structure
                        let mut properties = std::collections::HashMap::new();
                        let mut required = Vec::new();

                        for (key, value) in obj.iter() {
                            let prop_schema = match value {
                                serde_json::Value::String(_) => JsonSchema::string(),
                                serde_json::Value::Number(n) if n.is_f64() => JsonSchema::number(),
                                serde_json::Value::Number(_) => JsonSchema::integer(),
                                serde_json::Value::Bool(_) => JsonSchema::boolean(),
                                serde_json::Value::Array(arr) => {
                                    // Try to determine array item type from first element
                                    let item_type = arr.first().map(|first| match first {
                                        serde_json::Value::String(_) => JsonSchema::string(),
                                        serde_json::Value::Number(n) if n.is_f64() => JsonSchema::number(),
                                        serde_json::Value::Number(_) => JsonSchema::integer(),
                                        serde_json::Value::Bool(_) => JsonSchema::boolean(),
                                        _ => JsonSchema::string(), // Fallback
                                    });
                                    JsonSchema::Array {
                                        description: Some("Array of items".to_string()),
                                        items: item_type.map(Box::new),
                                        min_items: None,
                                        max_items: None,
                                    }
                                },
                                serde_json::Value::Object(_) => JsonSchema::Object {
                                    description: Some("Nested object".to_string()),
                                    properties: None,
                                    required: None,
                                    additional_properties: Some(true),
                                },
                                serde_json::Value::Null => continue, // Skip null values
                            };
                            properties.insert(key.clone(), prop_schema);
                            required.push(key.clone());
                        }

                        JsonSchema::Object {
                            description: Some("Generated from runtime object structure".to_string()),
                            properties: Some(properties),
                            required: Some(required),
                            additional_properties: Some(false), // We know the exact structure
                        }
                    },
                    serde_json::Value::Null => JsonSchema::string(), // Fallback
                };
                
                turul_mcp_protocol::tools::ToolSchema::object()
                    .with_properties(HashMap::from([
                        (field_name.to_string(), field_schema)
                    ]))
                    .with_required(vec![field_name.to_string()])
            }
        }

        impl turul_mcp_protocol::tools::HasAnnotations for #name {
            fn annotations(&self) -> Option<&turul_mcp_protocol::tools::ToolAnnotations> {
                // TODO: Extract from tool attributes when available
                None
            }
        }

        impl turul_mcp_protocol::tools::HasToolMeta for #name {
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
                use turul_mcp_protocol::tools::HasOutputSchema;
                
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
                                // Auto-determine field name based on output type
                                let type_name = std::any::type_name_of_val(&result);
                                if let Some(struct_name) = type_name.split("::").last() {
                                    // Check if this is a struct type (not a primitive)
                                    let is_primitive = matches!(struct_name,
                                        "f64" | "f32" | "i64" | "i32" | "i16" | "i8" | 
                                        "u64" | "u32" | "u16" | "u8" | "isize" | "usize" |
                                        "&str" | "String" | "bool"
                                    );
                                    
                                    if !is_primitive && matches!(result_value, serde_json::Value::Object(_)) {
                                        // Convert struct name to camelCase
                                        let mut chars: Vec<char> = struct_name.chars().collect();
                                        if !chars.is_empty() {
                                            chars[0] = chars[0].to_lowercase().next().unwrap_or(chars[0]);
                                        }
                                        chars.into_iter().collect::<String>()
                                    } else {
                                        // Use "output" as default for primitives (as requested by user)
                                        "output".to_string()
                                    }
                                } else {
                                    "output".to_string()
                                }
                            }
                        };
                        
                        // Wrap result to match MCP object schema format
                        let wrapped_result = serde_json::json!({field_name: result});

                        // Return structured result (schema already available from output_schema() method)
                        turul_mcp_protocol::tools::CallToolResult::from_result_auto(&wrapped_result, self.output_schema())
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