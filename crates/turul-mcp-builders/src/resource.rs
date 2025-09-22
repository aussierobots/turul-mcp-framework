//! Resource Builder for Runtime Resource Construction
//!
//! This module provides a builder pattern for creating resources at runtime
//! without requiring procedural macros. This enables dynamic resource creation
//! for configuration-driven systems.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use serde_json::Value;

// Import from protocol via alias
use turul_mcp_protocol::resources::{
    ResourceContent, 
    HasResourceMetadata, HasResourceDescription, HasResourceUri, 
    HasResourceMimeType, HasResourceSize, HasResourceAnnotations, HasResourceMeta
};
use turul_mcp_protocol::meta::Annotations;

/// Type alias for dynamic resource read function
pub type DynamicResourceFn = Box<dyn Fn(String) -> Pin<Box<dyn Future<Output = Result<ResourceContent, String>> + Send>> + Send + Sync>;

/// Builder for creating resources at runtime
pub struct ResourceBuilder {
    uri: String,
    name: String,
    title: Option<String>,
    description: Option<String>,
    mime_type: Option<String>,
    size: Option<u64>,
    content: Option<ResourceContent>,
    annotations: Option<Annotations>,
    meta: Option<HashMap<String, Value>>,
    read_fn: Option<DynamicResourceFn>,
}

impl ResourceBuilder {
    /// Create a new resource builder with the given URI and name
    pub fn new(uri: impl Into<String>) -> Self {
        let uri = uri.into();
        // Extract a reasonable default name from the URI
        let name = uri.split('/').next_back().unwrap_or(&uri).to_string();
        
        Self {
            uri,
            name,
            title: None,
            description: None,
            mime_type: None,
            size: None,
            content: None,
            annotations: None,
            meta: None,
            read_fn: None,
        }
    }

    /// Set the resource name (programmatic identifier)
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Set the resource title (display name)
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the resource description
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the MIME type
    pub fn mime_type(mut self, mime_type: impl Into<String>) -> Self {
        self.mime_type = Some(mime_type.into());
        self
    }

    /// Set the resource size in bytes
    pub fn size(mut self, size: u64) -> Self {
        self.size = Some(size);
        self
    }

    /// Set static text content for this resource
    pub fn text_content(mut self, text: impl Into<String>) -> Self {
        let text = text.into();
        self.size = Some(text.len() as u64);
        if self.mime_type.is_none() {
            self.mime_type = Some("text/plain".to_string());
        }
        self.content = Some(ResourceContent::text(&self.uri, text));
        self
    }

    /// Set static JSON content for this resource
    pub fn json_content(mut self, json_value: Value) -> Self {
        let text = serde_json::to_string_pretty(&json_value)
            .unwrap_or_else(|_| "{}".to_string());
        self.size = Some(text.len() as u64);
        self.mime_type = Some("application/json".to_string());
        self.content = Some(ResourceContent::text(&self.uri, text));
        self
    }

    /// Set static blob content for this resource (base64-encoded)
    pub fn blob_content(mut self, blob: impl Into<String>, mime_type: impl Into<String>) -> Self {
        let blob = blob.into();
        let mime_type = mime_type.into();
        
        // Estimate size from base64 (approximately 3/4 of encoded length)
        self.size = Some((blob.len() * 3 / 4) as u64);
        self.mime_type = Some(mime_type.clone());
        self.content = Some(ResourceContent::blob(&self.uri, blob, mime_type));
        self
    }

    /// Set annotations
    pub fn annotations(mut self, annotations: Annotations) -> Self {
        self.annotations = Some(annotations);
        self
    }

    /// Add annotation title (only field currently supported in Annotations)
    pub fn annotation_title(mut self, title: impl Into<String>) -> Self {
        let mut annotations = self.annotations.unwrap_or_default();
        annotations.title = Some(title.into());
        self.annotations = Some(annotations);
        self
    }

    /// Set meta information
    pub fn meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.meta = Some(meta);
        self
    }

    /// Set the read function for dynamic content
    pub fn read<F, Fut>(mut self, f: F) -> Self
    where
        F: Fn(String) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<ResourceContent, String>> + Send + 'static,
    {
        self.read_fn = Some(Box::new(move |uri| {
            Box::pin(f(uri))
        }));
        self
    }

    /// Convenience method to set a text read function
    pub fn read_text<F, Fut>(mut self, f: F) -> Self
    where
        F: Fn(String) -> Fut + Send + Sync + 'static + Clone,
        Fut: Future<Output = Result<String, String>> + Send + 'static,
    {
        self.read_fn = Some(Box::new(move |uri| {
            let f = f.clone();
            let uri_clone = uri.clone();
            Box::pin(async move {
                let text = f(uri.clone()).await?;
                Ok(ResourceContent::text(uri_clone, text))
            })
        }));
        self
    }

    /// Build the dynamic resource
    pub fn build(self) -> Result<DynamicResource, String> {
        Ok(DynamicResource {
            uri: self.uri,
            name: self.name,
            title: self.title,
            description: self.description,
            mime_type: self.mime_type,
            size: self.size,
            content: self.content,
            annotations: self.annotations,
            meta: self.meta,
            read_fn: self.read_fn,
        })
    }
}

/// Dynamic resource created by ResourceBuilder
pub struct DynamicResource {
    uri: String,
    name: String,
    title: Option<String>,
    description: Option<String>,
    mime_type: Option<String>,
    size: Option<u64>,
    content: Option<ResourceContent>,
    annotations: Option<Annotations>,
    meta: Option<HashMap<String, Value>>,
    read_fn: Option<DynamicResourceFn>,
}

impl DynamicResource {
    /// Read the resource content
    pub async fn read(&self) -> Result<ResourceContent, String> {
        if let Some(ref content) = self.content {
            // Static content
            Ok(content.clone())
        } else if let Some(ref read_fn) = self.read_fn {
            // Dynamic content
            read_fn(self.uri.clone()).await
        } else {
            Err("No content or read function provided".to_string())
        }
    }
}

// Implement all fine-grained traits for DynamicResource
impl HasResourceMetadata for DynamicResource {
    fn name(&self) -> &str {
        &self.name
    }

    fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }
}

impl HasResourceDescription for DynamicResource {
    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}

impl HasResourceUri for DynamicResource {
    fn uri(&self) -> &str {
        &self.uri
    }
}

impl HasResourceMimeType for DynamicResource {
    fn mime_type(&self) -> Option<&str> {
        self.mime_type.as_deref()
    }
}

impl HasResourceSize for DynamicResource {
    fn size(&self) -> Option<u64> {
        self.size
    }
}

impl HasResourceAnnotations for DynamicResource {
    fn annotations(&self) -> Option<&Annotations> {
        self.annotations.as_ref()
    }
}

impl HasResourceMeta for DynamicResource {
    fn resource_meta(&self) -> Option<&HashMap<String, Value>> {
        self.meta.as_ref()
    }
}

// ResourceDefinition is automatically implemented via blanket impl!

// Note: McpResource implementation will be provided by the turul-mcp-server crate
// since it depends on types from that crate (SessionContext, etc.)

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_resource_builder_basic() {
        let resource = ResourceBuilder::new("file:///test.txt")
            .name("test_resource")
            .description("A test resource")
            .text_content("Hello, World!")
            .build()
            .expect("Failed to build resource");

        assert_eq!(resource.name(), "test_resource");
        assert_eq!(resource.uri(), "file:///test.txt");
        assert_eq!(resource.description(), Some("A test resource"));
        assert_eq!(resource.mime_type(), Some("text/plain"));
        assert_eq!(resource.size(), Some(13)); // Length of "Hello, World!"
    }

    #[tokio::test]
    async fn test_resource_builder_static_content() {
        let resource = ResourceBuilder::new("file:///config.json")
            .description("Application configuration")
            .json_content(json!({"version": "1.0", "debug": true}))
            .build()
            .expect("Failed to build resource");

        let content = resource.read().await.expect("Failed to read content");
        
        match content {
            ResourceContent::Text(text_content) => {
                assert!(text_content.text.contains("version"));
                assert!(text_content.text.contains("1.0"));
                assert_eq!(text_content.uri, "file:///config.json");
            },
            _ => panic!("Expected text content"),
        }
        
        // Verify the resource itself has the correct MIME type
        assert_eq!(resource.mime_type(), Some("application/json"));
    }

    #[tokio::test]
    async fn test_resource_builder_dynamic_content() {
        let resource = ResourceBuilder::new("file:///dynamic.txt")
            .description("Dynamic content resource")
            .read_text(|_uri| async move {
                Ok("This is dynamic content!".to_string())
            })
            .build()
            .expect("Failed to build resource");

        let content = resource.read().await.expect("Failed to read content");
        
        match content {
            ResourceContent::Text(text_content) => {
                assert_eq!(text_content.text, "This is dynamic content!");
            },
            _ => panic!("Expected text content"),
        }
    }

    #[test]
    fn test_resource_builder_annotations() {
        let resource = ResourceBuilder::new("file:///important.txt")
            .description("Important resource")
            .annotation_title("Important File")
            .build()
            .expect("Failed to build resource");

        let annotations = resource.annotations().expect("Expected annotations");
        assert_eq!(annotations.title, Some("Important File".to_string()));
    }

    #[test] 
    fn test_resource_builder_blob_content() {
        let resource = ResourceBuilder::new("data://example.png")
            .description("Example image")
            .blob_content("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==", "image/png")
            .build()
            .expect("Failed to build resource");

        assert_eq!(resource.mime_type(), Some("image/png"));
        assert!(resource.size().unwrap() > 0);
    }
}