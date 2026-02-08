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

/// Role enum for messages (per MCP 2025-11-25 spec — only "user" | "assistant")
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
}

/// Model hint — an open-ended struct per MCP 2025-11-25 spec.
///
/// The `name` field can be any model identifier string. Clients use hints to
/// express model preferences without restricting to a hardcoded set.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModelHint {
    /// Optional model name hint (e.g., "claude-3-5-sonnet-20241022", "gpt-4o")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl ModelHint {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: Some(name.into()),
        }
    }
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

/// Tool choice mode for sampling requests (per MCP 2025-11-25)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ToolChoiceMode {
    /// Model decides whether to use tools
    Auto,
    /// Model must not use any tools
    None,
    /// Model must use at least one tool (MCP 2025-11-25: "required")
    #[serde(alias = "any")]
    Required,
}

/// Tool choice configuration for sampling requests (per MCP 2025-11-25)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolChoice {
    /// The mode for tool selection
    pub mode: ToolChoiceMode,
    /// Optional specific tool name to use (only meaningful with mode "required")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl ToolChoice {
    pub fn auto() -> Self {
        Self {
            mode: ToolChoiceMode::Auto,
            name: None,
        }
    }

    pub fn none() -> Self {
        Self {
            mode: ToolChoiceMode::None,
            name: None,
        }
    }

    /// Create tool choice requiring at least one tool (MCP 2025-11-25: "required")
    pub fn required() -> Self {
        Self {
            mode: ToolChoiceMode::Required,
            name: None,
        }
    }

    /// Compatibility alias for `required()` — MCP 2025-11-25 wire value is "required"
    pub fn any() -> Self {
        Self::required()
    }

    pub fn specific(name: impl Into<String>) -> Self {
        Self {
            mode: ToolChoiceMode::Required,
            name: Some(name.into()),
        }
    }
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
    /// Optional tools the LLM can use during sampling (MCP 2025-11-25)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<crate::tools::Tool>>,
    /// Optional tool choice configuration (MCP 2025-11-25)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,
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

/// Result for sampling/createMessage (per MCP 2025-11-25 spec)
///
/// Flattened structure: { role, content, model, stopReason?, _meta? }
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateMessageResult {
    /// Role of the generated message
    pub role: Role,
    /// Content of the generated message
    pub content: ContentBlock,
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
            tools: None,
            tool_choice: None,
            meta: None,
        }
    }

    pub fn with_tools(mut self, tools: Vec<crate::tools::Tool>) -> Self {
        self.tools = Some(tools);
        self
    }

    pub fn with_tool_choice(mut self, tool_choice: ToolChoice) -> Self {
        self.tool_choice = Some(tool_choice);
        self
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

    pub fn with_tools(mut self, tools: Vec<crate::tools::Tool>) -> Self {
        self.params = self.params.with_tools(tools);
        self
    }

    pub fn with_tool_choice(mut self, tool_choice: ToolChoice) -> Self {
        self.params = self.params.with_tool_choice(tool_choice);
        self
    }

    pub fn with_meta(mut self, meta: std::collections::HashMap<String, Value>) -> Self {
        self.params = self.params.with_meta(meta);
        self
    }
}

impl CreateMessageResult {
    pub fn new(role: Role, content: ContentBlock, model: impl Into<String>) -> Self {
        Self {
            role,
            content,
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
            serde_json::to_value(&self.role).unwrap_or(Value::String("user".to_string())),
        );
        data.insert(
            "content".to_string(),
            serde_json::to_value(&self.content).unwrap_or(Value::Null),
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
        &self.role
    }

    fn content(&self) -> &ContentBlock {
        &self.content
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_choice_mode_serializes_as_required() {
        let tc = ToolChoice::required();
        let json = serde_json::to_value(&tc).unwrap();
        assert_eq!(json["mode"], "required");
    }

    #[test]
    fn test_tool_choice_mode_deserializes_legacy_any() {
        let json = serde_json::json!({"mode": "any"});
        let tc: ToolChoice = serde_json::from_value(json).unwrap();
        assert_eq!(tc.mode, ToolChoiceMode::Required);
    }

    #[test]
    fn test_tool_choice_mode_deserializes_required() {
        let json = serde_json::json!({"mode": "required"});
        let tc: ToolChoice = serde_json::from_value(json).unwrap();
        assert_eq!(tc.mode, ToolChoiceMode::Required);
    }

    #[test]
    fn test_tool_choice_any_alias_returns_required() {
        let tc = ToolChoice::any();
        assert_eq!(tc.mode, ToolChoiceMode::Required);
    }

    #[test]
    fn test_tool_choice_specific_uses_required_mode() {
        let tc = ToolChoice::specific("my_tool");
        assert_eq!(tc.mode, ToolChoiceMode::Required);
        assert_eq!(tc.name, Some("my_tool".to_string()));
    }
}
