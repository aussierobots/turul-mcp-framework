use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, Type};

/// Generate a JsonSchema derive macro that introspects struct fields
pub fn derive_json_schema(input: DeriveInput) -> TokenStream {
    let name = &input.ident;
    
    match input.data {
        Data::Struct(data_struct) => {
            let schema_impl = generate_struct_schema(&name, &data_struct.fields);
            
            quote! {
                impl mcp_protocol::schema::JsonSchemaGenerator for #name {
                    fn json_schema() -> mcp_protocol::ToolSchema {
                        #schema_impl
                    }
                }
            }
        }
        _ => {
            syn::Error::new_spanned(&name, "JsonSchema can only be derived for structs")
                .to_compile_error()
        }
    }
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
        assert!(!result.is_empty());
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
                    
                    // For now, assume all fields are required
                    // TODO: Handle Option<T> fields as optional
                    if !is_option_type(&field.ty) {
                        required_fields.push(quote! { #field_name_str.to_string() });
                    }
                }
            }
            
            quote! {
                {
                    use std::collections::HashMap;
                    use mcp_protocol::schema::JsonSchema;
                    
                    // Generate schema for struct #struct_name
                    let mut properties = HashMap::new();
                    #(
                        properties.insert(#properties.0, #properties.1);
                    )*
                    
                    mcp_protocol::ToolSchema {
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
                mcp_protocol::ToolSchema::object()
            }
        }
    }
}

fn generate_field_schema(ty: &Type) -> TokenStream {
    match ty {
        Type::Path(type_path) => {
            if let Some(ident) = type_path.path.get_ident() {
                match ident.to_string().as_str() {
                    "String" => quote! { JsonSchema::string() },
                    "f64" | "f32" => quote! { JsonSchema::number() },
                    "i64" | "i32" | "i16" | "i8" | "u64" | "u32" | "u16" | "u8" | "isize" | "usize" => quote! { JsonSchema::integer() },
                    "bool" => quote! { JsonSchema::boolean() },
                    "Vec" => {
                        // Handle Vec<T> types - extract the inner type if possible
                        if let Some(segment) = type_path.path.segments.first() {
                            if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                                if let Some(syn::GenericArgument::Type(inner_type)) = args.args.first() {
                                    let inner_schema = generate_field_schema(inner_type);
                                    return quote! { JsonSchema::array(#inner_schema) };
                                }
                            }
                        }
                        quote! { JsonSchema::array(JsonSchema::string()) }
                    }
                    // For custom struct types, create a nested object schema
                    _ => quote! { JsonSchema::object() },
                }
            } else {
                // Check if this is a Vec type by examining the path segments
                if type_path.path.segments.len() == 1 {
                    let segment = &type_path.path.segments[0];
                    if segment.ident == "Vec" {
                        if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                            if let Some(syn::GenericArgument::Type(inner_type)) = args.args.first() {
                                let inner_schema = generate_field_schema(inner_type);
                                return quote! { JsonSchema::array(#inner_schema) };
                            }
                        }
                        return quote! { JsonSchema::array(JsonSchema::string()) };
                    }
                }
                quote! { JsonSchema::string() }
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
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.first() {
            return segment.ident == "Option";
        }
    }
    false
}