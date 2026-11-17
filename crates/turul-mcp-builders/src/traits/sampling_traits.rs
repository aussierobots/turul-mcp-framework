//! Defines framework traits for constructing MCP sampling requests.
//!
//! **IMPORTANT**: These are framework features, NOT part of the MCP specification.

use serde_json::Value;
use turul_mcp_protocol::Tool;
use turul_mcp_protocol::prompts::ContentBlock;
use turul_mcp_protocol::sampling::{CreateMessageParams, ModelPreferences, Role, SamplingMessage};

pub trait HasSamplingMessageMetadata {
    /// Returns the message role.
    fn role(&self) -> &Role;

    /// Returns the message content.
    fn content(&self) -> &ContentBlock;
}

/// Provides sampling configuration for a `CreateMessageRequest`.
pub trait HasSamplingConfig {
    /// Returns the maximum tokens to generate.
    fn max_tokens(&self) -> u32;

    /// Returns the sampling temperature, if any.
    fn temperature(&self) -> Option<f64> {
        None
    }

    /// Returns the stop sequences, if any.
    fn stop_sequences(&self) -> Option<&Vec<String>> {
        None
    }
}

/// Provides sampling context for a `CreateMessageRequest`.
pub trait HasSamplingContext {
    /// Returns the context messages.
    fn messages(&self) -> &[SamplingMessage];

    /// Returns the system prompt, if any.
    fn system_prompt(&self) -> Option<&str> {
        None
    }

    /// Returns the include-context setting, if any.
    fn include_context(&self) -> Option<&str> {
        None
    }
}

/// Provides model preferences for a `CreateMessageRequest`.
pub trait HasModelPreferences {
    /// Returns the model preferences, if any.
    fn model_preferences(&self) -> Option<&ModelPreferences> {
        None
    }

    /// Returns the request metadata, if any.
    fn metadata(&self) -> Option<&Value> {
        None
    }
}

/// Provides tools the model may use during sampling (MCP 2025-11-25).
///
/// These tools let the LLM act during a sampling request, enabling agentic workflows.
pub trait HasSamplingTools {
    /// Returns the tools available to the LLM during sampling, if any.
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
/// Implement these four traits on your struct:
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
pub trait SamplingDefinition:
    HasSamplingConfig + HasSamplingContext + HasModelPreferences + HasSamplingTools
{
    /// Builds `CreateMessageParams` from this definition, leaving `tool_choice`, `task` and `meta` unset.
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
            task: None,
            meta: None,
        }
    }
}

// Blanket implementation: any type implementing the fine-grained traits automatically gets SamplingDefinition
impl<T> SamplingDefinition for T where
    T: HasSamplingConfig + HasSamplingContext + HasModelPreferences + HasSamplingTools
{
}
