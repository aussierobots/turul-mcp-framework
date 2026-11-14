//! Capability negotiation for `io.modelcontextprotocol/skills` (`skills.mdx`
//! §Capability Negotiation). [`capability`] is the only producer of the value a
//! server places under [`EXTENSION_IDENTIFIER`] (ADR-033 §1).

use serde::{Deserialize, Serialize};
use serde_json::Value;
use turul_mcp_protocol_2026_07_28::initialize::ServerCapabilities;

/// The Skills extension identifier.
pub const EXTENSION_IDENTIFIER: &str = "io.modelcontextprotocol/skills";

/// The extension's settings object.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillsCapability {
    /// The server implements `resources/directory/read`. Defaults to `false`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub directory_read: bool,
}

/// The value to insert under [`EXTENSION_IDENTIFIER`] in
/// `ServerCapabilities.extensions`. An empty object means no optional features.
pub fn capability(settings: SkillsCapability) -> Value {
    serde_json::to_value(settings).expect("SkillsCapability serializes")
}

/// The server's Skills settings, or `None` when it did not declare the extension
/// (or declared a value that is not a settings object).
pub fn declared_by_server(caps: &ServerCapabilities) -> Option<SkillsCapability> {
    let value = caps.extensions.as_ref()?.get(EXTENSION_IDENTIFIER)?;
    serde_json::from_value(value.clone()).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// ADR-033 §1 / S-G1: the capability value has one producer and matches
    /// the spec's two shapes.
    #[test]
    fn s_g1_capability_value_is_owned() {
        assert_eq!(capability(SkillsCapability::default()), json!({}));
        assert_eq!(
            capability(SkillsCapability {
                directory_read: true
            }),
            json!({ "directoryRead": true })
        );
    }

    #[test]
    fn declared_by_server_round_trip() {
        let mut caps: ServerCapabilities = serde_json::from_value(json!({})).unwrap();
        assert_eq!(declared_by_server(&caps), None);
        caps.extensions = Some(
            [(
                EXTENSION_IDENTIFIER.to_string(),
                json!({ "directoryRead": true }),
            )]
            .into_iter()
            .collect(),
        );
        assert_eq!(
            declared_by_server(&caps),
            Some(SkillsCapability {
                directory_read: true
            })
        );
    }
}
