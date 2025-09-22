//! Implementation of #[derive(McpRoot)] macro

use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Result};

use crate::utils::extract_root_meta;

pub fn derive_mcp_root_impl(input: DeriveInput) -> Result<TokenStream> {
    let struct_name = &input.ident;

    // Extract struct-level attributes from #[root(...)]
    let root_meta = extract_root_meta(&input.attrs)?;
    let uri = &root_meta.uri;
    let name = &root_meta.name;
    let description = &root_meta.description;
    let read_only = root_meta.read_only;

    let expanded = quote! {
        #[automatically_derived]
        impl turul_mcp_protocol::roots::HasRootMetadata for #struct_name {
            fn uri(&self) -> &str {
                #uri
            }

            fn name(&self) -> Option<&str> {
                Some(#name)
            }

            fn description(&self) -> Option<&str> {
                Some(#description)
            }
        }

        #[automatically_derived]
        impl turul_mcp_protocol::roots::HasRootPermissions for #struct_name {
            fn can_read(&self, _path: &str) -> bool {
                true
            }

            fn can_write(&self, _path: &str) -> bool {
                !#read_only
            }

            fn max_depth(&self) -> Option<usize> {
                Some(50) // Reasonable default depth limit
            }
        }

        #[automatically_derived]
        impl turul_mcp_protocol::roots::HasRootFiltering for #struct_name {
            fn allowed_extensions(&self) -> Option<&[String]> {
                None // Allow all extensions by default
            }

            fn excluded_patterns(&self) -> Option<&[String]> {
                use std::sync::LazyLock;
                static EXCLUDED: LazyLock<Vec<String>> = LazyLock::new(|| {
                    vec![
                        ".git".to_string(),
                        ".DS_Store".to_string(),
                        "node_modules".to_string(),
                        "target".to_string(),
                        ".env".to_string(),
                    ]
                });
                Some(&EXCLUDED)
            }

            fn should_include(&self, path: &str) -> bool {
                // Default exclusion logic
                if let Some(patterns) = self.excluded_patterns() {
                    for pattern in patterns {
                        if path.contains(pattern) {
                            return false;
                        }
                    }
                }
                true
            }
        }

        #[automatically_derived]
        impl turul_mcp_protocol::roots::HasRootAnnotations for #struct_name {
            fn annotations(&self) -> Option<&std::collections::HashMap<String, serde_json::Value>> {
                None
            }

            fn tags(&self) -> Option<&[String]> {
                None
            }
        }

        // RootDefinition is automatically implemented via blanket impl

        #[automatically_derived]
        #[async_trait::async_trait]
        impl turul_mcp_server::McpRoot for #struct_name {
            async fn list_roots(&self, request: turul_mcp_protocol::roots::ListRootsRequest) -> turul_mcp_protocol::McpResult<turul_mcp_protocol::roots::ListRootsResult> {
                // Default implementation - this should be overridden by implementing list_roots_impl
                match self.list_roots_impl(request).await {
                    Ok(result) => Ok(result),
                    Err(e) => Err(turul_mcp_protocol::McpError::tool_execution(&e)),
                }
            }

            async fn list_files(&self, path: &str) -> turul_mcp_protocol::McpResult<Vec<turul_mcp_server::roots::FileInfo>> {
                // Default implementation - this should be overridden by implementing list_files_impl
                match self.list_files_impl(path).await {
                    Ok(result) => Ok(result),
                    Err(e) => Err(turul_mcp_protocol::McpError::tool_execution(&e)),
                }
            }

            async fn check_access(&self, path: &str) -> turul_mcp_protocol::McpResult<turul_mcp_server::roots::AccessLevel> {
                // Default implementation based on permissions
                if self.can_write(path) {
                    Ok(turul_mcp_server::roots::AccessLevel::Full)
                } else if self.can_read(path) {
                    Ok(turul_mcp_server::roots::AccessLevel::Read)
                } else {
                    Ok(turul_mcp_server::roots::AccessLevel::None)
                }
            }
        }

        impl #struct_name {
            /// Override this method to provide custom root listing logic
            pub async fn list_roots_impl(&self, _request: turul_mcp_protocol::roots::ListRootsRequest) -> Result<turul_mcp_protocol::roots::ListRootsResult, String> {
                // Default: return this root only
                let root = self.to_root();
                Ok(turul_mcp_protocol::roots::ListRootsResult::new(vec![root]))
            }

            /// Override this method to provide custom file listing logic
            pub async fn list_files_impl(&self, path: &str) -> Result<Vec<turul_mcp_server::roots::FileInfo>, String> {
                // Default: basic file listing using std::fs
                use std::fs;
                use std::path::Path;

                let root_path = self.uri().replace("file://", "");
                let full_path = if path.is_empty() || path == "/" {
                    root_path
                } else {
                    format!("{}/{}", root_path.trim_end_matches('/'), path.trim_start_matches('/'))
                };

                let path_obj = Path::new(&full_path);
                if !path_obj.exists() {
                    return Err(format!("Path does not exist: {}", full_path));
                }

                if !path_obj.is_dir() {
                    return Err(format!("Path is not a directory: {}", full_path));
                }

                let mut files = Vec::new();
                match fs::read_dir(&path_obj) {
                    Ok(entries) => {
                        for entry in entries {
                            match entry {
                                Ok(entry) => {
                                    let entry_path = entry.path();
                                    let relative_path = entry_path.strip_prefix(&root_path)
                                        .unwrap_or(&entry_path)
                                        .to_string_lossy()
                                        .to_string();

                                    if self.should_include(&relative_path) {
                                        let metadata = entry.metadata().ok();
                                        let file_info = turul_mcp_server::roots::FileInfo {
                                            path: relative_path,
                                            is_directory: metadata.map(|m| m.is_dir()).unwrap_or(false),
                                            size: metadata.as_ref().and_then(|m| if m.is_file() { Some(m.len()) } else { None }),
                                            modified: metadata.and_then(|m| {
                                                m.modified().ok().and_then(|t| {
                                                    t.duration_since(std::time::UNIX_EPOCH).ok().map(|d| d.as_secs())
                                                })
                                            }),
                                            mime_type: None, // Could be enhanced with mime detection
                                        };
                                        files.push(file_info);
                                    }
                                }
                                Err(_) => continue,
                            }
                        }
                    }
                    Err(e) => return Err(format!("Failed to read directory: {}", e)),
                }

                Ok(files)
            }
        }
    };

    Ok(expanded)
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_simple_root() {
        let input: DeriveInput = parse_quote! {
            #[root(uri = "file:///home/user/project", name = "Project Root")]
            struct ProjectRoot {
                path: String,
                read_only: bool,
            }
        };

        let result = derive_mcp_root_impl(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_read_only_root() {
        let input: DeriveInput = parse_quote! {
            #[root(uri = "file:///usr/share/docs", name = "Documentation", read_only = "true")]
            struct DocsRoot;
        };

        let result = derive_mcp_root_impl(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_missing_uri() {
        let input: DeriveInput = parse_quote! {
            #[root(name = "Test Root")]
            struct TestRoot {
                path: String,
            }
        };

        let result = derive_mcp_root_impl(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_minimal_root() {
        let input: DeriveInput = parse_quote! {
            #[root(uri = "file:///tmp")]
            struct TempRoot;
        };

        let result = derive_mcp_root_impl(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_complex_root_with_all_attributes() {
        let input: DeriveInput = parse_quote! {
            #[root(uri = "file:///project", name = "Project Files", description = "Project workspace", read_only = false)]
            struct ProjectRoot;
        };

        let result = derive_mcp_root_impl(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_root_with_boolean_literal() {
        let input: DeriveInput = parse_quote! {
            #[root(uri = "file:///readonly", name = "Read Only", read_only = true)]
            struct ReadOnlyRoot;
        };

        let result = derive_mcp_root_impl(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_root_trait_implementations() {
        // This is a compile-time test - ensuring traits are properly implemented
        let input: DeriveInput = parse_quote! {
            #[root(uri = "file:///test")]
            struct TestTraitRoot;
        };

        let result = derive_mcp_root_impl(input);
        assert!(result.is_ok());

        // Check that the generated code contains required trait implementations
        let code = result.unwrap().to_string();
        assert!(code.contains("HasRootMetadata"));
        assert!(code.contains("HasRootPermissions"));
        assert!(code.contains("McpRoot"));
    }
}
