//! Runtime builders for MCP (Model Context Protocol) components
//!
//! This crate provides builder patterns for all MCP areas, enabling runtime construction
//! of tools, resources, prompts, and other protocol components. This is "Level 3" of the
//! MCP creation spectrum - runtime flexibility for dynamic/configuration-driven systems.
//!
//! # Quick Start
//!
//! ```rust
//! use turul_mcp_builders::prelude::*;
//!
//! // All builders and common types available
//! ```
//!
//! # Features
//! - Runtime tool construction with `ToolBuilder`
//! - Dynamic resource creation with `ResourceBuilder`
//! - Prompt template building with `PromptBuilder`
//! - Message composition with `MessageBuilder`
//! - Completion context building with `CompletionBuilder`
//! - Root directory configuration with `RootBuilder`
//! - User input collection with `ElicitationBuilder`
//! - Notification creation with `NotificationBuilder`
//! - Logging message construction with `LoggingBuilder`
//!
//! # Example
//! ```rust
//! use turul_mcp_builders::{
//!     ToolBuilder, ResourceBuilder, PromptBuilder, MessageBuilder,
//!     CompletionBuilder, RootBuilder, ElicitationBuilder,
//!     NotificationBuilder, LoggingBuilder
//! };
//! use serde_json::json;
//! use std::collections::HashMap;
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a calculator tool at runtime
//! let tool = ToolBuilder::new("calculator")
//!     .description("Add two numbers")
//!     .number_param("a", "First number")
//!     .number_param("b", "Second number")
//!     .execute(|args| async move {
//!         let a = args.get("a").and_then(|v| v.as_f64()).ok_or("Missing 'a'")?;
//!         let b = args.get("b").and_then(|v| v.as_f64()).ok_or("Missing 'b'")?;
//!         Ok(json!({"result": a + b}))
//!     })
//!     .build()?;
//!
//! // Create a resource at runtime
//! let resource = ResourceBuilder::new("file:///config.json")
//!     .name("app_config")
//!     .description("Application configuration file")
//!     .json_content(json!({"version": "1.0", "debug": false}))
//!     .build()?;
//!
//! // Create a prompt template at runtime
//! let prompt = PromptBuilder::new("greeting")
//!     .description("Generate personalized greetings")
//!     .string_argument("name", "Person to greet")
//!     .user_message("Hello {name}! How are you today?")
//!     .assistant_message("Nice to meet you!")
//!     .build()?;
//!
//! // Execute the tool
//! let result = tool.execute(json!({"a": 5.0, "b": 3.0})).await?;
//! assert_eq!(result, json!({"result": 8.0}));
//!
//! // Read the resource
//! let content = resource.read().await?;
//! // content is ResourceContent::Text with JSON data
//!
//! // Get the prompt with arguments
//! let mut args = HashMap::new();
//! args.insert("name".to_string(), "Alice".to_string());
//! let prompt_result = prompt.get(args).await?;
//! // prompt_result contains processed messages with "Alice" substituted
//!
//! // Additional builders available:
//!
//! // Create sampling messages for LLM interaction
//! let message_request = MessageBuilder::new()
//!     .max_tokens(500)
//!     .temperature(0.7)
//!     .user_text("Explain quantum computing")
//!     .build_request();
//!
//! // Create completion requests for autocomplete
//! let completion = CompletionBuilder::prompt_argument("greeting", "name")
//!     .context_argument("user_id", "123")
//!     .build();
//!
//! // Create root directory configurations
//! let source_root = RootBuilder::source_code_root("/home/user/project")
//!     .name("My Project")
//!     .build()?;
//!
//! // Create elicitation forms for user input
//! let form_request = ElicitationBuilder::new("Please enter your details")
//!     .string_field("name", "Your full name")
//!     .integer_field_with_range("age", "Your age", Some(0.0), Some(120.0))
//!     .boolean_field("subscribe", "Subscribe to newsletter")
//!     .require_fields(vec!["name".to_string(), "age".to_string()])
//!     .build();
//!
//! // Create notifications for server events
//! let progress_notification = NotificationBuilder::progress("task-123", 75)
//!     .total(100)
//!     .message("Processing files...")
//!     .build();
//!
//! let resource_updated = NotificationBuilder::resource_updated("file:///data.json")
//!     .meta_value("change_type", json!("modified"))
//!     .build();
//!
//! // Create logging messages
//! let error_log = LoggingBuilder::error(json!({"error": "Connection failed", "retry_count": 3}))
//!     .logger("database")
//!     .meta_value("session_id", json!("sess-456"))
//!     .build();
//!
//! let performance_log = LoggingBuilder::structured(
//!     turul_mcp_protocol::logging::LoggingLevel::Info,
//!     [("operation", json!("query_execution")), ("duration_ms", json!(142.5))]
//!         .into_iter().map(|(k, v)| (k.to_string(), v)).collect()
//! ).logger("perf-monitor").build();
//! # Ok(())
//! # }
//! ```

pub mod prelude;
pub mod traits;
pub mod protocol_impls;

pub mod completion;
pub mod elicitation;
pub mod logging;
pub mod message;
pub mod notification;
pub mod prompt;
pub mod resource;
pub mod root;
pub mod tool;

// Schemars integration for JSON schema generation
pub mod schemars_helpers;
pub use schemars_helpers::{ToolSchemaExt, convert_value_to_json_schema, convert_value_to_json_schema_with_defs};

// Re-export schemars for use in generated code
// This ensures schemars::schema_for!() works in user code without requiring
// users to add schemars as an explicit dependency
pub use schemars;

// Re-export all framework traits for convenience
pub use traits::*;

// Re-export all builders for convenience
/// Builder for completion provider configuration with sampling parameters
pub use completion::CompletionBuilder;
/// Builders for interactive data collection and result formatting
pub use elicitation::{ElicitResultBuilder, ElicitationBuilder};
/// Builders for structured logging messages and level configuration
pub use logging::{LoggingBuilder, SetLevelBuilder};
/// Builder for constructing prompt messages with role and content
pub use message::MessageBuilder;
/// Builders for various MCP notification types (progress, cancellation, resource updates)
pub use notification::{
    CancelledNotificationBuilder, NotificationBuilder, ProgressNotificationBuilder,
    ResourceUpdatedNotificationBuilder,
};
/// Builder for prompt templates with arguments and message composition
pub use prompt::PromptBuilder;
/// Builder for resource definitions with content and metadata
pub use resource::ResourceBuilder;
/// Builders for root directory listings and workspace notifications
pub use root::{ListRootsRequestBuilder, RootBuilder, RootsNotificationBuilder};
/// Builder for executable tool definitions with parameters and handlers
pub use tool::ToolBuilder;

// Common types used across builders
/// JSON value type for dynamic content construction
pub use serde_json::{Value, json};
/// Hash map type for parameter collections and metadata
pub use std::collections::HashMap;
