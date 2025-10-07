//! Framework traits for MCP prompt construction
//!
//! **IMPORTANT**: These are framework features, NOT part of the MCP specification.
//! The MCP specification defines concrete types only.

use std::collections::HashMap;
use serde_json::Value;

// Import protocol types (spec-defined)
use turul_mcp_protocol::Prompt;
use turul_mcp_protocol::prompts::{PromptArgument, PromptAnnotations};

pub trait HasPromptMetadata {
    /// Programmatic identifier (fallback display name)
    fn name(&self) -> &str;

    /// Human-readable display name (UI contexts)
    fn title(&self) -> Option<&str> {
        None
    }
}

/// Prompt description trait
pub trait HasPromptDescription {
    fn description(&self) -> Option<&str> {
        None
    }
}

/// Prompt arguments trait
pub trait HasPromptArguments {
    fn arguments(&self) -> Option<&Vec<PromptArgument>> {
        None
    }
}

/// Prompt annotations trait
pub trait HasPromptAnnotations {
    fn annotations(&self) -> Option<&PromptAnnotations> {
        None
    }
}

/// Prompt-specific meta trait (separate from RPC _meta)
pub trait HasPromptMeta {
    fn prompt_meta(&self) -> Option<&HashMap<String, Value>> {
        None
    }
}

/// Complete prompt definition - composed from fine-grained traits
/// **Complete MCP Prompt Creation** - Build reusable prompt templates that generate contextual content.
///
/// This trait represents a **complete, working MCP prompt** that can be registered with a server
/// and invoked by clients. When you implement the required metadata traits, you automatically
/// get `PromptDefinition` for free via blanket implementation.
///
/// ## What This Enables
///
/// Prompts implementing `PromptDefinition` become **full MCP citizens** that are:
/// - 🔍 **Discoverable** via `prompts/list` requests
/// - 🎯 **Executable** via `prompts/get` requests with arguments
/// - ✅ **Validated** against their argument specifications
/// - 📝 **Template-driven** for consistent, reusable content generation
///
/// ## Complete Working Example
///
/// ```rust
/// use turul_mcp_protocol_2025_06_18::prompts::*;
/// use std::collections::HashMap;
///
/// // This struct will automatically implement PromptDefinition!
/// struct CodeReviewPrompt {
///     arguments: Vec<PromptArgument>,
/// }
///
/// impl CodeReviewPrompt {
///     fn new() -> Self {
///         Self {
///             arguments: vec![
///                 PromptArgument {
///                     name: "language".to_string(),
///                     title: None,
///                     description: Some("Programming language".to_string()),
///                     required: Some(true),
///                 },
///                 PromptArgument {
///                     name: "code".to_string(),
///                     title: None,
///                     description: Some("Source code to review".to_string()),
///                     required: Some(true),
///                 },
///             ],
///         }
///     }
/// }
///
/// impl HasPromptMetadata for CodeReviewPrompt {
///     fn name(&self) -> &str { "code_review" }
///     fn title(&self) -> Option<&str> { Some("AI Code Review") }
/// }
///
/// impl HasPromptDescription for CodeReviewPrompt {
///     fn description(&self) -> Option<&str> {
///         Some("Generate code review comments")
///     }
/// }
///
/// impl HasPromptArguments for CodeReviewPrompt {
///     fn arguments(&self) -> Option<&Vec<PromptArgument>> {
///         Some(&self.arguments)
///     }
/// }
///
/// impl HasPromptAnnotations for CodeReviewPrompt {
///     fn annotations(&self) -> Option<&PromptAnnotations> { None }
/// }
///
/// impl HasPromptMeta for CodeReviewPrompt {
///     fn prompt_meta(&self) -> Option<&HashMap<String, serde_json::Value>> { None }
/// }
///
/// // 🎉 CodeReviewPrompt now automatically implements PromptDefinition!
/// let prompt = CodeReviewPrompt::new();
/// assert_eq!(prompt.name(), "code_review");
/// assert_eq!(prompt.arguments().unwrap().len(), 2);
/// ```
///
/// ## Usage Patterns
///
/// ### Easy: Use Derive Macros (see turul-mcp-derive crate)
/// ```rust
/// // Example of manual implementation without macros
/// use turul_mcp_protocol_2025_06_18::prompts::*;
/// use std::collections::HashMap;
///
/// struct DocumentationPrompt;
///
/// impl HasPromptMetadata for DocumentationPrompt {
///     fn name(&self) -> &str { "doc_generator" }
///     fn title(&self) -> Option<&str> { None }
/// }
///
/// impl HasPromptDescription for DocumentationPrompt {
///     fn description(&self) -> Option<&str> {
///         Some("Generate API documentation")
///     }
/// }
///
/// impl HasPromptArguments for DocumentationPrompt {
///     fn arguments(&self) -> Option<&Vec<PromptArgument>> { None }
/// }
///
/// impl HasPromptAnnotations for DocumentationPrompt {
///     fn annotations(&self) -> Option<&PromptAnnotations> { None }
/// }
///
/// impl HasPromptMeta for DocumentationPrompt {
///     fn prompt_meta(&self) -> Option<&HashMap<String, serde_json::Value>> { None }
/// }
///
/// let prompt = DocumentationPrompt;
/// assert_eq!(prompt.name(), "doc_generator");
/// ```
///
/// ### Advanced: Manual Implementation (shown above)
/// Perfect when you need complex argument validation or dynamic prompt generation.
///
/// ## Real-World Prompt Ideas
///
/// - **Code Prompts**: Code reviews, documentation generation, refactoring suggestions, test creation
/// - **Content Prompts**: Blog post outlines, email templates, meeting summaries, report generation
/// - **Analysis Prompts**: Data insights, security assessments, performance analysis, bug reports
/// - **Learning Prompts**: Tutorial creation, concept explanations, example generation, Q&A formatting
/// - **Workflow Prompts**: Task breakdowns, project planning, requirement analysis, user stories
///
/// ## How It Works in MCP
///
/// 1. **Registration**: Server registers your prompt during startup
/// 2. **Discovery**: Client calls `prompts/list` → sees available prompts with arguments
/// 3. **Parameter Collection**: Client gathers required arguments from user/context
/// 4. **Generation**: Client calls `prompts/get` with prompt name and arguments
/// 5. **Template Processing**: Your prompt generates contextual content
/// 6. **Response**: Framework delivers generated content back to client
///
/// The framework handles argument validation, parameter substitution, and content delivery!
pub trait PromptDefinition:
    HasPromptMetadata +       // name, title (from BaseMetadata)
    HasPromptDescription +    // description
    HasPromptArguments +      // arguments
    HasPromptAnnotations +    // annotations
    HasPromptMeta +           // _meta (prompt-specific)
    Send +
    Sync
{
    /// Display name precedence: title > name (matches TypeScript spec)
    fn display_name(&self) -> &str {
        self.title().unwrap_or_else(|| self.name())
    }

    /// Convert to concrete Prompt struct for protocol serialization
    fn to_prompt(&self) -> Prompt {
        Prompt {
            name: self.name().to_string(),
            title: self.title().map(String::from),
            description: self.description().map(String::from),
            arguments: self.arguments().cloned(),
            meta: self.prompt_meta().cloned(),
        }
    }
}
impl<T> PromptDefinition for T where
    T: HasPromptMetadata
        + HasPromptDescription
        + HasPromptArguments
        + HasPromptAnnotations
        + HasPromptMeta
        + Send
        + Sync
{
}

