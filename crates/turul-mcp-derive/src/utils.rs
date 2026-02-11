//! Utility functions for macro implementations

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, Data, DeriveInput, Fields, Result};

/// Extract tool metadata from attributes
#[derive(Debug)]
pub struct ToolMeta {
    pub name: String,
    pub description: String,
    pub output_type: Option<syn::Type>,
    pub output_field: Option<String>, // Custom field name for output
}

pub fn extract_tool_meta(attrs: &[Attribute]) -> Result<ToolMeta> {
    let mut name = None;
    let mut description = None;
    let mut output_type = None;
    let mut output_field = None;

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
                } else if meta.path.is_ident("output") {
                    let value = meta.value()?;
                    let ty: syn::Type = value.parse()?;
                    output_type = Some(ty);
                } else if meta.path.is_ident("field") || meta.path.is_ident("output_field") {
                    let value = meta.value()?;
                    let s: syn::LitStr = value.parse()?;
                    output_field = Some(s.value());
                }
                Ok(())
            })?;
        } else if attr.path().is_ident("output_type") {
            // Parse #[output_type(TypeName)] syntax for backward compatibility
            if let syn::Meta::List(meta_list) = &attr.meta {
                let ty: syn::Type = syn::parse2(meta_list.tokens.clone())?;
                output_type = Some(ty);
            }
        }
    }

    let name = name.ok_or_else(|| {
        if attrs.is_empty() {
            syn::Error::new(
                proc_macro2::Span::call_site(),
                "Missing #[tool(name = \"...\", description = \"...\")] attribute",
            )
        } else {
            syn::Error::new_spanned(&attrs[0], "Missing 'name' attribute in #[tool(...)]")
        }
    })?;

    let description = description.ok_or_else(|| {
        if attrs.is_empty() {
            syn::Error::new(
                proc_macro2::Span::call_site(),
                "Missing #[tool(name = \"...\", description = \"...\")] attribute",
            )
        } else {
            syn::Error::new_spanned(&attrs[0], "Missing 'description' attribute in #[tool(...)]")
        }
    })?;

    Ok(ToolMeta {
        name,
        description,
        output_type,
        output_field,
    })
}

/// Extract parameter metadata from field attributes
#[derive(Debug, Default)]
pub struct ParamMeta {
    pub description: Option<String>,
    pub optional: bool,
    pub min: Option<f64>,
    pub max: Option<f64>,
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
                    // Handle both #[param(optional)] and #[param(optional = true/false)]
                    if nested_meta.input.peek(syn::Token![=]) {
                        // #[param(optional = true/false)]
                        let value = nested_meta.value()?;
                        if let Ok(lit_bool) = value.parse::<syn::LitBool>() {
                            meta.optional = lit_bool.value;
                        } else if let Ok(lit_str) = value.parse::<syn::LitStr>() {
                            meta.optional = lit_str.value().parse::<bool>().unwrap_or(false);
                        } else {
                            meta.optional = true; // Fallback
                        }
                    } else {
                        // #[param(optional)] - standalone flag defaults to true
                        meta.optional = true;
                    }
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
    let description = param_meta
        .description
        .as_ref()
        .map(|d| quote! { .with_description(#d) });

    // Basic type mapping
    match ty {
        syn::Type::Path(type_path) => {
            if let Some(ident) = type_path.path.get_ident() {
                match ident.to_string().as_str() {
                    "String" | "str" => {
                        quote! {
                            turul_mcp_protocol::schema::JsonSchema::string() #description
                        }
                    }
                    "f64" | "f32" => {
                        let min = param_meta.min.map(|m| quote! { .with_minimum(#m) });
                        let max = param_meta.max.map(|m| quote! { .with_maximum(#m) });
                        quote! {
                            turul_mcp_protocol::schema::JsonSchema::number() #description #min #max
                        }
                    }
                    "i64" | "i32" | "i16" | "i8" | "u64" | "u32" | "u16" | "u8" | "isize"
                    | "usize" => {
                        let min = param_meta.min.map(|m| {
                            let m_int = m as i64;
                            quote! { .with_minimum(#m_int as f64) }
                        });
                        let max = param_meta.max.map(|m| {
                            let m_int = m as i64;
                            quote! { .with_maximum(#m_int as f64) }
                        });
                        quote! {
                            turul_mcp_protocol::schema::JsonSchema::integer() #description #min #max
                        }
                    }
                    "bool" => {
                        quote! {
                            turul_mcp_protocol::schema::JsonSchema::boolean() #description
                        }
                    }
                    _ => {
                        // Check if this is Vec<T>
                        if type_path.path.segments.len() == 1
                            && type_path.path.segments[0].ident == "Vec"
                        {
                            quote! {
                                turul_mcp_protocol::schema::JsonSchema::array() #description
                            }
                        } else {
                            quote! {
                                turul_mcp_protocol::schema::JsonSchema::string() #description
                            }
                        }
                    }
                }
            } else {
                quote! {
                    turul_mcp_protocol::schema::JsonSchema::string() #description
                }
            }
        }
        _ => {
            quote! {
                turul_mcp_protocol::schema::JsonSchema::string() #description
            }
        }
    }
}

/// Generate parameter extraction code
pub fn generate_param_extraction(
    field_name: &syn::Ident,
    field_type: &syn::Type,
    optional: bool,
) -> TokenStream {
    let field_name_str = field_name.to_string();

    // Check if field_type is already an Option<T>
    let is_option_type = if let syn::Type::Path(type_path) = field_type {
        type_path.path.segments.len() == 1 && type_path.path.segments[0].ident == "Option"
    } else {
        false
    };

    if is_option_type {
        // Field is already Option<T>, extract the inner type
        if let syn::Type::Path(type_path) = field_type
            && let syn::PathArguments::AngleBracketed(args) = &type_path.path.segments[0].arguments
            && let Some(syn::GenericArgument::Type(inner_type)) = args.args.first()
        {
            // Handle Option<T> field
            return generate_option_extraction(field_name, inner_type, field_name_str);
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
fn generate_optional_required_extraction(
    field_name: &syn::Ident,
    field_type: &syn::Type,
    field_name_str: String,
) -> TokenStream {
    match field_type {
        syn::Type::Path(type_path) if type_path.path.get_ident().is_some_and(|i| i == "String") => {
            quote! {
                let #field_name: Option<String> = args.get(#field_name_str)
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
            }
        }
        syn::Type::Path(type_path) if type_path.path.get_ident().is_some_and(|i| i == "f64") => {
            quote! {
                let #field_name: Option<f64> = args.get(#field_name_str)
                    .and_then(|v| v.as_f64());
            }
        }
        syn::Type::Path(type_path) if type_path.path.get_ident().is_some_and(|i| i == "f32") => {
            quote! {
                let #field_name: Option<f32> = args.get(#field_name_str)
                    .and_then(|v| v.as_f64())
                    .map(|f| f as f32);
            }
        }
        syn::Type::Path(type_path) if type_path.path.get_ident().is_some_and(|i| i == "i32") => {
            quote! {
                let #field_name: Option<i32> = args.get(#field_name_str)
                    .and_then(|v| v.as_i64())
                    .and_then(|i| i.try_into().ok());
            }
        }
        syn::Type::Path(type_path) if type_path.path.get_ident().is_some_and(|i| i == "i64") => {
            quote! {
                let #field_name: Option<i64> = args.get(#field_name_str)
                    .and_then(|v| v.as_i64());
            }
        }
        syn::Type::Path(type_path) if type_path.path.get_ident().is_some_and(|i| i == "bool") => {
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
fn generate_option_extraction(
    field_name: &syn::Ident,
    inner_type: &syn::Type,
    field_name_str: String,
) -> TokenStream {
    match inner_type {
        syn::Type::Path(type_path) if type_path.path.get_ident().is_some_and(|i| i == "String") => {
            quote! {
                let #field_name: Option<String> = args.get(#field_name_str)
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
            }
        }
        syn::Type::Path(type_path) if type_path.path.get_ident().is_some_and(|i| i == "f64") => {
            quote! {
                let #field_name: Option<f64> = args.get(#field_name_str)
                    .and_then(|v| v.as_f64());
            }
        }
        syn::Type::Path(type_path) if type_path.path.get_ident().is_some_and(|i| i == "i32") => {
            quote! {
                let #field_name: Option<i32> = args.get(#field_name_str)
                    .and_then(|v| v.as_i64())
                    .and_then(|i| i.try_into().ok());
            }
        }
        syn::Type::Path(type_path) if type_path.path.get_ident().is_some_and(|i| i == "i64") => {
            quote! {
                let #field_name: Option<i64> = args.get(#field_name_str)
                    .and_then(|v| v.as_i64());
            }
        }
        syn::Type::Path(type_path) if type_path.path.get_ident().is_some_and(|i| i == "u32") => {
            quote! {
                let #field_name: Option<u32> = args.get(#field_name_str)
                    .and_then(|v| v.as_u64())
                    .and_then(|i| i.try_into().ok());
            }
        }
        syn::Type::Path(type_path) if type_path.path.get_ident().is_some_and(|i| i == "u64") => {
            quote! {
                let #field_name: Option<u64> = args.get(#field_name_str)
                    .and_then(|v| v.as_u64());
            }
        }
        syn::Type::Path(type_path) if type_path.path.get_ident().is_some_and(|i| i == "f32") => {
            quote! {
                let #field_name: Option<f32> = args.get(#field_name_str)
                    .and_then(|v| v.as_f64())
                    .map(|f| f as f32);
            }
        }
        syn::Type::Path(type_path) if type_path.path.get_ident().is_some_and(|i| i == "bool") => {
            quote! {
                let #field_name: Option<bool> = args.get(#field_name_str)
                    .and_then(|v| v.as_bool());
            }
        }
        _ => {
            // Check if this is Option<Vec<T>>
            if let syn::Type::Path(inner_path) = inner_type
                && inner_path.path.segments.len() == 1
                && inner_path.path.segments[0].ident == "Vec"
            {
                return quote! {
                    let #field_name: Option<#inner_type> = args.get(#field_name_str)
                        .and_then(|v| v.as_array())
                        .map(|arr| arr.iter()
                            .map(|item| serde_json::from_value(item.clone()))
                            .collect::<Result<Vec<_>, _>>()
                            .ok())
                        .flatten();
                };
            }

            // Generic serde deserialization for complex Option types
            quote! {
                let #field_name: Option<#inner_type> = args.get(#field_name_str)
                    .and_then(|v| serde_json::from_value(v.clone()).ok());
            }
        }
    }
}

/// Generate extraction code for required fields
fn generate_required_extraction(
    field_name: &syn::Ident,
    field_type: &syn::Type,
    field_name_str: String,
) -> TokenStream {
    match field_type {
        syn::Type::Path(type_path) if type_path.path.get_ident().is_some_and(|i| i == "String") => {
            quote! {
                let #field_name = args.get(#field_name_str)
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| turul_mcp_protocol::McpError::invalid_param_type(#field_name_str, "string", "other"))?
                    .to_string();
            }
        }
        syn::Type::Path(type_path) if type_path.path.get_ident().is_some_and(|i| i == "f64") => {
            quote! {
                let #field_name = args.get(#field_name_str)
                    .and_then(|v| v.as_f64())
                    .ok_or_else(|| turul_mcp_protocol::McpError::invalid_param_type(#field_name_str, "number", "other"))?;
            }
        }
        syn::Type::Path(type_path) if type_path.path.get_ident().is_some_and(|i| i == "i32") => {
            quote! {
                let #field_name = args.get(#field_name_str)
                    .and_then(|v| v.as_i64())
                    .and_then(|i| i.try_into().ok())
                    .ok_or_else(|| turul_mcp_protocol::McpError::invalid_param_type(#field_name_str, "integer", "other"))?;
            }
        }
        syn::Type::Path(type_path) if type_path.path.get_ident().is_some_and(|i| i == "i64") => {
            quote! {
                let #field_name = args.get(#field_name_str)
                    .and_then(|v| v.as_i64())
                    .ok_or_else(|| turul_mcp_protocol::McpError::invalid_param_type(#field_name_str, "integer", "other"))?;
            }
        }
        syn::Type::Path(type_path) if type_path.path.get_ident().is_some_and(|i| i == "u32") => {
            quote! {
                let #field_name = args.get(#field_name_str)
                    .and_then(|v| v.as_u64())
                    .and_then(|i| i.try_into().ok())
                    .ok_or_else(|| turul_mcp_protocol::McpError::invalid_param_type(#field_name_str, "integer", "other"))?;
            }
        }
        syn::Type::Path(type_path) if type_path.path.get_ident().is_some_and(|i| i == "u64") => {
            quote! {
                let #field_name = args.get(#field_name_str)
                    .and_then(|v| v.as_u64())
                    .ok_or_else(|| turul_mcp_protocol::McpError::invalid_param_type(#field_name_str, "integer", "other"))?;
            }
        }
        syn::Type::Path(type_path) if type_path.path.get_ident().is_some_and(|i| i == "f32") => {
            quote! {
                let #field_name = args.get(#field_name_str)
                    .and_then(|v| v.as_f64())
                    .map(|f| f as f32)
                    .ok_or_else(|| turul_mcp_protocol::McpError::invalid_param_type(#field_name_str, "number", "other"))?;
            }
        }
        syn::Type::Path(type_path) if type_path.path.get_ident().is_some_and(|i| i == "bool") => {
            quote! {
                let #field_name = args.get(#field_name_str)
                    .and_then(|v| v.as_bool())
                    .ok_or_else(|| turul_mcp_protocol::McpError::invalid_param_type(#field_name_str, "boolean", "other"))?;
            }
        }
        syn::Type::Path(type_path)
            if type_path.path.segments.len() == 1 && type_path.path.segments[0].ident == "Vec" =>
        {
            quote! {
                let #field_name: #field_type = args.get(#field_name_str)
                    .ok_or_else(|| turul_mcp_protocol::McpError::missing_param(#field_name_str))
                    .and_then(|v| {
                        // Handle both array values and string representations of arrays
                        if let Some(arr) = v.as_array() {
                            // Direct JSON array
                            arr.iter()
                                .map(|item| serde_json::from_value(item.clone()))
                                .collect::<Result<Vec<_>, _>>()
                                .map_err(|_| turul_mcp_protocol::McpError::invalid_param_type(#field_name_str, "array of valid items", "other"))
                        } else if let Some(s) = v.as_str() {
                            // String representation of array like "[1,2,3,4]"
                            serde_json::from_str::<Vec<serde_json::Value>>(s)
                                .map_err(|_| turul_mcp_protocol::McpError::invalid_param_type(#field_name_str, "array or array string", "invalid string"))
                                .and_then(|arr| {
                                    arr.iter()
                                        .map(|item| serde_json::from_value(item.clone()))
                                        .collect::<Result<Vec<_>, _>>()
                                        .map_err(|_| turul_mcp_protocol::McpError::invalid_param_type(#field_name_str, "array of valid items", "other"))
                                })
                        } else {
                            Err(turul_mcp_protocol::McpError::invalid_param_type(#field_name_str, "array or array string", "other"))
                        }
                    })?;
            }
        }
        _ => {
            // Generic serde deserialization for complex types
            quote! {
                let #field_name: #field_type = args.get(#field_name_str)
                    .ok_or_else(|| turul_mcp_protocol::McpError::missing_param(#field_name_str))
                    .and_then(|v| serde_json::from_value(v.clone())
                        .map_err(|_| turul_mcp_protocol::McpError::invalid_param_type(#field_name_str, "object", "other")))?;
            }
        }
    }
}

/// Extract a string value from an attribute by name
pub fn extract_string_attribute(attrs: &[Attribute], name: &str) -> Option<String> {
    for attr in attrs {
        if attr.path().is_ident(name)
            && let Ok(value) = attr.meta.require_name_value()
            && let syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(lit_str),
                ..
            }) = &value.value
        {
            return Some(lit_str.value());
        }
    }
    None
}

/// Field metadata for resources
#[derive(Default)]
pub struct FieldMeta {
    pub content: Option<bool>,
    pub content_type: Option<String>,
    pub description: Option<String>,
}

/// Prompt metadata extracted from attributes
#[derive(Debug)]
pub struct PromptMeta {
    pub name: String,
    pub description: String,
}

/// Resource metadata extracted from attributes
#[derive(Debug)]
#[allow(dead_code)] // Some fields may be unused in current implementation
pub struct ResourceMeta {
    pub name: String,
    pub uri: String,
    pub description: String,
    pub title: Option<String>,
    pub mime_type: Option<String>,
}

/// Extract prompt metadata from #[prompt(...)] attributes
pub fn extract_prompt_meta(attrs: &[Attribute]) -> Result<PromptMeta> {
    let mut name = None;
    let mut description = None;

    for attr in attrs {
        if attr.path().is_ident("prompt") {
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
        syn::Error::new(
            proc_macro2::Span::call_site(),
            "Missing 'name' in #[prompt(name = \"...\")]",
        )
    })?;

    let description = description.unwrap_or_else(|| "Generated prompt".to_string());

    Ok(PromptMeta { name, description })
}

/// Extract resource metadata from #[resource(...)] attributes
pub fn extract_resource_meta(attrs: &[Attribute]) -> Result<ResourceMeta> {
    let mut name = None;
    let mut uri = None;
    let mut description = None;
    let mut title = None;
    let mut mime_type = None;

    for attr in attrs {
        if attr.path().is_ident("resource") {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("name") {
                    let value = meta.value()?;
                    let s: syn::LitStr = value.parse()?;
                    name = Some(s.value());
                } else if meta.path.is_ident("uri") {
                    let value = meta.value()?;
                    let s: syn::LitStr = value.parse()?;
                    uri = Some(s.value());
                } else if meta.path.is_ident("description") {
                    let value = meta.value()?;
                    let s: syn::LitStr = value.parse()?;
                    description = Some(s.value());
                } else if meta.path.is_ident("title") {
                    let value = meta.value()?;
                    let s: syn::LitStr = value.parse()?;
                    title = Some(s.value());
                } else if meta.path.is_ident("mime_type") {
                    let value = meta.value()?;
                    let s: syn::LitStr = value.parse()?;
                    mime_type = Some(s.value());
                }
                Ok(())
            })?;
        }
    }

    let name = name.ok_or_else(|| {
        syn::Error::new(
            proc_macro2::Span::call_site(),
            "Missing 'name' in #[resource(name = \"...\")]",
        )
    })?;

    let uri = uri.ok_or_else(|| {
        syn::Error::new(
            proc_macro2::Span::call_site(),
            "Missing 'uri' in #[resource(uri = \"...\")]",
        )
    })?;

    let description = description.unwrap_or_else(|| "Generated resource".to_string());

    Ok(ResourceMeta {
        name,
        uri,
        description,
        title,
        mime_type,
    })
}

/// Root metadata extracted from attributes
#[derive(Debug)]
pub struct RootMeta {
    pub uri: String,
    pub name: String,
    pub description: String,
    pub read_only: bool,
}

/// Extract root metadata from #[root(...)] attributes
pub fn extract_root_meta(attrs: &[Attribute]) -> Result<RootMeta> {
    let mut uri = None;
    let mut name = None;
    let mut description = None;
    let mut read_only = None;

    for attr in attrs {
        if attr.path().is_ident("root") {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("uri") {
                    let value = meta.value()?;
                    let s: syn::LitStr = value.parse()?;
                    uri = Some(s.value());
                } else if meta.path.is_ident("name") {
                    let value = meta.value()?;
                    let s: syn::LitStr = value.parse()?;
                    name = Some(s.value());
                } else if meta.path.is_ident("description") {
                    let value = meta.value()?;
                    let s: syn::LitStr = value.parse()?;
                    description = Some(s.value());
                } else if meta.path.is_ident("read_only") {
                    let value = meta.value()?;
                    // Handle both boolean literals and string literals
                    if let Ok(b) = value.parse::<syn::LitBool>() {
                        read_only = Some(b.value);
                    } else if let Ok(s) = value.parse::<syn::LitStr>() {
                        read_only = Some(s.value().parse::<bool>().unwrap_or(false));
                    }
                }
                Ok(())
            })?;
        }
    }

    let uri = uri.ok_or_else(|| {
        syn::Error::new(
            proc_macro2::Span::call_site(),
            "Missing 'uri' in #[root(uri = \"...\")]",
        )
    })?;

    let name = name.unwrap_or_else(|| "Unnamed Root".to_string());
    let description = description.unwrap_or_else(|| "Root directory".to_string());
    let read_only = read_only.unwrap_or(false);

    Ok(RootMeta {
        uri,
        name,
        description,
        read_only,
    })
}

/// Elicitation metadata extracted from attributes
#[derive(Debug)]
pub struct ElicitationMeta {
    pub message: String,
}

/// Extract elicitation metadata from #[elicitation(...)] attributes
pub fn extract_elicitation_meta(attrs: &[Attribute]) -> Result<ElicitationMeta> {
    let mut message = None;

    for attr in attrs {
        if attr.path().is_ident("elicitation") {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("message") {
                    let value = meta.value()?;
                    let s: syn::LitStr = value.parse()?;
                    message = Some(s.value());
                }
                Ok(())
            })?;
        }
    }

    let message = message.ok_or_else(|| {
        syn::Error::new(
            proc_macro2::Span::call_site(),
            "Missing 'message' in #[elicitation(message = \"...\")]",
        )
    })?;

    Ok(ElicitationMeta { message })
}

/// Extract field metadata from attributes
pub fn extract_field_meta(attrs: &[Attribute]) -> Result<FieldMeta> {
    let mut meta = FieldMeta::default();

    for attr in attrs {
        if attr.path().is_ident("content") {
            meta.content = Some(true);
        } else if attr.path().is_ident("content_type") {
            if let Ok(value) = attr.meta.require_name_value()
                && let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(lit_str),
                    ..
                }) = &value.value
            {
                meta.content_type = Some(lit_str.value());
            }
        } else if attr.path().is_ident("description")
            && let Ok(value) = attr.meta.require_name_value()
            && let syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(lit_str),
                ..
            }) = &value.value
        {
            meta.description = Some(lit_str.value());
        }
    }

    Ok(meta)
}

pub fn generate_output_schema_for_type_with_field(ty: &syn::Type, field_name: &str) -> TokenStream {
    match ty {
        syn::Type::Path(type_path) => {
            // First check if it's a qualified path like serde_json::Value
            let path_string = type_path
                .path
                .segments
                .iter()
                .map(|seg| seg.ident.to_string())
                .collect::<Vec<_>>()
                .join("::");

            // Special case for serde_json::Value (qualified path)
            if path_string.contains("serde_json") && path_string.ends_with("Value") {
                return quote! {
                    fn output_schema(&self) -> Option<&turul_mcp_protocol::tools::ToolSchema> {
                        static OUTPUT_SCHEMA: std::sync::OnceLock<turul_mcp_protocol::tools::ToolSchema> = std::sync::OnceLock::new();
                        Some(OUTPUT_SCHEMA.get_or_init(|| {
                            use turul_mcp_protocol::schema::JsonSchema;
                            use std::collections::HashMap;
                            turul_mcp_protocol::tools::ToolSchema::object()
                                .with_properties(HashMap::from([
                                    (#field_name.to_string(), JsonSchema::Object {
                                        description: Some("JSON object or value".to_string()),
                                        properties: None,
                                        required: None,
                                        additional_properties: Some(true),
                                    })
                                ]))
                                .with_required(vec![#field_name.to_string()])
                        }))
                    }
                };
            }

            // Check for Vec<T> generic type
            if type_path.path.segments.len() == 1 && type_path.path.segments[0].ident == "Vec" {
                return quote! {
                    fn output_schema(&self) -> Option<&turul_mcp_protocol::tools::ToolSchema> {
                        static OUTPUT_SCHEMA: std::sync::OnceLock<turul_mcp_protocol::tools::ToolSchema> = std::sync::OnceLock::new();
                        Some(OUTPUT_SCHEMA.get_or_init(|| {
                            use turul_mcp_protocol::schema::JsonSchema;
                            use std::collections::HashMap;
                            turul_mcp_protocol::tools::ToolSchema::object()
                                .with_properties(HashMap::from([
                                    (#field_name.to_string(), JsonSchema::Array {
                                        description: Some("Array of items".to_string()),
                                        items: None,
                                        min_items: None,
                                        max_items: None,
                                    })
                                ]))
                                .with_required(vec![#field_name.to_string()])
                        }))
                    }
                };
            }

            // Then check for simple identifiers
            if let Some(ident) = type_path.path.get_ident() {
                match ident.to_string().as_str() {
                    "f64" | "f32" => {
                        quote! {
                            fn output_schema(&self) -> Option<&turul_mcp_protocol::tools::ToolSchema> {
                                static OUTPUT_SCHEMA: std::sync::OnceLock<turul_mcp_protocol::tools::ToolSchema> = std::sync::OnceLock::new();
                                Some(OUTPUT_SCHEMA.get_or_init(|| {
                                    use turul_mcp_protocol::schema::JsonSchema;
                                    turul_mcp_protocol::tools::ToolSchema::object().with_properties(
                                        std::collections::HashMap::from([
                                            (#field_name.to_string(), JsonSchema::number())
                                        ])
                                    ).with_required(vec![#field_name.to_string()])
                                }))
                            }
                        }
                    }
                    "i64" | "i32" | "i16" | "i8" | "u64" | "u32" | "u16" | "u8" | "isize"
                    | "usize" => {
                        quote! {
                            fn output_schema(&self) -> Option<&turul_mcp_protocol::tools::ToolSchema> {
                                static OUTPUT_SCHEMA: std::sync::OnceLock<turul_mcp_protocol::tools::ToolSchema> = std::sync::OnceLock::new();
                                Some(OUTPUT_SCHEMA.get_or_init(|| {
                                    use turul_mcp_protocol::schema::JsonSchema;
                                    turul_mcp_protocol::tools::ToolSchema::object().with_properties(
                                        std::collections::HashMap::from([
                                            (#field_name.to_string(), JsonSchema::integer())
                                        ])
                                    ).with_required(vec![#field_name.to_string()])
                                }))
                            }
                        }
                    }
                    "String" | "str" => {
                        quote! {
                            fn output_schema(&self) -> Option<&turul_mcp_protocol::tools::ToolSchema> {
                                static OUTPUT_SCHEMA: std::sync::OnceLock<turul_mcp_protocol::tools::ToolSchema> = std::sync::OnceLock::new();
                                Some(OUTPUT_SCHEMA.get_or_init(|| {
                                    use turul_mcp_protocol::schema::JsonSchema;
                                    turul_mcp_protocol::tools::ToolSchema::object().with_properties(
                                        std::collections::HashMap::from([
                                            (#field_name.to_string(), JsonSchema::string())
                                        ])
                                    ).with_required(vec![#field_name.to_string()])
                                }))
                            }
                        }
                    }
                    "bool" => {
                        quote! {
                            fn output_schema(&self) -> Option<&turul_mcp_protocol::tools::ToolSchema> {
                                static OUTPUT_SCHEMA: std::sync::OnceLock<turul_mcp_protocol::tools::ToolSchema> = std::sync::OnceLock::new();
                                Some(OUTPUT_SCHEMA.get_or_init(|| {
                                    use turul_mcp_protocol::schema::JsonSchema;
                                    turul_mcp_protocol::tools::ToolSchema::object().with_properties(
                                        std::collections::HashMap::from([
                                            (#field_name.to_string(), JsonSchema::boolean())
                                        ])
                                    ).with_required(vec![#field_name.to_string()])
                                }))
                            }
                        }
                    }
                    _ => {
                        // For custom struct types, generate a basic object schema
                        // This will be enhanced by the generate_enhanced_output_schema function
                        // when struct introspection is available
                        quote! {
                            fn output_schema(&self) -> Option<&turul_mcp_protocol::tools::ToolSchema> {
                                static OUTPUT_SCHEMA: std::sync::OnceLock<turul_mcp_protocol::tools::ToolSchema> = std::sync::OnceLock::new();
                                Some(OUTPUT_SCHEMA.get_or_init(|| {
                                    use turul_mcp_protocol::schema::JsonSchema;
                                    use std::collections::HashMap;
                                    turul_mcp_protocol::tools::ToolSchema::object()
                                        .with_properties(HashMap::from([
                                            (#field_name.to_string(), JsonSchema::object())
                                        ]))
                                        .with_required(vec![#field_name.to_string()])
                                }))
                            }
                        }
                    }
                }
            } else {
                quote! {
                    fn output_schema(&self) -> Option<&turul_mcp_protocol::tools::ToolSchema> {
                        None
                    }
                }
            }
        }
        _ => {
            // For struct types, generate a basic object schema
            quote! {
                fn output_schema(&self) -> Option<&turul_mcp_protocol::tools::ToolSchema> {
                    static OUTPUT_SCHEMA: std::sync::OnceLock<turul_mcp_protocol::tools::ToolSchema> = std::sync::OnceLock::new();
                    Some(OUTPUT_SCHEMA.get_or_init(|| {
                        turul_mcp_protocol::tools::ToolSchema::object()
                    }))
                }
            }
        }
    }
}

/// Generate output schema for function return type (handles McpResult<T>)
#[allow(dead_code)]
pub fn generate_output_schema_for_return_type(return_type: &syn::Type) -> Option<TokenStream> {
    generate_output_schema_for_return_type_with_field(return_type, "result")
}

pub fn generate_output_schema_for_return_type_with_field(
    return_type: &syn::Type,
    field_name: &str,
) -> Option<TokenStream> {
    // Handle McpResult<T> by extracting the T type
    if let syn::Type::Path(type_path) = return_type
        && let Some(segment) = type_path.path.segments.last()
    {
        if segment.ident == "McpResult" || segment.ident == "Result" {
            // Extract the T from Result<T, E>
            if let syn::PathArguments::AngleBracketed(args) = &segment.arguments
                && let Some(syn::GenericArgument::Type(inner_type)) = args.args.first()
            {
                return Some(generate_output_schema_for_type_with_field(
                    inner_type, field_name,
                ));
            }
        } else {
            // Direct type, not wrapped in Result
            return Some(generate_output_schema_for_type_with_field(
                return_type,
                field_name,
            ));
        }
    }
    None
}

/// Determine output field name based on type and user preferences
pub fn determine_output_field_name(ty: &syn::Type, custom_field: Option<&String>) -> String {
    // If user specified a custom field name, use it
    if let Some(field_name) = custom_field {
        return field_name.clone();
    }

    // For struct types, extract the struct name and convert to camelCase
    if let syn::Type::Path(type_path) = ty
        && let Some(ident) = type_path.path.get_ident()
    {
        let struct_name = ident.to_string();
        // Check if this looks like a custom struct (not a primitive)
        if !matches!(
            struct_name.as_str(),
            "f64"
                | "f32"
                | "i64"
                | "i32"
                | "i16"
                | "i8"
                | "u64"
                | "u32"
                | "u16"
                | "u8"
                | "isize"
                | "usize"
                | "String"
                | "str"
                | "bool"
        ) {
            // Convert struct name to camelCase (e.g., CalculationResult -> calculationResult)
            return struct_to_camel_case(&struct_name);
        }
    }

    // Default to "output" for primitives (as requested by user)
    "output".to_string()
}

/// Convert struct name to camelCase for field names
/// Handles special cases:
/// - All-caps acronyms: LLH → llh, GPS → gps
/// - Leading acronyms: HTTPServer → httpServer
/// - Regular names: LocationData → locationData
fn struct_to_camel_case(struct_name: &str) -> String {
    let chars: Vec<char> = struct_name.chars().collect();
    if chars.is_empty() {
        return String::new();
    }

    // Check if entire name is uppercase (acronym like LLH, GPS, HTTP)
    if chars.iter().all(|c| c.is_uppercase() || !c.is_alphabetic()) {
        return struct_name.to_lowercase();
    }

    // Find where the leading uppercase sequence ends
    // HTTPServer → http_Server (lowercase leading acronym)
    // LocationData → location_Data (just first letter)
    let mut lowercase_until = 1; // At minimum, lowercase first character

    for i in 1..chars.len() {
        if chars[i].is_uppercase() {
            if i + 1 < chars.len() && chars[i + 1].is_lowercase() {
                // HTTPServer: at 'S', next is 'e' (lowercase), so stop before 'S'
                break;
            }
            lowercase_until = i + 1;
        } else {
            // Hit a lowercase letter, stop
            break;
        }
    }

    let mut result = String::new();
    for (i, ch) in chars.iter().enumerate() {
        if i < lowercase_until {
            result.extend(ch.to_lowercase());
        } else {
            result.push(*ch);
        }
    }
    result
}

/// Extract schema content (without function wrapper) for reuse
/// This is the core logic used by both generate_enhanced_output_schema and generate_output_schema_auto
fn extract_schema_content(
    ty: &syn::Type,
    field_name: &str,
    input: Option<&DeriveInput>,
) -> TokenStream {
    // Try to introspect struct properties if we have the DeriveInput
    if let (syn::Type::Path(type_path), Some(derive_input)) = (ty, input)
        && let Some(ident) = type_path.path.get_ident()
    {
        // Check if this is Self or the same struct we're deriving for
        let is_self_type = ident == "Self";
        let is_same_struct = ident == &derive_input.ident;

        if (is_self_type || is_same_struct)
            && let Data::Struct(data_struct) = &derive_input.data
            && let Fields::Named(fields) = &data_struct.fields
        {
            return extract_struct_schema_content(fields, field_name);
        }
    }

    // Fallback - extract content from the existing function
    // Extract just the schema creation content (not the full function)
    // Use the basic type schema generation for types we can't introspect
    generate_basic_type_schema_content(ty, field_name)
}

/// Extract schema content for struct (without function wrapper) - used by extract_schema_content
fn extract_struct_schema_content(
    fields: &syn::FieldsNamed,
    output_field_name: &str,
) -> TokenStream {
    let mut property_definitions = Vec::new();
    let mut required_fields = Vec::new();

    for field in &fields.named {
        if let Some(struct_field_name) = &field.ident {
            let field_name_str = struct_field_name.to_string();
            let field_type = &field.ty;

            // Extract parameter metadata for better schema generation
            let param_meta = extract_param_meta(&field.attrs).unwrap_or_default();
            let schema = type_to_schema(field_type, &param_meta);

            property_definitions.push(quote! {
                (#field_name_str.to_string(), #schema)
            });

            // Check if field is required (not Option<T> and not marked as optional)
            let is_option = is_option_type(field_type);
            if !is_option && !param_meta.optional {
                required_fields.push(quote! {
                    #field_name_str.to_string()
                });
            }
        }
    }

    quote! {
        use turul_mcp_protocol::schema::JsonSchema;
        use std::collections::HashMap;

        // Create schema for the struct content
        let struct_schema = turul_mcp_protocol::tools::ToolSchema::object()
            .with_properties(HashMap::from([
                #(#property_definitions),*
            ]))
            .with_required(vec![
                #(#required_fields),*
            ]);

        // Wrap in outer object with custom output field name
        turul_mcp_protocol::tools::ToolSchema::object()
            .with_properties(HashMap::from([
                (#output_field_name.to_string(), JsonSchema::Object {
                    description: None,
                    properties: struct_schema.properties.clone(),
                    required: struct_schema.required.clone(),
                    additional_properties: Some(false),
                })
            ]))
            .with_required(vec![#output_field_name.to_string()])
    }
}

/// Check if a type is Option<T>
fn is_option_type(ty: &syn::Type) -> bool {
    if let syn::Type::Path(type_path) = ty {
        type_path.path.segments.len() == 1 && type_path.path.segments[0].ident == "Option"
    } else {
        false
    }
}

/// Check if a type is a basic primitive that doesn't need schemars
fn is_basic_type(ty: &syn::Type) -> bool {
    if let syn::Type::Path(type_path) = ty {
        if let Some(ident) = type_path.path.get_ident() {
            matches!(
                ident.to_string().as_str(),
                "f64"
                    | "f32"
                    | "i64"
                    | "i32"
                    | "i16"
                    | "i8"
                    | "u64"
                    | "u32"
                    | "u16"
                    | "u8"
                    | "String"
                    | "str"
                    | "bool"
            )
        } else {
            false
        }
    } else {
        false
    }
}

/// Generate output schema with automatic schemars detection
///
/// Automatically tries to use schemars::schema_for!() on the output type first.
/// Falls back to struct introspection if schemars is not available for that type.
///
/// This provides zero-configuration automatic detection: if the output type has
/// `#[derive(JsonSchema)]`, the framework automatically uses it - no manual flags needed!
pub fn generate_output_schema_auto(
    ty: &syn::Type,
    field_name: &str,
    input: Option<&DeriveInput>,
) -> TokenStream {
    // Extract schema content for fallback (struct introspection)
    let fallback_schema_content = extract_schema_content(ty, field_name, input);

    // For Self type, always use introspection
    let is_self_type = if let syn::Type::Path(type_path) = ty {
        type_path.path.is_ident("Self")
    } else {
        false
    };

    if is_self_type {
        // Self type - use introspection
        return quote! {
            fn output_schema(&self) -> Option<&turul_mcp_protocol::tools::ToolSchema> {
                static OUTPUT_SCHEMA: std::sync::OnceLock<turul_mcp_protocol::tools::ToolSchema> =
                    std::sync::OnceLock::new();
                Some(OUTPUT_SCHEMA.get_or_init(|| {
                    #fallback_schema_content
                }))
            }
        };
    }

    // For basic types (String, i32, etc.), use simple schema without requiring schemars
    if is_basic_type(ty) {
        return quote! {
            fn output_schema(&self) -> Option<&turul_mcp_protocol::tools::ToolSchema> {
                static OUTPUT_SCHEMA: std::sync::OnceLock<turul_mcp_protocol::tools::ToolSchema> =
                    std::sync::OnceLock::new();
                Some(OUTPUT_SCHEMA.get_or_init(|| {
                    #fallback_schema_content
                }))
            }
        };
    }

    // For complex external types, generate detailed schemas using schemars
    // REQUIRES: Output type must derive schemars::JsonSchema
    // If the type doesn't implement JsonSchema, compilation will fail with a clear error
    quote! {
        fn output_schema(&self) -> Option<&turul_mcp_protocol::tools::ToolSchema> {
            static OUTPUT_SCHEMA: std::sync::OnceLock<turul_mcp_protocol::tools::ToolSchema> =
                std::sync::OnceLock::new();
            Some(OUTPUT_SCHEMA.get_or_init(|| {
                use std::collections::HashMap;
                use turul_mcp_protocol::schema::JsonSchema;

                // Generate detailed schema via schemars
                // IMPORTANT: Output type MUST derive schemars::JsonSchema
                // If you get a compile error here, add #[derive(schemars::JsonSchema)] to your output type
                let schemars_schema = turul_mcp_builders::schemars::schema_for!(#ty);

                // Convert schemars RootSchema to JSON Value
                let mut schema_value = serde_json::to_value(&schemars_schema)
                    .expect("schemars Schema should always serialize to JSON");

                // Extract definitions for $ref resolution
                let definitions = if let Some(defs_value) = schema_value.get("definitions")
                    .or_else(|| schema_value.get("$defs")) {
                    // Parse definitions into HashMap<String, Value>
                    defs_value.as_object()
                        .map(|obj| obj.iter()
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect::<HashMap<String, serde_json::Value>>())
                        .unwrap_or_default()
                } else {
                    HashMap::new()
                };

                // Remove definitions from root to get clean schema
                if let Some(obj) = schema_value.as_object_mut() {
                    obj.remove("definitions");
                    obj.remove("$defs");
                }

                // Use safe converter with definitions for $ref resolution
                let inner_schema = turul_mcp_builders::convert_value_to_json_schema_with_defs(
                    &schema_value,
                    &definitions
                );

                // Wrap in output field
                turul_mcp_protocol::tools::ToolSchema::object()
                    .with_properties(HashMap::from([
                        (#field_name.to_string(), inner_schema)
                    ]))
                    .with_required(vec![#field_name.to_string()])
            }))
        }
    }
}

/// Generate basic type schema content (without function wrapper)
fn generate_basic_type_schema_content(ty: &syn::Type, field_name: &str) -> TokenStream {
    match ty {
        syn::Type::Path(type_path) => {
            if let Some(ident) = type_path.path.get_ident() {
                let schema_type = match ident.to_string().as_str() {
                    "f64" | "f32" => quote! { JsonSchema::number() },
                    "i64" | "i32" | "i16" | "i8" | "u64" | "u32" | "u16" | "u8" => {
                        quote! { JsonSchema::integer() }
                    }
                    "String" | "str" => quote! { JsonSchema::string() },
                    "bool" => quote! { JsonSchema::boolean() },
                    _ => quote! { JsonSchema::object() },
                };

                quote! {
                    use turul_mcp_protocol::schema::JsonSchema;
                    use std::collections::HashMap;

                    turul_mcp_protocol::tools::ToolSchema::object()
                        .with_properties(HashMap::from([
                            (#field_name.to_string(), #schema_type)
                        ]))
                        .with_required(vec![#field_name.to_string()])
                }
            } else {
                quote! {
                    turul_mcp_protocol::tools::ToolSchema::object()
                }
            }
        }
        _ => quote! {
            turul_mcp_protocol::tools::ToolSchema::object()
        },
    }
}
