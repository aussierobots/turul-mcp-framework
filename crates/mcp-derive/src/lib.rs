//! # MCP Derive Macros
//!
//! This crate provides procedural macros to simplify creating MCP tools and resources.
//! 
//! ## Features
//!
//! - `#[derive(McpTool)]` - Automatically derive McpTool implementations
//! - `#[mcp_tool]` - Function-style tools with automatic parameter extraction
//! - `#[derive(McpResource)]` - Automatically derive resource handlers
//! - `tool!` - Declarative macro for simple tool creation
//! - `resource!` - Declarative macro for simple resource creation
//! - `schema_for!` - Generate JSON schemas from Rust types
//!
//! ## Code Organization
//!
//! - **Derive Macros**: Implemented in individual modules (tool_derive, resource, json_schema_derive)
//! - **Attribute Macros**: Function-style macros in tool_attr module
//! - **Declarative Macros**: Concise syntax macros in the macros/ module
//! - **Utilities**: Shared functionality in utils module

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput, ItemFn, punctuated::Punctuated, Meta, Token};

mod tool_derive;
mod tool_attr;
mod resource;
mod json_schema_derive;
mod utils;
mod macros;

#[cfg(test)]
mod tests;

/// Derive macro for automatically implementing McpTool
///
/// This macro generates a complete McpTool implementation from a struct definition.
/// 
/// # Attributes
/// 
/// - `#[tool(name = "...", description = "...")]` - Tool metadata
/// - `#[param(description = "...", ...)]` - Parameter descriptions and validation
/// - `#[output_type(StructType)]` - Specify structured output type
/// 
/// # Example
/// 
/// ```rust
/// use mcp_derive::McpTool;
/// use mcp_protocol::McpResult;
/// 
/// #[derive(McpTool, Clone)]
/// #[tool(name = "add", description = "Add two numbers")]
/// struct AddTool {
///     #[param(description = "First number")]
///     a: f64,
///     #[param(description = "Second number")]
///     b: f64,
/// }
/// 
/// impl AddTool {
///     async fn execute(&self) -> McpResult<String> {
///         Ok(format!("{} + {} = {}", self.a, self.b, self.a + self.b))
///     }
/// }
/// ```
#[proc_macro_derive(McpTool, attributes(tool, param, output_type))]
pub fn derive_mcp_tool(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    tool_derive::derive_mcp_tool_impl(input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Function attribute macro for creating MCP tools
/// 
/// This macro converts a regular async function into an MCP tool with automatic
/// parameter extraction and schema generation.
/// 
/// # Example
/// 
/// ```rust
/// use mcp_derive::mcp_tool;
/// use mcp_protocol::McpResult;
/// 
/// #[mcp_tool(name = "multiply", description = "Multiply two numbers")]
/// async fn multiply(
///     #[param(description = "First number")] a: f64,
///     #[param(description = "Second number")] b: f64,
/// ) -> McpResult<String> {
///     Ok(format!("{} × {} = {}", a, b, a * b))
/// }
/// ```
#[proc_macro_attribute]
pub fn mcp_tool(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args with Punctuated::<Meta, Token![,]>::parse_terminated);
    let input = parse_macro_input!(input as ItemFn);
    tool_attr::mcp_tool_impl(args, input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Helper attribute for parameter metadata in function macros
/// 
/// This is used within #[mcp_tool] functions to provide parameter descriptions
/// and constraints.
#[proc_macro_attribute]
pub fn param(_args: TokenStream, input: TokenStream) -> TokenStream {
    // This attribute is only processed by the #[mcp_tool] macro
    // When used alone, it just passes through the input unchanged
    input
}

/// Derive macro for automatically implementing MCP resource handlers
/// 
/// # Example
/// 
/// ```rust
/// use mcp_derive::McpResource;
/// 
/// #[derive(McpResource)]
/// #[uri = "file://config.json"]
/// #[name = "Configuration File"]
/// #[description = "Application configuration file"]
/// struct ConfigResource {
///     #[content]
///     #[content_type = "application/json"]
///     data: String,
/// }
/// ```
#[proc_macro_derive(McpResource, attributes(uri, name, description, content, content_type))]
pub fn derive_mcp_resource(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    resource::mcp_resource_impl(input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// JsonSchema derive macro for generating JSON schema from struct definitions
/// 
/// This macro generates a JSON schema implementation for structs that can be used
/// as output types in MCP tools. It introspects the struct fields and generates
/// the appropriate schema properties and requirements.
/// 
/// # Example
/// 
/// ```rust
/// use mcp_derive::JsonSchema;
/// use serde::{Deserialize, Serialize};
/// 
/// #[derive(JsonSchema, Serialize, Deserialize)]
/// struct CalculationResult {
///     pub operation: String,
///     pub result: f64,
///     pub metadata: CalculationMetadata,
/// }
/// 
/// #[derive(JsonSchema, Serialize, Deserialize)]
/// struct CalculationMetadata {
///     pub precision: String,
///     pub is_exact: bool,
/// }
/// ```
#[proc_macro_derive(JsonSchema)]
pub fn derive_json_schema(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    json_schema_derive::derive_json_schema(input).into()
}

/// Declarative macro for creating simple resources
/// 
/// This provides a concise syntax for resource creation.
/// 
/// # Example
/// 
/// ```rust
/// use mcp_derive::resource;
/// 
/// let config_resource = resource! {
///     uri: "file://config.json",
///     name: "Configuration",
///     description: "Application configuration file",
///     content: |_| async {
///         let config = serde_json::json!({
///             "app_name": "Test App",
///             "version": "1.0.0"
///         });
///         Ok(vec![mcp_protocol::resources::ResourceContent::blob(
///             serde_json::to_string_pretty(&config).unwrap(),
///             "application/json".to_string()
///         )])
///     }
/// };
/// ```
#[proc_macro]
pub fn resource(input: TokenStream) -> TokenStream {
    match macros::resource_declarative_impl(input) {
        Ok(tokens) => tokens,
        Err(err) => err.to_compile_error().into(),
    }
}

/// Generate a JSON schema for a Rust type
/// 
/// This macro generates a JSON schema definition for any Rust type that implements
/// Serialize. It analyzes the type structure and creates appropriate schema constraints.
/// 
/// # Example
/// 
/// ```rust
/// use mcp_derive::schema_for;
/// use serde::{Serialize, Deserialize};
/// 
/// #[derive(Serialize, Deserialize)]
/// struct Point {
///     x: f64,
///     y: f64,
/// }
/// 
/// let schema = schema_for!(Point);
/// ```
#[proc_macro]
pub fn schema_for(input: TokenStream) -> TokenStream {
    match macros::schema_for_impl(input) {
        Ok(tokens) => tokens,
        Err(err) => err.to_compile_error().into(),
    }
}

/// Declarative macro for creating simple tools
/// 
/// This provides the most concise syntax for tool creation.
/// 
/// # Example
/// 
/// ```rust
/// use mcp_derive::tool;
/// 
/// let divide_tool = tool! {
///     name: "divide",
///     description: "Divide two numbers",
///     params: {
///         a: f64 => "Dividend",
///         b: f64 => "Divisor",
///     },
///     execute: |a, b| async move {
///         if b == 0.0 {
///             Err("Division by zero".to_string())
///         } else {
///             Ok(format!("{} ÷ {} = {}", a, b, a / b))
///         }
///     }
/// };
/// ```
#[proc_macro]
pub fn tool(input: TokenStream) -> TokenStream {
    match macros::tool_declarative_impl(input) {
        Ok(tokens) => tokens,
        Err(err) => err.to_compile_error().into(),
    }
}