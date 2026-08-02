//! Schema Declarative Macro Implementation
//!
//! Implements the `schema_for!{}` declarative macro for generating JSON schemas
//! from Rust types.

use proc_macro::TokenStream;
use quote::quote;
use syn::{Result, Type};

/// Implementation function for the schema_for!{} declarative macro
pub fn schema_for_impl(input: TokenStream) -> Result<TokenStream> {
    let input = syn::parse::<Type>(input)?;

    let expanded = quote! {
        {
            use turul_mcp_protocol::schema::JsonSchema;
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
