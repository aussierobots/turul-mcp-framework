//! Framework traits for MCP sampling construction
//!
//! **IMPORTANT**: These are framework features, NOT part of the MCP specification.

use turul_mcp_protocol::Tool;
use turul_mcp_protocol::sampling::{CreateMessageParams, SamplingMessage, ModelPreferences, Role};
use turul_mcp_protocol::prompts::ContentBlock;
use serde_json::Value;

pub trait HasSamplingMessageMetadata {
    /// Role of the message (from spec)
    fn role(&self) -> &Role;

    /// Content of the message (from spec)
    fn content(&self) -> &ContentBlock;
}

/// Trait for sampling configuration (from CreateMessageRequest spec)
pub trait HasSamplingConfig {
    /// Maximum tokens to generate (required field from spec)
    fn max_tokens(&self) -> u32;

    /// Temperature for sampling (optional from spec)
    fn temperature(&self) -> Option<f64> {
        None
    }

    /// Stop sequences (optional from spec)
    fn stop_sequences(&self) -> Option<&Vec<String>> {
        None
    }
}

/// Trait for sampling context (from CreateMessageRequest spec)
pub trait HasSamplingContext {
    /// Messages for context (required from spec)
    fn messages(&self) -> &[SamplingMessage];

    /// System prompt (optional from spec)
    fn system_prompt(&self) -> Option<&str> {
        None
    }

    /// Include context setting (optional from spec)
    fn include_context(&self) -> Option<&str> {
        None
    }
}

/// Trait for model preferences (from CreateMessageRequest spec)
pub trait HasModelPreferences {
    /// Model preferences (optional from spec)
    fn model_preferences(&self) -> Option<&ModelPreferences> {
        None
    }

    /// Metadata (optional from spec)
    fn metadata(&self) -> Option<&Value> {
        None
    }
}

/// Trait for sampling tools (MCP 2025-11-25)
///
/// Tools that the LLM can use during a sampling request, enabling agentic workflows.
pub trait HasSamplingTools {
    /// Tools available to the LLM during sampling (optional, MCP 2025-11-25)
    fn tools(&self) -> Option<&Vec<Tool>> {
        None
    }
}

/// **Complete MCP Sampling Creation** - Build AI model interaction and completion systems.
///
/// This trait represents a **complete, working MCP sampling configuration** that controls
/// how AI models generate completions with precise parameter control, context management,
/// and model preferences. When you implement the required metadata traits, you automatically
/// get `SamplingDefinition` for free via blanket implementation.
///
/// # What You're Building
///
/// A sampling configuration is an AI interaction system that:
/// - Controls model generation parameters (temperature, tokens, etc.)
/// - Manages conversation context and system prompts
/// - Specifies model preferences and capabilities
/// - Handles structured completion requests
///
/// # How to Create a Sampling Configuration
///
/// Implement these three traits on your struct:
///
/// ```rust
/// # use turul_mcp_protocol::sampling::*;
/// # use turul_mcp_builders::prelude::*;
/// # use serde_json::{Value, json};
/// # use std::collections::HashMap;
///
/// // This struct will automatically implement SamplingDefinition!
/// struct CodeReviewSampling {
///     review_type: String,
///     language: String,
/// }
///
/// impl HasSamplingConfig for CodeReviewSampling {
///     fn max_tokens(&self) -> u32 {
///         2000 // Enough for detailed code reviews
///     }
///
///     fn temperature(&self) -> Option<f64> {
///         Some(0.3) // Lower temperature for consistent code analysis
///     }
///
///     fn stop_sequences(&self) -> Option<&Vec<String>> {
///         None // No special stop sequences needed
///     }
/// }
///
/// impl HasSamplingContext for CodeReviewSampling {
///     fn messages(&self) -> &[SamplingMessage] {
///         // Static messages for this example
///         &[]
///     }
///
///     fn system_prompt(&self) -> Option<&str> {
///         Some("You are an expert code reviewer. Analyze the provided code for bugs, performance issues, and best practices.")
///     }
///
///     fn include_context(&self) -> Option<&str> {
///         Some(&self.language) // Include programming language context
///     }
/// }
///
/// impl HasModelPreferences for CodeReviewSampling {
///     fn model_preferences(&self) -> Option<&ModelPreferences> {
///         None // Use default model
///     }
/// }
///
/// impl HasSamplingTools for CodeReviewSampling {}
///
/// // Now you can use it with the server:
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let sampling = CodeReviewSampling {
///     review_type: "security".to_string(),
///     language: "rust".to_string(),
/// };
///
/// // The sampling automatically implements SamplingDefinition
/// let create_params = sampling.to_create_params();
/// # Ok(())
/// # }
/// ```
///
/// # Key Benefits
///
/// - **Precise Control**: Fine-tune model behavior for specific tasks
/// - **Context Management**: Rich conversation and system prompt support
/// - **Model Flexibility**: Support for different AI models and capabilities
/// - **MCP Compliant**: Fully compatible with MCP 2025-11-25 specification
///
/// # Common Use Cases
///
/// - Code review and analysis systems
/// - Creative writing assistance
/// - Technical documentation generation
/// - Question-answering with domain context
/// - Conversational AI with specific personalities
pub trait SamplingDefinition: HasSamplingConfig + HasSamplingContext + HasModelPreferences + HasSamplingTools {
    /// Convert to CreateMessageParams
    fn to_create_params(&self) -> CreateMessageParams {
        CreateMessageParams {
            messages: self.messages().to_vec(),
            model_preferences: self.model_preferences().cloned(),
            system_prompt: self.system_prompt().map(|s| s.to_string()),
            include_context: self.include_context().map(|s| s.to_string()),
            temperature: self.temperature(),
            max_tokens: self.max_tokens(),
            stop_sequences: self.stop_sequences().cloned(),
            metadata: self.metadata().cloned(),
            tools: self.tools().cloned(),
            tool_choice: None,
            meta: None,
        }
    }
}

// Blanket implementation: any type implementing the fine-grained traits automatically gets SamplingDefinition
impl<T> SamplingDefinition for T where
    T: HasSamplingConfig + HasSamplingContext + HasModelPreferences + HasSamplingTools
{
}
