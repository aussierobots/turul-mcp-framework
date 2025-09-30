//! MCP Initialize Protocol Types
//!
//! This module defines the types used for the MCP initialization handshake.

use crate::version::McpVersion;
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
}

impl Implementation {
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            title: None,
        }
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }
}

/// Capabilities related to root listing support
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RootsCapabilities {
    /// Whether the client supports notifications for root list changes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

/// Capabilities related to sampling support
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SamplingCapabilities {
    /// Whether the client supports sampling
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
}

/// Capabilities related to elicitation support
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ElicitationCapabilities {
    /// Whether the client supports elicitation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
}

/// Capabilities that a client may support
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ClientCapabilities {
    /// Root directory capabilities
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roots: Option<RootsCapabilities>,
    /// Sampling capabilities
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sampling: Option<SamplingCapabilities>,
    /// Elicitation capabilities
    #[serde(skip_serializing_if = "Option::is_none")]
    pub elicitation: Option<ElicitationCapabilities>,
    /// Experimental capabilities
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental: Option<HashMap<String, Value>>,
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

/// Capabilities that a server may support
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ServerCapabilities {
    /// Logging capabilities
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logging: Option<LoggingCapabilities>,
    /// Completion capabilities
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completions: Option<CompletionsCapabilities>,
    /// Prompt capabilities
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts: Option<PromptsCapabilities>,
    /// Resource capabilities
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<ResourcesCapabilities>,
    /// Tool capabilities
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<ToolsCapabilities>,
    /// Elicitation capabilities (server can make elicitation requests to clients)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub elicitation: Option<ElicitationCapabilities>,
    /// Experimental capabilities
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental: Option<HashMap<String, Value>>,
}

/// Parameters for initialize request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeRequest {
    /// The protocol version the client wants to use
    pub protocol_version: String,
    /// Capabilities the client supports
    pub capabilities: ClientCapabilities,
    /// Information about the client implementation
    pub client_info: Implementation,
}

impl InitializeRequest {
    pub fn new(
        protocol_version: McpVersion,
        capabilities: ClientCapabilities,
        client_info: Implementation,
    ) -> Self {
        Self {
            protocol_version: protocol_version.as_str().to_string(),
            capabilities,
            client_info,
        }
    }

    /// Get the protocol version as a parsed enum
    pub fn protocol_version(&self) -> Result<McpVersion, crate::McpError> {
        self.protocol_version
            .parse::<McpVersion>()
            .map_err(|_| crate::McpError::VersionMismatch {
                expected: McpVersion::CURRENT.as_str().to_string(),
                actual: self.protocol_version.clone(),
            })
    }
}

/// Result payload for initialize (per MCP spec)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeResult {
    /// The protocol version the server supports
    pub protocol_version: String,
    /// Capabilities the server supports
    pub capabilities: ServerCapabilities,
    /// Information about the server implementation
    pub server_info: Implementation,
    /// Optional instructions for the client
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
}

impl InitializeResult {
    pub fn new(
        protocol_version: McpVersion,
        capabilities: ServerCapabilities,
        server_info: Implementation,
    ) -> Self {
        Self {
            protocol_version: protocol_version.as_str().to_string(),
            capabilities,
            server_info,
            instructions: None,
        }
    }

    pub fn with_instructions(mut self, instructions: impl Into<String>) -> Self {
        self.instructions = Some(instructions.into());
        self
    }

    /// Get the protocol version as a parsed enum
    pub fn protocol_version(&self) -> Result<McpVersion, crate::McpError> {
        self.protocol_version
            .parse::<McpVersion>()
            .map_err(|_| crate::McpError::VersionMismatch {
                expected: McpVersion::CURRENT.as_str().to_string(),
                actual: self.protocol_version.clone(),
            })
    }
}

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

    #[test]
    fn test_initialize_request_serialization() {
        let client_info = Implementation::new("test-client", "1.0.0");
        let capabilities = ClientCapabilities::default();
        let request = InitializeRequest::new(McpVersion::V2025_06_18, capabilities, client_info);

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("2025-06-18"));
        assert!(json.contains("test-client"));
    }

    #[test]
    fn test_initialize_response_creation() {
        let server_info = Implementation::new("test-server", "1.0.0");
        let capabilities = ServerCapabilities::default();
        let response = InitializeResult::new(McpVersion::V2025_06_18, capabilities, server_info)
            .with_instructions("Welcome to the test server!");

        assert_eq!(response.protocol_version, "2025-06-18");
        assert!(response.instructions.is_some());
    }
}
