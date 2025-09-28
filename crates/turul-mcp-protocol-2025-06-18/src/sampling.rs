//! MCP Sampling Protocol Types
//!
//! This module defines types for sampling requests in MCP.

use crate::content::ContentBlock;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Sampling request parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SamplingRequest {
    /// The sampling method to use
    pub method: String,
    /// Parameters for the sampling method
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

/// Sampling response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SamplingResult {
    /// The sampled result
    pub result: Value,
}

impl SamplingResult {
    pub fn new(result: Value) -> Self {
        Self { result }
    }
}

/// Role enum for messages (per MCP spec)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
    System,
}

/// Model hint enum (per MCP spec)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ModelHint {
    #[serde(rename = "claude-3-5-sonnet-20241022")]
    Claude35Sonnet20241022,
    #[serde(rename = "claude-3-5-haiku-20241022")]
    Claude35Haiku20241022,
    #[serde(rename = "gpt-4o")]
    Gpt4o,
    #[serde(rename = "gpt-4o-mini")]
    Gpt4oMini,
    #[serde(rename = "o1-preview")]
    O1Preview,
    #[serde(rename = "o1-mini")]
    O1Mini,
}

/// Model preferences (per MCP spec)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelPreferences {
    /// Optional hints about which models to use
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hints: Option<Vec<ModelHint>>,
    /// Optional cost priority (0.0-1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_priority: Option<f64>,
    /// Optional speed priority (0.0-1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed_priority: Option<f64>,
    /// Optional intelligence priority (0.0-1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intelligence_priority: Option<f64>,
}

/// Sampling message (per MCP spec)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SamplingMessage {
    /// Role of the message
    pub role: Role,
    /// Content of the message
    pub content: ContentBlock,
}

/// Parameters for sampling/createMessage request (per MCP spec)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateMessageParams {
    /// Messages for context
    pub messages: Vec<SamplingMessage>,
    /// Optional model preferences
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_preferences: Option<ModelPreferences>,
    /// Optional system prompt
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    /// Optional include context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_context: Option<String>,
    /// Optional temperature
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    /// Maximum tokens (required field)
    pub max_tokens: u32,
    /// Optional stop sequences
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
    /// Optional metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
    /// Meta information (optional _meta field inside params)
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<std::collections::HashMap<String, Value>>,
}

/// Complete sampling/createMessage request (matches TypeScript CreateMessageRequest interface)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateMessageRequest {
    /// Method name (always "sampling/createMessage")
    pub method: String,
    /// Request parameters
    pub params: CreateMessageParams,
}

/// Result for sampling/createMessage (per MCP spec)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateMessageResult {
    /// The generated message
    pub message: SamplingMessage,
    /// Model used for generation
    pub model: String,
    /// Stop reason
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,
    /// Meta information (follows MCP Result interface)
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        alias = "_meta",
        rename = "_meta"
    )]
    pub meta: Option<std::collections::HashMap<String, Value>>,
}

impl CreateMessageParams {
    pub fn new(messages: Vec<SamplingMessage>, max_tokens: u32) -> Self {
        Self {
            messages,
            model_preferences: None,
            system_prompt: None,
            include_context: None,
            temperature: None,
            max_tokens,
            stop_sequences: None,
            metadata: None,
            meta: None,
        }
    }

    pub fn with_model_preferences(mut self, preferences: ModelPreferences) -> Self {
        self.model_preferences = Some(preferences);
        self
    }

    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    pub fn with_temperature(mut self, temperature: f64) -> Self {
        self.temperature = Some(temperature);
        self
    }

    pub fn with_stop_sequences(mut self, sequences: Vec<String>) -> Self {
        self.stop_sequences = Some(sequences);
        self
    }

    pub fn with_meta(mut self, meta: std::collections::HashMap<String, Value>) -> Self {
        self.meta = Some(meta);
        self
    }
}

impl CreateMessageRequest {
    pub fn new(messages: Vec<SamplingMessage>, max_tokens: u32) -> Self {
        Self {
            method: "sampling/createMessage".to_string(),
            params: CreateMessageParams::new(messages, max_tokens),
        }
    }

    pub fn with_model_preferences(mut self, preferences: ModelPreferences) -> Self {
        self.params = self.params.with_model_preferences(preferences);
        self
    }

    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.params = self.params.with_system_prompt(prompt);
        self
    }

    pub fn with_temperature(mut self, temperature: f64) -> Self {
        self.params = self.params.with_temperature(temperature);
        self
    }

    pub fn with_stop_sequences(mut self, sequences: Vec<String>) -> Self {
        self.params = self.params.with_stop_sequences(sequences);
        self
    }

    pub fn with_meta(mut self, meta: std::collections::HashMap<String, Value>) -> Self {
        self.params = self.params.with_meta(meta);
        self
    }
}

impl CreateMessageResult {
    pub fn new(message: SamplingMessage, model: impl Into<String>) -> Self {
        Self {
            message,
            model: model.into(),
            stop_reason: None,
            meta: None,
        }
    }

    pub fn with_stop_reason(mut self, reason: impl Into<String>) -> Self {
        self.stop_reason = Some(reason.into());
        self
    }

    pub fn with_meta(mut self, meta: std::collections::HashMap<String, Value>) -> Self {
        self.meta = Some(meta);
        self
    }
}

// Trait implementations for sampling

use crate::traits::*;
use std::collections::HashMap;

// Trait implementations for CreateMessageParams
impl Params for CreateMessageParams {}

impl HasCreateMessageParams for CreateMessageParams {
    fn messages(&self) -> &Vec<SamplingMessage> {
        &self.messages
    }

    fn model_preferences(&self) -> Option<&ModelPreferences> {
        self.model_preferences.as_ref()
    }

    fn system_prompt(&self) -> Option<&String> {
        self.system_prompt.as_ref()
    }

    fn include_context(&self) -> Option<&String> {
        self.include_context.as_ref()
    }

    fn temperature(&self) -> Option<&f64> {
        self.temperature.as_ref()
    }

    fn max_tokens(&self) -> u32 {
        self.max_tokens
    }

    fn stop_sequences(&self) -> Option<&Vec<String>> {
        self.stop_sequences.as_ref()
    }

    fn metadata(&self) -> Option<&Value> {
        self.metadata.as_ref()
    }
}

impl HasMetaParam for CreateMessageParams {
    fn meta(&self) -> Option<&std::collections::HashMap<String, Value>> {
        self.meta.as_ref()
    }
}

// Trait implementations for CreateMessageRequest
impl HasMethod for CreateMessageRequest {
    fn method(&self) -> &str {
        &self.method
    }
}

impl HasParams for CreateMessageRequest {
    fn params(&self) -> Option<&dyn Params> {
        Some(&self.params)
    }
}

// Trait implementations for CreateMessageResult
impl HasData for CreateMessageResult {
    fn data(&self) -> HashMap<String, Value> {
        let mut data = HashMap::new();
        data.insert(
            "role".to_string(),
            serde_json::to_value(&self.message.role).unwrap_or(Value::String("user".to_string())),
        );
        data.insert(
            "content".to_string(),
            serde_json::to_value(&self.message.content).unwrap_or(Value::Null),
        );
        data.insert("model".to_string(), Value::String(self.model.clone()));
        if let Some(ref stop_reason) = self.stop_reason {
            data.insert("stopReason".to_string(), Value::String(stop_reason.clone()));
        }
        data
    }
}

impl HasMeta for CreateMessageResult {
    fn meta(&self) -> Option<HashMap<String, Value>> {
        self.meta.clone()
    }
}

impl RpcResult for CreateMessageResult {}

impl crate::traits::CreateMessageResult for CreateMessageResult {
    fn role(&self) -> &Role {
        &self.message.role
    }

    fn content(&self) -> &ContentBlock {
        &self.message.content
    }

    fn model(&self) -> &String {
        &self.model
    }

    fn stop_reason(&self) -> Option<&String> {
        self.stop_reason.as_ref()
    }
}

// ===========================================
// === Fine-Grained Sampling Traits ===
// ===========================================

/// Trait for sampling message metadata (role, content from MCP spec)
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
/// # use turul_mcp_protocol_2025_06_18::sampling::*;
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
/// - **MCP Compliant**: Fully compatible with MCP 2025-06-18 specification
///
/// # Common Use Cases
///
/// - Code review and analysis systems
/// - Creative writing assistance
/// - Technical documentation generation
/// - Question-answering with domain context
/// - Conversational AI with specific personalities
pub trait SamplingDefinition: HasSamplingConfig + HasSamplingContext + HasModelPreferences {
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
            meta: None,
        }
    }
}

// Blanket implementation: any type implementing the fine-grained traits automatically gets SamplingDefinition
impl<T> SamplingDefinition for T where
    T: HasSamplingConfig + HasSamplingContext + HasModelPreferences
{
}

// ================== TRAIT IMPLEMENTATIONS FOR CONCRETE TYPES ==================

impl HasSamplingMessageMetadata for SamplingMessage {
    fn role(&self) -> &Role {
        &self.role
    }
    fn content(&self) -> &ContentBlock {
        &self.content
    }
}

impl HasSamplingConfig for CreateMessageParams {
    fn max_tokens(&self) -> u32 {
        self.max_tokens
    }
    fn temperature(&self) -> Option<f64> {
        self.temperature
    }
    fn stop_sequences(&self) -> Option<&Vec<String>> {
        self.stop_sequences.as_ref()
    }
}

impl HasSamplingContext for CreateMessageParams {
    fn messages(&self) -> &[SamplingMessage] {
        &self.messages
    }
    fn system_prompt(&self) -> Option<&str> {
        self.system_prompt.as_deref()
    }
    fn include_context(&self) -> Option<&str> {
        self.include_context.as_deref()
    }
}

impl HasModelPreferences for CreateMessageParams {
    fn model_preferences(&self) -> Option<&ModelPreferences> {
        self.model_preferences.as_ref()
    }
    fn metadata(&self) -> Option<&Value> {
        self.metadata.as_ref()
    }
}

// CreateMessageParams automatically implements SamplingDefinition via trait composition!

// ================== CONVENIENCE CONSTRUCTORS ==================

impl ModelPreferences {
    pub fn new() -> Self {
        Self {
            hints: None,
            cost_priority: None,
            speed_priority: None,
            intelligence_priority: None,
        }
    }

    pub fn with_hints(mut self, hints: Vec<ModelHint>) -> Self {
        self.hints = Some(hints);
        self
    }

    pub fn with_cost_priority(mut self, priority: f64) -> Self {
        self.cost_priority = Some(priority);
        self
    }

    pub fn with_speed_priority(mut self, priority: f64) -> Self {
        self.speed_priority = Some(priority);
        self
    }

    pub fn with_intelligence_priority(mut self, priority: f64) -> Self {
        self.intelligence_priority = Some(priority);
        self
    }
}

impl Default for ModelPreferences {
    fn default() -> Self {
        Self::new()
    }
}

impl SamplingMessage {
    pub fn new(role: Role, content: ContentBlock) -> Self {
        Self { role, content }
    }

    pub fn user_text(text: impl Into<String>) -> Self {
        Self::new(Role::User, ContentBlock::text(text))
    }

    pub fn assistant_text(text: impl Into<String>) -> Self {
        Self::new(Role::Assistant, ContentBlock::text(text))
    }

    pub fn system_text(text: impl Into<String>) -> Self {
        Self::new(Role::System, ContentBlock::text(text))
    }
}
