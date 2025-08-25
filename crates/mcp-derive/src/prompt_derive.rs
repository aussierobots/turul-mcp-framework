//! Implementation of #[derive(McpPrompt)] macro

use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Data, Fields, Result, Field};

use crate::utils::{extract_string_attribute, extract_field_meta};

pub fn derive_mcp_prompt_impl(input: DeriveInput) -> Result<TokenStream> {
    let struct_name = &input.ident;

    // Extract struct-level attributes
    let name = extract_string_attribute(&input.attrs, "name")
        .ok_or_else(|| syn::Error::new_spanned(&input, "McpPrompt derive requires #[prompt(name = \"...\")] attribute"))?;
    
    let description = extract_string_attribute(&input.attrs, "description")
        .unwrap_or_else(|| "Generated prompt".to_string());

    // Check if it's a struct
    let data = match &input.data {
        Data::Struct(data) => data,
        _ => return Err(syn::Error::new_spanned(&input, "McpPrompt can only be derived for structs")),
    };

    // Generate argument schema from struct fields
    let argument_fields = generate_argument_fields(&input.ident, data)?;

    let expanded = quote! {
        #[automatically_derived]
        impl mcp_protocol::prompts::HasPromptMetadata for #struct_name {
            fn name(&self) -> &str {
                #name
            }

            fn description(&self) -> Option<&str> {
                Some(#description)
            }
        }

        #[automatically_derived]
        impl mcp_protocol::prompts::HasPromptArguments for #struct_name {
            fn input_schema(&self) -> &mcp_protocol::schema::JsonSchema {
                use std::sync::LazyLock;
                static SCHEMA: LazyLock<mcp_protocol::schema::JsonSchema> = LazyLock::new(|| {
                    use std::collections::HashMap;
                    let mut properties = HashMap::new();
                    #(#argument_fields)*
                    
                    mcp_protocol::schema::JsonSchema::Object {
                        title: None,
                        description: Some(#description.to_string()),
                        properties,
                        required: Vec::new(),
                        additional_properties: Some(Box::new(mcp_protocol::schema::JsonSchema::Boolean(false))),
                    }
                });
                &SCHEMA
            }
        }

        #[automatically_derived]
        impl mcp_protocol::prompts::HasPromptMessages for #struct_name {
            fn messages(&self) -> Vec<mcp_protocol::prompts::Message> {
                // Default: create a simple user message template
                vec![
                    mcp_protocol::prompts::Message::user(
                        mcp_protocol::prompts::MessageContent::text("{{prompt_content}}")
                    )
                ]
            }
        }

        #[automatically_derived]
        impl mcp_protocol::prompts::HasPromptAnnotations for #struct_name {
            fn annotations(&self) -> Option<&std::collections::HashMap<String, serde_json::Value>> {
                None
            }
        }

        // PromptDefinition is automatically implemented via blanket impl

        #[automatically_derived]
        #[async_trait::async_trait]
        impl mcp_server::McpPrompt for #struct_name {
            async fn render(&self, args: serde_json::Value) -> mcp_protocol::McpResult<mcp_protocol::prompts::GetPromptResult> {
                // Default implementation - this should be overridden by implementing render_impl
                match self.render_impl(args).await {
                    Ok(result) => Ok(result),
                    Err(e) => Err(mcp_protocol::McpError::prompt(&e)),
                }
            }
        }

        impl #struct_name {
            /// Override this method to provide custom prompt rendering logic
            pub async fn render_impl(&self, _args: serde_json::Value) -> Result<mcp_protocol::prompts::GetPromptResult, String> {
                // Default: return template messages
                let messages = self.messages();
                Ok(mcp_protocol::prompts::GetPromptResult::new(
                    #description.to_string(),
                    messages
                ))
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
                
                let field_schema = generate_argument_schema(field)?;
                
                argument_fields.push(quote! {
                    properties.insert(#field_name_str.to_string(), #field_schema);
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

fn generate_argument_schema(field: &Field) -> Result<TokenStream> {
    let field_meta = extract_field_meta(&field.attrs)?;
    let description = field_meta.description.unwrap_or_else(|| "No description".to_string());

    // Generate schema based on field type
    let type_name = &field.ty;
    let schema = match type_name {
        syn::Type::Path(path) if path.path.is_ident("String") => {
            quote! {
                mcp_protocol::schema::JsonSchema::String {
                    title: None,
                    description: Some(#description.to_string()),
                    default: None,
                    examples: None,
                }
            }
        }
        syn::Type::Path(path) if path.path.is_ident("i32") || path.path.is_ident("i64") || path.path.is_ident("isize") => {
            quote! {
                mcp_protocol::schema::JsonSchema::Integer {
                    title: None,
                    description: Some(#description.to_string()),
                    default: None,
                    examples: None,
                }
            }
        }
        syn::Type::Path(path) if path.path.is_ident("f32") || path.path.is_ident("f64") => {
            quote! {
                mcp_protocol::schema::JsonSchema::Number {
                    title: None,
                    description: Some(#description.to_string()),
                    default: None,
                    examples: None,
                }
            }
        }
        syn::Type::Path(path) if path.path.is_ident("bool") => {
            quote! {
                mcp_protocol::schema::JsonSchema::Boolean {
                    title: None,
                    description: Some(#description.to_string()),
                    default: None,
                }
            }
        }
        _ => {
            // Default to string for unknown types
            quote! {
                mcp_protocol::schema::JsonSchema::String {
                    title: None,
                    description: Some(#description.to_string()),
                    default: None,
                    examples: None,
                }
            }
        }
    };

    Ok(schema)
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
}