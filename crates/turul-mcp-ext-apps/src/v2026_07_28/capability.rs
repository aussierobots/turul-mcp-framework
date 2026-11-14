//! Extension capability negotiation for `io.modelcontextprotocol/ui`.
//!
//! Clients advertise Apps support by inserting [`EXTENSION_IDENTIFIER`] into
//! `capabilities.extensions` with a [`UiClientCapabilities`] value whose
//! `mimeTypes` includes `text/html;profile=mcp-app`. The Apps spec defines no
//! server settings, so a server declares support with [`capability`] (`{}`).

use serde_json::Value;
use turul_mcp_protocol_2026_07_28::initialize::{ClientCapabilities, ServerCapabilities};

use super::types::UiClientCapabilities;

/// The Apps extension identifier (SEP-1865). The upstream spec reserves the
/// label `io.modelcontextprotocol/ui`.
pub const EXTENSION_IDENTIFIER: &str = "io.modelcontextprotocol/ui";

/// The value a server inserts under [`EXTENSION_IDENTIFIER`] in its
/// `capabilities.extensions` map. The core schema reads an empty object as
/// "support with no settings"; the Apps spec defines settings for clients only.
pub fn capability() -> Value {
    serde_json::json!({})
}

/// True when the server declared the Apps extension.
pub fn declared_by_server(caps: &ServerCapabilities) -> bool {
    caps.extensions
        .as_ref()
        .is_some_and(|m| m.contains_key(EXTENSION_IDENTIFIER))
}

/// The client's declared Apps capability, when present.
pub fn declared_by_client(caps: &ClientCapabilities) -> Option<UiClientCapabilities> {
    caps.extensions
        .as_ref()
        .and_then(|m| m.get(EXTENSION_IDENTIFIER))
        .and_then(|v| serde_json::from_value(v.clone()).ok())
}

/// True when the client declared Apps support for HTML views.
pub fn client_supports_html_views(caps: &ClientCapabilities) -> bool {
    declared_by_client(caps).is_some_and(|c| c.supports_html_views())
}
