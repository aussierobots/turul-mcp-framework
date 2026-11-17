//! MCP Sampling Protocol Types
//!
//! # Deprecation status (2026-07-28)
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
//! [`ModelPreferences`], [`ToolChoice`], and [`ModelHint`] carry their own
//! schema `@deprecated` markers and are `#[deprecated]` here accordingly;
//! they remain referenced by the `sampling/createMessage` shape and the
//! SEP-2322 MRTR `InputRequest::CreateMessage` variant through the migration
//! window. [`ToolChoiceMode`] has no schema symbol of its own (it binds the
//! inline `mode` union of the deprecated [`ToolChoice`]) and is left
//! unmarked. The only genuinely non-deprecated export is [`Role`], which is
//! used outside sampling (e.g. by `Annotations.audience` in `meta`).

use crate::content::ContentBlock;
use serde::{Deserialize, Serialize};
use serde_json::Value;

// The schema declares a single `Role` type ("user" | "assistant" — no
// "system"); this module re-exports the one binding rather than duplicating it.
pub use crate::prompts::Role;

#[deprecated(
    since = "0.4.0",
    note = "Deprecated per SEP-2577 (2026-07-28). \
            Replacement: integrate directly with LLM provider APIs. \
            Earliest removal: first release on/after 2027-07-28."
)]
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

#[allow(deprecated)]
impl ModelHint {
    /// Creates a hint naming the preferred model.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: Some(name.into()),
        }
    }
}

#[deprecated(
    since = "0.4.0",
    note = "Deprecated per SEP-2577 (2026-07-28). \
            Replacement: integrate directly with LLM provider APIs. \
            Earliest removal: first release on/after 2027-07-28."
)]
/// Model preferences (per MCP spec)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelPreferences {
    /// Optional hints about which models to use
    #[serde(skip_serializing_if = "Option::is_none")]
    #[allow(deprecated)]
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

#[deprecated(
    since = "0.4.0",
    note = "Deprecated per SEP-2577 (2026-07-28). \
            Replacement: integrate directly with LLM provider APIs. \
            Earliest removal: first release on/after 2027-07-28."
)]
/// Tool choice configuration for sampling requests.
///
/// Wire shape: `{ mode?: "auto" | "required" | "none" }` — `mode` is optional
/// and absent means `"auto"`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ToolChoice {
    /// The mode for tool selection. Absent = `"auto"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<ToolChoiceMode>,
}

#[allow(deprecated)]
impl ToolChoice {
    /// Returns a choice with mode `"auto"`: the model decides whether to use tools.
    pub fn auto() -> Self {
        Self {
            mode: Some(ToolChoiceMode::Auto),
        }
    }

    /// Returns a choice with mode `"none"`: the model must not use any tools.
    pub fn none() -> Self {
        Self {
            mode: Some(ToolChoiceMode::None),
        }
    }

    /// Returns a choice requiring the model to use at least one tool. Wire value: `"required"`.
    pub fn required() -> Self {
        Self {
            mode: Some(ToolChoiceMode::Required),
        }
    }

    /// Alias for [`Self::required`] — accepts the older `"any"` name on the
    /// caller side; emits `"required"` on the wire.
    pub fn any() -> Self {
        Self::required()
    }

    /// The effective mode: absent means `"auto"`.
    pub fn effective_mode(&self) -> ToolChoiceMode {
        self.mode.clone().unwrap_or(ToolChoiceMode::Auto)
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
    note = "Deprecated per SEP-2577 (2026-07-28). \
            Replacement: integrate directly with LLM provider APIs. \
            Earliest removal: first release on/after 2027-07-28."
)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SamplingMessageContentBlock {
    #[serde(rename = "text")]
    /// Text provided to or from an LLM.
    Text {
        /// The text content of the message.
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        /// Optional annotations for the client.
        annotations: Option<crate::meta::Annotations>,
        #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
        #[serde(serialize_with = "crate::serde_maps::sorted_opt")]
        /// Optional `_meta` object for protocol-reserved metadata.
        meta: Option<std::collections::HashMap<String, Value>>,
    },
    #[serde(rename = "image")]
    /// An image provided to or from an LLM.
    Image {
        /// The base64-encoded image data.
        data: String,
        #[serde(rename = "mimeType")]
        /// The MIME type of the image, for example `image/png`.
        mime_type: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        /// Optional annotations for the client.
        annotations: Option<crate::meta::Annotations>,
        #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
        #[serde(serialize_with = "crate::serde_maps::sorted_opt")]
        /// Optional `_meta` object for protocol-reserved metadata.
        meta: Option<std::collections::HashMap<String, Value>>,
    },
    #[serde(rename = "audio")]
    /// Audio provided to or from an LLM.
    Audio {
        /// The base64-encoded audio data.
        data: String,
        #[serde(rename = "mimeType")]
        /// The MIME type of the audio, for example `audio/wav`.
        mime_type: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        /// Optional annotations for the client.
        annotations: Option<crate::meta::Annotations>,
        #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
        #[serde(serialize_with = "crate::serde_maps::sorted_opt")]
        /// Optional `_meta` object for protocol-reserved metadata.
        meta: Option<std::collections::HashMap<String, Value>>,
    },
    #[serde(rename = "tool_use")]
    /// A request from the assistant to call a tool.
    ToolUse {
        /// A unique identifier for this tool use, which the corresponding tool result echoes as `toolUseId`.
        id: String,
        /// The name of the tool to call.
        name: String,
        #[serde(serialize_with = "crate::serde_maps::sorted")]
        /// The arguments to pass to the tool, conforming to the tool's input schema.
        input: std::collections::HashMap<String, Value>,
        #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
        #[serde(serialize_with = "crate::serde_maps::sorted_opt")]
        /// Optional `_meta` object for protocol-reserved metadata.
        meta: Option<std::collections::HashMap<String, Value>>,
    },
    #[serde(rename = "tool_result")]
    /// The result of a tool use, provided by the user back to the assistant.
    ToolResult {
        #[serde(rename = "toolUseId")]
        /// The `id` of the tool use this result corresponds to; it must match a previous tool use.
        tool_use_id: String,
        /// The unstructured result of the tool use, in the same format as a `tools/call` result's `content`.
        content: Vec<ContentBlock>,
        #[serde(rename = "structuredContent", skip_serializing_if = "Option::is_none")]
        /// The structured result of the tool use, as any JSON value; it should conform to the tool's output schema when one is defined.
        structured_content: Option<Value>,
        #[serde(rename = "isError", skip_serializing_if = "Option::is_none")]
        /// Whether the tool use resulted in an error; absent means `false`.
        is_error: Option<bool>,
        #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
        #[serde(serialize_with = "crate::serde_maps::sorted_opt")]
        /// Optional `_meta` object for protocol-reserved metadata.
        meta: Option<std::collections::HashMap<String, Value>>,
    },
}

#[allow(deprecated)]
impl SamplingMessageContentBlock {
    /// Constructs a text block.
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
    note = "Deprecated per SEP-2577 (2026-07-28). \
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
    note = "Deprecated per SEP-2577 (2026-07-28). \
            Replacement: integrate directly with LLM provider APIs. \
            Earliest removal: first release on/after 2027-07-28."
)]
#[allow(deprecated)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SamplingMessage {
    /// The sender of the message: `user` or `assistant`.
    pub role: Role,
    /// The message content, either a single block or an array of blocks.
    pub content: SamplingMessageContent,
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    #[serde(serialize_with = "crate::serde_maps::sorted_opt")]
    /// Optional `_meta` object for protocol-reserved metadata.
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
    note = "Deprecated per SEP-2577 (2026-07-28). \
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
    /// Optional metadata to pass through to the LLM provider.
    ///
    /// Schema: `metadata?: JSONObject` — same `HashMap<String, Value>`
    /// convention this crate uses for other `JSONObject` fields (e.g.
    /// `ClientCapabilities.experimental`).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(serialize_with = "crate::serde_maps::sorted_opt")]
    pub metadata: Option<std::collections::HashMap<String, Value>>,
    /// Optional tools the LLM can use during sampling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<crate::tools::Tool>>,
    /// Optional tool choice configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,
    // Per schema, `CreateMessageRequestParams` does NOT extend `RequestParams`
    // — no `_meta` field.
}

/// Complete sampling/createMessage request (matches TypeScript CreateMessageRequest interface).
///
/// **Deprecated** per SEP-2577 — see module-level docs.
#[deprecated(
    since = "0.4.0",
    note = "Deprecated per SEP-2577 (2026-07-28). \
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
    note = "Deprecated per SEP-2577 (2026-07-28). \
            Replacement: integrate directly with LLM provider APIs. \
            Earliest removal: first release on/after 2027-07-28."
)]
#[allow(deprecated)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateMessageResult {
    /// The role of the generated message.
    pub role: Role,
    /// The generated content, either a single block or an array of blocks.
    pub content: SamplingMessageContent,
    /// The name of the model that generated the message.
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Why sampling stopped, if known: standard values are `endTurn`, `stopSequence`, `maxTokens` and `toolUse`, and providers may send others.
    pub stop_reason: Option<String>,
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    /// Optional `_meta` object carrying result metadata.
    pub meta: Option<crate::meta::ResultMetaObject>,
}

#[allow(deprecated)]
impl CreateMessageRequestParams {
    /// Creates parameters from the conversation messages and the maximum number of tokens to sample; all other fields are unset.
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

    /// Sets the tools the model may use during sampling.
    pub fn with_tools(mut self, tools: Vec<crate::tools::Tool>) -> Self {
        self.tools = Some(tools);
        self
    }

    /// Sets how the model may choose among the supplied tools.
    pub fn with_tool_choice(mut self, tool_choice: ToolChoice) -> Self {
        self.tool_choice = Some(tool_choice);
        self
    }

    /// Sets the server's model preferences.
    pub fn with_model_preferences(mut self, preferences: ModelPreferences) -> Self {
        self.model_preferences = Some(preferences);
        self
    }

    /// Sets the system prompt requested for sampling.
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// Sets the sampling temperature.
    pub fn with_temperature(mut self, temperature: f64) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Sets the sequences that end generation when produced.
    pub fn with_stop_sequences(mut self, sequences: Vec<String>) -> Self {
        self.stop_sequences = Some(sequences);
        self
    }

    /// Sampling message-shape MUSTs:
    ///
    /// - A user-role message whose content contains a `ToolResult` block MUST
    ///   contain ONLY `ToolResult` blocks (no mixing with text/image/etc.).
    /// - Every assistant-role message containing a `ToolUse` block MUST be
    ///   immediately followed by a user-role message consisting entirely of
    ///   `ToolResult` blocks whose `tool_use_id`s match the preceding
    ///   `ToolUse` ids.
    pub fn validate_message_shape(&self) -> Result<(), String> {
        fn blocks(content: &SamplingMessageContent) -> Vec<&SamplingMessageContentBlock> {
            match content {
                SamplingMessageContent::Single(b) => vec![b],
                SamplingMessageContent::Multiple(bs) => bs.iter().collect(),
            }
        }

        fn is_tool_result(block: &SamplingMessageContentBlock) -> bool {
            matches!(block, SamplingMessageContentBlock::ToolResult { .. })
        }

        for (i, message) in self.messages.iter().enumerate() {
            let content_blocks = blocks(&message.content);

            if message.role == Role::User {
                let any_tool_result = content_blocks.iter().any(|b| is_tool_result(b));
                let all_tool_result = content_blocks.iter().all(|b| is_tool_result(b));
                if any_tool_result && !all_tool_result {
                    return Err(format!(
                        "message {i}: a user message containing a ToolResult block must contain ONLY ToolResult blocks"
                    ));
                }
            }

            if message.role == Role::Assistant {
                let tool_use_ids: Vec<&str> = content_blocks
                    .iter()
                    .filter_map(|b| match b {
                        SamplingMessageContentBlock::ToolUse { id, .. } => Some(id.as_str()),
                        _ => None,
                    })
                    .collect();
                if tool_use_ids.is_empty() {
                    continue;
                }

                let next = self.messages.get(i + 1).filter(|m| m.role == Role::User);
                let Some(next) = next else {
                    return Err(format!(
                        "message {i}: assistant ToolUse must be immediately followed by a user message of ToolResult blocks"
                    ));
                };
                let next_blocks = blocks(&next.content);
                if next_blocks.is_empty() || !next_blocks.iter().all(|b| is_tool_result(b)) {
                    return Err(format!(
                        "message {i}: assistant ToolUse must be immediately followed by a user message consisting entirely of ToolResult blocks"
                    ));
                }
                let next_ids: std::collections::HashSet<&str> = next_blocks
                    .iter()
                    .filter_map(|b| match b {
                        SamplingMessageContentBlock::ToolResult { tool_use_id, .. } => {
                            Some(tool_use_id.as_str())
                        }
                        _ => None,
                    })
                    .collect();
                for id in tool_use_ids {
                    if !next_ids.contains(id) {
                        return Err(format!(
                            "message {i}: ToolUse id '{id}' has no matching ToolResult in the following message"
                        ));
                    }
                }
            }
        }

        Ok(())
    }
}

#[allow(deprecated)]
impl CreateMessageRequest {
    /// Creates a `sampling/createMessage` request from the messages and the maximum number of tokens to sample.
    pub fn new(messages: Vec<SamplingMessage>, max_tokens: u32) -> Self {
        Self {
            method: "sampling/createMessage".to_string(),
            params: CreateMessageRequestParams::new(messages, max_tokens),
        }
    }

    /// Attaches a fully-constructed params struct.
    pub fn with_params(mut self, params: CreateMessageRequestParams) -> Self {
        self.params = params;
        self
    }

    /// Sets the server's model preferences on the request parameters.
    pub fn with_model_preferences(mut self, preferences: ModelPreferences) -> Self {
        self.params = self.params.with_model_preferences(preferences);
        self
    }

    /// Sets the system prompt on the request parameters.
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.params = self.params.with_system_prompt(prompt);
        self
    }

    /// Sets the sampling temperature on the request parameters.
    pub fn with_temperature(mut self, temperature: f64) -> Self {
        self.params = self.params.with_temperature(temperature);
        self
    }

    /// Sets the stop sequences on the request parameters.
    pub fn with_stop_sequences(mut self, sequences: Vec<String>) -> Self {
        self.params = self.params.with_stop_sequences(sequences);
        self
    }

    /// Sets the tools the model may use on the request parameters.
    pub fn with_tools(mut self, tools: Vec<crate::tools::Tool>) -> Self {
        self.params = self.params.with_tools(tools);
        self
    }

    /// Sets the tool choice on the request parameters.
    pub fn with_tool_choice(mut self, tool_choice: ToolChoice) -> Self {
        self.params = self.params.with_tool_choice(tool_choice);
        self
    }
}

#[allow(deprecated)]
impl CreateMessageResult {
    /// Creates a result from the role, content and model name; `stop_reason` and `meta` are unset.
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
    pub fn single(
        role: Role,
        block: SamplingMessageContentBlock,
        model: impl Into<String>,
    ) -> Self {
        Self::new(role, SamplingMessageContent::Single(block), model)
    }

    /// Sets the reason sampling stopped.
    pub fn with_stop_reason(mut self, reason: impl Into<String>) -> Self {
        self.stop_reason = Some(reason.into());
        self
    }

    /// Sets the result's `_meta` object.
    pub fn with_meta(mut self, meta: impl Into<crate::meta::ResultMetaObject>) -> Self {
        self.meta = Some(meta.into());
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

    fn metadata(&self) -> Option<&std::collections::HashMap<String, Value>> {
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
    fn meta(&self) -> Option<&crate::meta::ResultMetaObject> {
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

#[allow(deprecated)]
impl ModelPreferences {
    /// Creates preferences with no hints and no priorities.
    pub fn new() -> Self {
        Self {
            hints: None,
            cost_priority: None,
            speed_priority: None,
            intelligence_priority: None,
        }
    }

    /// Sets the model name hints; the client evaluates them in order and takes the first match.
    pub fn with_hints(mut self, hints: Vec<ModelHint>) -> Self {
        self.hints = Some(hints);
        self
    }

    /// Sets how much to prioritise cost when selecting a model, from 0 (unimportant) to 1 (most important).
    pub fn with_cost_priority(mut self, priority: f64) -> Self {
        self.cost_priority = Some(priority);
        self
    }

    /// Sets how much to prioritise sampling speed (latency) when selecting a model, from 0 (unimportant) to 1 (most important).
    pub fn with_speed_priority(mut self, priority: f64) -> Self {
        self.speed_priority = Some(priority);
        self
    }

    /// Sets how much to prioritise intelligence and capabilities when selecting a model, from 0 (unimportant) to 1 (most important).
    pub fn with_intelligence_priority(mut self, priority: f64) -> Self {
        self.intelligence_priority = Some(priority);
        self
    }
}

#[allow(deprecated)]
impl Default for ModelPreferences {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(deprecated)]
impl SamplingMessage {
    /// Creates a message with the given role and content and no `_meta`.
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

    /// Creates a user-role message holding a single text block.
    pub fn user_text(text: impl Into<String>) -> Self {
        Self::single(Role::User, SamplingMessageContentBlock::text(text))
    }

    /// Creates an assistant-role message holding a single text block.
    pub fn assistant_text(text: impl Into<String>) -> Self {
        Self::single(Role::Assistant, SamplingMessageContentBlock::text(text))
    }

    /// Sets the message's `_meta` object.
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
        assert_eq!(tc.mode, Some(ToolChoiceMode::Required));
    }

    #[test]
    fn test_tool_choice_mode_deserializes_required() {
        let json = serde_json::json!({"mode": "required"});
        let tc: ToolChoice = serde_json::from_value(json).unwrap();
        assert_eq!(tc.mode, Some(ToolChoiceMode::Required));
    }

    #[test]
    fn test_tool_choice_any_alias_returns_required() {
        let tc = ToolChoice::any();
        assert_eq!(tc.mode, Some(ToolChoiceMode::Required));
    }

    #[test]
    fn test_tool_choice_absent_mode_is_auto() {
        // Schema: mode is optional; absent means "auto".
        let tc: ToolChoice = serde_json::from_value(serde_json::json!({})).unwrap();
        assert_eq!(tc.mode, None);
        assert_eq!(tc.effective_mode(), ToolChoiceMode::Auto);
    }

    #[test]
    fn create_message_request_params_metadata_is_an_object() {
        // Schema: `metadata?: JSONObject` — this crate's convention for
        // JSONObject is `HashMap<String, Value>`, matching
        // `ClientCapabilities.experimental` and similar fields.
        let mut metadata = std::collections::HashMap::new();
        metadata.insert("traceId".to_string(), serde_json::json!("abc-123"));
        let params = CreateMessageRequestParams::new(vec![SamplingMessage::user_text("hi")], 100)
            .with_temperature(0.5);
        let mut params = params;
        params.metadata = Some(metadata);

        let v = serde_json::to_value(&params).unwrap();
        assert!(
            v["metadata"].is_object(),
            "metadata must serialize as a JSON object"
        );
        assert_eq!(v["metadata"]["traceId"], "abc-123");

        let back: CreateMessageRequestParams = serde_json::from_value(v).unwrap();
        assert_eq!(
            back.metadata.unwrap().get("traceId"),
            Some(&serde_json::json!("abc-123"))
        );
    }

    #[test]
    fn create_message_request_params_metadata_rejects_non_object_wire_value() {
        // A scalar `metadata` value must fail to deserialize now that the
        // field is a typed JSONObject, not an unrestricted `Value`.
        let wire = serde_json::json!({
            "messages": [],
            "maxTokens": 10,
            "metadata": "not-an-object"
        });
        let result: Result<CreateMessageRequestParams, _> = serde_json::from_value(wire);
        assert!(
            result.is_err(),
            "scalar metadata must be rejected per the JSONObject contract"
        );
    }

    fn tool_use(id: &str) -> SamplingMessageContentBlock {
        SamplingMessageContentBlock::ToolUse {
            id: id.to_string(),
            name: "get_weather".to_string(),
            input: std::collections::HashMap::new(),
            meta: None,
        }
    }

    fn tool_result(id: &str) -> SamplingMessageContentBlock {
        SamplingMessageContentBlock::ToolResult {
            tool_use_id: id.to_string(),
            content: vec![],
            structured_content: None,
            is_error: None,
            meta: None,
        }
    }

    #[test]
    fn valid_tool_use_tool_result_pairing_passes() {
        let params = CreateMessageRequestParams::new(
            vec![
                SamplingMessage::user_text("what's the weather?"),
                SamplingMessage::new(
                    Role::Assistant,
                    SamplingMessageContent::Single(tool_use("call-1")),
                ),
                SamplingMessage::new(
                    Role::User,
                    SamplingMessageContent::Single(tool_result("call-1")),
                ),
            ],
            100,
        );
        assert_eq!(params.validate_message_shape(), Ok(()));
    }

    #[test]
    fn user_message_mixing_text_and_tool_result_fails() {
        let params = CreateMessageRequestParams::new(
            vec![
                SamplingMessage::new(
                    Role::Assistant,
                    SamplingMessageContent::Single(tool_use("call-1")),
                ),
                SamplingMessage::new(
                    Role::User,
                    SamplingMessageContent::Multiple(vec![
                        SamplingMessageContentBlock::text("also here's some text"),
                        tool_result("call-1"),
                    ]),
                ),
            ],
            100,
        );
        assert!(
            params.validate_message_shape().is_err(),
            "a user message mixing text with a ToolResult block must be rejected"
        );
    }

    #[test]
    fn assistant_tool_use_not_followed_by_matching_tool_result_fails() {
        // No following message at all.
        let params = CreateMessageRequestParams::new(
            vec![SamplingMessage::new(
                Role::Assistant,
                SamplingMessageContent::Single(tool_use("call-1")),
            )],
            100,
        );
        assert!(params.validate_message_shape().is_err());

        // Following message is user but the ToolResult id doesn't match.
        let params = CreateMessageRequestParams::new(
            vec![
                SamplingMessage::new(
                    Role::Assistant,
                    SamplingMessageContent::Single(tool_use("call-1")),
                ),
                SamplingMessage::new(
                    Role::User,
                    SamplingMessageContent::Single(tool_result("call-DIFFERENT")),
                ),
            ],
            100,
        );
        assert!(
            params.validate_message_shape().is_err(),
            "a mismatched tool_use_id must be rejected"
        );

        // Following message is assistant, not user.
        let params = CreateMessageRequestParams::new(
            vec![
                SamplingMessage::new(
                    Role::Assistant,
                    SamplingMessageContent::Single(tool_use("call-1")),
                ),
                SamplingMessage::assistant_text("no tool result here"),
            ],
            100,
        );
        assert!(params.validate_message_shape().is_err());
    }
}
