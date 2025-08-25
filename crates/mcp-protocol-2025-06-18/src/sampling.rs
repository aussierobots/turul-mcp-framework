//! MCP Sampling Protocol Types
//!
//! This module defines types for sampling requests in MCP.

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

/// Message content for sampling
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum MessageContent {
    /// Text content
    Text {
        text: String,
    },
    /// Image content
    Image {
        data: String,
        #[serde(rename = "mimeType")]
        mime_type: String,
    },
}

/// Sampling message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SamplingMessage {
    /// Role of the message
    pub role: String,
    /// Content of the message
    pub content: MessageContent,
}

/// Parameters for sampling/createMessage request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateMessageParams {
    /// Messages for context
    pub messages: Vec<SamplingMessage>,
    /// Optional model preferences
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_preferences: Option<Value>,
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

    pub fn with_model_preferences(mut self, preferences: Value) -> Self {
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

    pub fn with_model_preferences(mut self, preferences: Value) -> Self {
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
    
    fn model_preferences(&self) -> Option<&Value> {
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
        data.insert("role".to_string(), Value::String(self.message.role.clone()));
        data.insert("content".to_string(), serde_json::to_value(&self.message.content).unwrap_or(Value::Null));
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
    fn role(&self) -> &str {
        &self.message.role
    }
    
    fn content(&self) -> &MessageContent {
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
    fn role(&self) -> &str;
    
    /// Content of the message (from spec)
    fn content(&self) -> &MessageContent;
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
    fn model_preferences(&self) -> Option<&Value> {
        None
    }
    
    /// Metadata (optional from spec)
    fn metadata(&self) -> Option<&Value> {
        None
    }
}

/// Composed sampling definition trait (automatically implemented via blanket impl)
pub trait SamplingDefinition: 
    HasSamplingConfig + 
    HasSamplingContext + 
    HasModelPreferences 
{
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
impl<T> SamplingDefinition for T 
where 
    T: HasSamplingConfig + HasSamplingContext + HasModelPreferences 
{}