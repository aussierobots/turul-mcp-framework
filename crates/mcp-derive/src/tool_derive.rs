//! Implementation of #[derive(McpTool)]

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, Result};

use crate::utils::{extract_tool_meta, extract_param_meta, type_to_schema, generate_param_extraction, generate_output_schema_for_type, generate_result_conversion};

pub fn derive_mcp_tool_impl(input: DeriveInput) -> Result<TokenStream> {
    let name = &input.ident;
    let tool_meta = extract_tool_meta(&input.attrs)?;
    
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
    
    // Generate output schema and result conversion based on return type
    let (output_schema_tokens, _result_conversion_tokens) = if let Some(ref output_type) = tool_meta.output_type {
        let schema = generate_output_schema_for_type(output_type);
        let conversion = generate_result_conversion(output_type, true); // Has output schema
        (schema, conversion)
    } else {
        // Default case - no output schema
        let default_schema = quote! {
            fn output_schema(&self) -> Option<&mcp_protocol::tools::ToolSchema> {
                None
            }
        };
        let default_conversion = quote! {
            match instance.execute().await {
                Ok(result) => {
                    Ok(vec![mcp_protocol::ToolResult::text(result.to_string())])
                }
                Err(e) => Err(e)
            }
        };
        (default_schema, default_conversion)
    };

    let expanded = quote! {
        #[automatically_derived]
        // Generate fine-grained trait implementations
        impl mcp_protocol::tools::HasBaseMetadata for #name {
            fn name(&self) -> &str {
                #tool_name
            }
            
            fn title(&self) -> Option<&str> {
                // TODO: Extract from tool attributes when available
                None
            }
        }

        impl mcp_protocol::tools::HasDescription for #name {
            fn description(&self) -> Option<&str> {
                Some(#tool_description)
            }
        }

        impl mcp_protocol::tools::HasInputSchema for #name {
            fn input_schema(&self) -> &mcp_protocol::tools::ToolSchema {
                // Generate static schema at compile time
                static INPUT_SCHEMA: std::sync::OnceLock<mcp_protocol::tools::ToolSchema> = std::sync::OnceLock::new();
                INPUT_SCHEMA.get_or_init(|| {
                    use std::collections::HashMap;
                    mcp_protocol::tools::ToolSchema::object()
                        .with_properties(HashMap::from([
                            #(#schema_properties),*
                        ]))
                        .with_required(vec![
                            #(#required_fields),*
                        ])
                })
            }
        }

        impl mcp_protocol::tools::HasOutputSchema for #name {
            #output_schema_tokens
        }

        impl mcp_protocol::tools::HasAnnotations for #name {
            fn annotations(&self) -> Option<&mcp_protocol::tools::ToolAnnotations> {
                // TODO: Extract from tool attributes when available
                None
            }
        }

        impl mcp_protocol::tools::HasToolMeta for #name {
            fn tool_meta(&self) -> Option<&std::collections::HashMap<String, serde_json::Value>> {
                None
            }
        }

        // ToolDefinition automatically implemented via trait composition!

        #[automatically_derived]
        #[async_trait::async_trait]
        impl mcp_server::McpTool for #name {
            async fn call(&self, args: serde_json::Value, _session: Option<mcp_server::SessionContext>) -> mcp_server::McpResult<mcp_protocol::tools::CallToolResult> {
                use serde_json::Value;
                use mcp_protocol::tools::HasOutputSchema;
                
                // Extract parameters
                #(#param_extractions)*

                // Create instance with extracted parameters
                let instance = #name {
                    #(#field_assignments),*
                };

                // Execute and convert result to CallToolResult
                match instance.execute().await {
                    Ok(result) => {
                        // Use smart response builder with automatic structured content
                        mcp_protocol::tools::CallToolResult::from_result_with_schema(&result, self.output_schema())
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
    fn test_missing_tool_attribute() {
        let input: DeriveInput = parse_quote! {
            struct TestTool {
                message: String,
            }
        };

        let result = derive_mcp_tool_impl(input);
        assert!(result.is_err());
    }
}