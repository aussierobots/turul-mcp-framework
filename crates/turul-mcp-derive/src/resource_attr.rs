//! Implements the `#[mcp_resource]` attribute macro.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{FnArg, ItemFn, Lit, Meta, Pat, Result, Token, punctuated::Punctuated};

use crate::macros::shared::capitalize;

/// Expands `#[mcp_resource(...)]` on a function into a resource wrapper struct implementing `McpResource`, keeping the function as `<name>_impl`.
///
/// # Errors
///
/// Returns a `syn::Error` when an argument is not one of the accepted keys, or when `uri` is missing.
pub fn mcp_resource_impl(args: Punctuated<Meta, Token![,]>, input: ItemFn) -> Result<TokenStream> {
    let async_trait = crate::utils::async_trait_path();
    let json_crate = crate::utils::serde_json_path();
    let mcp_builders = crate::utils::builders_path();
    let mcp_protocol = crate::utils::protocol_path();
    let mcp_server = crate::utils::server_path();
    let mut resource_uri = None;
    let mut resource_name = None;
    let mut resource_description = None;
    let mut resource_title = None;
    let mut mime_type: Option<syn::Expr> = None;
    let mut meta: Option<syn::Expr> = None;
    let mut icons: Option<syn::Expr> = None;
    let mut annotations: Option<syn::Expr> = None;
    let mut size: Option<syn::Expr> = None;

    for arg in args {
        match arg {
            Meta::NameValue(nv) if nv.path.is_ident("meta") => meta = Some(nv.value.clone()),
            Meta::NameValue(nv) if nv.path.is_ident("icons") => icons = Some(nv.value.clone()),
            Meta::NameValue(nv) if nv.path.is_ident("annotations") => {
                annotations = Some(nv.value.clone())
            }
            Meta::NameValue(nv) if nv.path.is_ident("size") => size = Some(nv.value.clone()),
            Meta::NameValue(nv) if nv.path.is_ident("uri") => {
                if let syn::Expr::Lit(expr_lit) = &nv.value
                    && let Lit::Str(s) = &expr_lit.lit
                {
                    resource_uri = Some(s.value());
                }
            }
            Meta::NameValue(nv) if nv.path.is_ident("name") => {
                if let syn::Expr::Lit(expr_lit) = &nv.value
                    && let Lit::Str(s) = &expr_lit.lit
                {
                    resource_name = Some(s.value());
                }
            }
            Meta::NameValue(nv) if nv.path.is_ident("description") => {
                if let syn::Expr::Lit(expr_lit) = &nv.value
                    && let Lit::Str(s) = &expr_lit.lit
                {
                    resource_description = Some(s.value());
                }
            }
            Meta::NameValue(nv) if nv.path.is_ident("title") => {
                if let syn::Expr::Lit(expr_lit) = &nv.value
                    && let Lit::Str(s) = &expr_lit.lit
                {
                    resource_title = Some(s.value());
                }
            }
            // A string literal or any `&'static str` expression, e.g. a constant.
            Meta::NameValue(nv) if nv.path.is_ident("mime_type") => {
                mime_type = Some(nv.value.clone());
            }
            other => {
                return Err(syn::Error::new_spanned(
                    other,
                    "unknown #[mcp_resource] argument; expected one of: uri, name, description, title, mime_type, size, meta, icons, annotations",
                ));
            }
        }
    }

    let resource_uri = resource_uri.ok_or_else(|| {
        syn::Error::new_spanned(
            &input.sig.ident,
            "Missing 'uri' parameter in #[mcp_resource(...)]",
        )
    })?;

    let fn_name = &input.sig.ident;
    let fn_vis = &input.vis;
    let _return_type = &input.sig.output;

    let struct_name = syn::Ident::new(
        &format!("{}ResourceImpl", capitalize(&fn_name.to_string())),
        fn_name.span(),
    );

    let meta_impl = crate::tool_derive::generate_meta_impl(
        &struct_name,
        "HasResourceMeta",
        "resource_meta",
        meta.as_ref(),
    );
    let icons_impl = crate::tool_derive::generate_icons_impl(&struct_name, icons.as_ref());
    let size_impl = crate::tool_derive::generate_resource_size_impl(&struct_name, size.as_ref());
    let annotations_impl =
        crate::tool_derive::generate_resource_annotations_impl(&struct_name, annotations.as_ref());

    let resource_name = resource_name.unwrap_or_else(|| fn_name.to_string());

    let resource_description =
        resource_description.unwrap_or_else(|| format!("Resource: {}", resource_name));

    let title_expr = match resource_title {
        Some(ref t) => quote! { Some(#t) },
        None => quote! { None },
    };

    let mime_type_expr = match mime_type {
        Some(mt) => quote! { Some(#mt) },
        None => quote! { None },
    };

    let mut template_vars = Vec::new();
    let mut fn_call_args = Vec::new();
    let mut has_params_arg = false;

    let re = regex::Regex::new(r"\{([^}]+)\}").unwrap();
    for capture in re.captures_iter(&resource_uri) {
        if let Some(var_name) = capture.get(1) {
            // `{+path}` (RFC 6570 reserved expansion) binds the argument `path`.
            let name = var_name.as_str();
            template_vars.push(name.strip_prefix('+').unwrap_or(name).to_string());
        }
    }

    for input_arg in &input.sig.inputs {
        if let FnArg::Typed(pat_type) = input_arg
            && let Pat::Ident(pat_ident) = pat_type.pat.as_ref()
        {
            let param_name = &pat_ident.ident;
            let param_type = &pat_type.ty;

            if template_vars.contains(&param_name.to_string()) {
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
                            .ok_or_else(|| #mcp_protocol::McpError::missing_param(#param_name_str))?
                    }
                });
            } else if param_name == "params"
                && matches!(**param_type, syn::Type::Path(ref p) if p.path.segments.last().unwrap().ident == "Value")
            {
                has_params_arg = true;
                fn_call_args.push(quote! { params.unwrap_or_default() });
            } else {
                let param_name_str = param_name.to_string();
                fn_call_args.push(quote! {
                    {
                        params
                            .as_ref()
                            .and_then(|p| p.get(#param_name_str))
                            .and_then(|v| #json_crate::from_value(v.clone()).ok())
                            .ok_or_else(|| #mcp_protocol::McpError::missing_param(#param_name_str))?
                    }
                });
            }
        }
    }

    // If function doesn't take params but we need template variables, add them
    if !has_params_arg && !template_vars.is_empty() {
        // For template variables, we still need to handle the extraction
        // but the function signature doesn't include params
    }

    // Rename the function to avoid name collision with the resource constructor
    let mut clean_input = input.clone();
    clean_input
        .attrs
        .retain(|attr| !attr.path().is_ident("mcp_resource"));

    let impl_fn_name = syn::Ident::new(&format!("{}_impl", fn_name), fn_name.span());
    clean_input.sig.ident = impl_fn_name.clone();

    let expanded = quote! {
        #clean_input

        #[derive(Clone)]
        #fn_vis struct #struct_name;

        impl #struct_name {
            #fn_vis fn new() -> Self {
                Self
            }
        }

        impl #mcp_builders::traits::HasResourceMetadata for #struct_name {
            fn name(&self) -> &str {
                #resource_name
            }

            fn title(&self) -> Option<&str> {
                #title_expr
            }
        }

        impl #mcp_builders::traits::HasResourceDescription for #struct_name {
            fn description(&self) -> Option<&str> {
                Some(#resource_description)
            }
        }

        impl #mcp_builders::traits::HasResourceUri for #struct_name {
            fn uri(&self) -> &str {
                #resource_uri
            }
        }

        impl #mcp_builders::traits::HasResourceMimeType for #struct_name {
            fn mime_type(&self) -> Option<&str> {
                #mime_type_expr
            }
        }

        #size_impl

        #annotations_impl

        #meta_impl

        #icons_impl

        #[#async_trait::async_trait]
        impl #mcp_server::McpResource for #struct_name {
            async fn read(&self, params: Option<#json_crate::Value>, _session: Option<&#mcp_server::SessionContext>) -> #mcp_server::McpResult<Vec<#mcp_protocol::resources::ResourceContent>> {
                #impl_fn_name(#(#fn_call_args),*).await
            }
        }

        #fn_vis fn #fn_name() -> #struct_name {
            #struct_name::new()
        }
    };

    Ok(expanded)
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn unknown_argument_is_refused_naming_the_accepted_keys() {
        let args = parse_quote! { uri = "file:///a.json", mimetype = "application/json" };
        let input = parse_quote! {
            async fn a() -> McpResult<Vec<ResourceContent>> { Ok(vec![]) }
        };
        let err = mcp_resource_impl(args, input).unwrap_err().to_string();
        assert!(err.contains("unknown #[mcp_resource] argument"), "{err}");
        assert!(err.contains("mime_type"), "{err}");
    }

    #[test]
    fn test_basic_mcp_resource() {
        let args = parse_quote! { uri = "file:///data/test.json", description = "Test resource" };
        let input = parse_quote! {
            async fn get_test_data() -> McpResult<Vec<ResourceContent>> {
                Ok(vec![])
            }
        };

        let result = mcp_resource_impl(args, input);
        assert!(result.is_ok());

        let code = result.unwrap().to_string();
        assert!(code.contains("GetTestDataResourceImpl"));
        assert!(code.contains("turul_mcp_server") && code.contains("McpResource"));
        assert!(code.contains("fn get_test_data") && code.contains("GetTestDataResourceImpl"));
    }

    #[test]
    fn test_mcp_resource_with_template_variables() {
        let args = parse_quote! { uri = "file:///data/{id}.json", description = "Data for ID" };
        let input = parse_quote! {
            async fn get_data(id: String) -> McpResult<Vec<ResourceContent>> {
                Ok(vec![])
            }
        };

        let result = mcp_resource_impl(args, input);
        assert!(result.is_ok());

        let code = result.unwrap().to_string();
        assert!(code.contains("GetDataResourceImpl"));
        assert!(code.contains("template_variables"));
        assert!(code.contains("fn get_data") && code.contains("GetDataResourceImpl"));
    }

    #[test]
    fn test_mcp_resource_with_multiple_parameters() {
        let args =
            parse_quote! { uri = "file:///timeline/{ticker}.json", description = "Timeline data" };
        let input = parse_quote! {
            async fn get_timeline(ticker: String, days: i32) -> McpResult<Vec<ResourceContent>> {
                Ok(vec![])
            }
        };

        let result = mcp_resource_impl(args, input);
        assert!(result.is_ok());

        let code = result.unwrap().to_string();
        assert!(code.contains("GetTimelineResourceImpl"));
        assert!(code.contains("template_variables"));
        assert!(code.contains("missing_param") && code.contains("days"));
    }

    #[test]
    fn test_mcp_resource_missing_uri() {
        let args = parse_quote! { description = "Missing URI" };
        let input = parse_quote! {
            async fn test_fn() -> McpResult<Vec<ResourceContent>> {
                Ok(vec![])
            }
        };

        let result = mcp_resource_impl(args, input);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Missing 'uri' parameter")
        );
    }

    #[test]
    fn test_mcp_resource_with_params_argument() {
        let json_crate = crate::utils::serde_json_path();
        let args =
            parse_quote! { uri = "file:///custom/{id}.json", description = "Custom resource" };
        let input = parse_quote! {
            async fn custom_resource(id: String, params: #json_crate::Value) -> McpResult<Vec<ResourceContent>> {
                Ok(vec![])
            }
        };

        let result = mcp_resource_impl(args, input);
        assert!(result.is_ok());

        let code = result.unwrap().to_string();
        assert!(code.contains("params"));
        assert!(code.contains("template_variables"));
    }

    #[test]
    fn test_generated_struct_name() {
        let args = parse_quote! { uri = "file:///test.json" };
        let input = parse_quote! {
            async fn load_user_profile() -> McpResult<Vec<ResourceContent>> {
                Ok(vec![])
            }
        };

        let result = mcp_resource_impl(args, input);
        assert!(result.is_ok());

        let code = result.unwrap().to_string();
        // Should capitalize properly: load_user_profile -> LoadUserProfileResourceImpl
        assert!(code.contains("LoadUserProfileResourceImpl"));
        assert!(
            code.contains("fn load_user_profile") && code.contains("LoadUserProfileResourceImpl")
        );
    }

    #[test]
    fn test_mcp_resource_with_title() {
        let args = parse_quote! { uri = "file:///data/test.json", title = "My Resource Title", description = "Test resource" };
        let input = parse_quote! {
            async fn get_titled_data() -> McpResult<Vec<ResourceContent>> {
                Ok(vec![])
            }
        };

        let result = mcp_resource_impl(args, input);
        assert!(result.is_ok());

        let code = result.unwrap().to_string();
        assert!(
            code.contains("My Resource Title"),
            "title should appear in generated code"
        );
        // Should have explicit title() method in HasResourceMetadata
        assert!(
            code.contains("fn title"),
            "should generate explicit title() method"
        );
    }

    #[test]
    fn test_mcp_resource_without_title() {
        let args = parse_quote! { uri = "file:///data/test.json", description = "Test resource" };
        let input = parse_quote! {
            async fn get_untitled_data() -> McpResult<Vec<ResourceContent>> {
                Ok(vec![])
            }
        };

        let result = mcp_resource_impl(args, input);
        assert!(result.is_ok());

        let code = result.unwrap().to_string();
        // Should have title() returning None
        assert!(
            code.contains("fn title"),
            "should generate explicit title() method even without title attribute"
        );
    }

    #[test]
    fn test_resource_metadata_traits() {
        let args = parse_quote! { uri = "file:///metadata.json", name = "test_meta", description = "Metadata test" };
        let input = parse_quote! {
            async fn test_metadata() -> McpResult<Vec<ResourceContent>> {
                Ok(vec![])
            }
        };

        let result = mcp_resource_impl(args, input);
        assert!(result.is_ok());

        let code = result.unwrap().to_string();
        assert!(code.contains("HasResourceMetadata"));
        assert!(code.contains("HasResourceDescription"));
        assert!(code.contains("HasResourceUri"));
        assert!(code.contains("HasResourceAnnotations"));
        assert!(code.contains("HasResourceMeta"));
        assert!(code.contains("\"test_meta\""));
        assert!(code.contains("\"Metadata test\""));
    }
}
