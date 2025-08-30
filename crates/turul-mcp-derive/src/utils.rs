//! Utility functions for macro implementations

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, Result, Fields, DeriveInput, Data};


/// Extract tool metadata from attributes
#[derive(Debug)]
pub struct ToolMeta {
    pub name: String,
    pub description: String,
    pub output_type: Option<syn::Type>,
    pub output_field: Option<String>,  // Custom field name for output
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
                } else if meta.path.is_ident("field") {
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

    Ok(ToolMeta { name, description, output_type, output_field })
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
                    "i64" | "i32" | "i16" | "i8" | "u64" | "u32" | "u16" | "u8" | "isize" | "usize" => {
                        quote! {
                            turul_mcp_protocol::schema::JsonSchema::integer() #description
                        }
                    }
                    "bool" => {
                        quote! {
                            turul_mcp_protocol::schema::JsonSchema::boolean() #description
                        }
                    }
                    _ => {
                        // Check if this is Vec<T>
                        if type_path.path.segments.len() == 1 && type_path.path.segments[0].ident == "Vec" {
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
                    .ok_or_else(|| turul_mcp_protocol::McpError::invalid_param_type(#field_name_str, "string", "other"))?
                    .to_string();
            }
        }
        syn::Type::Path(type_path) if type_path.path.get_ident().map_or(false, |i| i == "f64") => {
            quote! {
                let #field_name = args.get(#field_name_str)
                    .and_then(|v| v.as_f64())
                    .ok_or_else(|| turul_mcp_protocol::McpError::invalid_param_type(#field_name_str, "number", "other"))?;
            }
        }
        syn::Type::Path(type_path) if type_path.path.get_ident().map_or(false, |i| i == "i32") => {
            quote! {
                let #field_name = args.get(#field_name_str)
                    .and_then(|v| v.as_i64())
                    .and_then(|i| i.try_into().ok())
                    .ok_or_else(|| turul_mcp_protocol::McpError::invalid_param_type(#field_name_str, "integer", "other"))?;
            }
        }
        syn::Type::Path(type_path) if type_path.path.get_ident().map_or(false, |i| i == "i64") => {
            quote! {
                let #field_name = args.get(#field_name_str)
                    .and_then(|v| v.as_i64())
                    .ok_or_else(|| turul_mcp_protocol::McpError::invalid_param_type(#field_name_str, "integer", "other"))?;
            }
        }
        syn::Type::Path(type_path) if type_path.path.get_ident().map_or(false, |i| i == "u32") => {
            quote! {
                let #field_name = args.get(#field_name_str)
                    .and_then(|v| v.as_u64())
                    .and_then(|i| i.try_into().ok())
                    .ok_or_else(|| turul_mcp_protocol::McpError::invalid_param_type(#field_name_str, "integer", "other"))?;
            }
        }
        syn::Type::Path(type_path) if type_path.path.get_ident().map_or(false, |i| i == "u64") => {
            quote! {
                let #field_name = args.get(#field_name_str)
                    .and_then(|v| v.as_u64())
                    .ok_or_else(|| turul_mcp_protocol::McpError::invalid_param_type(#field_name_str, "integer", "other"))?;
            }
        }
        syn::Type::Path(type_path) if type_path.path.get_ident().map_or(false, |i| i == "f32") => {
            quote! {
                let #field_name = args.get(#field_name_str)
                    .and_then(|v| v.as_f64())
                    .map(|f| f as f32)
                    .ok_or_else(|| turul_mcp_protocol::McpError::invalid_param_type(#field_name_str, "number", "other"))?;
            }
        }
        syn::Type::Path(type_path) if type_path.path.get_ident().map_or(false, |i| i == "bool") => {
            quote! {
                let #field_name = args.get(#field_name_str)
                    .and_then(|v| v.as_bool())
                    .ok_or_else(|| turul_mcp_protocol::McpError::invalid_param_type(#field_name_str, "boolean", "other"))?;
            }
        }
        syn::Type::Path(type_path) if type_path.path.segments.len() == 1 && type_path.path.segments[0].ident == "Vec" => {
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

/// Prompt metadata extracted from attributes
#[derive(Debug)]
pub struct PromptMeta {
    pub name: String,
    pub description: String,
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
        syn::Error::new(proc_macro2::Span::call_site(), "Missing 'name' in #[prompt(name = \"...\")]")
    })?;
    
    let description = description.unwrap_or_else(|| "Generated prompt".to_string());

    Ok(PromptMeta { name, description })
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
        syn::Error::new(proc_macro2::Span::call_site(), "Missing 'uri' in #[root(uri = \"...\")]")
    })?;
    
    let name = name.unwrap_or_else(|| "Unnamed Root".to_string());
    let description = description.unwrap_or_else(|| "Root directory".to_string());
    let read_only = read_only.unwrap_or(false);

    Ok(RootMeta { uri, name, description, read_only })
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
        syn::Error::new(proc_macro2::Span::call_site(), "Missing 'message' in #[elicitation(message = \"...\")]")
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



pub fn generate_output_schema_for_type_with_field(ty: &syn::Type, field_name: &str) -> TokenStream {
    match ty {
        syn::Type::Path(type_path) => {
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
                    "i64" | "i32" | "i16" | "i8" | "u64" | "u32" | "u16" | "u8" | "isize" | "usize" => {
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

/// Determine output field name based on type and user preferences
pub fn determine_output_field_name(ty: &syn::Type, custom_field: Option<&String>) -> String {
    // If user specified a custom field name, use it
    if let Some(field_name) = custom_field {
        return field_name.clone();
    }
    
    // For struct types, extract the struct name and convert to camelCase
    if let syn::Type::Path(type_path) = ty {
        if let Some(ident) = type_path.path.get_ident() {
            let struct_name = ident.to_string();
            // Check if this looks like a custom struct (not a primitive)
            if !matches!(struct_name.as_str(), 
                "f64" | "f32" | "i64" | "i32" | "i16" | "i8" | 
                "u64" | "u32" | "u16" | "u8" | "isize" | "usize" |
                "String" | "str" | "bool"
            ) {
                // Convert struct name to camelCase (e.g., CalculationResult -> calculationResult)
                return struct_to_camel_case(&struct_name);
            }
        }
    }
    
    // Default to "output" for primitives (as requested by user)
    "output".to_string()
}

/// Convert struct name to camelCase for field names
fn struct_to_camel_case(struct_name: &str) -> String {
    let mut chars: Vec<char> = struct_name.chars().collect();
    if !chars.is_empty() {
        chars[0] = chars[0].to_lowercase().next().unwrap_or(chars[0]);
    }
    chars.into_iter().collect()
}

/// Generate enhanced output schema with struct property introspection
pub fn generate_enhanced_output_schema(ty: &syn::Type, field_name: &str, input: Option<&DeriveInput>) -> TokenStream {
    // Try to introspect struct properties if we have the DeriveInput
    if let (syn::Type::Path(type_path), Some(derive_input)) = (ty, input) {
        if let Some(ident) = type_path.path.get_ident() {
            // Check if this is the same struct we're deriving for
            if ident == &derive_input.ident {
                if let Data::Struct(data_struct) = &derive_input.data {
                    if let Fields::Named(fields) = &data_struct.fields {
                        return generate_struct_schema_with_properties(fields, field_name);
                    }
                }
            }
        }
    }
    
    // Fallback to basic type schema generation
    generate_output_schema_for_type_with_field(ty, field_name)
}

/// Generate schema for struct with all properties introspected
fn generate_struct_schema_with_properties(fields: &syn::FieldsNamed, field_name: &str) -> TokenStream {
    let mut property_definitions = Vec::new();
    let mut required_fields = Vec::new();
    
    for field in &fields.named {
        if let Some(field_name) = &field.ident {
            let field_name_str = field_name.to_string();
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
        fn output_schema(&self) -> Option<&turul_mcp_protocol::tools::ToolSchema> {
            static OUTPUT_SCHEMA: std::sync::OnceLock<turul_mcp_protocol::tools::ToolSchema> = std::sync::OnceLock::new();
            Some(OUTPUT_SCHEMA.get_or_init(|| {
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
                
                // Wrap in outer object with field name
                turul_mcp_protocol::tools::ToolSchema::object()
                    .with_properties(HashMap::from([
                        (#field_name.to_string(), JsonSchema::Object {
                            properties: struct_schema.properties.clone(),
                            required: struct_schema.required.clone(),
                            additional: HashMap::new(),
                            schema_type: "object".to_string(),
                            title: None,
                            description: None,
                        })
                    ]))
                    .with_required(vec![#field_name.to_string()])
            }))
        }
    }
}

/// Check if a type is Option<T>
fn is_option_type(ty: &syn::Type) -> bool {
    if let syn::Type::Path(type_path) = ty {
        type_path.path.segments.len() == 1 && 
        type_path.path.segments[0].ident == "Option"
    } else {
        false
    }
}

