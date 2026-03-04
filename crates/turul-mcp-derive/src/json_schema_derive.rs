use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, Type};

/// Generate a JsonSchema derive macro that introspects struct fields
pub fn derive_json_schema(input: DeriveInput) -> TokenStream {
    let name = &input.ident;

    match input.data {
        Data::Struct(data_struct) => {
            let schema_impl = generate_struct_schema(name, &data_struct.fields);

            quote! {
                impl turul_mcp_protocol::schema::JsonSchemaGenerator for #name {
                    fn json_schema() -> turul_mcp_protocol::ToolSchema {
                        #schema_impl
                    }
                }
            }
        }
        _ => syn::Error::new_spanned(name, "JsonSchema can only be derived for structs")
            .to_compile_error(),
    }
}

fn generate_struct_schema(struct_name: &syn::Ident, fields: &Fields) -> TokenStream {
    let _schema_comment = format!("Schema for {}", struct_name);
    match fields {
        Fields::Named(fields_named) => {
            let mut properties = Vec::new();
            let mut required_fields = Vec::new();

            for field in &fields_named.named {
                if let Some(field_name) = &field.ident {
                    let field_name_str = field_name.to_string();
                    let field_schema = generate_field_schema(&field.ty);

                    properties.push(quote! {
                        (#field_name_str.to_string(), #field_schema)
                    });

                    // Option<T> fields excluded from required (handled by is_option_type)
                    if !is_option_type(&field.ty) {
                        required_fields.push(quote! { #field_name_str.to_string() });
                    }
                }
            }

            quote! {
                {
                    use std::collections::HashMap;
                    use turul_mcp_protocol::schema::JsonSchema;

                    // Generate schema for struct #struct_name
                    let mut properties = HashMap::new();
                    #(
                        properties.insert(#properties.0, #properties.1);
                    )*

                    turul_mcp_protocol::ToolSchema {
                        schema_type: "object".to_string(),
                        properties: Some(properties),
                        required: Some(vec![#(#required_fields),*]),
                        additional: HashMap::new(),
                    }
                }
            }
        }
        _ => {
            quote! {
                turul_mcp_protocol::ToolSchema::object()
            }
        }
    }
}

fn generate_field_schema(ty: &Type) -> TokenStream {
    match ty {
        Type::Path(type_path) => {
            // Use segments.last() to handle both simple (String) and qualified
            // paths (std::option::Option<T>)
            let Some(segment) = type_path.path.segments.last() else {
                return quote! { JsonSchema::string() };
            };

            match segment.ident.to_string().as_str() {
                // Option<T>: unwrap inner type and recurse
                "Option" => {
                    if let syn::PathArguments::AngleBracketed(args) = &segment.arguments
                        && let Some(syn::GenericArgument::Type(inner_type)) = args.args.first()
                    {
                        return generate_field_schema(inner_type);
                    }
                    quote! { JsonSchema::string() }
                }
                "String" => quote! { JsonSchema::string() },
                "f64" | "f32" => quote! { JsonSchema::number() },
                "i64" | "i32" | "i16" | "i8" | "u64" | "u32" | "u16" | "u8" | "isize"
                | "usize" => quote! { JsonSchema::integer() },
                "bool" => quote! { JsonSchema::boolean() },
                "Vec" => {
                    if let syn::PathArguments::AngleBracketed(args) = &segment.arguments
                        && let Some(syn::GenericArgument::Type(inner_type)) = args.args.first()
                    {
                        let inner_schema = generate_field_schema(inner_type);
                        return quote! { JsonSchema::array(#inner_schema) };
                    }
                    quote! { JsonSchema::array(JsonSchema::string()) }
                }
                _ => quote! { JsonSchema::object() },
            }
        }
        Type::Reference(type_ref) => {
            // Handle &str, &String, etc.
            generate_field_schema(&type_ref.elem)
        }
        _ => quote! { JsonSchema::string() },
    }
}

fn is_option_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty
        && let Some(segment) = type_path.path.segments.last()
    {
        return segment.ident == "Option";
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_simple_struct_schema() {
        let input: DeriveInput = parse_quote! {
            struct TestStruct {
                name: String,
                age: i32,
            }
        };

        let result = derive_json_schema(input);
        assert!(!result.is_empty());
    }

    #[test]
    fn test_unit_struct_schema() {
        let input: DeriveInput = parse_quote! {
            struct UnitStruct;
        };

        let result = derive_json_schema(input);
        assert!(!result.is_empty());
    }

    #[test]
    fn test_complex_struct_schema() {
        let input: DeriveInput = parse_quote! {
            struct ComplexStruct {
                title: String,
                count: Option<u32>,
                active: bool,
                tags: Vec<String>,
            }
        };

        let result = derive_json_schema(input);
        let code = result.to_string();
        assert!(!code.is_empty());
        // Option<u32> should produce integer schema, not string
        assert!(code.contains("integer"), "Option<u32> should generate integer schema, got: {}", code);
    }

    #[test]
    fn test_option_type_schemas() {
        // Option<String> → string
        let ty: syn::Type = parse_quote! { Option<String> };
        let schema = generate_field_schema(&ty);
        assert!(schema.to_string().contains("string"), "Option<String> should produce string schema");

        // Option<u32> → integer
        let ty: syn::Type = parse_quote! { Option<u32> };
        let schema = generate_field_schema(&ty);
        assert!(schema.to_string().contains("integer"), "Option<u32> should produce integer schema");

        // Option<f64> → number
        let ty: syn::Type = parse_quote! { Option<f64> };
        let schema = generate_field_schema(&ty);
        assert!(schema.to_string().contains("number"), "Option<f64> should produce number schema");

        // Option<bool> → boolean
        let ty: syn::Type = parse_quote! { Option<bool> };
        let schema = generate_field_schema(&ty);
        assert!(schema.to_string().contains("boolean"), "Option<bool> should produce boolean schema");

        // Option<Vec<String>> → array
        let ty: syn::Type = parse_quote! { Option<Vec<String>> };
        let schema = generate_field_schema(&ty);
        assert!(schema.to_string().contains("array"), "Option<Vec<String>> should produce array schema");
    }

    #[test]
    fn test_qualified_option_type() {
        // std::option::Option<T> should be detected as Option
        let ty: syn::Type = parse_quote! { std::option::Option<i32> };
        assert!(is_option_type(&ty), "std::option::Option<i32> should be detected as Option");

        let schema = generate_field_schema(&ty);
        assert!(schema.to_string().contains("integer"), "std::option::Option<i32> should produce integer schema");
    }

    #[test]
    fn test_enum_should_error() {
        let input: DeriveInput = parse_quote! {
            enum TestEnum {
                Variant1,
                Variant2,
            }
        };

        let result = derive_json_schema(input);
        // Should contain error message about enums not being supported
        let result_string = result.to_string();
        assert!(result_string.contains("JsonSchema can only be derived for structs"));
    }
}
