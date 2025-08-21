//! Implementation of #[mcp_tool] attribute macro

use proc_macro2::TokenStream;
use quote::quote;
use syn::{ItemFn, Result, FnArg, Pat, Meta, Lit, punctuated::Punctuated, Token};

use crate::utils::{extract_param_meta, type_to_schema, generate_param_extraction};

pub fn mcp_tool_impl(args: Punctuated<Meta, Token![,]>, input: ItemFn) -> Result<TokenStream> {
    // Parse macro arguments
    let mut tool_name = None;
    let mut tool_description = None;

    for arg in args {
        match arg {
            Meta::NameValue(nv) if nv.path.is_ident("name") => {
                if let syn::Expr::Lit(expr_lit) = &nv.value {
                    if let Lit::Str(s) = &expr_lit.lit {
                        tool_name = Some(s.value());
                    }
                }
            }
            Meta::NameValue(nv) if nv.path.is_ident("description") => {
                if let syn::Expr::Lit(expr_lit) = &nv.value {
                    if let Lit::Str(s) = &expr_lit.lit {
                        tool_description = Some(s.value());
                    }
                }
            }
            _ => {}
        }
    }

    let tool_name = tool_name.ok_or_else(|| {
        syn::Error::new_spanned(&input.sig.ident, "Missing 'name' parameter in #[mcp_tool(...)]")
    })?;

    let tool_description = tool_description.ok_or_else(|| {
        syn::Error::new_spanned(&input.sig.ident, "Missing 'description' parameter in #[mcp_tool(...)]")
    })?;

    let fn_name = &input.sig.ident;
    let fn_vis = &input.vis;

    // Generate struct name from function name with proper capitalization
    let struct_name = syn::Ident::new(
        &format!("{}ToolImpl", capitalize(&fn_name.to_string())),
        fn_name.span()
    );

    // Process function parameters
    let mut schema_properties = Vec::new();
    let mut required_fields = Vec::new();
    let mut param_extractions = Vec::new();
    let mut fn_call_args = Vec::new();

    for input_arg in &input.sig.inputs {
        if let FnArg::Typed(pat_type) = input_arg {
            if let Pat::Ident(pat_ident) = pat_type.pat.as_ref() {
                let param_name = &pat_ident.ident;
                let param_type = &pat_type.ty;
                
                // Extract parameter metadata from attributes
                let param_meta = extract_param_meta(&pat_type.attrs)?;
                
                let param_name_str = param_name.to_string();
                
                // Generate schema for this parameter based on type
                let schema = type_to_schema(param_type, &param_meta);
                
                schema_properties.push(quote! {
                    (#param_name_str.to_string(), #schema)
                });

                if !param_meta.optional {
                    required_fields.push(quote! {
                        #param_name_str.to_string()
                    });
                }

                // Generate parameter extraction code based on type
                let extraction = generate_param_extraction(param_name, param_type, param_meta.optional);
                param_extractions.push(extraction);

                // Add to function call arguments - handle Optional unwrapping
                if param_meta.optional && !is_option_type(param_type) {
                    // Function expects T but we have Option<T>, so unwrap with error
                    fn_call_args.push(quote! { 
                        #param_name.ok_or_else(|| mcp_protocol::McpError::missing_param(#param_name_str))?
                    });
                } else {
                    // Direct use - either required T or Option<T>
                    fn_call_args.push(quote! { #param_name });
                }
            }
        }
    }

    // Remove function and parameter attributes from the original function to avoid conflicts
    let mut clean_input = input.clone();
    clean_input.attrs.retain(|attr| !attr.path().is_ident("mcp_tool"));
    
    // Clean parameter attributes
    for input_arg in &mut clean_input.sig.inputs {
        if let FnArg::Typed(pat_type) = input_arg {
            pat_type.attrs.retain(|attr| !attr.path().is_ident("param"));
        }
    }

    let expanded = quote! {
        // Keep the original function for direct use (with cleaned attributes)
        #clean_input

        // Generate a tool struct that wraps this function
        #[derive(Clone)]
        #fn_vis struct #struct_name;

        #[automatically_derived]
        #[async_trait::async_trait]
        impl mcp_server::McpTool for #struct_name {
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

                // Call the original function
                match #fn_name(#(#fn_call_args),*).await {
                    Ok(result) => {
                        // Convert result to ToolResult
                        let tool_result = match serde_json::to_value(&result) {
                            Ok(Value::String(s)) => mcp_protocol::ToolResult::text(s),
                            Ok(other) => mcp_protocol::ToolResult::text(other.to_string()),
                            Err(_) => mcp_protocol::ToolResult::text(format!("{:?}", result)),
                        };
                        Ok(vec![tool_result])
                    }
                    Err(e) => Err(mcp_protocol::McpError::ToolExecutionError(e.to_string()))
                }
            }
        }

        // Note: To create an instance of the tool, use: StructName default initialization
        // Example: let tool = TestAddToolImpl;
    };

    Ok(expanded)
}

fn capitalize(s: &str) -> String {
    // Convert snake_case or kebab-case to PascalCase
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => {
                    let mut result = first.to_uppercase().collect::<String>();
                    result.push_str(&chars.collect::<String>().to_lowercase());
                    result
                }
            }
        })
        .collect::<Vec<_>>()
        .join("")
}

/// Check if a type is Option<T>
fn is_option_type(field_type: &syn::Type) -> bool {
    if let syn::Type::Path(type_path) = field_type {
        type_path.path.segments.len() == 1 && 
        type_path.path.segments[0].ident == "Option"
    } else {
        false
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use syn::{parse_quote, punctuated::Punctuated, Token, Meta};

    #[test]
    fn test_capitalize() {
        assert_eq!(capitalize("hello"), "Hello");
        assert_eq!(capitalize("world"), "World");
        assert_eq!(capitalize(""), "");
    }

    #[test]
    fn test_simple_function_tool() {
        let args: Punctuated<Meta, Token![,]> = parse_quote! {
            name = "test", description = "A test function"
        };

        let input: ItemFn = parse_quote! {
            async fn test_fn(message: String) -> Result<String, String> {
                Ok(format!("Hello, {}", message))
            }
        };

        let result = mcp_tool_impl(args, input);
        assert!(result.is_ok());
    }
}