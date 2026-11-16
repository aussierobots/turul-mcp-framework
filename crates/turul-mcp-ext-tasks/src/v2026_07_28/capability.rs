//! Extension capability negotiation for `io.modelcontextprotocol/tasks`.
//!
//! Extensions are opt-in on both sides: a peer declares support by inserting
//! [`EXTENSION_IDENTIFIER`] into its `capabilities.extensions` map. The
//! capability value is an empty object — `TasksExtensionCapability` defines
//! no extension-specific settings.

use serde_json::Value;
use turul_mcp_protocol_2026_07_28::initialize::{ClientCapabilities, ServerCapabilities};

/// The Tasks extension identifier (SEP-2663).
pub const EXTENSION_IDENTIFIER: &str = "io.modelcontextprotocol/tasks";

/// The value to insert under [`EXTENSION_IDENTIFIER`] in a peer's
/// `capabilities.extensions` map — an empty object indicates support.
pub fn capability() -> Value {
    serde_json::json!({})
}

/// The `-32021` error for a request that needs the extension from a client
/// that did not declare it: `data.requiredCapabilities.extensions` names
/// [`EXTENSION_IDENTIFIER`] (tasks.md §Capability Negotiation).
pub fn missing_capability_error() -> turul_mcp_protocol_2026_07_28::McpError {
    turul_mcp_protocol_2026_07_28::McpError::MissingRequiredClientCapability {
        required: serde_json::json!({ "extensions": { EXTENSION_IDENTIFIER: capability() } }),
    }
}

/// True when the client declared the Tasks extension.
pub fn declared_by_client(caps: &ClientCapabilities) -> bool {
    caps.extensions
        .as_ref()
        .and_then(|m| m.get(EXTENSION_IDENTIFIER))
        .is_some()
}

/// True when the server declared the Tasks extension.
pub fn declared_by_server(caps: &ServerCapabilities) -> bool {
    caps.extensions
        .as_ref()
        .and_then(|m| m.get(EXTENSION_IDENTIFIER))
        .is_some()
}
