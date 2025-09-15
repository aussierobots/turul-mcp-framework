//! Implementation of #[mcp_resource] attribute macro

use proc_macro2::TokenStream;
use quote::quote;
use syn::{ItemFn, Result, FnArg, Pat, Meta, Lit, punctuated::Punctuated, Token};

use crate::macros::shared::capitalize;

pub fn mcp_resource_impl(args: Punctuated<Meta, Token![,]>, input: ItemFn) -> Result<TokenStream> {
    // Parse macro arguments
    let mut resource_uri = None;
    let mut resource_name = None;
    let mut resource_description = None;
    let mut mime_type = None;

    for arg in args {
        match arg {
            Meta::NameValue(nv) if nv.path.is_ident("uri") => {
                if let syn::Expr::Lit(expr_lit) = &nv.value {
                    if let Lit::Str(s) = &expr_lit.lit {
                        resource_uri = Some(s.value());
                    }
                }
            }
            Meta::NameValue(nv) if nv.path.is_ident("name") => {
                if let syn::Expr::Lit(expr_lit) = &nv.value {
                    if let Lit::Str(s) = &expr_lit.lit {
                        resource_name = Some(s.value());
                    }
                }
            }
            Meta::NameValue(nv) if nv.path.is_ident("description") => {
                if let syn::Expr::Lit(expr_lit) = &nv.value {
                    if let Lit::Str(s) = &expr_lit.lit {
                        resource_description = Some(s.value());
                    }
                }
            }
            Meta::NameValue(nv) if nv.path.is_ident("mime_type") => {
                if let syn::Expr::Lit(expr_lit) = &nv.value {
                    if let Lit::Str(s) = &expr_lit.lit {
                        mime_type = Some(s.value());
                    }
                }
            }
            _ => {}
        }
    }

    let resource_uri = resource_uri.ok_or_else(|| {
        syn::Error::new_spanned(&input.sig.ident, "Missing 'uri' parameter in #[mcp_resource(...)]")
    })?;

    let fn_name = &input.sig.ident;
    let fn_vis = &input.vis;
    let _return_type = &input.sig.output;

    // Generate struct name from function name with proper capitalization
    let struct_name = syn::Ident::new(
        &format!("{}ResourceImpl", capitalize(&fn_name.to_string())),
        fn_name.span()
    );

    // Use function name as resource name if not provided
    let resource_name = resource_name.unwrap_or_else(|| fn_name.to_string());

    // Default description if not provided
    let resource_description = resource_description.unwrap_or_else(|| format!("Resource: {}", resource_name));

    // Extract template variables from URI for parameter extraction
    let mut template_vars = Vec::new();
    let mut fn_call_args = Vec::new();
    let mut has_params_arg = false;

    // Check for template variables in URI (e.g., {ticker}, {id})
    let re = regex::Regex::new(r"\{([^}]+)\}").unwrap();
    for capture in re.captures_iter(&resource_uri) {
        if let Some(var_name) = capture.get(1) {
            template_vars.push(var_name.as_str().to_string());
        }
    }

    // Process function parameters
    for input_arg in &input.sig.inputs {
        if let FnArg::Typed(pat_type) = input_arg {
            if let Pat::Ident(pat_ident) = pat_type.pat.as_ref() {
                let param_name = &pat_ident.ident;
                let param_type = &pat_type.ty;

                // Check if this parameter name matches a template variable
                if template_vars.contains(&param_name.to_string()) {
                    // Generate parameter extraction from template variables
                    let param_name_str = param_name.to_string();
                    fn_call_args.push(quote! {
                        {
                            let template_vars = params
                                .as_ref()
                                .and_then(|p| p.get("template_variables"))
                                .and_then(|tv| tv.as_object());

                            template_vars
                                .and_then(|vars| vars.get(#param_name_str))
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string())
                                .ok_or_else(|| turul_mcp_protocol::McpError::missing_param(#param_name_str))?
                        }
                    });
                } else if param_name == "params" && matches!(**param_type, syn::Type::Path(ref p) if p.path.segments.last().unwrap().ident == "Value") {
                    // This is the params argument
                    has_params_arg = true;
                    fn_call_args.push(quote! { params });
                } else {
                    // Regular parameter - extract from params
                    let param_name_str = param_name.to_string();
                    fn_call_args.push(quote! {
                        {
                            params
                                .as_ref()
                                .and_then(|p| p.get(#param_name_str))
                                .and_then(|v| serde_json::from_value(v.clone()).ok())
                                .ok_or_else(|| turul_mcp_protocol::McpError::missing_param(#param_name_str))?
                        }
                    });
                }
            }
        }
    }

    // If function doesn't take params but we need template variables, add them
    if !has_params_arg && !template_vars.is_empty() {
        // For template variables, we still need to handle the extraction
        // but the function signature doesn't include params
    }

    // Function will be called directly from the generated McpResource implementation

    let expanded = quote! {
        // Keep the original function
        #input

        // Generate wrapper struct
        #[derive(Clone)]
        #fn_vis struct #struct_name;

        impl #struct_name {
            #fn_vis fn new() -> Self {
                Self
            }
        }

        // Implement resource metadata traits
        impl turul_mcp_protocol::resources::HasResourceMetadata for #struct_name {
            fn name(&self) -> &str {
                #resource_name
            }
        }

        impl turul_mcp_protocol::resources::HasResourceDescription for #struct_name {
            fn description(&self) -> Option<&str> {
                Some(#resource_description)
            }
        }

        impl turul_mcp_protocol::resources::HasResourceUri for #struct_name {
            fn uri(&self) -> &str {
                #resource_uri
            }
        }

        impl turul_mcp_protocol::resources::HasResourceMimeType for #struct_name {
            fn mime_type(&self) -> Option<&str> {
                #mime_type.as_deref()
            }
        }

        impl turul_mcp_protocol::resources::HasResourceSize for #struct_name {
            fn size(&self) -> Option<u64> {
                None
            }
        }

        impl turul_mcp_protocol::resources::HasResourceAnnotations for #struct_name {
            fn annotations(&self) -> Option<&turul_mcp_protocol::meta::Annotations> {
                None
            }
        }

        impl turul_mcp_protocol::resources::HasResourceMeta for #struct_name {
            fn resource_meta(&self) -> Option<&std::collections::HashMap<String, serde_json::Value>> {
                None
            }
        }

        // Implement McpResource trait
        #[async_trait::async_trait]
        impl turul_mcp_server::McpResource for #struct_name {
            async fn read(&self, params: Option<serde_json::Value>) -> turul_mcp_server::McpResult<Vec<turul_mcp_protocol::resources::ResourceContent>> {
                // Call the original function with extracted parameters
                #fn_name(#(#fn_call_args),*).await
            }
        }

        // Create a constructor function with the same name as the original function
        // This allows using the function name directly in .tool_fn() style
        #fn_vis fn #fn_name() -> #struct_name {
            #struct_name::new()
        }
    };

    Ok(expanded)
}

