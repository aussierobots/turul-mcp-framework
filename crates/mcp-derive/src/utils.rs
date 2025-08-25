//! Utility functions for macro implementations

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, Result};


/// Extract tool metadata from attributes
#[derive(Debug)]
pub struct ToolMeta {
    pub name: String,
    pub description: String,
    pub output_type: Option<syn::Type>,
}

pub fn extract_tool_meta(attrs: &[Attribute]) -> Result<ToolMeta> {
    let mut name = None;
    let mut description = None;
    let mut output_type = None;

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
        } else if attr.path().is_ident("output_type") {
            // Parse #[output_type(TypeName)] syntax
            if let syn::Meta::List(meta_list) = &attr.meta {
                let ty: syn::Type = syn::parse2(meta_list.tokens.clone())?;
                output_type = Some(ty);
            }
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

    Ok(ToolMeta { name, description, output_type })
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
                        // Check if this is Vec<T>
                        if type_path.path.segments.len() == 1 && type_path.path.segments[0].ident == "Vec" {
                            quote! {
                                mcp_protocol::schema::JsonSchema::array() #description
                            }
                        } else {
                            quote! {
                                mcp_protocol::schema::JsonSchema::string() #description
                            }
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
        syn::Type::Path(type_path) if type_path.path.get_ident().map_or(false, |i| i == "i64") => {
            quote! {
                let #field_name: Option<i64> = args.get(#field_name_str)
                    .and_then(|v| v.as_i64());
            }
        }
        syn::Type::Path(type_path) if type_path.path.get_ident().map_or(false, |i| i == "u32") => {
            quote! {
                let #field_name: Option<u32> = args.get(#field_name_str)
                    .and_then(|v| v.as_u64())
                    .and_then(|i| i.try_into().ok());
            }
        }
        syn::Type::Path(type_path) if type_path.path.get_ident().map_or(false, |i| i == "u64") => {
            quote! {
                let #field_name: Option<u64> = args.get(#field_name_str)
                    .and_then(|v| v.as_u64());
            }
        }
        syn::Type::Path(type_path) if type_path.path.get_ident().map_or(false, |i| i == "f32") => {
            quote! {
                let #field_name: Option<f32> = args.get(#field_name_str)
                    .and_then(|v| v.as_f64())
                    .map(|f| f as f32);
            }
        }
        syn::Type::Path(type_path) if type_path.path.get_ident().map_or(false, |i| i == "bool") => {
            quote! {
                let #field_name: Option<bool> = args.get(#field_name_str)
                    .and_then(|v| v.as_bool());
            }
        }
        _ => {
            // Check if this is Option<Vec<T>>
            if let syn::Type::Path(inner_path) = inner_type {
                if inner_path.path.segments.len() == 1 && inner_path.path.segments[0].ident == "Vec" {
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
        syn::Type::Path(type_path) if type_path.path.get_ident().map_or(false, |i| i == "i64") => {
            quote! {
                let #field_name = args.get(#field_name_str)
                    .and_then(|v| v.as_i64())
                    .ok_or_else(|| mcp_protocol::McpError::invalid_param_type(#field_name_str, "integer", "other"))?;
            }
        }
        syn::Type::Path(type_path) if type_path.path.get_ident().map_or(false, |i| i == "u32") => {
            quote! {
                let #field_name = args.get(#field_name_str)
                    .and_then(|v| v.as_u64())
                    .and_then(|i| i.try_into().ok())
                    .ok_or_else(|| mcp_protocol::McpError::invalid_param_type(#field_name_str, "integer", "other"))?;
            }
        }
        syn::Type::Path(type_path) if type_path.path.get_ident().map_or(false, |i| i == "u64") => {
            quote! {
                let #field_name = args.get(#field_name_str)
                    .and_then(|v| v.as_u64())
                    .ok_or_else(|| mcp_protocol::McpError::invalid_param_type(#field_name_str, "integer", "other"))?;
            }
        }
        syn::Type::Path(type_path) if type_path.path.get_ident().map_or(false, |i| i == "f32") => {
            quote! {
                let #field_name = args.get(#field_name_str)
                    .and_then(|v| v.as_f64())
                    .map(|f| f as f32)
                    .ok_or_else(|| mcp_protocol::McpError::invalid_param_type(#field_name_str, "number", "other"))?;
            }
        }
        syn::Type::Path(type_path) if type_path.path.get_ident().map_or(false, |i| i == "bool") => {
            quote! {
                let #field_name = args.get(#field_name_str)
                    .and_then(|v| v.as_bool())
                    .ok_or_else(|| mcp_protocol::McpError::invalid_param_type(#field_name_str, "boolean", "other"))?;
            }
        }
        syn::Type::Path(type_path) if type_path.path.segments.len() == 1 && type_path.path.segments[0].ident == "Vec" => {
            quote! {
                let #field_name: #field_type = args.get(#field_name_str)
                    .ok_or_else(|| mcp_protocol::McpError::missing_param(#field_name_str))
                    .and_then(|v| {
                        // Handle both array values and string representations of arrays
                        if let Some(arr) = v.as_array() {
                            // Direct JSON array
                            arr.iter()
                                .map(|item| serde_json::from_value(item.clone()))
                                .collect::<Result<Vec<_>, _>>()
                                .map_err(|_| mcp_protocol::McpError::invalid_param_type(#field_name_str, "array of valid items", "other"))
                        } else if let Some(s) = v.as_str() {
                            // String representation of array like "[1,2,3,4]"
                            serde_json::from_str::<Vec<serde_json::Value>>(s)
                                .map_err(|_| mcp_protocol::McpError::invalid_param_type(#field_name_str, "array or array string", "invalid string"))
                                .and_then(|arr| {
                                    arr.iter()
                                        .map(|item| serde_json::from_value(item.clone()))
                                        .collect::<Result<Vec<_>, _>>()
                                        .map_err(|_| mcp_protocol::McpError::invalid_param_type(#field_name_str, "array of valid items", "other"))
                                })
                        } else {
                            Err(mcp_protocol::McpError::invalid_param_type(#field_name_str, "array or array string", "other"))
                        }
                    })?;
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


/// Generate output schema from a specific type with struct introspection
pub fn generate_output_schema_for_type(ty: &syn::Type) -> TokenStream {
    generate_output_schema_for_type_with_field(ty, "result")
}

pub fn generate_output_schema_for_type_with_field(ty: &syn::Type, field_name: &str) -> TokenStream {
    match ty {
        syn::Type::Path(type_path) => {
            if let Some(ident) = type_path.path.get_ident() {
                match ident.to_string().as_str() {
                    "f64" | "f32" => {
                        quote! {
                            fn output_schema(&self) -> Option<&mcp_protocol::tools::ToolSchema> {
                                static OUTPUT_SCHEMA: std::sync::OnceLock<mcp_protocol::tools::ToolSchema> = std::sync::OnceLock::new();
                                Some(OUTPUT_SCHEMA.get_or_init(|| {
                                    use mcp_protocol::schema::JsonSchema;
                                    mcp_protocol::tools::ToolSchema::object().with_properties(
                                        std::collections::HashMap::from([
                                            (#field_name.to_string(), JsonSchema::number())
                                        ])
                                    ).with_required(vec![#field_name.to_string()])
                                }))
                            }
                        }
                    }
                    "i64" | "i32" | "i16" | "i8" | "u64" | "u32" | "u16" | "u8" | "isize" | "usize" => {
                        quote! {
                            fn output_schema(&self) -> Option<&mcp_protocol::tools::ToolSchema> {
                                static OUTPUT_SCHEMA: std::sync::OnceLock<mcp_protocol::tools::ToolSchema> = std::sync::OnceLock::new();
                                Some(OUTPUT_SCHEMA.get_or_init(|| {
                                    use mcp_protocol::schema::JsonSchema;
                                    mcp_protocol::tools::ToolSchema::object().with_properties(
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
                            fn output_schema(&self) -> Option<&mcp_protocol::tools::ToolSchema> {
                                static OUTPUT_SCHEMA: std::sync::OnceLock<mcp_protocol::tools::ToolSchema> = std::sync::OnceLock::new();
                                Some(OUTPUT_SCHEMA.get_or_init(|| {
                                    use mcp_protocol::schema::JsonSchema;
                                    mcp_protocol::tools::ToolSchema::object().with_properties(
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
                            fn output_schema(&self) -> Option<&mcp_protocol::tools::ToolSchema> {
                                static OUTPUT_SCHEMA: std::sync::OnceLock<mcp_protocol::tools::ToolSchema> = std::sync::OnceLock::new();
                                Some(OUTPUT_SCHEMA.get_or_init(|| {
                                    use mcp_protocol::schema::JsonSchema;
                                    mcp_protocol::tools::ToolSchema::object().with_properties(
                                        std::collections::HashMap::from([
                                            (#field_name.to_string(), JsonSchema::boolean())
                                        ])
                                    ).with_required(vec![#field_name.to_string()])
                                }))
                            }
                        }
                    }
                    _ => {
                        // For custom struct types, try to generate a detailed schema
                        // For now, we'll generate a basic object schema but this could be enhanced
                        // to introspect the struct definition if available
                        let type_name = ident.to_string();
                        let _schema_debug = format!("Generating schema for type: {}", type_name);
                        quote! {
                            fn output_schema(&self) -> Option<&mcp_protocol::tools::ToolSchema> {
                                static OUTPUT_SCHEMA: std::sync::OnceLock<mcp_protocol::tools::ToolSchema> = std::sync::OnceLock::new();
                                Some(OUTPUT_SCHEMA.get_or_init(|| {
                                    // Generate schema for custom type: #type_name
                                    <#ident as mcp_protocol::schema::JsonSchemaGenerator>::json_schema()
                                }))
                            }
                        }
                    }
                }
            } else {
                quote! {
                    fn output_schema(&self) -> Option<&mcp_protocol::tools::ToolSchema> {
                        None
                    }
                }
            }
        }
        _ => {
            // For struct types, generate a basic object schema
            quote! {
                fn output_schema(&self) -> Option<&mcp_protocol::tools::ToolSchema> {
                    static OUTPUT_SCHEMA: std::sync::OnceLock<mcp_protocol::tools::ToolSchema> = std::sync::OnceLock::new();
                    Some(OUTPUT_SCHEMA.get_or_init(|| {
                        mcp_protocol::tools::ToolSchema::object()
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

pub fn generate_output_schema_for_return_type_with_field(return_type: &syn::Type, field_name: &str) -> Option<TokenStream> {
    // Handle McpResult<T> by extracting the T type
    if let syn::Type::Path(type_path) = return_type {
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == "McpResult" || segment.ident == "Result" {
                // Extract the T from Result<T, E>
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(syn::GenericArgument::Type(inner_type)) = args.args.first() {
                        return Some(generate_output_schema_for_type_with_field(inner_type, field_name));
                    }
                }
            } else {
                // Direct type, not wrapped in Result
                return Some(generate_output_schema_for_type_with_field(return_type, field_name));
            }
        }
    }
    None
}

/// Generate tool result conversion from return type
pub fn generate_result_conversion(ty: &syn::Type, has_output_schema: bool) -> TokenStream {
    match ty {
        syn::Type::Path(type_path) => {
            if let Some(ident) = type_path.path.get_ident() {
                match ident.to_string().as_str() {
                    "f64" | "f32" | "i64" | "i32" | "i16" | "i8" | "u64" | "u32" | "u16" | "u8" | "isize" | "usize" | "bool" => {
                        if has_output_schema {
                            // For tools with output schema, still return Vec<ToolResult> for the call method
                            // The structured content will be handled by the execute method
                            quote! {
                                match instance.execute().await {
                                    Ok(result) => {
                                        // Return JSON text that matches the structured content format
                                        let json_text = serde_json::json!({"value": result}).to_string();
                                        Ok(vec![mcp_protocol::ToolResult::text(json_text)])
                                    }
                                    Err(e) => Err(e)
                                }
                            }
                        } else {
                            // No output schema - return as text
                            quote! {
                                match instance.execute().await {
                                    Ok(result) => {
                                        Ok(vec![mcp_protocol::ToolResult::text(result.to_string())])
                                    }
                                    Err(e) => Err(e)
                                }
                            }
                        }
                    }
                    "String" | "str" => {
                        if has_output_schema {
                            quote! {
                                match instance.execute().await {
                                    Ok(result) => {
                                        let structured_result = serde_json::json!(result);
                                        Ok(vec![
                                            mcp_protocol::ToolResult::text(structured_result.to_string())
                                        ])
                                    }
                                    Err(e) => Err(e)
                                }
                            }
                        } else {
                            quote! {
                                match instance.execute().await {
                                    Ok(result) => {
                                        Ok(vec![mcp_protocol::ToolResult::text(result.to_string())])
                                    }
                                    Err(e) => Err(e)
                                }
                            }
                        }
                    }
                    _ => {
                        // For complex types, use serde serialization
                        quote! {
                            match instance.execute().await {
                                Ok(result) => {
                                    let json_result = serde_json::to_value(result)
                                        .map_err(|e| mcp_protocol::McpError::tool_execution(&format!("Serialization error: {}", e)))?;
                                    Ok(vec![mcp_protocol::ToolResult::text(json_result.to_string())])
                                }
                                Err(e) => Err(e)
                            }
                        }
                    }
                }
            } else {
                // Generic complex type
                quote! {
                    match instance.execute().await {
                        Ok(result) => {
                            let json_result = serde_json::to_value(result)
                                .map_err(|e| mcp_protocol::McpError::tool_execution(&format!("Serialization error: {}", e)))?;
                            Ok(vec![mcp_protocol::ToolResult::text(json_result.to_string())])
                        }
                        Err(e) => Err(e)
                    }
                }
            }
        }
        _ => {
            // Generic type
            quote! {
                match instance.execute().await {
                    Ok(result) => {
                        let json_result = serde_json::to_value(result)
                            .map_err(|e| mcp_protocol::McpError::tool_execution(&format!("Serialization error: {}", e)))?;
                        Ok(vec![mcp_protocol::ToolResult::text(json_result.to_string())])
                    }
                    Err(e) => Err(e)
                }
            }
        }
    }
}