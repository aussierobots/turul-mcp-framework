//! Implementation of #[mcp_tool] attribute macro

use proc_macro2::TokenStream;
use quote::quote;
use syn::{ItemFn, Result, FnArg, Pat, Meta, Lit, punctuated::Punctuated, Token};

use crate::utils::{extract_param_meta, type_to_schema, generate_param_extraction, generate_output_schema_for_return_type_with_field};

pub fn mcp_tool_impl(args: Punctuated<Meta, Token![,]>, input: ItemFn) -> Result<TokenStream> {
    // Parse macro arguments
    let mut tool_name = None;
    let mut tool_description = None;
    let mut output_field_name = None;

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
            Meta::NameValue(nv) if nv.path.is_ident("output_field") => {
                if let syn::Expr::Lit(expr_lit) = &nv.value {
                    if let Lit::Str(s) = &expr_lit.lit {
                        output_field_name = Some(s.value());
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
    
    // Use custom output field name or default to "result"
    let output_field_name = output_field_name.unwrap_or_else(|| "result".to_string());

    let fn_name = &input.sig.ident;
    let fn_vis = &input.vis;
    let return_type = &input.sig.output;

    // Generate struct name from function name with proper capitalization
    let struct_name = syn::Ident::new(
        &format!("{}ToolImpl", capitalize(&fn_name.to_string())),
        fn_name.span()
    );


    // Analyze return type for output schema generation
    let output_schema_tokens = match return_type {
        syn::ReturnType::Type(_, ty) => {
            generate_output_schema_for_return_type_with_field(ty, &output_field_name).unwrap_or_else(|| {
                quote! {
                    fn output_schema(&self) -> Option<&mcp_protocol::tools::ToolSchema> { None }
                }
            })
        }
        _ => quote! {
            fn output_schema(&self) -> Option<&mcp_protocol::tools::ToolSchema> { None }
        },
    };

    // Process function parameters
    let mut schema_properties = Vec::new();
    let mut required_fields = Vec::new();
    let mut param_extractions = Vec::new();
    let mut fn_call_args = Vec::new();
    let mut param_types = Vec::new();

    for input_arg in &input.sig.inputs {
        if let FnArg::Typed(pat_type) = input_arg {
            if let Pat::Ident(pat_ident) = pat_type.pat.as_ref() {
                let param_name = &pat_ident.ident;
                let param_type = &pat_type.ty;
                
                // Collect parameter type for trait implementation
                param_types.push(param_type);
                
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

    // Rename the function to avoid name collision with the tool constructor
    let mut clean_input = input.clone();
    clean_input.attrs.retain(|attr| !attr.path().is_ident("mcp_tool"));
    
    // Rename the function with _impl suffix
    let impl_fn_name = syn::Ident::new(&format!("{}_impl", fn_name), fn_name.span());
    clean_input.sig.ident = impl_fn_name.clone();
    
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

        // Store schema statically for trait implementation
        impl #struct_name {
            fn get_input_schema() -> mcp_protocol::tools::ToolSchema {
                use std::collections::HashMap;
                mcp_protocol::tools::ToolSchema::object()
                    .with_properties(HashMap::from([
                        #(#schema_properties),*
                    ]))
                    .with_required(vec![
                        #(#required_fields),*
                    ])
            }
        }

        // Implement all fine-grained traits
        #[automatically_derived]
        impl mcp_protocol::tools::HasBaseMetadata for #struct_name {
            fn name(&self) -> &str { #tool_name }
            fn title(&self) -> Option<&str> { None }
        }

        #[automatically_derived]
        impl mcp_protocol::tools::HasDescription for #struct_name {
            fn description(&self) -> Option<&str> { Some(#tool_description) }
        }

        #[automatically_derived]
        impl mcp_protocol::tools::HasInputSchema for #struct_name {
            fn input_schema(&self) -> &mcp_protocol::tools::ToolSchema {
                static INPUT_SCHEMA: std::sync::OnceLock<mcp_protocol::tools::ToolSchema> = std::sync::OnceLock::new();
                INPUT_SCHEMA.get_or_init(|| Self::get_input_schema())
            }
        }

        #[automatically_derived]
        impl mcp_protocol::tools::HasOutputSchema for #struct_name {
            #output_schema_tokens
        }

        #[automatically_derived]
        impl mcp_protocol::tools::HasAnnotations for #struct_name {
            fn annotations(&self) -> Option<&mcp_protocol::tools::ToolAnnotations> { None }
        }

        #[automatically_derived]
        impl mcp_protocol::tools::HasToolMeta for #struct_name {
            fn tool_meta(&self) -> Option<&std::collections::HashMap<String, serde_json::Value>> { None }
        }

        // ToolDefinition automatically implemented via blanket impl!

        #[automatically_derived]
        #[async_trait::async_trait]
        impl mcp_server::McpTool for #struct_name {
            async fn call(&self, args: serde_json::Value, _session: Option<mcp_server::SessionContext>) -> mcp_server::McpResult<mcp_protocol::tools::CallToolResult> {
                use serde_json::Value;
                use mcp_protocol::tools::HasOutputSchema;
                
                // Extract parameters
                #(#param_extractions)*

                // Call the renamed implementation function
                match #impl_fn_name(#(#fn_call_args),*).await {
                    Ok(result) => {
                        // Wrap primitive results to match schema expectations
                        let schema_result = if self.output_schema().is_some() {
                            // Wrap in {field_name: value} to match generated schema
                            serde_json::json!({#output_field_name: result})
                        } else {
                            serde_json::to_value(&result).unwrap_or_else(|_| serde_json::json!(result.to_string()))
                        };
                        
                        // Use smart response builder with automatic structured content
                        mcp_protocol::tools::CallToolResult::from_result_with_schema(&schema_result, self.output_schema())
                    }
                    Err(e) => Err(mcp_protocol::McpError::tool_execution(&e.to_string()))
                }
            }
        }

        // Generate a constructor function with the original function name for intuitive usage
        #fn_vis fn #fn_name() -> #struct_name {
            #struct_name
        }
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