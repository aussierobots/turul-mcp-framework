//! Extension capability negotiation for `io.modelcontextprotocol/ui`.
//!
//! Clients advertise Apps support by inserting [`EXTENSION_IDENTIFIER`] into
//! `capabilities.extensions` with a [`UiClientCapabilities`] value whose
//! `mimeTypes` includes `text/html;profile=mcp-app`.

use turul_mcp_protocol_2026_07_28::initialize::ClientCapabilities;

use super::types::UiClientCapabilities;

/// The Apps extension identifier (SEP-1865). The upstream spec reserves the
/// label `io.modelcontextprotocol/ui`.
pub const EXTENSION_IDENTIFIER: &str = "io.modelcontextprotocol/ui";

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
