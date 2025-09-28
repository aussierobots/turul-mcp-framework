//! Tool Declarative Macro Implementation
//!
//! Implements the `tool!{}` declarative macro for creating MCP tools with
//! a concise syntax.

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    Expr, Ident, LitBool, LitFloat, LitInt, LitStr, Result, Token, Type, parse::Parse,
    parse::ParseStream,
};

use crate::macros::shared::capitalize;

/// Implementation function for the tool!{} declarative macro
pub fn tool_declarative_impl(input: TokenStream) -> Result<TokenStream> {
    let input = syn::parse::<ToolMacroInput>(input)?;

    let tool_name_ident = syn::Ident::new(
        &format!("{}Tool", capitalize(&input.name)),
        proc_macro2::Span::call_site(),
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
                let mut schema = quote! { turul_mcp_protocol::schema::JsonSchema::number().with_description(#param_desc) };
                if let Some(min) = min_value {
                    schema = quote! { #schema.with_minimum(#min) };
                }
                if let Some(max) = max_value {
                    schema = quote! { #schema.with_maximum(#max) };
                }
                schema
            }
            "i64" | "i32" | "i16" | "i8" | "u64" | "u32" | "u16" | "u8" | "isize" | "usize" => {
                let mut schema = quote! { turul_mcp_protocol::schema::JsonSchema::integer().with_description(#param_desc) };
                if let Some(min) = min_value {
                    let min_int = min as i64;
                    schema = quote! { #schema.with_minimum(#min_int) };
                }
                if let Some(max) = max_value {
                    let max_int = max as i64;
                    schema = quote! { #schema.with_maximum(#max_int) };
                }
                schema
            }
            "bool" => {
                quote! { turul_mcp_protocol::schema::JsonSchema::boolean().with_description(#param_desc) }
            }
            "String" => {
                quote! { turul_mcp_protocol::schema::JsonSchema::string().with_description(#param_desc) }
            }
            s if s.starts_with("Option<") => {
                // Handle Option types by extracting the inner type
                let inner_type_str = &s[7..s.len() - 1]; // Remove "Option<" and ">"
                match inner_type_str {
                    "f64" | "f32" => {
                        quote! { turul_mcp_protocol::schema::JsonSchema::number().with_description(#param_desc) }
                    }
                    "i64" | "i32" | "i16" | "i8" | "u64" | "u32" | "u16" | "u8" | "isize"
                    | "usize" => {
                        quote! { turul_mcp_protocol::schema::JsonSchema::integer().with_description(#param_desc) }
                    }
                    "bool" => {
                        quote! { turul_mcp_protocol::schema::JsonSchema::boolean().with_description(#param_desc) }
                    }
                    "String" => {
                        quote! { turul_mcp_protocol::schema::JsonSchema::string().with_description(#param_desc) }
                    }
                    _ => {
                        quote! { turul_mcp_protocol::schema::JsonSchema::string().with_description(#param_desc) }
                    }
                }
            }
            _ => {
                quote! { turul_mcp_protocol::schema::JsonSchema::string().with_description(#param_desc) }
            }
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
            let inner_type = &type_str[7..type_str.len() - 1];
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
                        .ok_or_else(|| turul_mcp_protocol::McpError::invalid_param_type(#param_name_str, "integer", "other"))?;
                },
                "i32" => quote! {
                    let #param_name = args.get(#param_name_str)
                        .and_then(|v| v.as_i64())
                        .and_then(|i| i.try_into().ok())
                        .ok_or_else(|| turul_mcp_protocol::McpError::invalid_param_type(#param_name_str, "integer", "other"))?;
                },
                "bool" => quote! {
                    let #param_name = args.get(#param_name_str)
                        .and_then(|v| v.as_bool())
                        .ok_or_else(|| turul_mcp_protocol::McpError::invalid_param_type(#param_name_str, "boolean", "other"))?;
                },
                "String" => quote! {
                    let #param_name = args.get(#param_name_str)
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                        .ok_or_else(|| turul_mcp_protocol::McpError::invalid_param_type(#param_name_str, "string", "other"))?;
                },
                _ => quote! {
                    let #param_name: #param_type = args.get(#param_name_str)
                        .ok_or_else(|| turul_mcp_protocol::McpError::missing_param(#param_name_str))
                        .and_then(|v| serde_json::from_value(v.clone())
                            .map_err(|_| turul_mcp_protocol::McpError::invalid_param_type(#param_name_str, "object", "other")))?;
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
            struct #tool_name_ident {
                input_schema: turul_mcp_protocol::ToolSchema,
            }

            impl #tool_name_ident {
                fn new() -> Self {
                    use std::collections::HashMap;
                    let input_schema = turul_mcp_protocol::ToolSchema::object()
                        .with_properties(HashMap::from([
                            #(#schema_properties),*
                        ]))
                        .with_required(vec![
                            #(#required_fields),*
                        ]);
                    Self { input_schema }
                }
            }

            // Implement fine-grained traits
            impl turul_mcp_protocol::tools::HasBaseMetadata for #tool_name_ident {
                fn name(&self) -> &str {
                    #tool_name
                }
            }

            impl turul_mcp_protocol::tools::HasDescription for #tool_name_ident {
                fn description(&self) -> Option<&str> {
                    Some(#tool_description)
                }
            }

            impl turul_mcp_protocol::tools::HasInputSchema for #tool_name_ident {
                fn input_schema(&self) -> &turul_mcp_protocol::ToolSchema {
                    &self.input_schema
                }
            }

            impl turul_mcp_protocol::tools::HasOutputSchema for #tool_name_ident {
                fn output_schema(&self) -> Option<&turul_mcp_protocol::ToolSchema> {
                    None
                }
            }

            impl turul_mcp_protocol::tools::HasAnnotations for #tool_name_ident {
                fn annotations(&self) -> Option<&turul_mcp_protocol::tools::ToolAnnotations> {
                    None
                }
            }

            impl turul_mcp_protocol::tools::HasToolMeta for #tool_name_ident {
                fn tool_meta(&self) -> Option<&std::collections::HashMap<String, serde_json::Value>> {
                    None
                }
            }

            // ToolDefinition automatically implemented via blanket impl!

            #[async_trait::async_trait]
            impl turul_mcp_server::McpTool for #tool_name_ident {
                async fn call(&self, args: serde_json::Value, _session: Option<turul_mcp_server::SessionContext>) -> turul_mcp_server::McpResult<turul_mcp_protocol::tools::CallToolResult> {
                    use serde_json::Value;

                    // Extract parameters
                    #(#param_extractions)*

                    // Call the execute closure
                    let execute_fn = #execute_closure;
                    match execute_fn(#(#param_names),*).await {
                        Ok(result) => {
                            Ok(turul_mcp_protocol::tools::CallToolResult::success(vec![turul_mcp_protocol::ToolResult::text(result)]))
                        }
                        Err(e) => Err(turul_mcp_protocol::McpError::tool_execution(&e.to_string()))
                    }
                }
            }

            #tool_name_ident::new()
        }
    };

    Ok(expanded.into())
}

/// Parser for the tool! macro syntax
pub struct ToolMacroInput {
    pub name: String,
    pub description: String,
    pub params: Vec<ToolParam>,
    pub execute: Expr,
}

pub struct ToolParam {
    pub name: Ident,
    pub param_type: Type,
    pub description: String,
    pub optional: bool,
    pub min_value: Option<f64>,
    pub max_value: Option<f64>,
    pub default_value: Option<Expr>,
}

impl Parse for ToolMacroInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut name = None;
        let mut description = None;
        let mut params = Vec::new();
        let mut execute = None;

        while !input.is_empty() {
            let ident: Ident = input.parse()?;
            input.parse::<Token![:]>()?;

            match ident.to_string().as_str() {
                "name" => {
                    let lit: LitStr = input.parse()?;
                    name = Some(lit.value());
                }
                "description" => {
                    let lit: LitStr = input.parse()?;
                    description = Some(lit.value());
                }
                "params" => {
                    let content;
                    syn::braced!(content in input);

                    while !content.is_empty() {
                        let param_name: Ident = content.parse()?;
                        content.parse::<Token![:]>()?;
                        let param_type: Type = content.parse()?;
                        content.parse::<Token![=>]>()?;
                        let param_desc: LitStr = content.parse()?;

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
                                let constraint_ident: Ident = constraint_content.parse()?;
                                constraint_content.parse::<Token![:]>()?;

                                match constraint_ident.to_string().as_str() {
                                    "optional" => {
                                        let value: LitBool = constraint_content.parse()?;
                                        optional = value.value;
                                    }
                                    "min" => {
                                        // Parse as a literal that could be int or float
                                        let lookahead = constraint_content.lookahead1();
                                        if lookahead.peek(LitFloat) {
                                            let value: LitFloat = constraint_content.parse()?;
                                            min_value = Some(value.base10_parse()?);
                                        } else if lookahead.peek(LitInt) {
                                            let value: LitInt = constraint_content.parse()?;
                                            min_value = Some(value.base10_parse::<i64>()? as f64);
                                        } else {
                                            return Err(lookahead.error());
                                        }
                                    }
                                    "max" => {
                                        // Parse as a literal that could be int or float
                                        let lookahead = constraint_content.lookahead1();
                                        if lookahead.peek(LitFloat) {
                                            let value: LitFloat = constraint_content.parse()?;
                                            max_value = Some(value.base10_parse()?);
                                        } else if lookahead.peek(LitInt) {
                                            let value: LitInt = constraint_content.parse()?;
                                            max_value = Some(value.base10_parse::<i64>()? as f64);
                                        } else {
                                            return Err(lookahead.error());
                                        }
                                    }
                                    "default" => {
                                        let expr: Expr = constraint_content.parse()?;
                                        default_value = Some(expr);
                                    }
                                    _ => {
                                        return Err(syn::Error::new_spanned(
                                            &constraint_ident,
                                            format!(
                                                "Unknown parameter constraint: {}",
                                                constraint_ident
                                            ),
                                        ));
                                    }
                                }

                                if constraint_content.peek(Token![,]) {
                                    constraint_content.parse::<Token![,]>()?;
                                }
                            }
                        }

                        // Auto-detect optional types (Option<T>)
                        if let Type::Path(type_path) = &param_type
                            && let Some(segment) = type_path.path.segments.last()
                            && segment.ident == "Option"
                        {
                            optional = true;
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

                        if content.peek(Token![,]) {
                            content.parse::<Token![,]>()?;
                        }
                    }
                }
                "execute" => {
                    let expr: Expr = input.parse()?;
                    execute = Some(expr);
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

        Ok(ToolMacroInput {
            name: name.ok_or_else(|| syn::Error::new(input.span(), "Missing 'name' field"))?,
            description: description
                .ok_or_else(|| syn::Error::new(input.span(), "Missing 'description' field"))?,
            params,
            execute: execute
                .ok_or_else(|| syn::Error::new(input.span(), "Missing 'execute' field"))?,
        })
    }
}
