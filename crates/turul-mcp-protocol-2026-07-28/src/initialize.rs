//! Shared capability and implementation identity types for MCP DRAFT-2026-v1.
//!
//! DRAFT-2026-v1 is stateless (SEP-2567, SEP-2575) — there is no
//! `initialize`/`notifications/initialized` handshake. These types
//! ([`Implementation`], [`ClientCapabilities`], [`ServerCapabilities`])
//! survive because they are referenced by
//! [`crate::meta::RequestMetaObject`] (per-request negotiation) and
//! [`crate::discover::DiscoverResult`].

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Describes the name and version of an MCP implementation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Implementation {
    /// Machine-readable name
    pub name: String,
    /// Version string (e.g., "1.0.0")
    pub version: String,
    /// Optional human-friendly display title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Optional human-readable description of this implementation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Optional URL for the implementation's website
    #[serde(skip_serializing_if = "Option::is_none")]
    pub website_url: Option<String>,
    /// Optional icons for display. Most implementations do not need icons.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icons: Option<Vec<crate::icons::Icon>>,
}

impl Implementation {
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            title: None,
            description: None,
            website_url: None,
            icons: None,
        }
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_website_url(mut self, url: impl Into<String>) -> Self {
        self.website_url = Some(url.into());
        self
    }

    pub fn with_icons(mut self, icons: Vec<crate::icons::Icon>) -> Self {
        self.icons = Some(icons);
        self
    }
}

/// Capabilities related to root listing support.
///
/// **Deprecated** per SEP-2577 alongside the Roots feature.
#[deprecated(
    since = "0.4.0",
    note = "Deprecated per SEP-2577 (DRAFT-2026-v1). \
            Replacement: pass directories or files via tool parameters, resource URIs, or server configuration. \
            Earliest removal: first release on/after 2027-07-28."
)]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RootsCapabilities {
    /// Whether the client supports notifications for root list changes.
    /// Note: `notifications/roots/list_changed` was REMOVED in DRAFT-2026-v1;
    /// declaring `listChanged: true` has no effect.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

/// Capabilities related to sampling support.
///
/// DRAFT-2026-v1 adds two named sub-capabilities:
/// - `context` — client supports `includeContext` parameter (soft-deprecated)
/// - `tools`   — client supports `tools` and `toolChoice` parameters
///
/// Presence of the parent `sampling` field indicates baseline sampling support;
/// presence of a sub-field declares the specific sub-capability. Empty `{}` is valid.
///
/// **Deprecated** per SEP-2577 alongside the Sampling feature.
#[deprecated(
    since = "0.4.0",
    note = "Deprecated per SEP-2577 (DRAFT-2026-v1). \
            Replacement: integrate directly with LLM provider APIs. \
            Earliest removal: first release on/after 2027-07-28."
)]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SamplingCapabilities {
    /// Whether the client supports context inclusion via `includeContext`.
    /// Server MAY use `includeContext: "thisServer"`/`"allServers"` only if this
    /// is declared.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<HashMap<String, Value>>,

    /// Whether the client supports tool use in sampling.
    /// Server MUST get an error if it sends `tools`/`toolChoice` without this
    /// declared.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<HashMap<String, Value>>,

    /// Additional forward-compatible capability data.
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

/// Capabilities related to elicitation support.
///
/// DRAFT-2026-v1 adds two named sub-capabilities:
/// - `form` — client supports form-mode elicitation
/// - `url`  — client supports URL-mode elicitation
///
/// Presence of the parent `elicitation` field indicates baseline elicitation
/// support. Empty `{}` is valid (implicit form mode).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ElicitationCapabilities {
    /// Form-mode elicitation support.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub form: Option<HashMap<String, Value>>,

    /// URL-mode elicitation support.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<HashMap<String, Value>>,

    /// Additional forward-compatible capability data.
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

/// Capabilities that a client may support.
///
/// DRAFT-2026-v1 shape:
/// - `experimental?: { [k]: JSONObject }`
/// - `roots?: {}`                — presence means client supports listing roots
/// - `sampling?: { context?, tools? }`
/// - `elicitation?: { form?, url? }`
/// - `extensions?: { [k]: JSONObject }`  — reverse-DNS keyed extension capability map
///
/// Note: `tasks` field is NOT present — tasks moved entirely to extension
/// in DRAFT-2026-v1 (SEP-2663). Advertise tasks support via
/// `extensions["io.modelcontextprotocol/tasks"]`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
#[allow(deprecated)]
pub struct ClientCapabilities {
    /// Root directory capabilities. **Deprecated** per SEP-2577.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roots: Option<RootsCapabilities>,
    /// Sampling capabilities (client can handle sampling requests from server).
    /// **Deprecated** per SEP-2577.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sampling: Option<SamplingCapabilities>,
    /// Elicitation capabilities (client can handle elicitation requests from server).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub elicitation: Option<ElicitationCapabilities>,
    /// Experimental capabilities.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental: Option<HashMap<String, Value>>,
    /// MCP extensions the client supports.
    /// Keys are reverse-DNS extension identifiers (e.g.
    /// `"io.modelcontextprotocol/oauth-client-credentials"`); values are
    /// per-extension settings. Empty `{}` means support without settings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<HashMap<String, Value>>,
}

/// Capabilities for prompts provided by the server
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PromptsCapabilities {
    /// Whether the server supports prompt list change notifications
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

/// Capabilities for tools provided by the server
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ToolsCapabilities {
    /// Whether the server supports tool list change notifications
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

// Tasks capabilities live in the tasks extension (SEP-2663). Advertise via
// `extensions["io.modelcontextprotocol/tasks"]` on `ServerCapabilities`.

/// Capabilities for resources provided by the server
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ResourcesCapabilities {
    /// Whether the server supports resource subscriptions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscribe: Option<bool>,
    /// Whether the server supports resource list change notifications
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

/// Capabilities for logging provided by the server
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LoggingCapabilities {
    /// Whether the server supports logging
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    /// Supported log levels
    #[serde(skip_serializing_if = "Option::is_none")]
    pub levels: Option<Vec<String>>,
}

/// Capabilities for completions provided by the server
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CompletionsCapabilities {
    /// Whether the server supports completions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
}

/// Capabilities that a server may support.
///
/// DRAFT-2026-v1 shape:
/// - `experimental?: { [k]: JSONObject }`
/// - `logging?: JSONObject`            — opaque object; presence means server can send `notifications/message`
/// - `completions?: JSONObject`        — opaque object; presence means `completion/complete` is supported
/// - `prompts?: { listChanged? }`
/// - `resources?: { subscribe?, listChanged? }`
/// - `tools?: { listChanged? }`
/// - `extensions?: { [k]: JSONObject }`  — reverse-DNS keyed extension capability map
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ServerCapabilities {
    /// Logging capabilities.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logging: Option<LoggingCapabilities>,
    /// Completion capabilities.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completions: Option<CompletionsCapabilities>,
    /// Prompt capabilities.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts: Option<PromptsCapabilities>,
    /// Resource capabilities.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<ResourcesCapabilities>,
    /// Tool capabilities.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<ToolsCapabilities>,
    /// Experimental capabilities.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental: Option<HashMap<String, Value>>,
    /// MCP extensions the server supports.
    /// Keys are reverse-DNS extension identifiers (e.g.
    /// `"io.modelcontextprotocol/apps"`); values are per-extension settings.
    /// Empty `{}` means support without settings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<HashMap<String, Value>>,
}

// DRAFT-2026-v1 is stateless (SEP-2567, SEP-2575) — there is no initialize
// handshake. Client info and capabilities travel in `RequestMetaObject` on
// every request; server info and capabilities come from `DiscoverResult`
// (see [`crate::discover`]).

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_implementation_creation() {
        let impl_info = Implementation::new("test-client", "1.0.0").with_title("Test Client");

        assert_eq!(impl_info.name, "test-client");
        assert_eq!(impl_info.version, "1.0.0");
        assert_eq!(impl_info.title, Some("Test Client".to_string()));
    }
}
