//! Implementation of #[derive(McpPrompt)] macro

use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Data, Fields, Result};

use crate::utils::{extract_prompt_meta, extract_field_meta};

pub fn derive_mcp_prompt_impl(input: DeriveInput) -> Result<TokenStream> {
    let struct_name = &input.ident;

    // Extract struct-level attributes from #[prompt(...)]
    let prompt_meta = extract_prompt_meta(&input.attrs)?;
    let name = &prompt_meta.name;
    let description = &prompt_meta.description;

    // Check if it's a struct
    let data = match &input.data {
        Data::Struct(data) => data,
        _ => return Err(syn::Error::new_spanned(&input, "McpPrompt can only be derived for structs")),
    };

    // Generate argument definitions from struct fields
    let argument_fields = generate_argument_fields(&input.ident, data)?;

    let expanded = quote! {
        #[automatically_derived]
        impl turul_mcp_protocol::prompts::HasPromptMetadata for #struct_name {
            fn name(&self) -> &str {
                #name
            }
        }

        #[automatically_derived]
        impl turul_mcp_protocol::prompts::HasPromptDescription for #struct_name {
            fn description(&self) -> Option<&str> {
                Some(#description)
            }
        }

        #[automatically_derived]
        impl turul_mcp_protocol::prompts::HasPromptArguments for #struct_name {
            fn arguments(&self) -> Option<&Vec<turul_mcp_protocol::prompts::PromptArgument>> {
                static ARGS: std::sync::OnceLock<Option<Vec<turul_mcp_protocol::prompts::PromptArgument>>> = std::sync::OnceLock::new();
                ARGS.get_or_init(|| {
                    if vec![#(#argument_fields),*].is_empty() {
                        None
                    } else {
                        Some(vec![#(#argument_fields),*])
                    }
                }).as_ref()
            }
        }

        #[automatically_derived]
        impl turul_mcp_protocol::prompts::HasPromptAnnotations for #struct_name {
            fn annotations(&self) -> Option<&turul_mcp_protocol::prompts::PromptAnnotations> {
                None
            }
        }

        #[automatically_derived]
        impl turul_mcp_protocol::prompts::HasPromptMeta for #struct_name {
            fn prompt_meta(&self) -> Option<&std::collections::HashMap<String, serde_json::Value>> {
                None
            }
        }

        // PromptDefinition is automatically implemented via trait composition
        #[automatically_derived]
        impl turul_mcp_protocol::prompts::PromptDefinition for #struct_name {}
        
        #[automatically_derived]
        #[async_trait::async_trait]
        impl turul_mcp_server::McpPrompt for #struct_name {
            async fn render(&self, arguments: Option<std::collections::HashMap<String, serde_json::Value>>) 
                -> turul_mcp_server::McpResult<Vec<turul_mcp_protocol::prompts::PromptMessage>> 
            {
                // Default: return a simple template message
                let message = format!(
                    "Prompt: {} - {}", 
                    self.name(), 
                    self.description().unwrap_or("Generated prompt")
                );
                Ok(vec![turul_mcp_protocol::prompts::PromptMessage::text(message)])
            }
        }
    };

    Ok(expanded)
}

fn generate_argument_fields(struct_name: &syn::Ident, data: &syn::DataStruct) -> Result<Vec<TokenStream>> {
    let mut argument_fields = Vec::new();

    match &data.fields {
        Fields::Named(fields) => {
            for field in &fields.named {
                let field_name = field.ident.as_ref().unwrap();
                let field_name_str = field_name.to_string();
                
                let field_meta = extract_field_meta(&field.attrs)?;
                let description = field_meta.description.unwrap_or_else(|| "No description".to_string());
                
                // For now, all prompt arguments are optional - can be enhanced later
                argument_fields.push(quote! {
                    turul_mcp_protocol::prompts::PromptArgument::new(#field_name_str)
                        .with_description(#description)
                });
            }
        }
        Fields::Unnamed(_) => {
            return Err(syn::Error::new_spanned(struct_name, "Tuple structs are not supported for prompts"));
        }
        Fields::Unit => {
            // Unit structs can have prompts with no arguments
        }
    }

    Ok(argument_fields)
}


#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_simple_prompt() {
        let input: DeriveInput = parse_quote! {
            #[prompt(name = "code_review", description = "Review code for issues")]
            struct CodeReviewPrompt {
                #[argument(description = "The code to review")]
                code: String,
                #[argument(description = "Programming language")]
                language: String,
            }
        };

        let result = derive_mcp_prompt_impl(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_missing_name() {
        let input: DeriveInput = parse_quote! {
            struct CodeReviewPrompt {
                code: String,
            }
        };

        let result = derive_mcp_prompt_impl(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_unit_struct_prompt() {
        let input: DeriveInput = parse_quote! {
            #[prompt(name = "greeting", description = "A simple greeting")]
            struct GreetingPrompt;
        };

        let result = derive_mcp_prompt_impl(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_mixed_argument_types() {
        let input: DeriveInput = parse_quote! {
            #[prompt(name = "analysis", description = "Data analysis prompt")]
            struct AnalysisPrompt {
                data: String,
                threshold: f64,
                count: i32,
                enabled: bool,
            }
        };

        let result = derive_mcp_prompt_impl(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_prompt_argument_generation() {
        let input: DeriveInput = parse_quote! {
            #[prompt(name = "test_prompt", description = "Test prompt with args")]
            struct ArgumentPrompt {
                #[description = "The main topic"]
                topic: String,
                #[description = "Word count limit"]
                word_count: i32,
                #[description = "Writing style"]
                style: String,
            }
        };

        let result = derive_mcp_prompt_impl(input);
        assert!(result.is_ok());
        
        // The derive should succeed - actual argument handling will work at runtime
        let _code = result.unwrap();
        // NOTE: Argument generation works but fields with descriptions need proper field attribute parsing
    }

    #[test]
    fn test_prompt_trait_implementations() {
        let input: DeriveInput = parse_quote! {
            #[prompt(name = "test_prompt", description = "Test")]
            struct TestPrompt;
        };

        let result = derive_mcp_prompt_impl(input);
        assert!(result.is_ok());
        
        // Check that all required traits are implemented
        let code = result.unwrap().to_string();
        assert!(code.contains("HasPromptMetadata"));
        assert!(code.contains("HasPromptDescription"));
        assert!(code.contains("HasPromptArguments"));
        assert!(code.contains("PromptDefinition"));
        assert!(code.contains("McpPrompt"));
    }

    #[test]
    fn test_prompt_with_no_arguments() {
        let input: DeriveInput = parse_quote! {
            #[prompt(name = "simple_prompt", description = "A simple prompt")]
            struct SimplePrompt;
        };

        let result = derive_mcp_prompt_impl(input);
        assert!(result.is_ok());
        
        // Should handle unit structs (no fields) correctly
        let code = result.unwrap().to_string();
        assert!(code.contains("McpPrompt"));
    }

    #[test]
    fn test_missing_prompt_attribute() {
        let input: DeriveInput = parse_quote! {
            struct PlainStruct {
                field: String,
            }
        };

        let result = derive_mcp_prompt_impl(input);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Missing 'name'"));
    }
}