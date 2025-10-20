//! Declarative sampling! macro implementation
//!
//! This module provides the `sampling!` macro for creating MCP sampling configurations using concise syntax.

use proc_macro::TokenStream;
// use proc_macro2::TokenStream as TokenStream2; // Currently unused
use quote::quote;
use syn::{
    // punctuated::Punctuated, // Currently unused
    Expr,
    Ident,
    /* Lit, */ LitFloat,
    LitInt,
    LitStr,
    Result,
    Token,
    parse::{Parse, ParseStream},
};

/// Parsed content of a sampling! macro
pub struct SamplingMacro {
    pub name: Option<LitStr>,
    pub max_tokens: LitInt,
    pub temperature: Option<LitFloat>,
    pub model_preferences: Option<Expr>,
    pub system_prompt: Option<LitStr>,
    pub stop_sequences: Option<Expr>,
    pub handler: Expr,
}

impl Parse for SamplingMacro {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut name = None;
        let mut max_tokens = None;
        let mut temperature = None;
        let mut model_preferences = None;
        let mut system_prompt = None;
        let mut stop_sequences = None;
        let mut handler = None;

        while !input.is_empty() {
            let field_name: Ident = input.parse()?;
            input.parse::<Token![:]>()?;

            match field_name.to_string().as_str() {
                "name" => {
                    name = Some(input.parse::<LitStr>()?);
                }
                "max_tokens" => {
                    max_tokens = Some(input.parse::<LitInt>()?);
                }
                "temperature" => {
                    temperature = Some(input.parse::<LitFloat>()?);
                }
                "model_preferences" => {
                    model_preferences = Some(input.parse::<Expr>()?);
                }
                "system_prompt" => {
                    system_prompt = Some(input.parse::<LitStr>()?);
                }
                "stop_sequences" => {
                    stop_sequences = Some(input.parse::<Expr>()?);
                }
                "handler" => {
                    handler = Some(input.parse::<Expr>()?);
                }
                _ => {
                    return Err(syn::Error::new(
                        field_name.span(),
                        format!("Unknown field: {}", field_name),
                    ));
                }
            }

            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(SamplingMacro {
            name,
            max_tokens: max_tokens
                .ok_or_else(|| input.error("Missing required 'max_tokens' field"))?,
            temperature,
            model_preferences,
            system_prompt,
            stop_sequences,
            handler: handler.ok_or_else(|| input.error("Missing required 'handler' field"))?,
        })
    }
}

/// Implementation of the sampling! declarative macro
pub fn sampling_declarative_impl(input: TokenStream) -> Result<TokenStream> {
    let sampling_def = syn::parse::<SamplingMacro>(input)?;

    let max_tokens = &sampling_def.max_tokens;
    let temperature = sampling_def
        .temperature
        .as_ref()
        .map(|t| quote! { Some(#t) })
        .unwrap_or_else(|| quote! { None });

    let system_prompt = sampling_def
        .system_prompt
        .as_ref()
        .map(|p| quote! { Some(#p) })
        .unwrap_or_else(|| quote! { None });

    let model_preferences = sampling_def
        .model_preferences
        .as_ref()
        .map(|p| quote! { Some(&#p) })
        .unwrap_or_else(|| quote! { None });

    let stop_sequences = sampling_def
        .stop_sequences
        .as_ref()
        .map(|s| quote! { Some(&#s) })
        .unwrap_or_else(|| quote! { None });

    let handler = &sampling_def.handler;

    // Use the provided name or default to "GeneratedSampling"
    let struct_name = sampling_def
        .name
        .as_ref()
        .map(|n| {
            let name_str = n.value();
            syn::Ident::new(&name_str, n.span())
        })
        .unwrap_or_else(|| syn::Ident::new("GeneratedSampling", proc_macro2::Span::call_site()));

    let result = quote! {
        {
            use turul_mcp_protocol::sampling::*;
            use turul_mcp_server::McpSampling;
            use std::collections::HashMap;
            use serde_json::Value;

            #[derive(Clone)]
            struct #struct_name {
                messages: Vec<SamplingMessage>,
            }

            impl #struct_name {
                pub fn with_messages(messages: Vec<SamplingMessage>) -> Self {
                    Self { messages }
                }
            }

            impl turul_mcp_builders::HasSamplingConfig for #struct_name {
                fn max_tokens(&self) -> u32 { #max_tokens }
                fn temperature(&self) -> Option<f64> { #temperature }
                fn stop_sequences(&self) -> Option<&Vec<String>> { #stop_sequences }
            }

            impl turul_mcp_builders::HasSamplingContext for #struct_name {
                fn messages(&self) -> &[SamplingMessage] { &self.messages }
                fn system_prompt(&self) -> Option<&str> { #system_prompt }
                fn include_context(&self) -> Option<&str> { None }
            }

            impl turul_mcp_builders::HasModelPreferences for #struct_name {
                fn model_preferences(&self) -> Option<&turul_mcp_protocol::sampling::ModelPreferences> { #model_preferences }
                fn metadata(&self) -> Option<&serde_json::Value> { None }
            }

            #[async_trait::async_trait]
            impl McpSampling for #struct_name {
                async fn sample(&self, request: turul_mcp_protocol::sampling::CreateMessageRequest)
                    -> turul_mcp_protocol::McpResult<turul_mcp_protocol::sampling::CreateMessageResult>
                {
                    let handler_fn = #handler;
                    handler_fn(request).await
                }
            }

            #struct_name::with_messages(Vec::new())
        }
    };

    Ok(result.into())
}
