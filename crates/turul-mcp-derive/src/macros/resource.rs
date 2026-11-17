//! Resource Declarative Macro Implementation
//!
//! Implements the `resource!{}` declarative macro for creating MCP resources with
//! a concise syntax.

use proc_macro::TokenStream;
use quote::quote;
use syn::{Expr, Ident, LitStr, Result, Token, parse::Parse, parse::ParseStream};

use crate::macros::shared::capitalize;

/// Expands the `resource!{}` declarative macro.
pub fn resource_declarative_impl(input: TokenStream) -> Result<TokenStream> {
    let async_trait = crate::utils::async_trait_path();
    let json_crate = crate::utils::serde_json_path();
    let mcp_builders = crate::utils::builders_path();
    let mcp_protocol = crate::utils::protocol_path();
    let mcp_server = crate::utils::server_path();
    let input = syn::parse::<ResourceMacroInput>(input)?;

    let resource_name_ident = syn::Ident::new(
        &format!("{}Resource", capitalize(&input.name.replace(" ", ""))),
        proc_macro2::Span::call_site(),
    );

    let uri = &input.uri;
    let name = &input.name;
    let description = &input.description;
    let content_closure = &input.content;

    // Generate title expression for HasResourceMetadata::title()
    let title_expr = match &input.title {
        Some(t) => quote! { Some(#t) },
        None => quote! { None },
    };
    let mime_type_expr = match &input.mime_type {
        Some(m) => quote! { Some(#m) },
        None => quote! { None },
    };
    let size_impl =
        crate::tool_derive::generate_resource_size_impl(&resource_name_ident, input.size.as_ref());
    let annotations_impl = crate::tool_derive::generate_resource_annotations_impl(
        &resource_name_ident,
        input.annotations.as_ref(),
    );
    let meta_impl = crate::tool_derive::generate_meta_impl(
        &resource_name_ident,
        "HasResourceMeta",
        "resource_meta",
        input.meta.as_ref(),
    );
    let icons_impl =
        crate::tool_derive::generate_icons_impl(&resource_name_ident, input.icons.as_ref());

    let expanded = quote! {
        {
            #[derive(Clone)]
            struct #resource_name_ident;

            impl #resource_name_ident {
                fn new() -> Self {
                    Self
                }
            }

            // Implement fine-grained traits
            impl #mcp_builders::HasResourceMetadata for #resource_name_ident {
                fn name(&self) -> &str {
                    #name
                }

                fn title(&self) -> Option<&str> {
                    #title_expr
                }
            }

            impl #mcp_builders::HasResourceDescription for #resource_name_ident {
                fn description(&self) -> Option<&str> {
                    Some(#description)
                }
            }

            impl #mcp_builders::HasResourceUri for #resource_name_ident {
                fn uri(&self) -> &str {
                    #uri
                }
            }

            impl #mcp_builders::HasResourceMimeType for #resource_name_ident {
                fn mime_type(&self) -> Option<&str> {
                    #mime_type_expr
                }
            }

            #size_impl

            #annotations_impl

            #meta_impl

            #icons_impl

            // ResourceDefinition automatically implemented via blanket impl!

            #[#async_trait::async_trait]
            impl #mcp_server::McpResource for #resource_name_ident {
                async fn read(&self, params: Option<#json_crate::Value>, session: Option<&#mcp_server::SessionContext>) -> #mcp_server::McpResult<Vec<#mcp_protocol::resources::ResourceContent>> {
                    let content_fn = #content_closure;
                    content_fn(params, session).await
                }
            }

            #resource_name_ident::new()
        }
    };

    Ok(expanded.into())
}

/// Parsed input of the `resource!{}` macro.
pub struct ResourceMacroInput {
    pub uri: String,
    pub name: String,
    pub description: String,
    pub title: Option<String>,
    pub mime_type: Option<Expr>,
    pub size: Option<Expr>,
    pub meta: Option<Expr>,
    pub icons: Option<Expr>,
    pub annotations: Option<Expr>,
    pub content: Expr,
}

impl Parse for ResourceMacroInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut uri = None;
        let mut name = None;
        let mut description = None;
        let mut title = None;
        let mut mime_type = None;
        let mut size = None;
        let mut meta = None;
        let mut icons = None;
        let mut annotations = None;
        let mut content = None;

        while !input.is_empty() {
            let ident: Ident = input.parse()?;
            input.parse::<Token![:]>()?;

            match ident.to_string().as_str() {
                "uri" => {
                    let lit: LitStr = input.parse()?;
                    uri = Some(lit.value());
                }
                "name" => {
                    let lit: LitStr = input.parse()?;
                    name = Some(lit.value());
                }
                "description" => {
                    let lit: LitStr = input.parse()?;
                    description = Some(lit.value());
                }
                "title" => {
                    let lit: LitStr = input.parse()?;
                    title = Some(lit.value());
                }
                "mime_type" => mime_type = Some(input.parse::<Expr>()?),
                "size" => size = Some(input.parse::<Expr>()?),
                "meta" => meta = Some(input.parse::<Expr>()?),
                "icons" => icons = Some(input.parse::<Expr>()?),
                "annotations" => annotations = Some(input.parse::<Expr>()?),
                "content" => {
                    let expr: Expr = input.parse()?;
                    content = Some(expr);
                }
                _ => {
                    return Err(syn::Error::new_spanned(
                        &ident,
                        format!("Unknown field: {}", ident),
                    ));
                }
            }

            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(ResourceMacroInput {
            uri: uri.ok_or_else(|| syn::Error::new(input.span(), "Missing 'uri' field"))?,
            name: name.ok_or_else(|| syn::Error::new(input.span(), "Missing 'name' field"))?,
            description: description
                .ok_or_else(|| syn::Error::new(input.span(), "Missing 'description' field"))?,
            title,
            mime_type,
            size,
            meta,
            icons,
            annotations,
            content: content
                .ok_or_else(|| syn::Error::new(input.span(), "Missing 'content' field"))?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_macro_parse() {
        let input = quote::quote! {
            uri: "file://test.txt",
            name: "Test Resource",
            description: "A test resource",
            content: |_self| async { Ok(vec![]) }
        };

        let parsed = syn::parse2::<ResourceMacroInput>(input).unwrap();
        assert_eq!(parsed.uri, "file://test.txt");
        assert_eq!(parsed.name, "Test Resource");
        assert_eq!(parsed.description, "A test resource");
        assert_eq!(parsed.title, None);
    }

    #[test]
    fn test_resource_macro_parse_with_title() {
        let input = quote::quote! {
            uri: "file://test.txt",
            name: "Test Resource",
            description: "A test resource",
            title: "My Custom Title",
            content: |_self| async { Ok(vec![]) }
        };

        let parsed = syn::parse2::<ResourceMacroInput>(input).unwrap();
        assert_eq!(parsed.uri, "file://test.txt");
        assert_eq!(parsed.name, "Test Resource");
        assert_eq!(parsed.title, Some("My Custom Title".to_string()));
    }

    #[test]
    fn test_resource_macro_parse_spec_fields() {
        let input = quote::quote! {
            uri: "ui://test/view.html",
            name: "view",
            description: "A view",
            mime_type: MIME,
            size: 4,
            meta: meta(),
            icons: vec![],
            annotations: Annotations::new(),
            content: |_, _| async { Ok(vec![]) }
        };

        let parsed = syn::parse2::<ResourceMacroInput>(input).unwrap();
        for (field, expr) in [
            ("mime_type", &parsed.mime_type),
            ("size", &parsed.size),
            ("meta", &parsed.meta),
            ("icons", &parsed.icons),
            ("annotations", &parsed.annotations),
        ] {
            assert!(expr.is_some(), "{field} not parsed");
        }
    }
}
