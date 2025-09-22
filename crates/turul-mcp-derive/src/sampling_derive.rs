//! Implementation of #[derive(McpSampling)] macro

use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Result};

use crate::utils::extract_string_attribute;

pub fn derive_mcp_sampling_impl(input: DeriveInput) -> Result<TokenStream> {
    let struct_name = &input.ident;

    // Extract struct-level attributes
    let max_tokens = extract_string_attribute(&input.attrs, "max_tokens")
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(1000);

    let temperature =
        extract_string_attribute(&input.attrs, "temperature").and_then(|s| s.parse::<f64>().ok());

    let model = extract_string_attribute(&input.attrs, "model");

    let expanded = quote! {
        #[automatically_derived]
        impl turul_mcp_protocol::sampling::HasSamplingConfig for #struct_name {
            fn max_tokens(&self) -> u32 {
                #max_tokens
            }

            fn temperature(&self) -> Option<f64> {
                #temperature
            }

            fn stop_sequences(&self) -> Option<&Vec<String>> {
                None
            }
        }

        #[automatically_derived]
        impl turul_mcp_protocol::sampling::HasSamplingContext for #struct_name {
            fn messages(&self) -> &[turul_mcp_protocol::sampling::SamplingMessage] {
                // Default: empty messages - should be overridden
                &[]
            }

            fn system_prompt(&self) -> Option<&str> {
                None
            }

            fn include_context(&self) -> Option<&str> {
                None
            }
        }

        #[automatically_derived]
        impl turul_mcp_protocol::sampling::HasModelPreferences for #struct_name {
            fn model_preferences(&self) -> Option<&serde_json::Value> {
                use std::sync::LazyLock;
                static MODEL_PREFS: LazyLock<Option<serde_json::Value>> = LazyLock::new(|| {
                    #model.map(|m| serde_json::json!({"model": m}))
                });
                MODEL_PREFS.as_ref()
            }

            fn metadata(&self) -> Option<&serde_json::Value> {
                None
            }
        }

        // SamplingDefinition is automatically implemented via blanket impl

        #[automatically_derived]
        #[async_trait::async_trait]
        impl turul_mcp_server::McpSampling for #struct_name {
            async fn sample(&self, request: turul_mcp_protocol::sampling::CreateMessageRequest) -> turul_mcp_protocol::McpResult<turul_mcp_protocol::sampling::CreateMessageResponse> {
                // Default implementation - this should be overridden by implementing sample_impl
                match self.sample_impl(request).await {
                    Ok(result) => Ok(result),
                    Err(e) => Err(turul_mcp_protocol::McpError::tool_execution(&e)),
                }
            }
        }

        impl #struct_name {
            /// Override this method to provide custom sampling logic
            pub async fn sample_impl(&self, _request: turul_mcp_protocol::sampling::CreateMessageRequest) -> Result<turul_mcp_protocol::sampling::CreateMessageResponse, String> {
                // Default: return a simple response
                let response_message = turul_mcp_protocol::sampling::SamplingMessage {
                    role: "assistant".to_string(),
                    content: turul_mcp_protocol::sampling::MessageContent::Text {
                        text: "This is a generated response. Override sample_impl() to customize.".to_string(),
                    },
                };

                let model_name = #model.unwrap_or("unknown-model");
                Ok(turul_mcp_protocol::sampling::CreateMessageResponse::new(response_message, model_name))
            }
        }
    };

    Ok(expanded)
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_simple_sampling() {
        let input: DeriveInput = parse_quote! {
            #[sampling(model = "claude-3-haiku", temperature = 0.7, max_tokens = 1000)]
            struct TextGenerator {
                prompt: String,
            }
        };

        let result = derive_mcp_sampling_impl(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_minimal_sampling() {
        let input: DeriveInput = parse_quote! {
            struct SimpleGenerator;
        };

        let result = derive_mcp_sampling_impl(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_sampling_with_custom_tokens() {
        let input: DeriveInput = parse_quote! {
            #[sampling(max_tokens = 500)]
            struct ShortResponseGenerator {
                context: String,
            }
        };

        let result = derive_mcp_sampling_impl(input);
        assert!(result.is_ok());
    }
}
