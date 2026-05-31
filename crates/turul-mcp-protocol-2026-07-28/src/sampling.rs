//! MCP Sampling Protocol Types
//!
//! # Deprecation status (DRAFT-2026-v1)
//!
//! Per SEP-2577, the entire Sampling client capability (`sampling/createMessage`
//! RPC, `SamplingMessage` shape, `SamplingCapabilities`) is **deprecated** in
//! this revision. New implementations SHOULD NOT adopt it. Earliest removal:
//! first revision released on or after **2027-07-28**.
//!
//! Replacement: integrate directly with LLM provider APIs.
//!
//! Soft-deprecated since 2025-11-25 and now reclassified per SEP-2596:
//! `CreateMessageRequestParams.include_context` values `"thisServer"` and
//! `"allServers"`. Omit the field or use `"none"`.
//!
//! Types NOT deprecated:
//! - [`Role`] — used outside sampling (e.g. by `Annotations.audience` in `meta`).
//! - [`ModelPreferences`], [`ToolChoice`], [`ToolChoiceMode`], [`ModelHint`] —
//!   referenced by the in-spec `sampling/createMessage` shape and the SEP-2322
//!   MRTR `InputRequest::CreateMessage` variant during the migration window.

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

/// Role enum for messages — `"user"` or `"assistant"`. The MCP spec has no `"system"` role.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
}

/// Model hint — an open-ended struct.
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

/// Tool choice mode for sampling requests.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ToolChoiceMode {
    /// Model decides whether to use tools
    Auto,
    /// Model must not use any tools
    None,
    /// Model must use at least one tool. Wire value: `"required"`; legacy
    /// `"any"` is accepted on deserialize for backward compatibility.
    #[serde(alias = "any")]
    Required,
}

/// Tool choice configuration for sampling requests.
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

    /// Create tool choice requiring at least one tool. Wire value: `"required"`.
    pub fn required() -> Self {
        Self {
            mode: ToolChoiceMode::Required,
            name: None,
        }
    }

    /// Alias for [`Self::required`] — accepts the older `"any"` name on the
    /// caller side; emits `"required"` on the wire.
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

/// Content block variant allowed inside a [`SamplingMessage`].
///
/// Strict 5-element union — excludes the `ResourceLink` and `EmbeddedResource`
/// variants that the general [`ContentBlock`] allows. Discriminated on the
/// `type` field exactly like [`ContentBlock`] for wire-format symmetry across
/// the 5 shared shapes.
///
/// **Deprecated** per SEP-2577 — see module-level docs.
#[deprecated(
    since = "0.4.0",
    note = "Deprecated per SEP-2577 (DRAFT-2026-v1). \
            Replacement: integrate directly with LLM provider APIs. \
            Earliest removal: first release on/after 2027-07-28."
)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SamplingMessageContentBlock {
    #[serde(rename = "text")]
    Text {
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        annotations: Option<crate::meta::Annotations>,
        #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
        meta: Option<std::collections::HashMap<String, Value>>,
    },
    #[serde(rename = "image")]
    Image {
        data: String,
        #[serde(rename = "mimeType")]
        mime_type: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        annotations: Option<crate::meta::Annotations>,
        #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
        meta: Option<std::collections::HashMap<String, Value>>,
    },
    #[serde(rename = "audio")]
    Audio {
        data: String,
        #[serde(rename = "mimeType")]
        mime_type: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        annotations: Option<crate::meta::Annotations>,
        #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
        meta: Option<std::collections::HashMap<String, Value>>,
    },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: std::collections::HashMap<String, Value>,
        #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
        meta: Option<std::collections::HashMap<String, Value>>,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        #[serde(rename = "toolUseId")]
        tool_use_id: String,
        content: Vec<ContentBlock>,
        #[serde(rename = "structuredContent", skip_serializing_if = "Option::is_none")]
        structured_content: Option<Value>,
        #[serde(rename = "isError", skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
        #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
        meta: Option<std::collections::HashMap<String, Value>>,
    },
}

#[allow(deprecated)]
impl SamplingMessageContentBlock {
    /// Construct a text block.
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text {
            text: text.into(),
            annotations: None,
            meta: None,
        }
    }
}

/// Content payload of a [`SamplingMessage`] — single block OR array of blocks.
///
/// `content: SamplingMessageContentBlock | SamplingMessageContentBlock[]`.
/// Untagged — the wire decides which shape is sent.
///
/// **Deprecated** per SEP-2577 — see module-level docs.
#[deprecated(
    since = "0.4.0",
    note = "Deprecated per SEP-2577 (DRAFT-2026-v1). \
            Replacement: integrate directly with LLM provider APIs. \
            Earliest removal: first release on/after 2027-07-28."
)]
#[allow(deprecated)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SamplingMessageContent {
    /// Single content block on the wire.
    Single(SamplingMessageContentBlock),
    /// Array of content blocks on the wire.
    Multiple(Vec<SamplingMessageContentBlock>),
}

/// Sampling message.
///
/// **Deprecated** per SEP-2577 — see module-level docs.
#[deprecated(
    since = "0.4.0",
    note = "Deprecated per SEP-2577 (DRAFT-2026-v1). \
            Replacement: integrate directly with LLM provider APIs. \
            Earliest removal: first release on/after 2027-07-28."
)]
#[allow(deprecated)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SamplingMessage {
    pub role: Role,
    pub content: SamplingMessageContent,
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<std::collections::HashMap<String, Value>>,
}

/// Parameters for sampling/createMessage request (per MCP spec).
///
/// **Deprecated** per SEP-2577 — see module-level docs.
///
/// `include_context` value note: `"thisServer"` and `"allServers"` are
/// soft-deprecated per SEP-2596 and conditional on
/// `ClientCapabilities.sampling.context`. Omit the field or use `"none"`.
#[deprecated(
    since = "0.4.0",
    note = "Deprecated per SEP-2577 (DRAFT-2026-v1). \
            Replacement: integrate directly with LLM provider APIs. \
            Earliest removal: first release on/after 2027-07-28."
)]
#[allow(deprecated)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateMessageRequestParams {
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
    /// Optional tools the LLM can use during sampling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<crate::tools::Tool>>,
    /// Optional tool choice configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,
    // Per schema, `CreateMessageRequestParams` does NOT extend `RequestParams`
    // — no `_meta` field. The earlier Rust-side `meta: Option<HashMap>` was a
    // non-spec carryover, removed for Protocol Crate Purity.
}

/// Complete sampling/createMessage request (matches TypeScript CreateMessageRequest interface).
///
/// **Deprecated** per SEP-2577 — see module-level docs.
#[deprecated(
    since = "0.4.0",
    note = "Deprecated per SEP-2577 (DRAFT-2026-v1). \
            Replacement: integrate directly with LLM provider APIs. \
            Earliest removal: first release on/after 2027-07-28."
)]
#[allow(deprecated)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateMessageRequest {
    /// Method name (always "sampling/createMessage")
    pub method: String,
    /// Request parameters
    pub params: CreateMessageRequestParams,
}

/// Result for `sampling/createMessage` — `extends SamplingMessage`
/// (role, content, _meta) plus `model` and optional `stopReason`.
///
/// **Deprecated** per SEP-2577 — see module-level docs.
#[deprecated(
    since = "0.4.0",
    note = "Deprecated per SEP-2577 (DRAFT-2026-v1). \
            Replacement: integrate directly with LLM provider APIs. \
            Earliest removal: first release on/after 2027-07-28."
)]
#[allow(deprecated)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateMessageResult {
    pub role: Role,
    pub content: SamplingMessageContent,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<std::collections::HashMap<String, Value>>,
}

#[allow(deprecated)]
impl CreateMessageRequestParams {
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
}

#[allow(deprecated)]
impl CreateMessageRequest {
    pub fn new(messages: Vec<SamplingMessage>, max_tokens: u32) -> Self {
        Self {
            method: "sampling/createMessage".to_string(),
            params: CreateMessageRequestParams::new(messages, max_tokens),
        }
    }

    /// Attach a fully-constructed params struct.
    pub fn with_params(mut self, params: CreateMessageRequestParams) -> Self {
        self.params = params;
        self
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

}

#[allow(deprecated)]
impl CreateMessageResult {
    pub fn new(role: Role, content: SamplingMessageContent, model: impl Into<String>) -> Self {
        Self {
            role,
            content,
            model: model.into(),
            stop_reason: None,
            meta: None,
        }
    }

    /// Convenience: single-block result (most common LLM response shape).
    pub fn single(role: Role, block: SamplingMessageContentBlock, model: impl Into<String>) -> Self {
        Self::new(role, SamplingMessageContent::Single(block), model)
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

// Trait implementations for sampling.
//
// **Deprecated** per SEP-2577 — trait impls retained during the 12-month
// migration window so existing call sites that depend on the trait surface
// (e.g. `InputRequest::CreateMessage` in the MRTR flow) continue to compile.
// Concrete `#[deprecated]` attributes live on the struct definitions above;
// the `#[allow(deprecated)]` blocks below suppress the cascading warning
// inside the protocol crate itself.

use crate::traits::*;
use std::collections::HashMap;

// Trait implementations for CreateMessageRequestParams
#[allow(deprecated)]
impl Params for CreateMessageRequestParams {}

#[allow(deprecated)]
impl HasCreateMessageRequestParams for CreateMessageRequestParams {
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

// `HasMetaParam` intentionally NOT implemented — per schema
// `CreateMessageRequestParams` does NOT extend `RequestParams`, so it has no
// `_meta` field on the wire.

// Trait implementations for CreateMessageRequest
#[allow(deprecated)]
impl HasMethod for CreateMessageRequest {
    fn method(&self) -> &str {
        &self.method
    }
}

#[allow(deprecated)]
impl HasParams for CreateMessageRequest {
    fn params(&self) -> Option<&dyn Params> {
        Some(&self.params)
    }
}

// Trait implementations for CreateMessageResult
#[allow(deprecated)]
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

#[allow(deprecated)]
impl HasMeta for CreateMessageResult {
    fn meta(&self) -> Option<&crate::meta::MetaObject> {
        self.meta.as_ref()
    }
}

// `CreateMessageResult` does NOT implement `RpcResult` — per the schema it
// `extends SamplingMessage` (not `Result`), so it has no `resultType`
// discriminator and the `RpcResult: HasMeta + HasResultType` bound doesn't fit.
// See `crate::traits::RpcResult` for the contract.

#[allow(deprecated)]
impl crate::traits::CreateMessageResult for CreateMessageResult {
    fn role(&self) -> &Role {
        &self.role
    }

    fn content(&self) -> &SamplingMessageContent {
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

#[allow(deprecated)]
impl SamplingMessage {
    pub fn new(role: Role, content: SamplingMessageContent) -> Self {
        Self {
            role,
            content,
            meta: None,
        }
    }

    /// Single-block convenience.
    pub fn single(role: Role, block: SamplingMessageContentBlock) -> Self {
        Self::new(role, SamplingMessageContent::Single(block))
    }

    pub fn user_text(text: impl Into<String>) -> Self {
        Self::single(Role::User, SamplingMessageContentBlock::text(text))
    }

    pub fn assistant_text(text: impl Into<String>) -> Self {
        Self::single(Role::Assistant, SamplingMessageContentBlock::text(text))
    }

    pub fn with_meta(mut self, meta: std::collections::HashMap<String, Value>) -> Self {
        self.meta = Some(meta);
        self
    }
}

#[cfg(test)]
#[allow(deprecated)]
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
