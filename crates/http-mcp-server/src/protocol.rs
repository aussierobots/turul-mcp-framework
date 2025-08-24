//! MCP Protocol Version Detection and Features
//!
//! This module handles MCP protocol version detection from HTTP headers
//! and provides feature flags for different protocol versions.

/// Supported MCP protocol versions and features
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpProtocolVersion {
    /// Original protocol without streamable HTTP (introduced 2024-11-05)
    V2024_11_05,
    /// Protocol including streamable HTTP (introduced 2025-03-26)
    V2025_03_26,
    /// Protocol with structured _meta, cursor, progressToken, and elicitation (introduced 2025-06-18)
    V2025_06_18,
}

impl McpProtocolVersion {
    /// Parses a version string like "2024-11-05", "2025-03-26", or "2025-06-18".
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "2024-11-05" => Some(McpProtocolVersion::V2024_11_05),
            "2025-03-26" => Some(McpProtocolVersion::V2025_03_26),
            "2025-06-18" => Some(McpProtocolVersion::V2025_06_18),
            _ => None,
        }
    }

    /// Converts this version to its string representation.
    pub fn to_string(&self) -> &'static str {
        match self {
            McpProtocolVersion::V2024_11_05 => "2024-11-05",
            McpProtocolVersion::V2025_03_26 => "2025-03-26",
            McpProtocolVersion::V2025_06_18 => "2025-06-18",
        }
    }

    /// Returns whether this version supports streamable HTTP (SSE).
    pub fn supports_streamable_http(&self) -> bool {
        matches!(self, McpProtocolVersion::V2025_03_26 | McpProtocolVersion::V2025_06_18)
    }

    /// Returns whether this version supports `_meta` fields in requests, responses, and notifications.
    pub fn supports_meta_fields(&self) -> bool {
        matches!(self, McpProtocolVersion::V2025_06_18)
    }

    /// Returns whether this version supports the use of `progressToken` and `cursor` in `_meta`.
    pub fn supports_progress_and_cursor(&self) -> bool {
        matches!(self, McpProtocolVersion::V2025_06_18)
    }

    /// Returns whether this version supports structured user elicitation via JSON Schema.
    pub fn supports_elicitation(&self) -> bool {
        matches!(self, McpProtocolVersion::V2025_06_18)
    }

    /// Get a list of supported features for this protocol version
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

    /// The latest protocol version this server implements.
    pub const LATEST: McpProtocolVersion = McpProtocolVersion::V2025_06_18;
}

impl Default for McpProtocolVersion {
    fn default() -> Self {
        Self::LATEST
    }
}

impl std::fmt::Display for McpProtocolVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

/// Extract MCP protocol version from HTTP request headers
pub fn extract_protocol_version(headers: &hyper::HeaderMap) -> McpProtocolVersion {
    headers
        .get("MCP-Protocol-Version")
        .and_then(|h| h.to_str().ok())
        .and_then(McpProtocolVersion::from_str)
        .unwrap_or(McpProtocolVersion::LATEST)
}

/// Extract MCP session ID from HTTP request headers
pub fn extract_session_id(headers: &hyper::HeaderMap) -> Option<String> {
    headers
        .get("Mcp-Session-Id")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::HeaderMap;

    #[test]
    fn test_version_parsing() {
        assert_eq!(McpProtocolVersion::from_str("2024-11-05"), Some(McpProtocolVersion::V2024_11_05));
        assert_eq!(McpProtocolVersion::from_str("2025-03-26"), Some(McpProtocolVersion::V2025_03_26));
        assert_eq!(McpProtocolVersion::from_str("2025-06-18"), Some(McpProtocolVersion::V2025_06_18));
        assert_eq!(McpProtocolVersion::from_str("invalid"), None);
    }

    #[test]
    fn test_version_features() {
        let v2024 = McpProtocolVersion::V2024_11_05;
        assert!(!v2024.supports_streamable_http());
        assert!(!v2024.supports_meta_fields());
        assert!(!v2024.supports_elicitation());

        let v2025_03 = McpProtocolVersion::V2025_03_26;
        assert!(v2025_03.supports_streamable_http());
        assert!(!v2025_03.supports_meta_fields());

        let v2025_06 = McpProtocolVersion::V2025_06_18;
        assert!(v2025_06.supports_streamable_http());
        assert!(v2025_06.supports_meta_fields());
        assert!(v2025_06.supports_elicitation());
    }

    #[test]
    fn test_header_extraction() {
        let mut headers = HeaderMap::new();
        headers.insert("MCP-Protocol-Version", "2025-06-18".parse().unwrap());
        headers.insert("Mcp-Session-Id", "test-session-123".parse().unwrap());

        let version = extract_protocol_version(&headers);
        assert_eq!(version, McpProtocolVersion::V2025_06_18);

        let session_id = extract_session_id(&headers);
        assert_eq!(session_id, Some("test-session-123".to_string()));
    }
}