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
//! ## Quick Start
//!
//! ```rust,no_run
//! use turul_mcp_derive::{McpTool, McpResource, mcp_tool};
//!
//! // Derive macros and function attributes available
//! ```
//!
//! ## Code Organization
//!
//! - **Derive Macros**: Implemented in individual modules (tool_derive, resource_derive, json_schema_derive)
//! - **Attribute Macros**: Function-style macros in tool_attr module
//! - **Declarative Macros**: Concise syntax macros in the macros/ module
//! - **Utilities**: Shared functionality in utils module

use proc_macro::TokenStream;
use syn::{DeriveInput, ItemFn, Meta, Token, parse_macro_input, punctuated::Punctuated};

mod completion_derive;
mod elicitation_derive;
mod json_schema_derive;
mod logging_derive;
mod macros;
mod notification_derive;
mod prompt_derive;
mod resource_attr;
mod resource_derive;
mod roots_derive;
mod sampling_derive;
mod tool_attr;
mod tool_derive;
mod utils;

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
/// ```rust,ignore
/// use turul_mcp_derive::McpTool;
/// use turul_mcp_protocol::McpResult;
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
/// ```rust,ignore
/// use turul_mcp_derive::mcp_tool;
/// use turul_mcp_protocol::McpResult;
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

/// Function attribute macro for creating MCP resources
///
/// This macro automatically generates an MCP resource implementation from an async function.
/// It supports both static and template resources based on the URI pattern.
///
/// # Example
///
/// ```rust,ignore
/// use turul_mcp_derive::mcp_resource;
/// use turul_mcp_protocol::resources::ResourceContent;
/// use turul_mcp_server::McpResult;
///
/// #[mcp_resource(uri = "file:///asx/timeline/{ticker}.json", description = "Timeline for ticker")]
/// async fn ticker_timeline(ticker: String) -> McpResult<Vec<ResourceContent>> {
///     // Implementation
///     Ok(vec![ResourceContent::text(
///         format!("file:///asx/timeline/{}.json", ticker),
///         format!("Timeline data for {}", ticker)
///     )])
/// }
/// ```
#[proc_macro_attribute]
pub fn mcp_resource(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args with Punctuated::<Meta, Token![,]>::parse_terminated);
    let input = parse_macro_input!(input as ItemFn);
    resource_attr::mcp_resource_impl(args, input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Derive macro for automatically implementing MCP resource handlers
///
/// # Example
///
/// ```rust,ignore
/// use turul_mcp_derive::McpResource;
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
#[proc_macro_derive(
    McpResource,
    attributes(resource, uri, name, description, content, content_type)
)]
pub fn derive_mcp_resource(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    resource_derive::derive_mcp_resource_impl(input)
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
/// ```rust,ignore
/// use turul_mcp_derive::JsonSchema;
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
/// ```rust,ignore
/// use turul_mcp_derive::resource;
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
///         Ok(vec![turul_mcp_protocol::resources::ResourceContent::blob(
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
/// ```rust,ignore
/// use turul_mcp_derive::schema_for;
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

/// Derive macro for automatically implementing McpElicitation
///
/// # Example
///
/// ```rust,ignore
/// use turul_mcp_derive::McpElicitation;
///
/// #[derive(McpElicitation)]
/// #[elicitation(message = "Please enter your details")]
/// struct UserDetailsElicitation {
///     name: String,
///     email: String,
/// }
/// ```
#[proc_macro_derive(McpElicitation, attributes(elicitation, field))]
pub fn derive_mcp_elicitation(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    elicitation_derive::derive_mcp_elicitation_impl(input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Derive macro for automatically implementing McpPrompt
///
/// # Example
///
/// ```rust,ignore
/// use turul_mcp_derive::McpPrompt;
///
/// #[derive(McpPrompt)]
/// #[prompt(name = "code_review", description = "Review code")]
/// struct CodeReviewPrompt {
///     code: String,
///     language: String,
/// }
/// ```
#[proc_macro_derive(McpPrompt, attributes(prompt, argument))]
pub fn derive_mcp_prompt(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    prompt_derive::derive_mcp_prompt_impl(input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Derive macro for automatically implementing McpSampling
///
/// # Example
///
/// ```rust,ignore
/// use turul_mcp_derive::McpSampling;
///
/// #[derive(McpSampling)]
/// #[sampling(model = "claude-3-haiku", temperature = 0.7)]
/// struct TextGenerationSampling {
///     prompt: String,
///     max_tokens: u32,
/// }
/// ```
#[proc_macro_derive(McpSampling, attributes(sampling, config))]
pub fn derive_mcp_sampling(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    sampling_derive::derive_mcp_sampling_impl(input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Derive macro for automatically implementing McpCompletion
///
/// # Example
///
/// ```rust,ignore
/// use turul_mcp_derive::McpCompletion;
///
/// #[derive(McpCompletion)]
/// #[completion(reference = "prompt://code_assist")]
/// struct CodeCompletionProvider {
///     context: String,
///     cursor_position: usize,
/// }
/// ```
#[proc_macro_derive(McpCompletion, attributes(completion, reference))]
pub fn derive_mcp_completion(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    completion_derive::derive_mcp_completion_impl(input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Derive macro for automatically implementing McpLogger
///
/// # Example
///
/// ```rust,ignore
/// use turul_mcp_derive::McpLogger;
///
/// #[derive(McpLogger)]
/// #[logger(name = "app_logger", level = "info")]
/// struct ApplicationLogger {
///     format: String,
///     output_path: Option<String>,
/// }
/// ```
#[proc_macro_derive(McpLogger, attributes(logger, level))]
pub fn derive_mcp_logger(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    logging_derive::derive_mcp_logger_impl(input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Derive macro for automatically implementing McpRoot
///
/// # Example
///
/// ```rust,ignore
/// use turul_mcp_derive::McpRoot;
///
/// #[derive(McpRoot)]
/// #[root(uri = "file:///home/user/project", name = "Project Root")]
/// struct ProjectRoot {
///     path: String,
///     read_only: bool,
/// }
/// ```
#[proc_macro_derive(McpRoot, attributes(root, permission))]
pub fn derive_mcp_root(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    roots_derive::derive_mcp_root_impl(input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Derive macro for automatically implementing McpNotification
///
/// ZERO CONFIGURATION - Framework auto-determines method from struct name for MCP spec notifications:
/// - `ProgressNotification` → `"notifications/progress"`
/// - `LoggingMessageNotification` → `"notifications/logging/message"`
/// - `ResourceUpdatedNotification` → `"notifications/resources/updated"`
/// - `ResourceListChangedNotification` → `"notifications/resources/list_changed"`
/// - `ToolListChangedNotification` → `"notifications/tools/list_changed"`
///
/// # Example
///
/// ```rust,ignore
/// use turul_mcp_derive::McpNotification;
///
/// #[derive(McpNotification, Default)]
/// struct ProgressNotification {
///     progress_token: String,
///     progress: u64,
///     total: Option<u64>,
///     message: Option<String>,
/// }
/// ```
#[proc_macro_derive(McpNotification, attributes(notification, payload))]
pub fn derive_mcp_notification(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    notification_derive::derive_mcp_notification_impl(input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Declarative macro for creating simple tools
///
/// This provides the most concise syntax for tool creation.
///
/// # Example
///
/// ```rust,ignore
/// use turul_mcp_derive::tool;
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

/// Declarative macro for creating simple prompts
///
/// This provides a concise syntax for prompt creation.
///
/// # Example
///
/// ```rust,ignore
/// use turul_mcp_derive::prompt;
///
/// let code_review_prompt = prompt! {
///     name: "code_review",
///     description: "Review code for quality and best practices",
///     arguments: {
///         code: String => "Code to review",
///         language: String => "Programming language", required,
///     },
///     template: |args| async move {
///         let code = args.get("code").and_then(|v| v.as_str()).unwrap_or("");
///         let lang = args.get("language").and_then(|v| v.as_str()).unwrap_or("text");
///
///         Ok(vec![
///             turul_mcp_protocol::prompts::PromptMessage::user(format!(
///                 "Please review this {} code for quality, security, and best practices:\n\n```{}\n{}\n```",
///                 lang, lang, code
///             ))
///         ])
///     }
/// };
/// ```
#[proc_macro]
pub fn prompt(input: TokenStream) -> TokenStream {
    match macros::prompt_declarative_impl(input) {
        Ok(tokens) => tokens,
        Err(err) => err.to_compile_error().into(),
    }
}

/// Declarative macro for creating sampling configurations
///
/// This provides a concise syntax for sampling configuration.
///
/// # Example
///
/// ```rust,ignore
/// use turul_mcp_derive::sampling;
///
/// let text_generator = sampling! {
///     max_tokens: 1000,
///     temperature: 0.7,
///     system_prompt: "You are a helpful AI assistant",
///     handler: |request| async move {
///         // Implementation would call actual model API
///         let response_text = "Generated response based on the input";
///         Ok(turul_mcp_protocol::sampling::CreateMessageResult::new(
///             turul_mcp_protocol::sampling::SamplingMessage {
///                 role: "assistant".to_string(),
///                 content: turul_mcp_protocol::sampling::MessageContent::Text {
///                     text: response_text.to_string()
///                 }
///             },
///             "claude-3-haiku"
///         ))
///     }
/// };
/// ```
#[proc_macro]
pub fn sampling(input: TokenStream) -> TokenStream {
    match macros::sampling_declarative_impl(input) {
        Ok(tokens) => tokens,
        Err(err) => err.to_compile_error().into(),
    }
}

/// Declarative macro for creating MCP notifications with concise syntax.
///
/// Supports zero-configuration method generation based on struct name.
///
/// # Example
///
/// ```rust,ignore
/// use turul_mcp_derive::notification;
///
/// notification! {
///     progress {
///         message: String = "Progress message",
///         percent: u32 = "Completion percentage"
///     }
/// };
/// ```
#[proc_macro]
pub fn notification(input: TokenStream) -> TokenStream {
    match macros::notification_declarative_impl(input) {
        Ok(tokens) => tokens,
        Err(err) => err.to_compile_error().into(),
    }
}

/// Declarative macro for creating MCP completion handlers with concise syntax.
///
/// # Example
///
/// ```rust,ignore
/// use turul_mcp_derive::completion;
///
/// completion! {
///     text_editor {
///         context: String = "Editor context",
///         cursor_position: u32 = "Cursor position"
///     }
/// };
/// ```
#[proc_macro]
pub fn completion(input: TokenStream) -> TokenStream {
    match macros::completion_declarative_impl(input) {
        Ok(tokens) => tokens,
        Err(err) => err.to_compile_error().into(),
    }
}

/// Declarative macro for creating MCP elicitation handlers with concise syntax.
///
/// # Example
///
/// ```rust,ignore
/// use turul_mcp_derive::elicitation;
///
/// elicitation! {
///     user_details, "Please provide your information" {
///         name: String = "Full name",
///         email: String = "Email address"
///     }
/// };
/// ```
#[proc_macro]
pub fn elicitation(input: TokenStream) -> TokenStream {
    match macros::elicitation_declarative_impl(input) {
        Ok(tokens) => tokens,
        Err(err) => err.to_compile_error().into(),
    }
}

/// Declarative macro for creating MCP root handlers with concise syntax.
///
/// # Example
///
/// ```rust,ignore
/// use turul_mcp_derive::roots;
///
/// roots! {
///     project, "/path/to/project", name = "Project Files", read_only = false
/// };
/// ```
#[proc_macro]
pub fn roots(input: TokenStream) -> TokenStream {
    match macros::roots_declarative_impl(input) {
        Ok(tokens) => tokens,
        Err(err) => err.to_compile_error().into(),
    }
}

/// Declarative macro for creating MCP logging handlers with concise syntax.
///
/// # Example
///
/// ```rust,ignore
/// use turul_mcp_derive::logging;
///
/// logging! {
///     file_logger {
///         log_level: String = "Logging level",
///         file_path: String = "Log file path"
///     }
/// };
/// ```
#[proc_macro]
pub fn logging(input: TokenStream) -> TokenStream {
    match macros::logging_declarative_impl(input) {
        Ok(tokens) => tokens,
        Err(err) => err.to_compile_error().into(),
    }
}
