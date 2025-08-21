//! # MCP Derive Macros
//!
//! This crate provides procedural macros to simplify creating MCP tools and resources.
//! 
//! ## Features
//!
//! - `#[derive(McpTool)]` - Automatically derive McpTool implementations
//! - `#[mcp_tool]` - Function-style tools with automatic parameter extraction
//! - `#[derive(McpResource)]` - Automatically derive resource handlers
//! - `tool!` - Declarative macro for simple tool creation
//! - `resource!` - Declarative macro for simple resource creation
//! - `schema_for!` - Generate JSON schemas from Rust types
//!
//! ## Code Organization Note
//!
//! This crate previously contained duplicate/unused implementations in `resource_derive.rs` 
//! and `schema_gen.rs` that were not referenced in lib.rs. These files have been removed 
//! to eliminate dead code and confusion. All functionality is now consolidated in the 
//! primary implementation modules.

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, ItemFn, punctuated::Punctuated, Meta, Token};

mod tool_derive;
mod tool_attr;
mod resource;
mod utils;

#[cfg(test)]
mod tests;

/// Derive macro for automatically implementing McpTool
///
/// This macro generates a complete McpTool implementation from a struct definition.
/// 
/// # Attributes
/// 
/// - `#[tool(name = "...", description = "...")]` - Tool metadata
/// - `#[param(description = "...", ...)]` - Parameter descriptions and validation
/// 
/// # Example
/// 
/// ```rust
/// use mcp_derive::McpTool;
/// use mcp_protocol::McpResult;
/// 
/// #[derive(McpTool, Clone)]
/// #[tool(name = "add", description = "Add two numbers")]
/// struct AddTool {
///     #[param(description = "First number")]
///     a: f64,
///     #[param(description = "Second number")]
///     b: f64,
/// }
/// 
/// impl AddTool {
///     async fn execute(&self) -> McpResult<String> {
///         Ok(format!("{} + {} = {}", self.a, self.b, self.a + self.b))
///     }
/// }
/// ```
#[proc_macro_derive(McpTool, attributes(tool, param))]
pub fn derive_mcp_tool(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    tool_derive::derive_mcp_tool_impl(input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Function attribute macro for creating MCP tools
/// 
/// This macro converts a regular async function into an MCP tool with automatic
/// parameter extraction and schema generation.
/// 
/// # Example
/// 
/// ```rust
/// use mcp_derive::mcp_tool;
/// use mcp_protocol::McpResult;
/// 
/// #[mcp_tool(name = "multiply", description = "Multiply two numbers")]
/// async fn multiply(
///     #[param(description = "First number")] a: f64,
///     #[param(description = "Second number")] b: f64,
/// ) -> McpResult<String> {
///     Ok(format!("{} × {} = {}", a, b, a * b))
/// }
/// ```
#[proc_macro_attribute]
pub fn mcp_tool(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args with Punctuated::<Meta, Token![,]>::parse_terminated);
    let input = parse_macro_input!(input as ItemFn);
    tool_attr::mcp_tool_impl(args, input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Helper attribute for parameter metadata in function macros
/// 
/// This is used within #[mcp_tool] functions to provide parameter descriptions
/// and constraints.
#[proc_macro_attribute]
pub fn param(_args: TokenStream, input: TokenStream) -> TokenStream {
    // This attribute is only processed by the #[mcp_tool] macro
    // When used alone, it just passes through the input unchanged
    input
}

/// Derive macro for automatically implementing MCP resource handlers
/// 
/// # Example
/// 
/// ```rust
/// use mcp_derive::McpResource;
/// 
/// #[derive(McpResource)]
/// #[uri = "file://config.json"]
/// #[name = "Configuration File"]
/// #[description = "Application configuration file"]
/// struct ConfigResource {
///     #[content]
///     #[content_type = "application/json"]
///     data: String,
/// }
/// ```
#[proc_macro_derive(McpResource, attributes(uri, name, description, content, content_type))]
pub fn derive_mcp_resource(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    resource::mcp_resource_impl(input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Declarative macro for creating simple resources
/// 
/// This provides a concise syntax for resource creation.
/// 
/// # Example
/// 
/// ```rust
/// use mcp_derive::resource;
/// 
/// let config_resource = resource! {
///     uri: "file://config.json",
///     name: "Configuration",
///     description: "Application configuration file",
///     content: |_| async {
///         let config = serde_json::json!({
///             "app_name": "Test App",
///             "version": "1.0.0"
///         });
///         Ok(vec![mcp_protocol::resources::ResourceContent::blob(
///             serde_json::to_string_pretty(&config).unwrap(),
///             "application/json".to_string()
///         )])
///     }
/// };
/// ```
#[proc_macro]
pub fn resource(input: TokenStream) -> TokenStream {
    match resource_declarative_impl(input) {
        Ok(tokens) => tokens,
        Err(err) => err.to_compile_error().into(),
    }
}

/// Generate a JSON schema for a Rust type
/// 
/// This macro generates a JSON schema definition for any Rust type that implements
/// Serialize. It analyzes the type structure and creates appropriate schema constraints.
/// 
/// # Example
/// 
/// ```rust
/// use mcp_derive::schema_for;
/// use serde::{Serialize, Deserialize};
/// 
/// #[derive(Serialize, Deserialize)]
/// struct Point {
///     x: f64,
///     y: f64,
/// }
/// 
/// let schema = schema_for!(Point);
/// ```
#[proc_macro]
pub fn schema_for(input: TokenStream) -> TokenStream {
    match schema_for_impl(input) {
        Ok(tokens) => tokens,
        Err(err) => err.to_compile_error().into(),
    }
}

/// Declarative macro for creating simple tools
/// 
/// This provides the most concise syntax for tool creation.
/// 
/// # Example
/// 
/// ```rust
/// use mcp_derive::tool;
/// 
/// let divide_tool = tool! {
///     name: "divide",
///     description: "Divide two numbers",
///     params: {
///         a: f64 => "Dividend",
///         b: f64 => "Divisor",
///     },
///     execute: |a, b| async move {
///         if b == 0.0 {
///             Err("Division by zero".to_string())
///         } else {
///             Ok(format!("{} ÷ {} = {}", a, b, a / b))
///         }
///     }
/// };
/// ```
#[proc_macro]
pub fn tool(input: TokenStream) -> TokenStream {
    match tool_declarative_impl(input) {
        Ok(tokens) => tokens,
        Err(err) => err.to_compile_error().into(),
    }
}

fn tool_declarative_impl(input: TokenStream) -> Result<TokenStream, syn::Error> {
    let input = syn::parse::<ToolMacroInput>(input)?;
    
    let tool_name_ident = syn::Ident::new(
        &format!("{}Tool", capitalize(&input.name)),
        proc_macro2::Span::call_site()
    );
    
    // Generate parameter extraction
    let mut param_extractions = Vec::new();
    let mut param_names = Vec::new();
    let mut schema_properties = Vec::new();
    let mut required_fields = Vec::new();
    
    for param in &input.params {
        let param_name = &param.name;
        let param_type = &param.param_type;
        let param_desc = &param.description;
        let optional = param.optional;
        let min_value = param.min_value;
        let max_value = param.max_value;
        
        param_names.push(param_name);
        let param_name_str = param_name.to_string();
        
        // Generate schema based on type with enhanced constraints
        let type_str = quote!(#param_type).to_string();
        let base_schema = match type_str.as_str() {
            "f64" | "f32" => {
                let mut schema = quote! { mcp_protocol::schema::JsonSchema::number().with_description(#param_desc) };
                if let Some(min) = min_value {
                    schema = quote! { #schema.with_minimum(#min) };
                }
                if let Some(max) = max_value {
                    schema = quote! { #schema.with_maximum(#max) };
                }
                schema
            },
            "i64" | "i32" | "i16" | "i8" | "u64" | "u32" | "u16" | "u8" | "isize" | "usize" => {
                let mut schema = quote! { mcp_protocol::schema::JsonSchema::integer().with_description(#param_desc) };
                if let Some(min) = min_value {
                    let min_int = min as i64;
                    schema = quote! { #schema.with_minimum(#min_int) };
                }
                if let Some(max) = max_value {
                    let max_int = max as i64;
                    schema = quote! { #schema.with_maximum(#max_int) };
                }
                schema
            },
            "bool" => quote! { mcp_protocol::schema::JsonSchema::boolean().with_description(#param_desc) },
            "String" => quote! { mcp_protocol::schema::JsonSchema::string().with_description(#param_desc) },
            s if s.starts_with("Option<") => {
                // Handle Option types by extracting the inner type
                let inner_type_str = &s[7..s.len()-1]; // Remove "Option<" and ">"
                match inner_type_str {
                    "f64" | "f32" => quote! { mcp_protocol::schema::JsonSchema::number().with_description(#param_desc) },
                    "i64" | "i32" | "i16" | "i8" | "u64" | "u32" | "u16" | "u8" | "isize" | "usize" => quote! { mcp_protocol::schema::JsonSchema::integer().with_description(#param_desc) },
                    "bool" => quote! { mcp_protocol::schema::JsonSchema::boolean().with_description(#param_desc) },
                    "String" => quote! { mcp_protocol::schema::JsonSchema::string().with_description(#param_desc) },
                    _ => quote! { mcp_protocol::schema::JsonSchema::string().with_description(#param_desc) },
                }
            },
            _ => quote! { mcp_protocol::schema::JsonSchema::string().with_description(#param_desc) },
        };
        
        schema_properties.push(quote! {
            (#param_name_str.to_string(), #base_schema)
        });
        
        // Only add to required fields if not optional
        if !optional {
            required_fields.push(quote! {
                #param_name_str.to_string()
            });
        }
        
        // Generate parameter extraction based on type and constraints
        let default_expr = param.default_value.as_ref();
        
        let extraction = if optional && !type_str.starts_with("Option<") {
            // Handle explicitly optional parameters that aren't Option<T> types
            match type_str.as_str() {
                "f64" => quote! {
                    let #param_name = args.get(#param_name_str)
                        .and_then(|v| v.as_f64());
                },
                "f32" => quote! {
                    let #param_name = args.get(#param_name_str)
                        .and_then(|v| v.as_f64())
                        .map(|f| f as f32);
                },
                "i64" => quote! {
                    let #param_name = args.get(#param_name_str)
                        .and_then(|v| v.as_i64());
                },
                "String" => quote! {
                    let #param_name = args.get(#param_name_str)
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                },
                _ => quote! {
                    let #param_name: Option<#param_type> = args.get(#param_name_str)
                        .and_then(|v| serde_json::from_value(v.clone()).ok());
                },
            }
        } else if type_str.starts_with("Option<") {
            // Handle Option<T> types
            let inner_type = &type_str[7..type_str.len()-1];
            match inner_type {
                "f64" => quote! {
                    let #param_name = args.get(#param_name_str).and_then(|v| v.as_f64());
                },
                "f32" => quote! {
                    let #param_name = args.get(#param_name_str)
                        .and_then(|v| v.as_f64())
                        .map(|f| f as f32);
                },
                "i64" => quote! {
                    let #param_name = args.get(#param_name_str).and_then(|v| v.as_i64());
                },
                "String" => quote! {
                    let #param_name = args.get(#param_name_str)
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                },
                "bool" => quote! {
                    let #param_name = args.get(#param_name_str).and_then(|v| v.as_bool());
                },
                _ => quote! {
                    let #param_name: #param_type = args.get(#param_name_str)
                        .and_then(|v| serde_json::from_value(v.clone()).ok());
                },
            }
        } else {
            // Handle required parameters
            match type_str.as_str() {
                "f64" => quote! {
                    let #param_name = args.get(#param_name_str)
                        .and_then(|v| v.as_f64())
                        .ok_or_else(|| format!("Missing or invalid parameter '{}'", #param_name_str))?;
                },
                "f32" => quote! {
                    let #param_name = args.get(#param_name_str)
                        .and_then(|v| v.as_f64())
                        .map(|f| f as f32)
                        .ok_or_else(|| format!("Missing or invalid parameter '{}'", #param_name_str))?;
                },
                "i64" => quote! {
                    let #param_name = args.get(#param_name_str)
                        .and_then(|v| v.as_i64())
                        .ok_or_else(|| mcp_protocol::McpError::invalid_param_type(#param_name_str, "integer", "other"))?;
                },
                "i32" => quote! {
                    let #param_name = args.get(#param_name_str)
                        .and_then(|v| v.as_i64())
                        .and_then(|i| i.try_into().ok())
                        .ok_or_else(|| mcp_protocol::McpError::invalid_param_type(#param_name_str, "integer", "other"))?;
                },
                "bool" => quote! {
                    let #param_name = args.get(#param_name_str)
                        .and_then(|v| v.as_bool())
                        .ok_or_else(|| mcp_protocol::McpError::invalid_param_type(#param_name_str, "boolean", "other"))?;
                },
                "String" => quote! {
                    let #param_name = args.get(#param_name_str)
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                        .ok_or_else(|| mcp_protocol::McpError::invalid_param_type(#param_name_str, "string", "other"))?;
                },
                _ => quote! {
                    let #param_name: #param_type = args.get(#param_name_str)
                        .ok_or_else(|| mcp_protocol::McpError::missing_param(#param_name_str))
                        .and_then(|v| serde_json::from_value(v.clone())
                            .map_err(|_| mcp_protocol::McpError::invalid_param_type(#param_name_str, "object", "other")))?;
                },
            }
        };
        
        // If there's a default value and the parameter is optional, apply the default
        let final_extraction = if let Some(default) = default_expr {
            if optional {
                quote! {
                    #extraction
                    let #param_name = #param_name.unwrap_or_else(|| #default);
                }
            } else {
                extraction
            }
        } else {
            extraction
        };
        
        param_extractions.push(final_extraction);
    }
    
    let tool_name = &input.name;
    let tool_description = &input.description;
    let execute_closure = &input.execute;
    
    let expanded = quote! {
        {
            #[derive(Clone)]
            struct #tool_name_ident;
            
            #[async_trait::async_trait]
            impl mcp_server::McpTool for #tool_name_ident {
                fn name(&self) -> &str {
                    #tool_name
                }
                
                fn description(&self) -> &str {
                    #tool_description
                }
                
                fn input_schema(&self) -> mcp_protocol::ToolSchema {
                    use std::collections::HashMap;
                    
                    mcp_protocol::ToolSchema::object()
                        .with_properties(HashMap::from([
                            #(#schema_properties),*
                        ]))
                        .with_required(vec![
                            #(#required_fields),*
                        ])
                }
                
                async fn call(&self, args: serde_json::Value, _session: Option<mcp_server::SessionContext>) -> mcp_server::McpResult<Vec<mcp_protocol::ToolResult>> {
                    use serde_json::Value;
                    
                    // Extract parameters
                    #(#param_extractions)*
                    
                    // Call the execute closure
                    let execute_fn = #execute_closure;
                    match execute_fn(#(#param_names),*).await {
                        Ok(result) => {
                            Ok(vec![mcp_protocol::ToolResult::text(result)])
                        }
                        Err(e) => Err(mcp_protocol::McpError::ToolExecutionError(e.to_string()))
                    }
                }
            }
            
            #tool_name_ident
        }
    };
    
    Ok(expanded.into())
}

fn resource_declarative_impl(input: TokenStream) -> Result<TokenStream, syn::Error> {
    let input = syn::parse::<ResourceMacroInput>(input)?;
    
    let resource_name_ident = syn::Ident::new(
        &format!("{}Resource", capitalize(&input.name.replace(" ", ""))),
        proc_macro2::Span::call_site()
    );
    
    let uri = &input.uri;
    let name = &input.name;
    let description = &input.description;
    let content_closure = &input.content;
    
    let expanded = quote! {
        {
            #[derive(Clone)]
            struct #resource_name_ident;
            
            #[async_trait::async_trait]
            impl mcp_server::McpResource for #resource_name_ident {
                fn uri(&self) -> &str {
                    #uri
                }
                
                fn name(&self) -> &str {
                    #name
                }
                
                fn description(&self) -> &str {
                    #description
                }
                
                async fn read(&self) -> mcp_server::McpResult<Vec<mcp_protocol::resources::ResourceContent>> {
                    let content_fn = #content_closure;
                    content_fn(self).await
                }
            }
            
            #resource_name_ident
        }
    };
    
    Ok(expanded.into())
}

fn schema_for_impl(input: TokenStream) -> Result<TokenStream, syn::Error> {
    let input = syn::parse::<syn::Type>(input)?;
    
    let expanded = quote! {
        {
            use mcp_protocol::schema::JsonSchema;
            use std::collections::HashMap;
            
            // Generate schema based on the type
            let schema = match stringify!(#input) {
                "f64" | "f32" => JsonSchema::number(),
                "i64" | "i32" | "i16" | "i8" | "u64" | "u32" | "u16" | "u8" | "isize" | "usize" => JsonSchema::integer(),
                "bool" => JsonSchema::boolean(),
                "String" | "&str" => JsonSchema::string(),
                "Vec<String>" => JsonSchema::array(JsonSchema::string()),
                "Vec<f64>" => JsonSchema::array(JsonSchema::number()),
                "Vec<i32>" => JsonSchema::array(JsonSchema::integer()),
                "HashMap<String, String>" => JsonSchema::object(),
                _ => {
                    // For complex types, try to generate a basic object schema
                    // This is a simplified implementation - a full implementation would
                    // use reflection or compile-time analysis
                    JsonSchema::object().with_description("Custom type - manual schema recommended")
                }
            };
            
            schema
        }
    };
    
    Ok(expanded.into())
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().chain(chars).collect(),
    }
}

// Parser for the tool! macro syntax
struct ToolMacroInput {
    name: String,
    description: String,
    params: Vec<ToolParam>,
    execute: syn::Expr,
}

struct ToolParam {
    name: syn::Ident,
    param_type: syn::Type,
    description: String,
    optional: bool,
    min_value: Option<f64>,
    max_value: Option<f64>,
    default_value: Option<syn::Expr>,
}

// Parser for the resource! macro syntax
struct ResourceMacroInput {
    uri: String,
    name: String,
    description: String,
    content: syn::Expr,
}

impl syn::parse::Parse for ToolMacroInput {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut name = None;
        let mut description = None;
        let mut params = Vec::new();
        let mut execute = None;
        
        while !input.is_empty() {
            let ident: syn::Ident = input.parse()?;
            input.parse::<syn::Token![:]>()?;
            
            match ident.to_string().as_str() {
                "name" => {
                    let lit: syn::LitStr = input.parse()?;
                    name = Some(lit.value());
                }
                "description" => {
                    let lit: syn::LitStr = input.parse()?;
                    description = Some(lit.value());
                }
                "params" => {
                    let content;
                    syn::braced!(content in input);
                    
                    while !content.is_empty() {
                        let param_name: syn::Ident = content.parse()?;
                        content.parse::<syn::Token![:]>()?;
                        let param_type: syn::Type = content.parse()?;
                        content.parse::<syn::Token![=>]>()?;
                        let param_desc: syn::LitStr = content.parse()?;
                        
                        // Parse optional parameter attributes
                        let mut optional = false;
                        let mut min_value = None;
                        let mut max_value = None;
                        let mut default_value = None;
                        
                        // Check if there are parameter constraints in braces
                        if content.peek(syn::token::Brace) {
                            let constraint_content;
                            syn::braced!(constraint_content in content);
                            
                            while !constraint_content.is_empty() {
                                let constraint_ident: syn::Ident = constraint_content.parse()?;
                                constraint_content.parse::<syn::Token![:]>()?;
                                
                                match constraint_ident.to_string().as_str() {
                                    "optional" => {
                                        let value: syn::LitBool = constraint_content.parse()?;
                                        optional = value.value;
                                    }
                                    "min" => {
                                        // Parse as a literal that could be int or float
                                        let lookahead = constraint_content.lookahead1();
                                        if lookahead.peek(syn::LitFloat) {
                                            let value: syn::LitFloat = constraint_content.parse()?;
                                            min_value = Some(value.base10_parse()?);
                                        } else if lookahead.peek(syn::LitInt) {
                                            let value: syn::LitInt = constraint_content.parse()?;
                                            min_value = Some(value.base10_parse::<i64>()? as f64);
                                        } else {
                                            return Err(lookahead.error());
                                        }
                                    }
                                    "max" => {
                                        // Parse as a literal that could be int or float
                                        let lookahead = constraint_content.lookahead1();
                                        if lookahead.peek(syn::LitFloat) {
                                            let value: syn::LitFloat = constraint_content.parse()?;
                                            max_value = Some(value.base10_parse()?);
                                        } else if lookahead.peek(syn::LitInt) {
                                            let value: syn::LitInt = constraint_content.parse()?;
                                            max_value = Some(value.base10_parse::<i64>()? as f64);
                                        } else {
                                            return Err(lookahead.error());
                                        }
                                    }
                                    "default" => {
                                        let expr: syn::Expr = constraint_content.parse()?;
                                        default_value = Some(expr);
                                    }
                                    _ => {
                                        return Err(syn::Error::new_spanned(&constraint_ident, 
                                            format!("Unknown parameter constraint: {}", constraint_ident)));
                                    }
                                }
                                
                                if constraint_content.peek(syn::Token![,]) {
                                    constraint_content.parse::<syn::Token![,]>()?;
                                }
                            }
                        }
                        
                        // Auto-detect optional types (Option<T>)
                        if let syn::Type::Path(type_path) = &param_type {
                            if let Some(segment) = type_path.path.segments.last() {
                                if segment.ident == "Option" {
                                    optional = true;
                                }
                            }
                        }
                        
                        params.push(ToolParam {
                            name: param_name,
                            param_type,
                            description: param_desc.value(),
                            optional,
                            min_value,
                            max_value,
                            default_value,
                        });
                        
                        if content.peek(syn::Token![,]) {
                            content.parse::<syn::Token![,]>()?;
                        }
                    }
                }
                "execute" => {
                    let expr: syn::Expr = input.parse()?;
                    execute = Some(expr);
                }
                _ => {
                    return Err(syn::Error::new_spanned(&ident, format!("Unknown field: {}", ident)));
                }
            }
            
            if input.peek(syn::Token![,]) {
                input.parse::<syn::Token![,]>()?;
            }
        }
        
        Ok(ToolMacroInput {
            name: name.ok_or_else(|| syn::Error::new(input.span(), "Missing 'name' field"))?,
            description: description.ok_or_else(|| syn::Error::new(input.span(), "Missing 'description' field"))?,
            params,
            execute: execute.ok_or_else(|| syn::Error::new(input.span(), "Missing 'execute' field"))?,
        })
    }
}

impl syn::parse::Parse for ResourceMacroInput {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut uri = None;
        let mut name = None;
        let mut description = None;
        let mut content = None;
        
        while !input.is_empty() {
            let ident: syn::Ident = input.parse()?;
            input.parse::<syn::Token![:]>()?;
            
            match ident.to_string().as_str() {
                "uri" => {
                    let lit: syn::LitStr = input.parse()?;
                    uri = Some(lit.value());
                }
                "name" => {
                    let lit: syn::LitStr = input.parse()?;
                    name = Some(lit.value());
                }
                "description" => {
                    let lit: syn::LitStr = input.parse()?;
                    description = Some(lit.value());
                }
                "content" => {
                    let expr: syn::Expr = input.parse()?;
                    content = Some(expr);
                }
                _ => {
                    return Err(syn::Error::new_spanned(&ident, format!("Unknown field: {}", ident)));
                }
            }
            
            if input.peek(syn::Token![,]) {
                input.parse::<syn::Token![,]>()?;
            }
        }
        
        Ok(ResourceMacroInput {
            uri: uri.ok_or_else(|| syn::Error::new(input.span(), "Missing 'uri' field"))?,
            name: name.ok_or_else(|| syn::Error::new(input.span(), "Missing 'name' field"))?,
            description: description.ok_or_else(|| syn::Error::new(input.span(), "Missing 'description' field"))?,
            content: content.ok_or_else(|| syn::Error::new(input.span(), "Missing 'content' field"))?,
        })
    }
}