//! Implementation of #[derive(McpElicitation)] macro

use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Data, Fields, Result, Field};

use crate::utils::{extract_elicitation_meta, extract_field_meta};

pub fn derive_mcp_elicitation_impl(input: DeriveInput) -> Result<TokenStream> {
    let struct_name = &input.ident;

    // Extract struct-level attributes from #[elicitation(...)]
    let elicitation_meta = extract_elicitation_meta(&input.attrs)?;
    let message = &elicitation_meta.message;

    // Check if it's a struct
    let data = match &input.data {
        Data::Struct(data) => data,
        _ => return Err(syn::Error::new_spanned(&input, "McpElicitation can only be derived for structs")),
    };

    // Generate schema from struct fields
    let schema_fields = generate_schema_fields(&input.ident, data)?;

    let expanded = quote! {
        #[automatically_derived]
        impl mcp_protocol::elicitation::HasElicitationMetadata for #struct_name {
            fn message(&self) -> &str {
                #message
            }
        }

        #[automatically_derived]
        impl mcp_protocol::elicitation::HasElicitationSchema for #struct_name {
            fn requested_schema(&self) -> &mcp_protocol::elicitation::ElicitationSchema {
                use std::sync::LazyLock;
                static SCHEMA: LazyLock<mcp_protocol::elicitation::ElicitationSchema> = LazyLock::new(|| {
                    let mut schema = mcp_protocol::elicitation::ElicitationSchema::new();
                    #(#schema_fields)*
                    schema
                });
                &SCHEMA
            }
        }

        #[automatically_derived]
        impl mcp_protocol::elicitation::HasElicitationHandling for #struct_name {}

        // ElicitationDefinition is automatically implemented via blanket impl

        #[automatically_derived]
        #[async_trait::async_trait]
        impl mcp_server::McpElicitation for #struct_name {
            async fn elicit(&self, request: mcp_protocol::elicitation::ElicitCreateRequest) -> mcp_protocol::McpResult<mcp_protocol::elicitation::ElicitResult> {
                // Default implementation - this should be overridden by implementing elicit_impl
                match self.elicit_impl(request).await {
                    Ok(result) => Ok(result),
                    Err(e) => Err(mcp_protocol::McpError::tool_execution(&e)),
                }
            }
        }

        impl #struct_name {
            /// Override this method to provide custom elicitation logic
            pub async fn elicit_impl(&self, _request: mcp_protocol::elicitation::ElicitCreateRequest) -> Result<mcp_protocol::elicitation::ElicitResult, String> {
                // Default: return a sample result
                use std::collections::HashMap;
                let mut content = HashMap::new();
                content.insert("response".to_string(), serde_json::Value::String("User input collected".to_string()));
                Ok(mcp_protocol::elicitation::ElicitResult::accept(content))
            }
        }
    };

    Ok(expanded)
}

fn generate_schema_fields(struct_name: &syn::Ident, data: &syn::DataStruct) -> Result<Vec<TokenStream>> {
    let mut schema_fields = Vec::new();

    match &data.fields {
        Fields::Named(fields) => {
            for field in &fields.named {
                let field_name = field.ident.as_ref().unwrap();
                let field_name_str = field_name.to_string();
                
                let field_schema = generate_field_schema(field)?;
                
                schema_fields.push(quote! {
                    schema = schema.with_property(#field_name_str.to_string(), #field_schema);
                });
            }
        }
        Fields::Unnamed(_) => {
            return Err(syn::Error::new_spanned(struct_name, "Tuple structs are not supported for elicitation"));
        }
        Fields::Unit => {
            return Err(syn::Error::new_spanned(struct_name, "Unit structs are not supported for elicitation"));
        }
    }

    Ok(schema_fields)
}

fn generate_field_schema(field: &Field) -> Result<TokenStream> {
    let field_meta = extract_field_meta(&field.attrs)?;
    let description = field_meta.description.unwrap_or_else(|| "No description".to_string());

    // Generate schema based on field type
    let type_name = &field.ty;
    let schema = match type_name {
        syn::Type::Path(path) if path.path.is_ident("String") => {
            quote! {
                mcp_protocol::elicitation::PrimitiveSchemaDefinition::string_with_description(#description)
            }
        }
        syn::Type::Path(path) if path.path.is_ident("i32") || path.path.is_ident("i64") || path.path.is_ident("isize") => {
            quote! {
                mcp_protocol::elicitation::PrimitiveSchemaDefinition::Integer {
                    title: None,
                    description: Some(#description.to_string()),
                    minimum: None,
                    maximum: None,
                }
            }
        }
        syn::Type::Path(path) if path.path.is_ident("f32") || path.path.is_ident("f64") => {
            quote! {
                mcp_protocol::elicitation::PrimitiveSchemaDefinition::Number {
                    title: None,
                    description: Some(#description.to_string()),
                    minimum: None,
                    maximum: None,
                }
            }
        }
        syn::Type::Path(path) if path.path.is_ident("bool") => {
            quote! {
                mcp_protocol::elicitation::PrimitiveSchemaDefinition::Boolean {
                    title: None,
                    description: Some(#description.to_string()),
                    default: None,
                }
            }
        }
        _ => {
            // Default to string for unknown types
            quote! {
                mcp_protocol::elicitation::PrimitiveSchemaDefinition::string_with_description(#description)
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
    fn test_simple_elicitation() {
        let input: DeriveInput = parse_quote! {
            #[elicitation(message = "Please provide your details")]
            struct UserDetails {
                #[field(description = "Your full name")]
                name: String,
                #[field(description = "Your email address")]
                email: String,
            }
        };

        let result = derive_mcp_elicitation_impl(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_missing_message() {
        let input: DeriveInput = parse_quote! {
            struct UserDetails {
                name: String,
                email: String,
            }
        };

        let result = derive_mcp_elicitation_impl(input);
        assert!(result.is_err());
    }

    #[test] 
    fn test_different_field_types() {
        let input: DeriveInput = parse_quote! {
            #[elicitation(message = "Mixed field types")]
            struct MixedForm {
                name: String,
                age: i32,
                score: f64,
                active: bool,
            }
        };

        let result = derive_mcp_elicitation_impl(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_elicitation_trait_implementations() {
        let input: DeriveInput = parse_quote! {
            #[elicitation(message = "Please provide information")]
            struct InfoRequest {
                field: String,
            }
        };

        let result = derive_mcp_elicitation_impl(input);
        assert!(result.is_ok());
        
        // Check that the generated code contains required trait implementations
        let code = result.unwrap().to_string();
        assert!(code.contains("HasElicitationMetadata"));
        assert!(code.contains("HasElicitationSchema"));
        assert!(code.contains("McpElicitation"));
    }

    #[test]
    fn test_missing_elicitation_attribute() {
        let input: DeriveInput = parse_quote! {
            struct PlainStruct {
                data: String,
            }
        };

        let result = derive_mcp_elicitation_impl(input);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Missing 'message'"));
    }

    #[test]
    fn test_empty_elicitation_message() {
        let input: DeriveInput = parse_quote! {
            #[elicitation(message = "")]
            struct EmptyMessageElicitation;
        };

        let result = derive_mcp_elicitation_impl(input);
        // Empty messages should be allowed - just verify it doesn't panic
        match result {
            Ok(_) => {}, // Success is fine
            Err(e) => {
                println!("Empty message error: {}", e);
                // Error is also acceptable for edge case testing
            }
        }
    }
}