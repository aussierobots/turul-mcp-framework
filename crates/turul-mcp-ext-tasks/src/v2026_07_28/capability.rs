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

/// Error from [`validate_identifier`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidExtensionIdentifier(pub String);

impl std::fmt::Display for InvalidExtensionIdentifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid extension identifier: {}", self.0)
    }
}

impl std::error::Error for InvalidExtensionIdentifier {}

/// Validate an extension identifier against the SEP-2133 naming rules:
/// a reverse-DNS prefix, a `/` separator, and a non-empty name segment
/// (e.g. `io.modelcontextprotocol/tasks`, `com.example/my-ext`).
pub fn validate_identifier(s: &str) -> Result<(), InvalidExtensionIdentifier> {
    let Some((prefix, name)) = s.split_once('/') else {
        return Err(InvalidExtensionIdentifier(format!(
            "{s:?} has no '/' separator"
        )));
    };
    if name.is_empty() {
        return Err(InvalidExtensionIdentifier(format!(
            "{s:?} has an empty name segment"
        )));
    }
    let labels: Vec<&str> = prefix.split('.').collect();
    if labels.len() < 2 || labels.iter().any(|l| l.is_empty()) {
        return Err(InvalidExtensionIdentifier(format!(
            "{s:?} prefix {prefix:?} is not reverse-DNS (need ≥2 non-empty dot-separated labels)"
        )));
    }
    Ok(())
}
