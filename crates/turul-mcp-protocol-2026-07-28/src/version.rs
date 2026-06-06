//! MCP Protocol Version Support
//!
//! This module defines the supported MCP protocol versions and their capabilities.
//!
//! ## Version History
//! - **2024-11-05**: Initial MCP specification with HTTP+SSE transport
//! - **2025-03-26**: Introduced Streamable HTTP, OAuth 2.1 authorization, tool annotations
//! - **2025-06-18**: Added Elicitation, Tool Output Schemas, enhanced _meta fields
//! - **2025-11-25**: Added Tasks (experimental), Icons, URL elicitation, sampling tools
//! - **2026-07-28**: Stateless core (no initialize/Mcp-Session-Id), `server/discover`,
//!   `_meta.io.modelcontextprotocol/clientInfo`, `InputRequiredResult` for elicitation,
//!   routing/caching/tracing headers, extensions framework (Tasks demoted to extension),
//!   JSON Schema 2020-12 input/output schemas, error code `-32002` → `-32602`,
//!   Roots/Sampling/Logging deprecated (12-month window).

use serde::{Deserialize, Serialize};

/// Supported MCP protocol versions
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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
    /// Protocol with tasks, icons, URL elicitation, and sampling tools (introduced 2025-11-25)
    #[serde(rename = "2025-11-25")]
    V2025_11_25,
    /// Protocol with stateless core, extensions framework, JSON Schema 2020-12 (introduced 2026-07-28)
    ///
    /// The pre-finalization draft emitted `"DRAFT-2026-v1"`; the finalized schema
    /// emits `"2026-07-28"`. We serialize the finalized literal and accept the
    /// draft literal on deserialize for back-compat.
    #[serde(rename = "2026-07-28", alias = "DRAFT-2026-v1")]
    V2026_07_28,
}

impl McpVersion {
    /// Convert this version to its string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            McpVersion::V2024_11_05 => "2024-11-05",
            McpVersion::V2025_03_26 => "2025-03-26",
            McpVersion::V2025_06_18 => "2025-06-18",
            McpVersion::V2025_11_25 => "2025-11-25",
            McpVersion::V2026_07_28 => "2026-07-28",
        }
    }

    /// Returns whether this version supports streamable HTTP (SSE)
    pub fn supports_streamable_http(&self) -> bool {
        matches!(
            self,
            McpVersion::V2025_03_26
                | McpVersion::V2025_06_18
                | McpVersion::V2025_11_25
                | McpVersion::V2026_07_28
        )
    }

    /// Returns whether this version supports `_meta` fields in requests, responses, and notifications
    pub fn supports_meta_fields(&self) -> bool {
        matches!(
            self,
            McpVersion::V2025_06_18 | McpVersion::V2025_11_25 | McpVersion::V2026_07_28
        )
    }

    /// Returns whether this version supports the use of `progressToken` and `cursor` in `_meta`
    pub fn supports_progress_and_cursor(&self) -> bool {
        matches!(
            self,
            McpVersion::V2025_06_18 | McpVersion::V2025_11_25 | McpVersion::V2026_07_28
        )
    }

    /// Returns whether this version supports structured user elicitation via JSON Schema
    pub fn supports_elicitation(&self) -> bool {
        matches!(
            self,
            McpVersion::V2025_06_18 | McpVersion::V2025_11_25 | McpVersion::V2026_07_28
        )
    }

    /// Returns whether this version supports the task system *in the core protocol*.
    ///
    /// In 2025-11-25 tasks were experimental in the core spec. In DRAFT-2026-v1
    /// tasks graduated to an official extension (SEP-2663) and are NO LONGER in
    /// the core schema. Servers that want tasks must advertise the extension via
    /// `ServerCapabilities.extensions`; this flag therefore reads `false` for
    /// V2026_07_28 in the core protocol crate.
    pub fn supports_tasks(&self) -> bool {
        matches!(self, McpVersion::V2025_11_25)
    }

    /// Returns whether this version supports icons on tools, resources, prompts, and implementation
    pub fn supports_icons(&self) -> bool {
        matches!(self, McpVersion::V2025_11_25 | McpVersion::V2026_07_28)
    }

    /// Returns whether this version supports URL mode elicitation
    pub fn supports_url_elicitation(&self) -> bool {
        matches!(self, McpVersion::V2025_11_25 | McpVersion::V2026_07_28)
    }

    /// Returns whether this version supports tools in sampling requests
    pub fn supports_sampling_tools(&self) -> bool {
        matches!(self, McpVersion::V2025_11_25 | McpVersion::V2026_07_28)
    }

    /// Returns whether this version is stateless at the protocol level
    /// (no `initialize`/`Mcp-Session-Id` handshake; client info and capabilities
    /// travel in `_meta` on every request — see SEP-2567, SEP-2575).
    pub fn is_stateless(&self) -> bool {
        matches!(self, McpVersion::V2026_07_28)
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
        if self.supports_tasks() {
            features.push("tasks");
        }
        if self.supports_icons() {
            features.push("icons");
        }
        if self.supports_url_elicitation() {
            features.push("url-elicitation");
        }
        if self.supports_sampling_tools() {
            features.push("sampling-tools");
        }
        if self.is_stateless() {
            features.push("stateless-core");
        }
        features
    }

    /// The latest protocol version implemented by this crate
    pub const LATEST: McpVersion = McpVersion::V2026_07_28;

    /// The current protocol version implemented by this crate
    pub const CURRENT: McpVersion = McpVersion::V2026_07_28;
}

impl std::fmt::Display for McpVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for McpVersion {
    type Err = crate::McpError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "2024-11-05" => Ok(McpVersion::V2024_11_05),
            "2025-03-26" => Ok(McpVersion::V2025_03_26),
            "2025-06-18" => Ok(McpVersion::V2025_06_18),
            "2025-11-25" => Ok(McpVersion::V2025_11_25),
            "2026-07-28" | "DRAFT-2026-v1" => Ok(McpVersion::V2026_07_28),
            _ => Err(crate::McpError::VersionMismatch {
                expected: Self::CURRENT.as_str().to_string(),
                actual: s.to_string(),
            }),
        }
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
        assert_eq!(
            "2024-11-05".parse::<McpVersion>().unwrap(),
            McpVersion::V2024_11_05
        );
        assert_eq!(
            "2025-03-26".parse::<McpVersion>().unwrap(),
            McpVersion::V2025_03_26
        );
        assert_eq!(
            "2025-06-18".parse::<McpVersion>().unwrap(),
            McpVersion::V2025_06_18
        );
        assert_eq!(
            "2025-11-25".parse::<McpVersion>().unwrap(),
            McpVersion::V2025_11_25
        );
        assert_eq!(
            "2026-07-28".parse::<McpVersion>().unwrap(),
            McpVersion::V2026_07_28
        );
        // Draft literal still parses for back-compat (deserialize-only alias).
        assert_eq!(
            "DRAFT-2026-v1".parse::<McpVersion>().unwrap(),
            McpVersion::V2026_07_28
        );
        assert!("invalid".parse::<McpVersion>().is_err());
    }

    #[test]
    fn test_version_string_conversion() {
        assert_eq!(McpVersion::V2024_11_05.as_str(), "2024-11-05");
        assert_eq!(McpVersion::V2025_03_26.as_str(), "2025-03-26");
        assert_eq!(McpVersion::V2025_06_18.as_str(), "2025-06-18");
        assert_eq!(McpVersion::V2025_11_25.as_str(), "2025-11-25");
        assert_eq!(McpVersion::V2026_07_28.as_str(), "2026-07-28");
    }

    #[test]
    fn test_capabilities() {
        let v2024 = McpVersion::V2024_11_05;
        assert!(!v2024.supports_streamable_http());
        assert!(!v2024.supports_meta_fields());
        assert!(!v2024.supports_progress_and_cursor());
        assert!(!v2024.supports_elicitation());
        assert!(!v2024.supports_tasks());
        assert!(!v2024.supports_icons());
        assert!(!v2024.supports_url_elicitation());
        assert!(!v2024.supports_sampling_tools());
        assert!(!v2024.is_stateless());

        let v2025_03 = McpVersion::V2025_03_26;
        assert!(v2025_03.supports_streamable_http());
        assert!(!v2025_03.supports_meta_fields());
        assert!(!v2025_03.supports_progress_and_cursor());
        assert!(!v2025_03.supports_elicitation());
        assert!(!v2025_03.supports_tasks());
        assert!(!v2025_03.supports_icons());
        assert!(!v2025_03.is_stateless());

        let v2025_06 = McpVersion::V2025_06_18;
        assert!(v2025_06.supports_streamable_http());
        assert!(v2025_06.supports_meta_fields());
        assert!(v2025_06.supports_progress_and_cursor());
        assert!(v2025_06.supports_elicitation());
        assert!(!v2025_06.supports_tasks());
        assert!(!v2025_06.supports_icons());
        assert!(!v2025_06.is_stateless());

        let v2025_11 = McpVersion::V2025_11_25;
        assert!(v2025_11.supports_streamable_http());
        assert!(v2025_11.supports_meta_fields());
        assert!(v2025_11.supports_progress_and_cursor());
        assert!(v2025_11.supports_elicitation());
        assert!(v2025_11.supports_tasks());
        assert!(v2025_11.supports_icons());
        assert!(v2025_11.supports_url_elicitation());
        assert!(v2025_11.supports_sampling_tools());
        assert!(!v2025_11.is_stateless());

        let v2026_07 = McpVersion::V2026_07_28;
        assert!(v2026_07.supports_streamable_http());
        assert!(v2026_07.supports_meta_fields());
        assert!(v2026_07.supports_progress_and_cursor());
        assert!(v2026_07.supports_elicitation());
        // Tasks moved to extension in DRAFT-2026-v1 (SEP-2663); core no longer supports.
        assert!(!v2026_07.supports_tasks());
        assert!(v2026_07.supports_icons());
        assert!(v2026_07.supports_url_elicitation());
        assert!(v2026_07.supports_sampling_tools());
        assert!(v2026_07.is_stateless());
    }

    #[test]
    fn test_feature_list() {
        let features = McpVersion::V2026_07_28.supported_features();
        assert!(features.contains(&"streamable-http"));
        assert!(features.contains(&"_meta-fields"));
        assert!(features.contains(&"progress-token"));
        assert!(features.contains(&"cursor"));
        assert!(features.contains(&"elicitation"));
        // Tasks moved to extension in DRAFT-2026-v1; not in core feature list.
        assert!(!features.contains(&"tasks"));
        assert!(features.contains(&"icons"));
        assert!(features.contains(&"url-elicitation"));
        assert!(features.contains(&"sampling-tools"));
        assert!(features.contains(&"stateless-core"));
    }

    #[test]
    fn test_current_is_2026_07_28() {
        assert_eq!(McpVersion::CURRENT, McpVersion::V2026_07_28);
        assert_eq!(McpVersion::LATEST, McpVersion::V2026_07_28);
        assert_eq!(McpVersion::default(), McpVersion::V2026_07_28);
    }
}
