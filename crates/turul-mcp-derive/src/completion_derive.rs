//! Implementation of #[derive(McpCompletion)] macro

use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Result};

use crate::utils::extract_string_attribute;

pub fn derive_mcp_completion_impl(input: DeriveInput) -> Result<TokenStream> {
    let struct_name = &input.ident;

    // Extract struct-level attributes
    let reference = extract_string_attribute(&input.attrs, "reference")
        .unwrap_or_else(|| "ref/prompt".to_string());

    let argument_name =
        extract_string_attribute(&input.attrs, "argument").unwrap_or_else(|| "query".to_string());

    let expanded = quote! {
        #[automatically_derived]
        impl turul_mcp_builders::HasCompletionMetadata for #struct_name {
            fn method(&self) -> &str {
                "completion/complete"
            }

            fn reference(&self) -> &turul_mcp_protocol::completion::CompletionReference {
                use std::sync::LazyLock;
                static REFERENCE: LazyLock<turul_mcp_protocol::completion::CompletionReference> = LazyLock::new(|| {
                    if #reference.starts_with("ref/resource") {
                        turul_mcp_protocol::completion::CompletionReference::resource("resource://completion")
                    } else {
                        turul_mcp_protocol::completion::CompletionReference::prompt("completion-prompt")
                    }
                });
                &REFERENCE
            }
        }

        #[automatically_derived]
        impl turul_mcp_builders::HasCompletionContext for #struct_name {
            fn argument(&self) -> &turul_mcp_protocol::completion::CompleteArgument {
                use std::sync::LazyLock;
                static ARGUMENT: LazyLock<turul_mcp_protocol::completion::CompleteArgument> = LazyLock::new(|| {
                    turul_mcp_protocol::completion::CompleteArgument::new(#argument_name, "")
                });
                &ARGUMENT
            }

            fn context(&self) -> Option<&turul_mcp_protocol::completion::CompletionContext> {
                None
            }
        }

        #[automatically_derived]
        impl turul_mcp_builders::HasCompletionHandling for #struct_name {
            fn validate_request(&self, _request: &turul_mcp_protocol::completion::CompleteRequest) -> Result<(), String> {
                Ok(())
            }

            fn filter_completions(&self, values: Vec<String>, current_value: &str) -> Vec<String> {
                // Default: prefix matching
                values
                    .into_iter()
                    .filter(|v| v.to_lowercase().starts_with(&current_value.to_lowercase()))
                    .collect()
            }
        }

        // CompletionDefinition is automatically implemented via blanket impl

        #[automatically_derived]
        #[async_trait::async_trait]
        impl turul_mcp_server::McpCompletion for #struct_name {
            async fn complete(&self, request: turul_mcp_protocol::completion::CompleteRequest) -> turul_mcp_protocol::McpResult<turul_mcp_protocol::completion::CompleteResult> {
                // Default implementation - this should be overridden by implementing complete_impl
                match self.complete_impl(request).await {
                    Ok(result) => Ok(result),
                    Err(e) => Err(turul_mcp_protocol::McpError::tool_execution(&e)),
                }
            }
        }

        impl #struct_name {
            /// Override this method to provide custom completion logic
            pub async fn complete_impl(&self, request: turul_mcp_protocol::completion::CompleteRequest) -> Result<turul_mcp_protocol::completion::CompleteResult, String> {
                // Default: return sample completions
                let current_value = &request.params.argument.value;
                let completions = vec![
                    format!("{}suggestion1", current_value),
                    format!("{}suggestion2", current_value),
                    format!("{}suggestion3", current_value),
                ];

                let filtered = self.filter_completions(completions, current_value);
                let completion_result = turul_mcp_protocol::completion::CompletionResult::new(filtered)
                    .with_total(3)
                    .with_has_more(false);

                Ok(turul_mcp_protocol::completion::CompleteResult::new(completion_result))
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
    fn test_simple_completion() {
        let input: DeriveInput = parse_quote! {
            #[completion(reference = "ref/prompt", argument = "query")]
            struct CodeCompletion {
                context: String,
                cursor_position: usize,
            }
        };

        let result = derive_mcp_completion_impl(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_resource_completion() {
        let input: DeriveInput = parse_quote! {
            #[completion(reference = "ref/resource")]
            struct FileCompletion {
                directory: String,
            }
        };

        let result = derive_mcp_completion_impl(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_minimal_completion() {
        let input: DeriveInput = parse_quote! {
            struct BasicCompletion;
        };

        let result = derive_mcp_completion_impl(input);
        assert!(result.is_ok());
    }
}
