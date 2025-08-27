//! Root Builder for Runtime Root Configuration
//!
//! This module provides a builder pattern for creating MCP root directory configurations
//! at runtime. This enables dynamic root setup for file system access and directory management.

use std::collections::HashMap;
use serde_json::Value;
use std::path::PathBuf;

// Import from protocol via alias
use mcp_protocol::roots::{
    ListRootsRequest, RootsListChangedNotification,
    HasRootMetadata, HasRootPermissions, HasRootFiltering, HasRootAnnotations
};

/// Builder for creating root configurations at runtime
pub struct RootBuilder {
    uri: String,
    name: Option<String>,
    description: Option<String>,
    meta: Option<HashMap<String, Value>>,
    // Permission settings
    read_only: bool,
    max_depth: Option<usize>,
    // Filtering settings
    allowed_extensions: Option<Vec<String>>,
    excluded_patterns: Option<Vec<String>>,
    tags: Option<Vec<String>>,
}

impl RootBuilder {
    /// Create a new root builder with the given URI
    pub fn new(uri: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
            name: None,
            description: None,
            meta: None,
            read_only: true, // Safe default
            max_depth: None,
            allowed_extensions: None,
            excluded_patterns: None,
            tags: None,
        }
    }

    /// Create a root builder from a file path (automatically converts to file:// URI)
    pub fn from_path(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let uri = format!("file://{}", path.display());
        Self::new(uri)
    }

    /// Set the human-readable name for this root
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the description for this root
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set meta information
    pub fn meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.meta = Some(meta);
        self
    }

    /// Add a meta key-value pair
    pub fn meta_value(mut self, key: impl Into<String>, value: Value) -> Self {
        if self.meta.is_none() {
            self.meta = Some(HashMap::new());
        }
        self.meta.as_mut().unwrap().insert(key.into(), value);
        self
    }

    /// Set whether this root is read-only (default: true for safety)
    pub fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    /// Allow read and write access (convenience method)
    pub fn read_write(mut self) -> Self {
        self.read_only = false;
        self
    }

    /// Set maximum directory traversal depth
    pub fn max_depth(mut self, depth: usize) -> Self {
        self.max_depth = Some(depth);
        self
    }

    /// Set allowed file extensions (None means all extensions allowed)
    pub fn allowed_extensions(mut self, extensions: Vec<String>) -> Self {
        self.allowed_extensions = Some(extensions);
        self
    }

    /// Add an allowed file extension
    pub fn allow_extension(mut self, extension: impl Into<String>) -> Self {
        if self.allowed_extensions.is_none() {
            self.allowed_extensions = Some(Vec::new());
        }
        self.allowed_extensions.as_mut().unwrap().push(extension.into());
        self
    }

    /// Set excluded file patterns (glob-style patterns)
    pub fn excluded_patterns(mut self, patterns: Vec<String>) -> Self {
        self.excluded_patterns = Some(patterns);
        self
    }

    /// Add an excluded file pattern
    pub fn exclude_pattern(mut self, pattern: impl Into<String>) -> Self {
        if self.excluded_patterns.is_none() {
            self.excluded_patterns = Some(Vec::new());
        }
        self.excluded_patterns.as_mut().unwrap().push(pattern.into());
        self
    }

    /// Set tags for this root
    pub fn tags(mut self, tags: Vec<String>) -> Self {
        self.tags = Some(tags);
        self
    }

    /// Add a tag
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        if self.tags.is_none() {
            self.tags = Some(Vec::new());
        }
        self.tags.as_mut().unwrap().push(tag.into());
        self
    }

    /// Build the dynamic root configuration
    pub fn build(self) -> Result<DynamicRoot, String> {
        // Validate URI
        if !self.uri.starts_with("file://") {
            return Err("Root URI must start with 'file://'".to_string());
        }

        Ok(DynamicRoot {
            uri: self.uri,
            name: self.name,
            description: self.description,
            meta: self.meta,
            read_only: self.read_only,
            max_depth: self.max_depth,
            allowed_extensions: self.allowed_extensions,
            excluded_patterns: self.excluded_patterns,
            tags: self.tags,
        })
    }
}

/// Dynamic root configuration created by RootBuilder
#[derive(Debug)]
pub struct DynamicRoot {
    uri: String,
    name: Option<String>,
    description: Option<String>,
    meta: Option<HashMap<String, Value>>,
    read_only: bool,
    max_depth: Option<usize>,
    allowed_extensions: Option<Vec<String>>,
    excluded_patterns: Option<Vec<String>>,
    tags: Option<Vec<String>>,
}

// Implement all fine-grained traits for DynamicRoot
impl HasRootMetadata for DynamicRoot {
    fn uri(&self) -> &str {
        &self.uri
    }

    fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}

impl HasRootPermissions for DynamicRoot {
    fn can_read(&self, _path: &str) -> bool {
        true // Always allow reading within the root
    }

    fn can_write(&self, _path: &str) -> bool {
        !self.read_only
    }

    fn max_depth(&self) -> Option<usize> {
        self.max_depth
    }
}

impl HasRootFiltering for DynamicRoot {
    fn allowed_extensions(&self) -> Option<&[String]> {
        self.allowed_extensions.as_deref()
    }

    fn excluded_patterns(&self) -> Option<&[String]> {
        self.excluded_patterns.as_deref()
    }
}

impl HasRootAnnotations for DynamicRoot {
    fn annotations(&self) -> Option<&HashMap<String, Value>> {
        self.meta.as_ref()
    }

    fn tags(&self) -> Option<&[String]> {
        self.tags.as_deref()
    }
}

// RootDefinition is automatically implemented via blanket impl!

/// Builder for ListRootsRequest
pub struct ListRootsRequestBuilder {
    meta: Option<HashMap<String, Value>>,
}

impl ListRootsRequestBuilder {
    pub fn new() -> Self {
        Self {
            meta: None,
        }
    }

    /// Set meta information
    pub fn meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.meta = Some(meta);
        self
    }

    /// Add a meta key-value pair
    pub fn meta_value(mut self, key: impl Into<String>, value: Value) -> Self {
        if self.meta.is_none() {
            self.meta = Some(HashMap::new());
        }
        self.meta.as_mut().unwrap().insert(key.into(), value);
        self
    }

    /// Build the ListRootsRequest
    pub fn build(self) -> ListRootsRequest {
        if let Some(meta) = self.meta {
            ListRootsRequest::new().with_meta(meta)
        } else {
            ListRootsRequest::new()
        }
    }
}

impl Default for ListRootsRequestBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for RootsListChangedNotification
pub struct RootsNotificationBuilder {
    meta: Option<HashMap<String, Value>>,
}

impl RootsNotificationBuilder {
    pub fn new() -> Self {
        Self {
            meta: None,
        }
    }

    /// Set meta information
    pub fn meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.meta = Some(meta);
        self
    }

    /// Add a meta key-value pair
    pub fn meta_value(mut self, key: impl Into<String>, value: Value) -> Self {
        if self.meta.is_none() {
            self.meta = Some(HashMap::new());
        }
        self.meta.as_mut().unwrap().insert(key.into(), value);
        self
    }

    /// Build the RootsListChangedNotification
    pub fn build(self) -> RootsListChangedNotification {
        if let Some(meta) = self.meta {
            RootsListChangedNotification::new().with_meta(meta)
        } else {
            RootsListChangedNotification::new()
        }
    }
}

impl Default for RootsNotificationBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience methods for common root patterns
impl RootBuilder {
    /// Create a source code root with common file extensions
    pub fn source_code_root(path: impl Into<PathBuf>) -> Self {
        Self::from_path(path)
            .name("Source Code")
            .description("Source code directory with common programming files")
            .allowed_extensions(vec![
                "rs".to_string(), "py".to_string(), "js".to_string(), "ts".to_string(),
                "java".to_string(), "cpp".to_string(), "c".to_string(), "h".to_string(),
                "go".to_string(), "rb".to_string(), "php".to_string(), "swift".to_string(),
                "kt".to_string(), "scala".to_string(), "clj".to_string(), "hs".to_string(),
            ])
            .excluded_patterns(vec![
                "node_modules".to_string(),
                "target".to_string(),
                ".git".to_string(),
                "build".to_string(),
                "dist".to_string(),
            ])
            .tag("source-code")
    }

    /// Create a documentation root
    pub fn docs_root(path: impl Into<PathBuf>) -> Self {
        Self::from_path(path)
            .name("Documentation")
            .description("Documentation and README files")
            .allowed_extensions(vec![
                "md".to_string(), "txt".to_string(), "rst".to_string(),
                "adoc".to_string(), "org".to_string(), "tex".to_string(),
                "html".to_string(), "pdf".to_string(),
            ])
            .tag("documentation")
    }

    /// Create a configuration root
    pub fn config_root(path: impl Into<PathBuf>) -> Self {
        Self::from_path(path)
            .name("Configuration")
            .description("Configuration and settings files")
            .allowed_extensions(vec![
                "json".to_string(), "yaml".to_string(), "yml".to_string(),
                "toml".to_string(), "ini".to_string(), "cfg".to_string(),
                "conf".to_string(), "config".to_string(), "env".to_string(),
            ])
            .tag("configuration")
    }

    /// Create a temporary workspace root with write access
    pub fn workspace_root(path: impl Into<PathBuf>) -> Self {
        Self::from_path(path)
            .name("Workspace")
            .description("Temporary workspace with read-write access")
            .read_write()
            .max_depth(10)
            .excluded_patterns(vec![
                ".DS_Store".to_string(),
                "Thumbs.db".to_string(),
                "*.tmp".to_string(),
            ])
            .tag("workspace")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use mcp_protocol::roots::RootDefinition;

    #[test]
    fn test_root_builder_basic() {
        let root = RootBuilder::new("file:///home/user/project")
            .name("My Project")
            .description("A sample project")
            .read_write()
            .max_depth(5)
            .build()
            .expect("Failed to build root");

        assert_eq!(root.uri(), "file:///home/user/project");
        assert_eq!(root.name(), Some("My Project"));
        assert_eq!(root.description(), Some("A sample project"));
        assert!(root.can_write("any/path"));
        assert_eq!(root.max_depth(), Some(5));
    }

    #[test]
    fn test_root_builder_from_path() {
        let path = PathBuf::from("/home/user/project");
        let root = RootBuilder::from_path(&path)
            .name("Project Root")
            .build()
            .expect("Failed to build root");

        assert!(root.uri().starts_with("file://"));
        assert!(root.uri().contains("/home/user/project"));
        assert_eq!(root.name(), Some("Project Root"));
    }

    #[test]
    fn test_root_builder_filtering() {
        let root = RootBuilder::new("file:///home/user/src")
            .allowed_extensions(vec!["rs".to_string(), "toml".to_string()])
            .excluded_patterns(vec!["target".to_string(), ".git".to_string()])
            .tags(vec!["rust".to_string(), "source".to_string()])
            .build()
            .expect("Failed to build root");

        assert_eq!(root.allowed_extensions(), Some(&["rs".to_string(), "toml".to_string()][..]));
        assert_eq!(root.excluded_patterns(), Some(&["target".to_string(), ".git".to_string()][..]));
        assert_eq!(root.tags(), Some(&["rust".to_string(), "source".to_string()][..]));
    }

    #[test]
    fn test_root_builder_meta() {
        let mut meta = HashMap::new();
        meta.insert("version".to_string(), json!("1.0"));
        meta.insert("type".to_string(), json!("workspace"));

        let root = RootBuilder::new("file:///workspace")
            .meta(meta.clone())
            .build()
            .expect("Failed to build root");

        assert_eq!(root.annotations(), Some(&meta));
    }

    #[test]
    fn test_root_builder_fluent_meta() {
        let root = RootBuilder::new("file:///project")
            .meta_value("project_id", json!("proj-123"))
            .meta_value("owner", json!("alice"))
            .build()
            .expect("Failed to build root");

        let annotations = root.annotations().expect("Expected annotations");
        assert_eq!(annotations.get("project_id"), Some(&json!("proj-123")));
        assert_eq!(annotations.get("owner"), Some(&json!("alice")));
    }

    #[test]
    fn test_root_builder_permissions() {
        // Read-only root (default)
        let readonly_root = RootBuilder::new("file:///readonly")
            .build()
            .expect("Failed to build root");
        assert!(readonly_root.can_read("any/file"));
        assert!(!readonly_root.can_write("any/file"));

        // Read-write root
        let readwrite_root = RootBuilder::new("file:///readwrite")
            .read_write()
            .build()
            .expect("Failed to build root");
        assert!(readwrite_root.can_read("any/file"));
        assert!(readwrite_root.can_write("any/file"));
    }

    #[test]
    fn test_root_builder_convenience_extensions() {
        let root = RootBuilder::new("file:///src")
            .allow_extension("rs")
            .allow_extension("toml")
            .exclude_pattern("target")
            .exclude_pattern(".git")
            .tag("rust")
            .tag("project")
            .build()
            .expect("Failed to build root");

        assert_eq!(root.allowed_extensions(), Some(&["rs".to_string(), "toml".to_string()][..]));
        assert_eq!(root.excluded_patterns(), Some(&["target".to_string(), ".git".to_string()][..]));
        assert_eq!(root.tags(), Some(&["rust".to_string(), "project".to_string()][..]));
    }

    #[test]
    fn test_root_validation() {
        // Valid file:// URI
        let valid = RootBuilder::new("file:///valid/path").build();
        assert!(valid.is_ok());

        // Invalid URI (not file://)
        let invalid = RootBuilder::new("http://invalid/path").build();
        assert!(invalid.is_err());
        assert!(invalid.unwrap_err().contains("must start with 'file://'"));
    }

    #[test]
    fn test_root_definition_trait_implementation() {
        let root = RootBuilder::new("file:///test")
            .name("Test Root")
            .build()
            .expect("Failed to build root");

        // Test that it implements RootDefinition
        let protocol_root = root.to_root();
        assert_eq!(protocol_root.uri, "file:///test");
        assert_eq!(protocol_root.name, Some("Test Root".to_string()));

        // Test validation
        assert!(root.validate().is_ok());
    }

    #[test]
    fn test_preset_builders() {
        // Source code root
        let src_root = RootBuilder::source_code_root("/home/user/project")
            .build()
            .expect("Failed to build source root");
        assert_eq!(src_root.name(), Some("Source Code"));
        assert!(src_root.allowed_extensions().unwrap().contains(&"rs".to_string()));
        assert!(src_root.excluded_patterns().unwrap().contains(&"node_modules".to_string()));
        assert!(src_root.tags().unwrap().contains(&"source-code".to_string()));

        // Docs root
        let docs_root = RootBuilder::docs_root("/home/user/docs")
            .build()
            .expect("Failed to build docs root");
        assert_eq!(docs_root.name(), Some("Documentation"));
        assert!(docs_root.allowed_extensions().unwrap().contains(&"md".to_string()));

        // Config root
        let config_root = RootBuilder::config_root("/etc/myapp")
            .build()
            .expect("Failed to build config root");
        assert_eq!(config_root.name(), Some("Configuration"));
        assert!(config_root.allowed_extensions().unwrap().contains(&"json".to_string()));

        // Workspace root
        let workspace_root = RootBuilder::workspace_root("/tmp/workspace")
            .build()
            .expect("Failed to build workspace root");
        assert_eq!(workspace_root.name(), Some("Workspace"));
        assert!(workspace_root.can_write("any/file")); // Read-write enabled
    }

    #[test]
    fn test_list_roots_request_builder() {
        let request = ListRootsRequestBuilder::new()
            .meta_value("client_id", json!("client-123"))
            .build();

        assert_eq!(request.method, "roots/list");
        let params = request.params.expect("Expected params");
        let meta = params.meta.expect("Expected meta");
        assert_eq!(meta.get("client_id"), Some(&json!("client-123")));
    }

    #[test]
    fn test_roots_notification_builder() {
        let notification = RootsNotificationBuilder::new()
            .meta_value("timestamp", json!("2025-01-01T00:00:00Z"))
            .build();

        assert_eq!(notification.method, "notifications/roots/list_changed");
        let params = notification.params.expect("Expected params");
        let meta = params.meta.expect("Expected meta");
        assert_eq!(meta.get("timestamp"), Some(&json!("2025-01-01T00:00:00Z")));
    }

    #[test]
    fn test_root_filtering_functionality() {
        let root = RootBuilder::new("file:///src")
            .allowed_extensions(vec!["rs".to_string(), "toml".to_string()])
            .excluded_patterns(vec!["target".to_string(), ".git".to_string()])
            .build()
            .expect("Failed to build root");

        // Test should_include functionality (via trait implementation)
        assert!(root.should_include("main.rs"));
        assert!(root.should_include("Cargo.toml"));
        assert!(!root.should_include("main.py")); // Wrong extension
        assert!(!root.should_include("target/debug/main")); // Excluded pattern
        assert!(!root.should_include(".git/config")); // Excluded pattern
    }
}