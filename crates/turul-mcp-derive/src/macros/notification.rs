//! Notification Declarative Macro Implementation
//!
//! Implements the `notification!{}` declarative macro for creating MCP notifications with
//! a concise syntax and zero-configuration method generation.

use proc_macro::TokenStream;
use quote::quote;
use syn::{Ident, LitStr, Result, Token, parse::Parse, parse::ParseStream};

use crate::macros::shared::capitalize;

/// Implementation function for the notification!{} declarative macro
pub fn notification_declarative_impl(input: TokenStream) -> Result<TokenStream> {
    let input = syn::parse::<NotificationMacroInput>(input)?;
    let output = notification_declarative_impl_inner(input);
    Ok(output.into())
}

/// Internal implementation that works with proc_macro2::TokenStream for testing
pub fn notification_declarative_impl_inner(
    input: NotificationMacroInput,
) -> proc_macro2::TokenStream {
    let notification_name_ident = syn::Ident::new(
        &format!("{}Notification", capitalize(&input.name)),
        proc_macro2::Span::call_site(),
    );

    let struct_fields = generate_struct_fields(&input.fields);
    let default_impl = generate_default_impl(&notification_name_ident, &input.fields);

    quote! {
        #[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
        #[derive(mcp_derive::McpNotification)]
        pub struct #notification_name_ident {
            #struct_fields
        }

        #default_impl
    }
}

/// Parse input for the notification!{} macro
pub struct NotificationMacroInput {
    name: String,
    fields: Vec<NotificationField>,
}

impl Parse for NotificationMacroInput {
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

        Ok(NotificationMacroInput { name, fields })
    }
}

/// Individual field in notification macro
struct NotificationField {
    name: String,
    field_type: syn::Type,
    description: Option<String>,
}

impl Parse for NotificationField {
    fn parse(input: ParseStream) -> Result<Self> {
        let name_ident: Ident = input.parse()?;
        let name = name_ident.to_string();

        input.parse::<Token![:]>()?;
        let field_type: syn::Type = input.parse()?;

        let mut description = None;
        if input.peek(Token![=]) {
            input.parse::<Token![=]>()?;
            let desc_str: LitStr = input.parse()?;
            description = Some(desc_str.value());
        }

        Ok(NotificationField {
            name,
            field_type,
            description,
        })
    }
}

fn generate_struct_fields(fields: &[NotificationField]) -> proc_macro2::TokenStream {
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

fn generate_default_impl(
    struct_name: &syn::Ident,
    fields: &[NotificationField],
) -> proc_macro2::TokenStream {
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
        // Generate sensible defaults based on type
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
    fn test_notification_macro_parse() {
        let input =
            syn::parse_str::<NotificationMacroInput>("progress { message: String, percent: u32 }")
                .unwrap();

        let result = notification_declarative_impl_inner(input);
        let code = result.to_string();
        assert!(code.contains("ProgressNotification"));
        assert!(code.contains("McpNotification"));
    }

    #[test]
    fn test_notification_macro_unit_struct() {
        let input = syn::parse_str::<NotificationMacroInput>("simple {}").unwrap();

        let result = notification_declarative_impl_inner(input);
        let code = result.to_string();
        assert!(code.contains("SimpleNotification"));
    }

    #[test]
    fn test_zero_config_method_generation() {
        // The derive macro should auto-generate method names
        let input =
            syn::parse_str::<NotificationMacroInput>("resources_changed { uri: String }").unwrap();

        let result = notification_declarative_impl_inner(input);
        // Should generate ResourcesChangedNotification which maps to "notifications/resources/list_changed"
        let code = result.to_string();
        assert!(code.contains("ResourcesChangedNotification"));
    }
}
