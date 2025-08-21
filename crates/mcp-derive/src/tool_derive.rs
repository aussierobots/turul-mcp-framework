//! Implementation of #[derive(McpTool)]

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, Result};

use crate::utils::{extract_tool_meta, extract_param_meta, type_to_schema, generate_param_extraction};

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

    let expanded = quote! {
        #[automatically_derived]
        #[async_trait::async_trait]
        impl mcp_server::McpTool for #name {
            fn name(&self) -> &str {
                #tool_name
            }

            fn description(&self) -> &str {
                #tool_description  
            }

            fn input_schema(&self) -> mcp_protocol::ToolSchema {
                use std::collections::HashMap;
                
                mcp_protocol::ToolSchema::object()
                    .with_properties(HashMap::from([
                        #(#schema_properties),*
                    ]))
                    .with_required(vec![
                        #(#required_fields),*
                    ])
            }

            async fn call(&self, args: serde_json::Value, _session: Option<mcp_server::SessionContext>) -> mcp_server::McpResult<Vec<mcp_protocol::ToolResult>> {
                use serde_json::Value;
                
                // Extract parameters
                #(#param_extractions)*

                // Create instance with extracted parameters
                let instance = #name {
                    #(#field_assignments),*
                };

                // Call the execute method if it exists, otherwise provide a default implementation
                // This uses the execute method that the user must implement
                match instance.execute().await {
                    Ok(result) => {
                        // Convert result to ToolResult
                        Ok(vec![mcp_protocol::ToolResult::text(result)])
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