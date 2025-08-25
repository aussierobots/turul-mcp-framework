//! MCP Roots Protocol Types
//!
//! This module defines types for root directory listing in MCP.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Root directory entry (per MCP spec)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Root {
    /// URI of the root (must start with "file://" currently)
    pub uri: String,
    /// Optional human-readable name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Optional metadata
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<HashMap<String, Value>>,
}

impl Root {
    pub fn new(uri: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
            name: None,
            meta: None,
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.meta = Some(meta);
        self
    }

    /// Validate that the URI follows MCP requirements
    pub fn validate(&self) -> Result<(), String> {
        if !self.uri.starts_with("file://") {
            return Err("Root URI must start with 'file://'".to_string());
        }
        Ok(())
    }
}

/// Complete roots/list request (per MCP spec - no params required)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListRootsRequest {
    /// Method name (always "roots/list")
    pub method: String,
}


/// Response for roots/list (per MCP spec)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListRootsResult {
    /// Available roots
    pub roots: Vec<Root>,
    /// Meta information (follows MCP Result interface)
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        alias = "_meta",
        rename = "_meta"
    )]
    pub meta: Option<HashMap<String, Value>>,
}

/// Notification for when roots list changes (per MCP spec)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RootsListChangedNotification {
    /// Method name (always "notifications/roots/list_changed")
    pub method: String,
}

impl ListRootsRequest {
    pub fn new() -> Self {
        Self {
            method: "roots/list".to_string(),
        }
    }
}


impl ListRootsResult {
    pub fn new(roots: Vec<Root>) -> Self {
        Self { 
            roots,
            meta: None,
        }
    }

    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.meta = Some(meta);
        self
    }
}

impl RootsListChangedNotification {
    pub fn new() -> Self {
        Self {
            method: "notifications/roots/list_changed".to_string(),
        }
    }
}

// Trait implementations for protocol compliance
// ListRootsRequest has no params per MCP spec - using ping::EmptyParams instead

// ===========================================
// === Fine-Grained Roots Traits ===
// ===========================================

/// Trait for root metadata (URI, name, path info)
pub trait HasRootMetadata {
    /// The root URI (must start with "file://")
    fn uri(&self) -> &str;
    
    /// Optional human-readable name
    fn name(&self) -> Option<&str> {
        None
    }
    
    /// Optional description or additional metadata
    fn description(&self) -> Option<&str> {
        None
    }
}

/// Trait for root permissions and security
pub trait HasRootPermissions {
    /// Check if read access is allowed for this path
    fn can_read(&self, _path: &str) -> bool {
        true
    }
    
    /// Check if write access is allowed for this path
    fn can_write(&self, _path: &str) -> bool {
        false // Default: read-only
    }
    
    /// Get maximum depth for directory traversal
    fn max_depth(&self) -> Option<usize> {
        None // No limit by default
    }
}

/// Trait for root filtering and exclusions
pub trait HasRootFiltering {
    /// File extensions to include (None = all)
    fn allowed_extensions(&self) -> Option<&[String]> {
        None
    }
    
    /// File patterns to exclude (glob patterns)
    fn excluded_patterns(&self) -> Option<&[String]> {
        None
    }
    
    /// Check if a file should be included
    fn should_include(&self, path: &str) -> bool {
        // Default: include everything unless filtered
        if let Some(patterns) = self.excluded_patterns() {
            for pattern in patterns {
                if path.contains(pattern) {
                    return false;
                }
            }
        }
        
        if let Some(extensions) = self.allowed_extensions() {
            if let Some(ext) = path.split('.').last() {
                return extensions.contains(&ext.to_string());
            }
            return false;
        }
        
        true
    }
}

/// Trait for root annotations and custom metadata
pub trait HasRootAnnotations {
    /// Get custom metadata
    fn annotations(&self) -> Option<&HashMap<String, Value>> {
        None
    }
    
    /// Get root-specific tags or labels
    fn tags(&self) -> Option<&[String]> {
        None
    }
}

/// Composed root definition trait (automatically implemented via blanket impl)
pub trait RootDefinition: 
    HasRootMetadata + 
    HasRootPermissions + 
    HasRootFiltering + 
    HasRootAnnotations 
{
    /// Convert this root definition to a protocol Root
    fn to_root(&self) -> Root {
        let mut root = Root::new(self.uri());
        if let Some(name) = self.name() {
            root = root.with_name(name);
        }
        if let Some(annotations) = self.annotations() {
            root = root.with_meta(annotations.clone());
        }
        root
    }
    
    /// Validate this root definition
    fn validate(&self) -> Result<(), String> {
        if !self.uri().starts_with("file://") {
            return Err("Root URI must start with 'file://'".to_string());
        }
        Ok(())
    }
}

// Blanket implementation: any type implementing the fine-grained traits automatically gets RootDefinition
impl<T> RootDefinition for T 
where 
    T: HasRootMetadata + HasRootPermissions + HasRootFiltering + HasRootAnnotations 
{}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_root_creation() {
        let mut root = Root::new("file:///home/user/project")
            .with_name("My Project");
        
        let meta = HashMap::from([
            ("version".to_string(), json!("1.0")),
            ("type".to_string(), json!("workspace")),
        ]);
        root = root.with_meta(meta.clone());
        
        assert_eq!(root.uri, "file:///home/user/project");
        assert_eq!(root.name, Some("My Project".to_string()));
        assert_eq!(root.meta, Some(meta));
    }

    #[test]
    fn test_root_validation() {
        let valid_root = Root::new("file:///valid/path");
        assert!(valid_root.validate().is_ok());
        
        let invalid_root = Root::new("http://invalid/path");
        assert!(invalid_root.validate().is_err());
    }

    #[test]
    fn test_list_roots_request() {
        let request = ListRootsRequest::new();
        assert_eq!(request.method, "roots/list");
    }

    #[test]
    fn test_list_roots_result() {
        let roots = vec![
            Root::new("file:///path1").with_name("Root 1"),
            Root::new("file:///path2").with_name("Root 2"),
        ];
        
        let result = ListRootsResult::new(roots.clone());
        assert_eq!(result.roots.len(), 2);
        assert_eq!(result.roots[0].name, Some("Root 1".to_string()));
    }

    #[test]
    fn test_roots_list_changed_notification() {
        let notification = RootsListChangedNotification::new();
        assert_eq!(notification.method, "notifications/roots/list_changed");
    }

    #[test]
    fn test_serialization() {
        let root = Root::new("file:///test/path").with_name("Test Root");
        let json = serde_json::to_string(&root).unwrap();
        assert!(json.contains("file:///test/path"));
        assert!(json.contains("Test Root"));
        
        let parsed: Root = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.uri, "file:///test/path");
        assert_eq!(parsed.name, Some("Test Root".to_string()));
    }
}