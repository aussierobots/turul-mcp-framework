//! RFC 9728 Protected Resource Metadata
//!
//! Defines the metadata document served at `/.well-known/oauth-protected-resource`
//! that clients use to discover the authorization server for this resource.

use serde::{Deserialize, Serialize};

/// RFC 9728 Protected Resource Metadata
///
/// This document tells clients which authorization server(s) protect this resource
/// and what capabilities the resource supports.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtectedResourceMetadata {
    /// The resource identifier (MUST match the resource's origin URL)
    pub resource: String,

    /// Authorization servers that can issue tokens for this resource
    pub authorization_servers: Vec<String>,

    /// JWKS URI for direct key retrieval (optional — usually from AS metadata)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jwks_uri: Option<String>,

    /// OAuth 2.0 scopes supported by this resource
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scopes_supported: Option<Vec<String>>,

    /// Methods for presenting Bearer tokens (default: ["header"])
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bearer_methods_supported: Option<Vec<String>>,

    /// Resource documentation URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_documentation: Option<String>,

    /// Resource signing algorithms supported
    #[serde(
        rename = "resource_signing_alg_values_supported",
        skip_serializing_if = "Option::is_none"
    )]
    pub resource_signing_alg_values_supported: Option<Vec<String>>,
}

impl ProtectedResourceMetadata {
    /// Create minimal metadata with resource URL and authorization servers
    pub fn new(resource: impl Into<String>, authorization_servers: Vec<String>) -> Self {
        Self {
            resource: resource.into(),
            authorization_servers,
            jwks_uri: None,
            scopes_supported: None,
            bearer_methods_supported: None,
            resource_documentation: None,
            resource_signing_alg_values_supported: None,
        }
    }

    /// Set the JWKS URI
    pub fn with_jwks_uri(mut self, uri: impl Into<String>) -> Self {
        self.jwks_uri = Some(uri.into());
        self
    }

    /// Set supported scopes
    pub fn with_scopes(mut self, scopes: Vec<String>) -> Self {
        self.scopes_supported = Some(scopes);
        self
    }

    /// The well-known path for this metadata document
    pub const WELL_KNOWN_PATH: &'static str = "/.well-known/oauth-protected-resource";

    /// Compute the metadata URL (origin + well-known path) per RFC 9728 §3
    ///
    /// Extracts the origin (scheme + authority) from the resource URL and appends
    /// the well-known path. For `resource = "https://example.com/mcp"`, returns
    /// `"https://example.com/.well-known/oauth-protected-resource"`.
    pub fn metadata_url(&self) -> String {
        // Extract origin: everything up to the path
        // "https://example.com/mcp" → "https://example.com"
        if let Some(pos) = self.resource.find("://") {
            let after_scheme = &self.resource[pos + 3..];
            if let Some(slash_pos) = after_scheme.find('/') {
                // Has path component — origin is scheme + authority
                let origin = &self.resource[..pos + 3 + slash_pos];
                return format!("{}{}", origin, Self::WELL_KNOWN_PATH);
            }
        }
        // No path component or unparseable — append directly to resource
        format!("{}{}", self.resource, Self::WELL_KNOWN_PATH)
    }

    /// Extract the path component from the resource URL, if any.
    ///
    /// Returns `Some("/mcp")` for `"https://example.com/mcp"`,
    /// `None` for `"https://example.com"` (root resource).
    /// Query strings and fragments are stripped — only the path is returned.
    pub fn resource_path(&self) -> Option<&str> {
        if let Some(pos) = self.resource.find("://") {
            let after_scheme = &self.resource[pos + 3..];
            if let Some(slash_pos) = after_scheme.find('/') {
                let path_and_rest = &after_scheme[slash_pos..];
                // Strip query and fragment — use only the path component
                let path = path_and_rest
                    .split_once('?')
                    .map_or(path_and_rest, |(p, _)| p);
                let path = path.split_once('#').map_or(path, |(p, _)| p);
                // "/" alone means root — no path-form needed
                if path != "/" {
                    return Some(path);
                }
            }
        }
        None
    }

    /// Return all well-known paths that should serve this metadata document.
    ///
    /// Per RFC 9728 §3, resources with a path component need both:
    /// - **Root form**: `/.well-known/oauth-protected-resource`
    /// - **Path form**: `/.well-known/oauth-protected-resource{path}`
    ///
    /// Resources at the origin root only need the root form.
    pub fn well_known_paths(&self) -> Vec<String> {
        let mut paths = vec![Self::WELL_KNOWN_PATH.to_string()];
        if let Some(resource_path) = self.resource_path() {
            paths.push(format!("{}{}", Self::WELL_KNOWN_PATH, resource_path));
        }
        paths
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // T29: Metadata JSON matches RFC 9728
    #[test]
    fn test_metadata_json_matches_rfc9728() {
        let metadata = ProtectedResourceMetadata::new(
            "https://example.com/mcp",
            vec!["https://auth.example.com".to_string()],
        )
        .with_jwks_uri("https://auth.example.com/.well-known/jwks.json")
        .with_scopes(vec!["mcp:read".to_string(), "mcp:write".to_string()]);

        let json = serde_json::to_value(&metadata).unwrap();

        assert_eq!(json["resource"], "https://example.com/mcp");
        assert_eq!(
            json["authorization_servers"],
            serde_json::json!(["https://auth.example.com"])
        );
        assert_eq!(
            json["jwks_uri"],
            "https://auth.example.com/.well-known/jwks.json"
        );
        assert_eq!(
            json["scopes_supported"],
            serde_json::json!(["mcp:read", "mcp:write"])
        );
        // Fields not set should be absent
        assert!(json.get("bearer_methods_supported").is_none());
        assert!(json.get("resource_documentation").is_none());
    }

    #[test]
    fn test_metadata_url_extracts_origin() {
        // Resource with path → origin-based URL
        let m = ProtectedResourceMetadata::new(
            "https://example.com/mcp",
            vec!["https://auth.example.com".to_string()],
        );
        assert_eq!(
            m.metadata_url(),
            "https://example.com/.well-known/oauth-protected-resource"
        );

        // Resource at root → same as resource + well-known path
        let m = ProtectedResourceMetadata::new(
            "https://example.com",
            vec!["https://auth.example.com".to_string()],
        );
        assert_eq!(
            m.metadata_url(),
            "https://example.com/.well-known/oauth-protected-resource"
        );

        // Resource with port and path
        let m = ProtectedResourceMetadata::new(
            "https://example.com:8443/api/mcp",
            vec!["https://auth.example.com".to_string()],
        );
        assert_eq!(
            m.metadata_url(),
            "https://example.com:8443/.well-known/oauth-protected-resource"
        );
    }

    #[test]
    fn test_resource_path_extraction() {
        // Resource with path → Some
        let m = ProtectedResourceMetadata::new(
            "https://example.com/mcp",
            vec!["https://auth.example.com".to_string()],
        );
        assert_eq!(m.resource_path(), Some("/mcp"));

        // Resource at root → None
        let m = ProtectedResourceMetadata::new(
            "https://example.com",
            vec!["https://auth.example.com".to_string()],
        );
        assert_eq!(m.resource_path(), None);

        // Resource at root with trailing slash → None (just "/" is root)
        let m = ProtectedResourceMetadata::new(
            "https://example.com/",
            vec!["https://auth.example.com".to_string()],
        );
        assert_eq!(m.resource_path(), None);

        // Deeper path
        let m = ProtectedResourceMetadata::new(
            "https://example.com/api/mcp",
            vec!["https://auth.example.com".to_string()],
        );
        assert_eq!(m.resource_path(), Some("/api/mcp"));

        // With port
        let m = ProtectedResourceMetadata::new(
            "https://example.com:8443/mcp",
            vec!["https://auth.example.com".to_string()],
        );
        assert_eq!(m.resource_path(), Some("/mcp"));

        // Query string stripped
        let m = ProtectedResourceMetadata::new(
            "https://example.com/mcp?x=1&y=2",
            vec!["https://auth.example.com".to_string()],
        );
        assert_eq!(m.resource_path(), Some("/mcp"));

        // Fragment stripped
        let m = ProtectedResourceMetadata::new(
            "https://example.com/mcp#section",
            vec!["https://auth.example.com".to_string()],
        );
        assert_eq!(m.resource_path(), Some("/mcp"));

        // Both query and fragment stripped
        let m = ProtectedResourceMetadata::new(
            "https://example.com/api/mcp?token=abc#top",
            vec!["https://auth.example.com".to_string()],
        );
        assert_eq!(m.resource_path(), Some("/api/mcp"));

        // Root with query → still None
        let m = ProtectedResourceMetadata::new(
            "https://example.com/?x=1",
            vec!["https://auth.example.com".to_string()],
        );
        assert_eq!(m.resource_path(), None);
    }

    #[test]
    fn test_well_known_paths() {
        // Resource with path → root form + path form
        let m = ProtectedResourceMetadata::new(
            "https://example.com/mcp",
            vec!["https://auth.example.com".to_string()],
        );
        let paths = m.well_known_paths();
        assert_eq!(paths.len(), 2);
        assert_eq!(paths[0], "/.well-known/oauth-protected-resource");
        assert_eq!(paths[1], "/.well-known/oauth-protected-resource/mcp");

        // Resource at root → root form only
        let m = ProtectedResourceMetadata::new(
            "https://example.com",
            vec!["https://auth.example.com".to_string()],
        );
        let paths = m.well_known_paths();
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0], "/.well-known/oauth-protected-resource");

        // Deeper path
        let m = ProtectedResourceMetadata::new(
            "https://example.com/api/mcp",
            vec!["https://auth.example.com".to_string()],
        );
        let paths = m.well_known_paths();
        assert_eq!(paths.len(), 2);
        assert_eq!(paths[0], "/.well-known/oauth-protected-resource");
        assert_eq!(paths[1], "/.well-known/oauth-protected-resource/api/mcp");

        // Query/fragment stripped from path-form route
        let m = ProtectedResourceMetadata::new(
            "https://example.com/mcp?x=1#top",
            vec!["https://auth.example.com".to_string()],
        );
        let paths = m.well_known_paths();
        assert_eq!(paths.len(), 2);
        assert_eq!(paths[0], "/.well-known/oauth-protected-resource");
        assert_eq!(paths[1], "/.well-known/oauth-protected-resource/mcp");
    }

    #[test]
    fn test_metadata_roundtrip() {
        let metadata = ProtectedResourceMetadata::new(
            "https://example.com/mcp",
            vec!["https://auth.example.com".to_string()],
        );

        let json = serde_json::to_string(&metadata).unwrap();
        let parsed: ProtectedResourceMetadata = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.resource, metadata.resource);
        assert_eq!(parsed.authorization_servers, metadata.authorization_servers);
    }
}
