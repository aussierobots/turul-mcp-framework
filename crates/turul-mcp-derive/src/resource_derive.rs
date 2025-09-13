//! Implementation of #[derive(McpResource)] macro

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, Result};

use crate::utils::{extract_field_meta, extract_resource_meta};

pub fn derive_mcp_resource_impl(input: DeriveInput) -> Result<TokenStream> {
    let struct_name = &input.ident;
    let _vis = &input.vis;

    // Extract struct-level attributes from #[resource(...)]
    let resource_meta = extract_resource_meta(&input.attrs)?;
    let uri = &resource_meta.uri;
    let name = &resource_meta.name;
    let description = &resource_meta.description;

    // Check if it's a struct
    let data = match &input.data {
        Data::Struct(data) => data,
        _ => {
            return Err(syn::Error::new_spanned(
                &input,
                "McpResource can only be derived for structs",
            ));
        }
    };

    // Generate read method based on struct fields
    let read_impl = generate_read_method(data)?;

    let expanded = quote! {
        #[automatically_derived]
        impl turul_mcp_protocol::resources::HasResourceMetadata for #struct_name {
            fn name(&self) -> &str {
                #name
            }

            fn title(&self) -> Option<&str> {
                None  // TODO: Support title from attributes
            }
        }

        #[automatically_derived]
        impl turul_mcp_protocol::resources::HasResourceDescription for #struct_name {
            fn description(&self) -> Option<&str> {
                Some(#description)
            }
        }

        #[automatically_derived]
        impl turul_mcp_protocol::resources::HasResourceUri for #struct_name {
            fn uri(&self) -> &str {
                #uri
            }
        }

        #[automatically_derived]
        impl turul_mcp_protocol::resources::HasResourceMimeType for #struct_name {
            fn mime_type(&self) -> Option<&str> {
                None  // TODO: Support mime_type from attributes
            }
        }

        #[automatically_derived]
        impl turul_mcp_protocol::resources::HasResourceSize for #struct_name {
            fn size(&self) -> Option<u64> {
                None
            }
        }

        #[automatically_derived]
        impl turul_mcp_protocol::resources::HasResourceAnnotations for #struct_name {
            fn annotations(&self) -> Option<&turul_mcp_protocol::meta::Annotations> {
                None
            }
        }

        #[automatically_derived]
        impl turul_mcp_protocol::resources::HasResourceMeta for #struct_name {
            fn resource_meta(&self) -> Option<&std::collections::HashMap<String, serde_json::Value>> {
                None
            }
        }

        // ResourceDefinition automatically implemented via blanket impl in resources.rs
        // No need for explicit impl - blanket impl handles it

        #[automatically_derived]
        #[async_trait::async_trait]
        impl turul_mcp_server::McpResource for #struct_name {
            async fn read(&self, _params: Option<serde_json::Value>) -> turul_mcp_server::McpResult<Vec<turul_mcp_protocol::resources::ResourceContent>> {
                #read_impl
            }
        }
    };

    Ok(expanded)
}

fn generate_read_method(data: &syn::DataStruct) -> Result<TokenStream> {
    match &data.fields {
        Fields::Named(fields) => {
            let mut content_parts = Vec::new();

            for field in &fields.named {
                let field_name = field.ident.as_ref().unwrap();
                let field_meta = extract_field_meta(&field.attrs)?;

                if field_meta.content_type.is_some() || field_meta.content.unwrap_or(false) {
                    if let Some(content_type) = field_meta.content_type {
                        // Use blob() for content with specific MIME types
                        content_parts.push(quote! {
                            turul_mcp_protocol::resources::ResourceContent::blob(
                                self.uri().to_string(),
                                self.#field_name.to_string(),
                                #content_type.to_string()
                            )
                        });
                    } else {
                        // Use text() for plain text content
                        content_parts.push(quote! {
                            turul_mcp_protocol::resources::ResourceContent::text(
                                self.uri().to_string(),
                                self.#field_name.to_string()
                            )
                        });
                    }
                }
            }

            if content_parts.is_empty() {
                // Default implementation - serialize entire struct as JSON
                Ok(quote! {
                    let json_content = serde_json::to_string_pretty(self)
                        .map_err(|e| turul_mcp_protocol::McpError::resource_execution(&format!("Failed to serialize resource: {}", e)))?;
                    Ok(vec![
                        turul_mcp_protocol::resources::ResourceContent::blob(
                            self.uri().to_string(),
                            json_content,
                            "application/json".to_string()
                        )
                    ])
                })
            } else {
                Ok(quote! {
                    Ok(vec![
                        #(#content_parts),*
                    ])
                })
            }
        }
        Fields::Unnamed(_) => {
            // For tuple structs, use first field as content
            Ok(quote! {
                let content = self.0.to_string();
                Ok(vec![
                    turul_mcp_protocol::resources::ResourceContent::text(
                        self.uri(),
                        content
                    )
                ])
            })
        }
        Fields::Unit => {
            // For unit structs, return empty content
            Ok(quote! {
                Ok(vec![
                    turul_mcp_protocol::resources::ResourceContent::text(
                        self.uri(),
                        "Empty resource".to_string()
                    )
                ])
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_simple_resource() {
        let input: DeriveInput = parse_quote! {
            #[uri = "file://test.txt"]
            #[name = "Test File"]
            #[description = "A test file resource"]
            struct TestResource {
                #[content]
                data: String,
            }
        };

        let result = derive_mcp_resource_impl(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_missing_attributes() {
        let input: DeriveInput = parse_quote! {
            struct TestResource {
                data: String,
            }
        };

        let result = derive_mcp_resource_impl(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_unit_struct() {
        let input: DeriveInput = parse_quote! {
            #[uri = "system://status"]
            #[name = "System Status"]
            #[description = "Current system status"]
            struct SystemStatus;
        };

        let result = derive_mcp_resource_impl(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_tuple_struct() {
        let input: DeriveInput = parse_quote! {
            #[uri = "data://message"]
            #[name = "Message"]
            #[description = "A simple message"]
            struct Message(String);
        };

        let result = derive_mcp_resource_impl(input);
        assert!(result.is_ok());
    }
}
