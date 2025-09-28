//! Resource Declarative Macro Implementation
//!
//! Implements the `resource!{}` declarative macro for creating MCP resources with
//! a concise syntax.

use proc_macro::TokenStream;
use quote::quote;
use syn::{Expr, Ident, LitStr, Result, Token, parse::Parse, parse::ParseStream};

use crate::macros::shared::capitalize;

/// Implementation function for the resource!{} declarative macro
pub fn resource_declarative_impl(input: TokenStream) -> Result<TokenStream> {
    let input = syn::parse::<ResourceMacroInput>(input)?;

    let resource_name_ident = syn::Ident::new(
        &format!("{}Resource", capitalize(&input.name.replace(" ", ""))),
        proc_macro2::Span::call_site(),
    );

    let uri = &input.uri;
    let name = &input.name;
    let description = &input.description;
    let content_closure = &input.content;

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
            impl turul_mcp_protocol::resources::HasResourceMetadata for #resource_name_ident {
                fn name(&self) -> &str {
                    #name
                }

                fn title(&self) -> Option<&str> {
                    None
                }
            }

            impl turul_mcp_protocol::resources::HasResourceDescription for #resource_name_ident {
                fn description(&self) -> Option<&str> {
                    Some(#description)
                }
            }

            impl turul_mcp_protocol::resources::HasResourceUri for #resource_name_ident {
                fn uri(&self) -> &str {
                    #uri
                }
            }

            impl turul_mcp_protocol::resources::HasResourceMimeType for #resource_name_ident {
                fn mime_type(&self) -> Option<&str> {
                    None
                }
            }

            impl turul_mcp_protocol::resources::HasResourceSize for #resource_name_ident {
                fn size(&self) -> Option<u64> {
                    None
                }
            }

            impl turul_mcp_protocol::resources::HasResourceAnnotations for #resource_name_ident {
                fn annotations(&self) -> Option<&turul_mcp_protocol::meta::Annotations> {
                    None
                }
            }

            impl turul_mcp_protocol::resources::HasResourceMeta for #resource_name_ident {
                fn resource_meta(&self) -> Option<&std::collections::HashMap<String, serde_json::Value>> {
                    None
                }
            }

            // ResourceDefinition automatically implemented via blanket impl!

            #[async_trait::async_trait]
            impl turul_mcp_server::McpResource for #resource_name_ident {
                async fn read(&self, params: Option<serde_json::Value>, session: Option<&turul_mcp_server::SessionContext>) -> turul_mcp_server::McpResult<Vec<turul_mcp_protocol::resources::ResourceContent>> {
                    let content_fn = #content_closure;
                    content_fn(params, session).await
                }
            }

            #resource_name_ident::new()
        }
    };

    Ok(expanded.into())
}

/// Parser for the resource! macro syntax
pub struct ResourceMacroInput {
    pub uri: String,
    pub name: String,
    pub description: String,
    pub content: Expr,
}

impl Parse for ResourceMacroInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut uri = None;
        let mut name = None;
        let mut description = None;
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
    }
}
