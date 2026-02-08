//! Framework traits for MCP root construction
//!
//! **IMPORTANT**: These are framework features, NOT part of the MCP specification.

use turul_mcp_protocol::roots::Root;
use serde_json::Value;
use std::collections::HashMap;

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
            if let Some(ext) = path.split('.').next_back() {
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

/// **Complete MCP Root Creation** - Build secure file system access boundaries.
///
/// This trait represents a **complete, working MCP root** that defines secure access
/// boundaries for file system operations with permissions, filtering, and metadata.
/// When you implement the required metadata traits, you automatically get
/// `RootDefinition` for free via blanket implementation.
///
/// # What You're Building
///
/// A root is a secure file system boundary that:
/// - Defines accessible file system paths for clients
/// - Enforces security permissions and access control
/// - Filters files and directories based on rules
/// - Provides metadata annotations for client context
///
/// # How to Create a Root
///
/// Implement these four traits on your struct:
///
/// ```rust
/// # use turul_mcp_protocol::roots::*;
/// # use turul_mcp_builders::prelude::*;
/// # use serde_json::{Value, json};
/// # use std::collections::HashMap;
///
/// // This struct will automatically implement RootDefinition!
/// struct ProjectRoot {
///     base_path: String,
///     project_name: String,
/// }
///
/// impl HasRootMetadata for ProjectRoot {
///     fn uri(&self) -> &str {
///         &self.base_path
///     }
///
///     fn name(&self) -> Option<&str> {
///         Some(&self.project_name)
///     }
/// }
///
/// impl HasRootPermissions for ProjectRoot {
///     fn can_read(&self, _path: &str) -> bool {
///         true // Allow reading all files in project
///     }
///
///     fn can_write(&self, path: &str) -> bool {
///         // Only allow writing to src/ and tests/ directories
///         path.contains("/src/") || path.contains("/tests/")
///     }
///
///     fn max_depth(&self) -> Option<usize> {
///         Some(10) // Limit depth to prevent infinite recursion
///     }
/// }
///
/// impl HasRootFiltering for ProjectRoot {
///     fn excluded_patterns(&self) -> Option<&[String]> {
///         static PATTERNS: &[String] = &[];
///         None // Use default filtering
///     }
///
///     fn should_include(&self, path: &str) -> bool {
///         // Exclude hidden files and build artifacts
///         !path.contains("/.") && !path.contains("/target/")
///     }
/// }
///
/// impl HasRootAnnotations for ProjectRoot {
///     fn annotations(&self) -> Option<&HashMap<String, Value>> {
///         // Static annotations for this example
///         None
///     }
/// }
///
/// // Now you can use it with the server:
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let root = ProjectRoot {
///     base_path: "file:///workspace/my-project".to_string(),
///     project_name: "My Rust Project".to_string(),
/// };
///
/// // The root automatically implements RootDefinition
/// let protocol_root = root.to_root();
/// let validation_result = root.validate();
/// # Ok(())
/// # }
/// ```
///
/// # Key Benefits
///
/// - **Security**: Fine-grained access control for file operations
/// - **Filtering**: Automatic exclusion of unwanted files/directories
/// - **Metadata**: Rich annotations for client context
/// - **MCP Compliant**: Fully compatible with MCP 2025-11-25 specification
///
/// # Common Use Cases
///
/// - Project workspace boundaries
/// - Secure document repositories
/// - Code review access control
/// - Filtered file system views
/// - Multi-tenant file access
pub trait RootDefinition:
    HasRootMetadata + HasRootPermissions + HasRootFiltering + HasRootAnnotations
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
impl<T> RootDefinition for T where
    T: HasRootMetadata + HasRootPermissions + HasRootFiltering + HasRootAnnotations
{
}
