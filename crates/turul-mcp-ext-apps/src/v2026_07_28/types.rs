//! MCP-side types for the Apps extension (SEP-1865).
//!
//! Maps to the vendored `schema/apps-2026-01-26.mdx` (normative) and its
//! machine-readable companion `schema/spec.types.ts`:
//! - `McpUiClientCapabilities` → [`UiClientCapabilities`]
//! - `McpUiToolVisibility`     → [`UiToolVisibility`]
//! - `McpUiToolMeta`           → [`UiToolMeta`] (tool `_meta.ui`)
//! - `McpUiResourceMeta`       → [`UiResourceMeta`] (UI-resource `_meta.ui`)
//! - `McpUiResourceCsp`        → [`UiResourceCsp`]
//! - `McpUiResourcePermissions`→ [`UiResourcePermissions`]

use serde::{Deserialize, Serialize};

/// The MIME profile a client MUST list in its declared `mimeTypes` to
/// support MCP Apps HTML views.
pub const MCP_APP_HTML_MIME: &str = "text/html;profile=mcp-app";

/// `_meta` key carrying UI metadata on tools and UI resources.
/// (The flat `_meta["ui/resourceUri"]` form is deprecated upstream and is
/// not modeled by this binding.)
pub const META_KEY_UI: &str = "ui";

/// Apps capability settings a client advertises under the extension
/// identifier in `capabilities.extensions` — `McpUiClientCapabilities`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct UiClientCapabilities {
    /// Supported MIME types for UI resources. Must include
    /// [`MCP_APP_HTML_MIME`] for MCP Apps support.
    #[serde(rename = "mimeTypes", skip_serializing_if = "Option::is_none")]
    pub mime_types: Option<Vec<String>>,
}

impl UiClientCapabilities {
    /// True when the client supports MCP Apps HTML views.
    pub fn supports_html_views(&self) -> bool {
        self.mime_types
            .as_ref()
            .is_some_and(|m| m.iter().any(|t| t == MCP_APP_HTML_MIME))
    }
}

/// Who can access a tool — `McpUiToolVisibility`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UiToolVisibility {
    /// Tool visible to and callable by the agent.
    Model,
    /// Tool callable by the app from this server only.
    App,
}

/// UI metadata for tools, carried in `Tool._meta.ui` — `McpUiToolMeta`.
/// `csp`/`permissions` belong on the UI **resource**, not the tool.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct UiToolMeta {
    /// URI of the UI resource to display for this tool (e.g.
    /// `ui://weather/view.html`).
    #[serde(rename = "resourceUri", skip_serializing_if = "Option::is_none")]
    pub resource_uri: Option<String>,

    /// Access scope. Default when absent: `["model", "app"]`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visibility: Option<Vec<UiToolVisibility>>,
}

/// Content Security Policy configuration for a UI resource —
/// `McpUiResourceCsp`. Empty/omitted lists are the secure default
/// (no network access of that class).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct UiResourceCsp {
    /// Origins for network requests (fetch/XHR/WebSocket) → `connect-src`.
    #[serde(rename = "connectDomains", skip_serializing_if = "Option::is_none")]
    pub connect_domains: Option<Vec<String>>,
    /// Origins for static resources → `img-src`/`script-src`/`style-src`/
    /// `font-src`/`media-src`. Wildcard subdomains supported.
    #[serde(rename = "resourceDomains", skip_serializing_if = "Option::is_none")]
    pub resource_domains: Option<Vec<String>>,
    /// Origins for nested iframes → `frame-src`.
    #[serde(rename = "frameDomains", skip_serializing_if = "Option::is_none")]
    pub frame_domains: Option<Vec<String>>,
    /// Allowed base URIs → `base-uri`.
    #[serde(rename = "baseUriDomains", skip_serializing_if = "Option::is_none")]
    pub base_uri_domains: Option<Vec<String>>,
}

/// Sandbox permissions a UI resource requests — `McpUiResourcePermissions`.
/// Hosts MAY honor these via iframe `allow` attributes; apps SHOULD feature-
/// detect rather than assume grants. Each present key is a strictly empty
/// object on the wire (the generated upstream schema declares
/// `{type: "object", properties: {}, additionalProperties: false}` per key).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct UiResourcePermissions {
    /// Permission Policy `camera`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub camera: Option<EmptyObject>,
    /// Permission Policy `microphone`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub microphone: Option<EmptyObject>,
    /// Permission Policy `geolocation`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub geolocation: Option<EmptyObject>,
    /// Permission Policy `clipboard-write`.
    #[serde(rename = "clipboardWrite", skip_serializing_if = "Option::is_none")]
    pub clipboard_write: Option<EmptyObject>,
}

/// Presence marker that only accepts `{}` on the wire — non-objects and
/// objects with any member are rejected, matching the upstream schema's
/// `additionalProperties: false` empty-object permission values.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmptyObject {}

/// UI metadata for a UI resource, carried in `_meta.ui` on the resource
/// declaration and on each `resources/read` content item —
/// `McpUiResourceMeta`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct UiResourceMeta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub csp: Option<UiResourceCsp>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<UiResourcePermissions>,
    /// Dedicated sandbox origin (host-dependent format).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    /// Visual boundary preference — `true` requests a visible border +
    /// background; hosts' defaults vary, so setting it is recommended.
    #[serde(rename = "prefersBorder", skip_serializing_if = "Option::is_none")]
    pub prefers_border: Option<bool>,
}
