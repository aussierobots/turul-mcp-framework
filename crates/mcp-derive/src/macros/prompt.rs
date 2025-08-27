//! Declarative prompt! macro implementation
//!
//! This module provides the `prompt!` macro for creating MCP prompts using concise syntax.

use proc_macro::TokenStream;
// use proc_macro2::TokenStream as TokenStream2; // Currently unused
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Expr, Ident, LitStr, Result, Token,
};

/// Parsed content of a prompt! macro
pub struct PromptMacro {
    pub name: LitStr,
    pub description: Option<LitStr>,
    pub title: Option<LitStr>,
    pub arguments: Vec<PromptArgument>,
    pub template: Expr,
}

/// A single prompt argument
pub struct PromptArgument {
    pub name: Ident,
    pub arg_type: Ident,
    pub description: LitStr,
    pub required: bool,
}

impl Parse for PromptMacro {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut name = None;
        let mut description = None;
        let mut title = None;
        let mut arguments = Vec::new();
        let mut template = None;

        while !input.is_empty() {
            let field_name: Ident = input.parse()?;
            input.parse::<Token![:]>()?;

            match field_name.to_string().as_str() {
                "name" => {
                    name = Some(input.parse::<LitStr>()?);
                }
                "title" => {
                    title = Some(input.parse::<LitStr>()?);
                }
                "description" => {
                    description = Some(input.parse::<LitStr>()?);
                }
                "arguments" => {
                    let content;
                    syn::braced!(content in input);
                    let args: Punctuated<PromptArgument, Token![,]> =
                        content.parse_terminated(PromptArgument::parse, Token![,])?;
                    arguments = args.into_iter().collect();
                }
                "template" => {
                    template = Some(input.parse::<Expr>()?);
                }
                _ => {
                    return Err(syn::Error::new(
                        field_name.span(),
                        format!("Unknown field: {}", field_name),
                    ))
                }
            }

            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(PromptMacro {
            name: name.ok_or_else(|| input.error("Missing required 'name' field"))?,
            description,
            title,
            arguments,
            template: template.ok_or_else(|| input.error("Missing required 'template' field"))?,
        })
    }
}

impl Parse for PromptArgument {
    fn parse(input: ParseStream) -> Result<Self> {
        let name: Ident = input.parse()?;
        input.parse::<Token![:]>()?;
        let arg_type: Ident = input.parse()?;
        input.parse::<Token![=>]>()?;
        let description: LitStr = input.parse()?;

        // Optional required flag
        let required = if input.peek(Token![,]) && input.peek2(Ident) {
            let lookahead = input.fork();
            lookahead.parse::<Token![,]>()?;
            let maybe_required: Ident = lookahead.parse()?;
            if maybe_required == "required" {
                input.parse::<Token![,]>()?;
                input.parse::<Ident>()?; // consume "required"
                true
            } else {
                false
            }
        } else {
            false
        };

        Ok(PromptArgument {
            name,
            arg_type,
            description,
            required,
        })
    }
}

/// Implementation of the prompt! declarative macro
pub fn prompt_declarative_impl(input: TokenStream) -> Result<TokenStream> {
    let prompt_def = syn::parse::<PromptMacro>(input)?;

    let name = &prompt_def.name;
    let description = prompt_def
        .description
        .as_ref()
        .map(|d| quote! { Some(#d) })
        .unwrap_or_else(|| quote! { None });
    
    let title = prompt_def
        .title
        .as_ref()
        .map(|t| quote! { Some(#t) })
        .unwrap_or_else(|| quote! { None });

    let template = &prompt_def.template;

    // Generate arguments
    let prompt_args = if prompt_def.arguments.is_empty() {
        quote! { None }
    } else {
        let arg_definitions = prompt_def.arguments.iter().map(|arg| {
            let arg_name = &arg.name.to_string();
            let _arg_type = match arg.arg_type.to_string().as_str() {
                "String" | "string" => quote! { "string" },
                "i32" | "i64" | "u32" | "u64" | "isize" | "usize" | "int" | "integer" => quote! { "integer" },
                "f32" | "f64" | "float" | "number" => quote! { "number" },
                "bool" | "boolean" => quote! { "boolean" },
                _ => quote! { "string" }, // Default to string for unknown types
            };
            let description = &arg.description;
            let required = arg.required;

            quote! {
                mcp_protocol::prompts::PromptArgument {
                    name: #arg_name.to_string(),
                    description: Some(#description.to_string()),
                    required: Some(#required),
                }
            }
        });

        quote! {
            Some(vec![#(#arg_definitions),*])
        }
    };

    let result = quote! {
        {
            use mcp_protocol::prompts::*;
            use mcp_server::McpPrompt;
            use std::collections::HashMap;
            use serde_json::Value;
            
            #[derive(Clone)]
            struct GeneratedPrompt;

            impl HasPromptMetadata for GeneratedPrompt {
                fn name(&self) -> &str { #name }
                fn title(&self) -> Option<&str> { #title }
            }

            impl HasPromptDescription for GeneratedPrompt {
                fn description(&self) -> Option<&str> { #description }
            }

            impl HasPromptArguments for GeneratedPrompt {
                fn arguments(&self) -> Option<&Vec<PromptArgument>> { 
                    static ARGS: std::sync::OnceLock<Option<Vec<PromptArgument>>> = std::sync::OnceLock::new();
                    ARGS.get_or_init(|| #prompt_args).as_ref()
                }
            }

            impl HasPromptAnnotations for GeneratedPrompt {
                fn annotations(&self) -> Option<&PromptAnnotations> { None }
            }

            impl HasPromptMeta for GeneratedPrompt {
                fn prompt_meta(&self) -> Option<&HashMap<String, Value>> { None }
            }

            #[async_trait::async_trait]
            impl McpPrompt for GeneratedPrompt {
                async fn get_prompt(&self, arguments: Option<HashMap<String, Value>>) 
                    -> mcp_server::McpResult<mcp_protocol::prompts::GetPromptResponse> 
                {
                    let template_fn = #template;
                    let messages = template_fn(arguments).await?;
                    
                    Ok(mcp_protocol::prompts::GetPromptResponse::success(
                        messages,
                        #description.map(String::from)
                    ))
                }
            }

            GeneratedPrompt
        }
    };

    Ok(result.into())
}