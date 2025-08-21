//! Utility functions for macro implementations

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, Result};


/// Extract tool metadata from attributes
#[derive(Debug)]
pub struct ToolMeta {
    pub name: String,
    pub description: String,
}

pub fn extract_tool_meta(attrs: &[Attribute]) -> Result<ToolMeta> {
    let mut name = None;
    let mut description = None;

    for attr in attrs {
        if attr.path().is_ident("tool") {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("name") {
                    let value = meta.value()?;
                    let s: syn::LitStr = value.parse()?;
                    name = Some(s.value());
                } else if meta.path.is_ident("description") {
                    let value = meta.value()?;
                    let s: syn::LitStr = value.parse()?;
                    description = Some(s.value());
                }
                Ok(())
            })?;
        }
    }

    let name = name.ok_or_else(|| {
        if attrs.is_empty() {
            syn::Error::new(proc_macro2::Span::call_site(), "Missing #[tool(name = \"...\", description = \"...\")] attribute")
        } else {
            syn::Error::new_spanned(&attrs[0], "Missing 'name' attribute in #[tool(...)]")
        }
    })?;
    
    let description = description.ok_or_else(|| {
        if attrs.is_empty() {
            syn::Error::new(proc_macro2::Span::call_site(), "Missing #[tool(name = \"...\", description = \"...\")] attribute")
        } else {
            syn::Error::new_spanned(&attrs[0], "Missing 'description' attribute in #[tool(...)]")
        }
    })?;

    Ok(ToolMeta { name, description })
}

/// Extract parameter metadata from field attributes
#[derive(Debug)]
pub struct ParamMeta {
    pub description: Option<String>,
    pub optional: bool,
    pub min: Option<f64>,
    pub max: Option<f64>,
}

impl Default for ParamMeta {
    fn default() -> Self {
        Self {
            description: None,
            optional: false,
            min: None,
            max: None,
        }
    }
}

pub fn extract_param_meta(attrs: &[Attribute]) -> Result<ParamMeta> {
    let mut meta = ParamMeta::default();

    for attr in attrs {
        if attr.path().is_ident("param") {
            attr.parse_nested_meta(|nested_meta| {
                if nested_meta.path.is_ident("description") {
                    let value = nested_meta.value()?;
                    let s: syn::LitStr = value.parse()?;
                    meta.description = Some(s.value());
                } else if nested_meta.path.is_ident("optional") {
                    meta.optional = true;
                } else if nested_meta.path.is_ident("min") {
                    let value = nested_meta.value()?;
                    let lit: syn::LitFloat = value.parse()?;
                    meta.min = Some(lit.base10_parse()?);
                } else if nested_meta.path.is_ident("max") {
                    let value = nested_meta.value()?;
                    let lit: syn::LitFloat = value.parse()?;
                    meta.max = Some(lit.base10_parse()?);
                }
                Ok(())
            })?;
        }
    }

    Ok(meta)
}

/// Generate JSON schema for a Rust type
pub fn type_to_schema(ty: &syn::Type, param_meta: &ParamMeta) -> TokenStream {
    let description = param_meta.description.as_ref().map(|d| quote! { .with_description(#d) });
    
    // Basic type mapping
    match ty {
        syn::Type::Path(type_path) => {
            if let Some(ident) = type_path.path.get_ident() {
                match ident.to_string().as_str() {
                    "String" | "str" => {
                        quote! {
                            mcp_protocol::schema::JsonSchema::string() #description
                        }
                    }
                    "f64" | "f32" => {
                        let min = param_meta.min.map(|m| quote! { .with_minimum(#m) });
                        let max = param_meta.max.map(|m| quote! { .with_maximum(#m) });
                        quote! {
                            mcp_protocol::schema::JsonSchema::number() #description #min #max
                        }
                    }
                    "i64" | "i32" | "i16" | "i8" | "u64" | "u32" | "u16" | "u8" | "isize" | "usize" => {
                        quote! {
                            mcp_protocol::schema::JsonSchema::integer() #description
                        }
                    }
                    "bool" => {
                        quote! {
                            mcp_protocol::schema::JsonSchema::boolean() #description
                        }
                    }
                    _ => {
                        quote! {
                            mcp_protocol::schema::JsonSchema::string() #description
                        }
                    }
                }
            } else {
                quote! {
                    mcp_protocol::schema::JsonSchema::string() #description
                }
            }
        }
        _ => {
            quote! {
                mcp_protocol::schema::JsonSchema::string() #description
            }
        }
    }
}

/// Generate parameter extraction code
pub fn generate_param_extraction(field_name: &syn::Ident, field_type: &syn::Type, optional: bool) -> TokenStream {
    let field_name_str = field_name.to_string();
    
    // Check if field_type is already an Option<T>
    let is_option_type = if let syn::Type::Path(type_path) = field_type {
        type_path.path.segments.len() == 1 && 
        type_path.path.segments[0].ident == "Option"
    } else {
        false
    };
    
    if is_option_type {
        // Field is already Option<T>, extract the inner type
        if let syn::Type::Path(type_path) = field_type {
            if let syn::PathArguments::AngleBracketed(args) = &type_path.path.segments[0].arguments {
                if let Some(syn::GenericArgument::Type(inner_type)) = args.args.first() {
                    // Handle Option<T> field
                    return generate_option_extraction(field_name, inner_type, field_name_str);
                }
            }
        }
        // Fallback for complex Option types
        quote! {
            let #field_name: #field_type = args.get(#field_name_str)
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or(None);
        }
    } else if optional {
        // Field is T but marked as optional, wrap in Option
        generate_optional_required_extraction(field_name, field_type, field_name_str)
    } else {
        // Required field of type T
        generate_required_extraction(field_name, field_type, field_name_str)
    }
}

/// Generate extraction code for required fields that are marked as optional via #[param(optional)]
fn generate_optional_required_extraction(field_name: &syn::Ident, field_type: &syn::Type, field_name_str: String) -> TokenStream {
    match field_type {
        syn::Type::Path(type_path) if type_path.path.get_ident().map_or(false, |i| i == "String") => {
            quote! {
                let #field_name: Option<String> = args.get(#field_name_str)
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
            }
        }
        syn::Type::Path(type_path) if type_path.path.get_ident().map_or(false, |i| i == "f64") => {
            quote! {
                let #field_name: Option<f64> = args.get(#field_name_str)
                    .and_then(|v| v.as_f64());
            }
        }
        syn::Type::Path(type_path) if type_path.path.get_ident().map_or(false, |i| i == "f32") => {
            quote! {
                let #field_name: Option<f32> = args.get(#field_name_str)
                    .and_then(|v| v.as_f64())
                    .map(|f| f as f32);
            }
        }
        syn::Type::Path(type_path) if type_path.path.get_ident().map_or(false, |i| i == "i32") => {
            quote! {
                let #field_name: Option<i32> = args.get(#field_name_str)
                    .and_then(|v| v.as_i64())
                    .and_then(|i| i.try_into().ok());
            }
        }
        syn::Type::Path(type_path) if type_path.path.get_ident().map_or(false, |i| i == "i64") => {
            quote! {
                let #field_name: Option<i64> = args.get(#field_name_str)
                    .and_then(|v| v.as_i64());
            }
        }
        syn::Type::Path(type_path) if type_path.path.get_ident().map_or(false, |i| i == "bool") => {
            quote! {
                let #field_name: Option<bool> = args.get(#field_name_str)
                    .and_then(|v| v.as_bool());
            }
        }
        _ => {
            // Generic serde deserialization for complex types
            quote! {
                let #field_name: Option<#field_type> = args.get(#field_name_str)
                    .and_then(|v| serde_json::from_value(v.clone()).ok());
            }
        }
    }
}

/// Generate extraction code for Option<T> fields
fn generate_option_extraction(field_name: &syn::Ident, inner_type: &syn::Type, field_name_str: String) -> TokenStream {
    match inner_type {
        syn::Type::Path(type_path) if type_path.path.get_ident().map_or(false, |i| i == "String") => {
            quote! {
                let #field_name: Option<String> = args.get(#field_name_str)
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
            }
        }
        syn::Type::Path(type_path) if type_path.path.get_ident().map_or(false, |i| i == "f64") => {
            quote! {
                let #field_name: Option<f64> = args.get(#field_name_str)
                    .and_then(|v| v.as_f64());
            }
        }
        syn::Type::Path(type_path) if type_path.path.get_ident().map_or(false, |i| i == "i32") => {
            quote! {
                let #field_name: Option<i32> = args.get(#field_name_str)
                    .and_then(|v| v.as_i64())
                    .and_then(|i| i.try_into().ok());
            }
        }
        syn::Type::Path(type_path) if type_path.path.get_ident().map_or(false, |i| i == "bool") => {
            quote! {
                let #field_name: Option<bool> = args.get(#field_name_str)
                    .and_then(|v| v.as_bool());
            }
        }
        _ => {
            // Generic serde deserialization for complex Option types
            quote! {
                let #field_name: Option<#inner_type> = args.get(#field_name_str)
                    .and_then(|v| serde_json::from_value(v.clone()).ok());
            }
        }
    }
}

/// Generate extraction code for required fields
fn generate_required_extraction(field_name: &syn::Ident, field_type: &syn::Type, field_name_str: String) -> TokenStream {
    match field_type {
        syn::Type::Path(type_path) if type_path.path.get_ident().map_or(false, |i| i == "String") => {
            quote! {
                let #field_name = args.get(#field_name_str)
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| mcp_protocol::McpError::invalid_param_type(#field_name_str, "string", "other"))?
                    .to_string();
            }
        }
        syn::Type::Path(type_path) if type_path.path.get_ident().map_or(false, |i| i == "f64") => {
            quote! {
                let #field_name = args.get(#field_name_str)
                    .and_then(|v| v.as_f64())
                    .ok_or_else(|| mcp_protocol::McpError::invalid_param_type(#field_name_str, "number", "other"))?;
            }
        }
        syn::Type::Path(type_path) if type_path.path.get_ident().map_or(false, |i| i == "i32") => {
            quote! {
                let #field_name = args.get(#field_name_str)
                    .and_then(|v| v.as_i64())
                    .and_then(|i| i.try_into().ok())
                    .ok_or_else(|| mcp_protocol::McpError::invalid_param_type(#field_name_str, "integer", "other"))?;
            }
        }
        syn::Type::Path(type_path) if type_path.path.get_ident().map_or(false, |i| i == "bool") => {
            quote! {
                let #field_name = args.get(#field_name_str)
                    .and_then(|v| v.as_bool())
                    .ok_or_else(|| mcp_protocol::McpError::invalid_param_type(#field_name_str, "boolean", "other"))?;
            }
        }
        _ => {
            // Generic serde deserialization for complex types
            quote! {
                let #field_name: #field_type = args.get(#field_name_str)
                    .ok_or_else(|| mcp_protocol::McpError::missing_param(#field_name_str))
                    .and_then(|v| serde_json::from_value(v.clone())
                        .map_err(|_| mcp_protocol::McpError::invalid_param_type(#field_name_str, "object", "other")))?;
            }
        }
    }
}

/// Extract a string value from an attribute by name
pub fn extract_string_attribute(attrs: &[Attribute], name: &str) -> Option<String> {
    for attr in attrs {
        if attr.path().is_ident(name) {
            if let Ok(value) = attr.meta.require_name_value() {
                if let syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(lit_str), .. }) = &value.value {
                    return Some(lit_str.value());
                }
            }
        }
    }
    None
}

/// Field metadata for resources
pub struct FieldMeta {
    pub content: Option<bool>,
    pub content_type: Option<String>,
    pub description: Option<String>,
}

impl Default for FieldMeta {
    fn default() -> Self {
        Self {
            content: None,
            content_type: None,
            description: None,
        }
    }
}

/// Extract field metadata from attributes
pub fn extract_field_meta(attrs: &[Attribute]) -> Result<FieldMeta> {
    let mut meta = FieldMeta::default();

    for attr in attrs {
        if attr.path().is_ident("content") {
            meta.content = Some(true);
        } else if attr.path().is_ident("content_type") {
            if let Ok(value) = attr.meta.require_name_value() {
                if let syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(lit_str), .. }) = &value.value {
                    meta.content_type = Some(lit_str.value());
                }
            }
        } else if attr.path().is_ident("description") {
            if let Ok(value) = attr.meta.require_name_value() {
                if let syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(lit_str), .. }) = &value.value {
                    meta.description = Some(lit_str.value());
                }
            }
        }
    }

    Ok(meta)
}