//! MCP Protocol Version Support
//!
//! This module defines the supported MCP protocol versions and their capabilities.
//!
//! ## Version History
//! - **2024-11-05**: Initial MCP specification with HTTP+SSE transport
//! - **2025-03-26**: Introduced Streamable HTTP, OAuth 2.1 authorization, tool annotations
//! - **2025-06-18**: Added Elicitation, Tool Output Schemas, enhanced _meta fields

use serde::{Deserialize, Serialize};

/// Supported MCP protocol versions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum McpVersion {
    /// Original protocol without streamable HTTP (introduced 2024-11-05)
    #[serde(rename = "2024-11-05")]
    V2024_11_05,
    /// Protocol including streamable HTTP (introduced 2025-03-26)  
    #[serde(rename = "2025-03-26")]
    V2025_03_26,
    /// Protocol with structured _meta, cursor, progressToken, and elicitation (introduced 2025-06-18)
    #[serde(rename = "2025-06-18")]
    V2025_06_18,
}

impl McpVersion {
    /// Parse a version string like "2024-11-05", "2025-03-26", or "2025-06-18"
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "2024-11-05" => Some(McpVersion::V2024_11_05),
            "2025-03-26" => Some(McpVersion::V2025_03_26),
            "2025-06-18" => Some(McpVersion::V2025_06_18),
            _ => None,
        }
    }

    /// Convert this version to its string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            McpVersion::V2024_11_05 => "2024-11-05",
            McpVersion::V2025_03_26 => "2025-03-26",
            McpVersion::V2025_06_18 => "2025-06-18",
        }
    }

    /// Returns whether this version supports streamable HTTP (SSE)
    pub fn supports_streamable_http(&self) -> bool {
        matches!(self, McpVersion::V2025_03_26 | McpVersion::V2025_06_18)
    }

    /// Returns whether this version supports `_meta` fields in requests, responses, and notifications
    pub fn supports_meta_fields(&self) -> bool {
        matches!(self, McpVersion::V2025_06_18)
    }

    /// Returns whether this version supports the use of `progressToken` and `cursor` in `_meta`
    pub fn supports_progress_and_cursor(&self) -> bool {
        matches!(self, McpVersion::V2025_06_18)
    }

    /// Returns whether this version supports structured user elicitation via JSON Schema
    pub fn supports_elicitation(&self) -> bool {
        matches!(self, McpVersion::V2025_06_18)
    }

    /// Get a list of feature names supported by this version
    pub fn supported_features(&self) -> Vec<&'static str> {
        let mut features = vec![];
        if self.supports_streamable_http() {
            features.push("streamable-http");
        }
        if self.supports_meta_fields() {
            features.push("_meta-fields");
        }
        if self.supports_progress_and_cursor() {
            features.push("progress-token");
            features.push("cursor");
        }
        if self.supports_elicitation() {
            features.push("elicitation");
        }
        features
    }

    /// The latest protocol version implemented by this crate
    pub const LATEST: McpVersion = McpVersion::V2025_06_18;

    /// The current protocol version implemented by this crate
    pub const CURRENT: McpVersion = McpVersion::V2025_06_18;
}

impl std::fmt::Display for McpVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for McpVersion {
    type Err = crate::McpError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_str(s).ok_or_else(|| crate::McpError::VersionMismatch {
            expected: Self::CURRENT.as_str().to_string(),
            actual: s.to_string(),
        })
    }
}

impl Default for McpVersion {
    fn default() -> Self {
        Self::CURRENT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parsing() {
        assert_eq!(McpVersion::from_str("2024-11-05"), Some(McpVersion::V2024_11_05));
        assert_eq!(McpVersion::from_str("2025-03-26"), Some(McpVersion::V2025_03_26));
        assert_eq!(McpVersion::from_str("2025-06-18"), Some(McpVersion::V2025_06_18));
        assert_eq!(McpVersion::from_str("invalid"), None);
    }

    #[test]
    fn test_version_string_conversion() {
        assert_eq!(McpVersion::V2024_11_05.as_str(), "2024-11-05");
        assert_eq!(McpVersion::V2025_03_26.as_str(), "2025-03-26");
        assert_eq!(McpVersion::V2025_06_18.as_str(), "2025-06-18");
    }

    #[test]
    fn test_capabilities() {
        let v2024 = McpVersion::V2024_11_05;
        assert!(!v2024.supports_streamable_http());
        assert!(!v2024.supports_meta_fields());
        assert!(!v2024.supports_progress_and_cursor());
        assert!(!v2024.supports_elicitation());

        let v2025_03 = McpVersion::V2025_03_26;
        assert!(v2025_03.supports_streamable_http());
        assert!(!v2025_03.supports_meta_fields());
        assert!(!v2025_03.supports_progress_and_cursor());
        assert!(!v2025_03.supports_elicitation());

        let v2025_06 = McpVersion::V2025_06_18;
        assert!(v2025_06.supports_streamable_http());
        assert!(v2025_06.supports_meta_fields());
        assert!(v2025_06.supports_progress_and_cursor());
        assert!(v2025_06.supports_elicitation());
    }

    #[test]
    fn test_feature_list() {
        let features = McpVersion::V2025_06_18.supported_features();
        assert!(features.contains(&"streamable-http"));
        assert!(features.contains(&"_meta-fields"));
        assert!(features.contains(&"progress-token"));
        assert!(features.contains(&"cursor"));
        assert!(features.contains(&"elicitation"));
    }
}