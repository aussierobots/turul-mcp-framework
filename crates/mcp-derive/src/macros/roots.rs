//! Roots Declarative Macro Implementation
//!
//! Implements the `roots!{}` declarative macro for creating MCP root handlers
//! with a concise syntax.

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse::Parse, parse::ParseStream, Result, Token, Ident, LitStr, LitBool};

use crate::macros::shared::capitalize;

/// Implementation function for the roots!{} declarative macro
pub fn roots_declarative_impl(input: TokenStream) -> Result<TokenStream> {
    let input = syn::parse::<RootsMacroInput>(input)?;
    let output = roots_declarative_impl_inner(input);
    Ok(output.into())
}

/// Internal implementation that works with proc_macro2::TokenStream for testing
pub fn roots_declarative_impl_inner(input: RootsMacroInput) -> proc_macro2::TokenStream {
    let roots_name_ident = syn::Ident::new(
        &format!("{}Root", capitalize(&input.name)),
        proc_macro2::Span::call_site()
    );
    
    let uri = &input.uri;
    let name = input.display_name.as_deref().unwrap_or(&input.name);
    let description = input.description.as_deref().unwrap_or("Root directory");
    let read_only = input.read_only;

    quote! {
        #[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
        #[derive(mcp_derive::McpRoot)]
        #[root(uri = #uri, name = #name, description = #description, read_only = #read_only)]
        pub struct #roots_name_ident;

        impl Default for #roots_name_ident {
            fn default() -> Self {
                Self
            }
        }
    }
}

/// Parse input for the roots!{} macro
struct RootsMacroInput {
    name: String,
    uri: String,
    display_name: Option<String>,
    description: Option<String>,
    read_only: bool,
}

impl Parse for RootsMacroInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let name_ident: Ident = input.parse()?;
        let name = name_ident.to_string();
        
        input.parse::<Token![,]>()?;
        let uri_str: LitStr = input.parse()?;
        let uri = uri_str.value();
        
        let mut display_name = None;
        let mut description = None;
        let mut read_only = false;
        
        // Parse optional parameters
        while input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            
            if input.is_empty() {
                break;
            }
            
            let param_name: Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            
            match param_name.to_string().as_str() {
                "name" => {
                    let name_str: LitStr = input.parse()?;
                    display_name = Some(name_str.value());
                }
                "description" => {
                    let desc_str: LitStr = input.parse()?;
                    description = Some(desc_str.value());
                }
                "read_only" => {
                    // Handle both boolean literals and string literals
                    if input.peek(LitBool) {
                        let bool_lit: LitBool = input.parse()?;
                        read_only = bool_lit.value;
                    } else {
                        let bool_str: LitStr = input.parse()?;
                        read_only = bool_str.value().parse().unwrap_or(false);
                    }
                }
                _ => {
                    return Err(syn::Error::new_spanned(
                        param_name,
                        "Unknown parameter. Valid parameters are: name, description, read_only"
                    ));
                }
            }
        }
        
        Ok(RootsMacroInput {
            name,
            uri,
            display_name,
            description,
            read_only,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roots_macro_simple() {
        let input = syn::parse_str::<RootsMacroInput>(r#"project, "/path/to/project""#).unwrap();

        let result = roots_declarative_impl_inner(input);
        let code = result.to_string();
        assert!(code.contains("ProjectRoot"));
        assert!(code.contains("McpRoot"));
        assert!(code.contains("/path/to/project"));
    }

    #[test]
    fn test_roots_macro_with_params() {
        let input = syn::parse_str::<RootsMacroInput>(r#"docs, "/usr/share/docs", name = "Documentation", description = "System documentation", read_only = true"#).unwrap();

        let result = roots_declarative_impl_inner(input);
        let code = result.to_string();
        assert!(code.contains("DocsRoot"));
        assert!(code.contains("Documentation"));
        assert!(code.contains("System documentation"));
        assert!(code.contains("read_only = true"));
    }

    #[test]
    fn test_roots_macro_read_only_string() {
        let input = syn::parse_str::<RootsMacroInput>(r#"readonly, "/readonly", read_only = "true""#).unwrap();

        let result = roots_declarative_impl_inner(input);
        let code = result.to_string();
        assert!(code.contains("ReadonlyRoot"));
        assert!(code.contains("read_only = true"));
    }
}