//! Logging Declarative Macro Implementation
//!
//! Implements the `logging!{}` declarative macro for creating MCP logging handlers
//! with a concise syntax.

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse::Parse, parse::ParseStream, Result, Token, Ident, LitStr, Type};

use crate::macros::shared::capitalize;

/// Implementation function for the logging!{} declarative macro
pub fn logging_declarative_impl(input: TokenStream) -> Result<TokenStream> {
    let input = syn::parse::<LoggingMacroInput>(input)?;
    let output = logging_declarative_impl_inner(input);
    Ok(output.into())
}

/// Internal implementation that works with proc_macro2::TokenStream for testing
pub fn logging_declarative_impl_inner(input: LoggingMacroInput) -> proc_macro2::TokenStream {
    let logging_name_ident = syn::Ident::new(
        &format!("{}Logger", capitalize(&input.name)),
        proc_macro2::Span::call_site()
    );
    
    let struct_fields = generate_struct_fields(&input.fields);
    let default_impl = generate_default_impl(&logging_name_ident, &input.fields);

    quote! {
        #[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
        #[derive(mcp_derive::McpLogging)]
        pub struct #logging_name_ident {
            #struct_fields
        }

        #default_impl
    }
}

/// Parse input for the logging!{} macro
struct LoggingMacroInput {
    name: String,
    fields: Vec<LoggingField>,
}

impl Parse for LoggingMacroInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let name_ident: Ident = input.parse()?;
        let name = name_ident.to_string();
        
        let content;
        syn::braced!(content in input);
        
        let mut fields = Vec::new();
        while !content.is_empty() {
            fields.push(content.parse()?);
            if content.peek(Token![,]) {
                content.parse::<Token![,]>()?;
            }
        }
        
        Ok(LoggingMacroInput { name, fields })
    }
}

/// Individual field in logging macro
struct LoggingField {
    name: String,
    field_type: Type,
    description: Option<String>,
}

impl Parse for LoggingField {
    fn parse(input: ParseStream) -> Result<Self> {
        let name_ident: Ident = input.parse()?;
        let name = name_ident.to_string();
        
        input.parse::<Token![:]>()?;
        let field_type: Type = input.parse()?;
        
        let mut description = None;
        if input.peek(Token![=]) {
            input.parse::<Token![=]>()?;
            let desc_str: LitStr = input.parse()?;
            description = Some(desc_str.value());
        }
        
        Ok(LoggingField { name, field_type, description })
    }
}

fn generate_struct_fields(fields: &[LoggingField]) -> proc_macro2::TokenStream {
    if fields.is_empty() {
        return quote! {};
    }
    
    let field_definitions = fields.iter().map(|field| {
        let field_name = syn::Ident::new(&field.name, proc_macro2::Span::call_site());
        let field_type = &field.field_type;
        
        if let Some(desc) = &field.description {
            quote! {
                #[doc = #desc]
                pub #field_name: #field_type,
            }
        } else {
            quote! {
                pub #field_name: #field_type,
            }
        }
    });
    
    quote! {
        #(#field_definitions)*
    }
}

fn generate_default_impl(struct_name: &syn::Ident, fields: &[LoggingField]) -> proc_macro2::TokenStream {
    if fields.is_empty() {
        return quote! {
            impl Default for #struct_name {
                fn default() -> Self {
                    Self
                }
            }
        };
    }
    
    let field_defaults = fields.iter().map(|field| {
        let field_name = syn::Ident::new(&field.name, proc_macro2::Span::call_site());
        let field_type = &field.field_type;
        let default_value = match field_type {
            syn::Type::Path(path) if path.path.is_ident("String") => quote! { String::new() },
            syn::Type::Path(path) if path.path.is_ident("i32") => quote! { 0 },
            syn::Type::Path(path) if path.path.is_ident("u32") => quote! { 0 },
            syn::Type::Path(path) if path.path.is_ident("f64") => quote! { 0.0 },
            syn::Type::Path(path) if path.path.is_ident("bool") => quote! { false },
            _ => quote! { Default::default() },
        };
        
        quote! {
            #field_name: #default_value,
        }
    });
    
    quote! {
        impl Default for #struct_name {
            fn default() -> Self {
                Self {
                    #(#field_defaults)*
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logging_macro_parse() {
        let input = syn::parse_str::<LoggingMacroInput>("file_logger { log_level: String = \"Logging level\", file_path: String = \"Log file path\" }").unwrap();

        let result = logging_declarative_impl_inner(input);
        let code = result.to_string();
        assert!(code.contains("FileLoggerLogger"));
        assert!(code.contains("McpLogging"));
    }

    #[test]
    fn test_logging_macro_unit_struct() {
        let input = syn::parse_str::<LoggingMacroInput>("console {}").unwrap();

        let result = logging_declarative_impl_inner(input);
        let code = result.to_string();
        assert!(code.contains("ConsoleLogger"));
    }
}