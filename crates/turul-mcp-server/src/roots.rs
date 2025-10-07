//! MCP Roots Trait
//!
//! This module defines the high-level trait for implementing MCP roots functionality.

use async_trait::async_trait;
use std::path::PathBuf;
use turul_mcp_builders::prelude::*;
use turul_mcp_protocol::{
    McpResult,
    roots::{ListRootsRequest, ListRootsResult, RootsListChangedNotification},
};

/// File information for root directory listings
#[derive(Debug, Clone)]
pub struct FileInfo {
    /// File path relative to root
    pub path: String,
    /// Whether this is a directory
    pub is_directory: bool,
    /// File size in bytes (for files)
    pub size: Option<u64>,
    /// Last modified timestamp (Unix timestamp)
    pub modified: Option<u64>,
    /// MIME type (for files)
    pub mime_type: Option<String>,
}

/// Access level for files and directories
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessLevel {
    None,
    Read,
    Write,
    Full,
}

/// High-level trait for implementing MCP roots functionality
///
/// McpRoot extends RootDefinition with execution capabilities.
/// All metadata is provided by the RootDefinition trait, ensuring
/// consistency between concrete Root structs and dynamic implementations.
#[async_trait]
pub trait McpRoot: RootDefinition + Send + Sync {
    /// List available roots (per MCP spec)
    ///
    /// This method processes the roots/list request and returns
    /// the complete list of available root directories.
    async fn list_roots(&self, request: ListRootsRequest) -> McpResult<ListRootsResult>;

    /// List files within a root directory
    ///
    /// This method lists files and directories within the specified path,
    /// respecting permissions and filtering rules.
    async fn list_files(&self, path: &str) -> McpResult<Vec<FileInfo>>;

    /// Check access level for a specific path
    ///
    /// This method determines what access level the client has
    /// for the specified file or directory path.
    async fn check_access(&self, path: &str) -> McpResult<AccessLevel>;

    /// Optional: Check if this root handler can manage the given path
    ///
    /// This allows for conditional root handling based on path patterns,
    /// URI schemes, or other factors.
    fn can_handle(&self, path: &str) -> bool {
        path.starts_with(&self.uri().replace("file://", ""))
    }

    /// Optional: Get root priority for request routing
    ///
    /// Higher priority handlers are tried first when multiple handlers
    /// can manage the same path.
    fn priority(&self) -> u32 {
        0
    }

    /// Optional: Validate a file path
    ///
    /// This method can perform additional validation beyond basic access checks.
    async fn validate_path(&self, path: &str) -> McpResult<()> {
        // Basic validation - ensure path is within root
        let root_path = self.uri().replace("file://", "");
        let canonical_path = PathBuf::from(path);
        let canonical_root = PathBuf::from(&root_path);

        if !canonical_path.starts_with(&canonical_root) {
            return Err(turul_mcp_protocol::McpError::validation(
                "Path is outside root directory",
            ));
        }

        Ok(())
    }

    /// Optional: Watch for changes in root directories
    ///
    /// This method can be used to monitor root directories for changes
    /// and send RootsListChangedNotification when needed.
    async fn start_watching(&self) -> McpResult<()> {
        // Default: no-op for non-watching roots
        Ok(())
    }

    /// Optional: Stop watching for changes
    async fn stop_watching(&self) -> McpResult<()> {
        // Default: no-op
        Ok(())
    }

    /// Optional: Send a roots list changed notification
    ///
    /// This method should be called when the list of roots changes
    /// to notify clients about the update.
    async fn notify_roots_changed(&self) -> McpResult<RootsListChangedNotification> {
        Ok(RootsListChangedNotification::new())
    }

    /// Optional: Get file metadata
    ///
    /// This method retrieves detailed metadata for a specific file or directory.
    async fn get_file_info(&self, path: &str) -> McpResult<Option<FileInfo>> {
        use std::fs;
        use std::time::UNIX_EPOCH;

        let full_path = self.uri().replace("file://", "") + "/" + path;

        match fs::metadata(&full_path) {
            Ok(metadata) => {
                let modified = metadata
                    .modified()
                    .ok()
                    .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                    .map(|duration| duration.as_secs());

                let info = FileInfo {
                    path: path.to_string(),
                    is_directory: metadata.is_dir(),
                    size: if metadata.is_file() {
                        Some(metadata.len())
                    } else {
                        None
                    },
                    modified,
                    mime_type: if metadata.is_file() {
                        // Simple MIME type detection based on extension
                        match path.split('.').next_back() {
                            Some("txt") => Some("text/plain".to_string()),
                            Some("json") => Some("application/json".to_string()),
                            Some("html") => Some("text/html".to_string()),
                            Some("md") => Some("text/markdown".to_string()),
                            _ => Some("application/octet-stream".to_string()),
                        }
                    } else {
                        None
                    },
                };
                Ok(Some(info))
            }
            Err(_) => Ok(None),
        }
    }
}

/// Convert an McpRoot trait object to a ListRootsRequest
///
/// This is a convenience function for creating protocol requests.
pub fn root_to_list_request(_root: &dyn McpRoot) -> ListRootsRequest {
    ListRootsRequest::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use turul_mcp_protocol::roots::{
        HasRootAnnotations, HasRootFiltering, HasRootMetadata, HasRootPermissions,
    };

    struct TestRoot {
        uri: String,
        name: Option<String>,
        read_only: bool,
    }

    // Implement fine-grained traits (MCP spec compliant)
    impl HasRootMetadata for TestRoot {
        fn uri(&self) -> &str {
            &self.uri
        }

        fn name(&self) -> Option<&str> {
            self.name.as_deref()
        }
    }

    impl HasRootPermissions for TestRoot {
        fn can_read(&self, _path: &str) -> bool {
            true
        }

        fn can_write(&self, _path: &str) -> bool {
            !self.read_only
        }

        fn max_depth(&self) -> Option<usize> {
            Some(10) // Limit depth for testing
        }
    }

    impl HasRootFiltering for TestRoot {
        fn excluded_patterns(&self) -> Option<&[String]> {
            // Example: exclude hidden files and build directories
            static PATTERNS: &[String] = &[];
            if PATTERNS.is_empty() {
                None
            } else {
                Some(PATTERNS)
            }
        }
    }

    impl HasRootAnnotations for TestRoot {
        fn annotations(&self) -> Option<&HashMap<std::string::String, serde_json::Value>> {
            None
        }
    }

    // RootDefinition automatically implemented via blanket impl!

    #[async_trait]
    impl McpRoot for TestRoot {
        async fn list_roots(&self, _request: ListRootsRequest) -> McpResult<ListRootsResult> {
            let root = self.to_root();
            Ok(ListRootsResult::new(vec![root]))
        }

        async fn list_files(&self, path: &str) -> McpResult<Vec<FileInfo>> {
            // Simulate file listing
            if path.is_empty() || path == "/" {
                Ok(vec![
                    FileInfo {
                        path: "README.md".to_string(),
                        is_directory: false,
                        size: Some(1024),
                        modified: Some(1640995200), // 2022-01-01
                        mime_type: Some("text/markdown".to_string()),
                    },
                    FileInfo {
                        path: "src".to_string(),
                        is_directory: true,
                        size: None,
                        modified: Some(1640995200),
                        mime_type: None,
                    },
                ])
            } else {
                Ok(vec![])
            }
        }

        async fn check_access(&self, _path: &str) -> McpResult<AccessLevel> {
            if self.read_only {
                Ok(AccessLevel::Read)
            } else {
                Ok(AccessLevel::Full)
            }
        }
    }

    #[test]
    fn test_root_trait() {
        let root = TestRoot {
            uri: "file:///home/user/project".to_string(),
            name: Some("Test Project".to_string()),
            read_only: false,
        };

        assert_eq!(root.uri(), "file:///home/user/project");
        assert_eq!(root.name(), Some("Test Project"));
        assert!(root.can_read("any/path"));
        assert!(root.can_write("any/path"));
        assert_eq!(root.max_depth(), Some(10));
    }

    #[tokio::test]
    async fn test_root_validation() {
        let root = TestRoot {
            uri: "file:///home/user".to_string(),
            name: None,
            read_only: true,
        };

        let valid_result = root.validate_path("/home/user/project/file.txt").await;
        assert!(valid_result.is_ok());
    }

    #[tokio::test]
    async fn test_file_listing() {
        let root = TestRoot {
            uri: "file:///test".to_string(),
            name: Some("Test Root".to_string()),
            read_only: false,
        };

        let files = root.list_files("").await.unwrap();
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].path, "README.md");
        assert!(!files[0].is_directory);
        assert_eq!(files[1].path, "src");
        assert!(files[1].is_directory);
    }

    #[tokio::test]
    async fn test_access_levels() {
        let read_only_root = TestRoot {
            uri: "file:///readonly".to_string(),
            name: None,
            read_only: true,
        };

        let full_access_root = TestRoot {
            uri: "file:///writable".to_string(),
            name: None,
            read_only: false,
        };

        assert_eq!(
            read_only_root.check_access("test").await.unwrap(),
            AccessLevel::Read
        );
        assert_eq!(
            full_access_root.check_access("test").await.unwrap(),
            AccessLevel::Full
        );
    }

    #[tokio::test]
    async fn test_roots_changed_notification() {
        let root = TestRoot {
            uri: "file:///test".to_string(),
            name: None,
            read_only: false,
        };

        let notification = root.notify_roots_changed().await.unwrap();
        assert_eq!(notification.method, "notifications/roots/listChanged");
    }
}
