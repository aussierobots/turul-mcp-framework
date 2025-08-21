//! MCP Resources Protocol Types
//!
//! This module defines the types used for the MCP resources functionality.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::tools::Cursor;

/// A resource descriptor
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Resource {
    /// URI identifier for the resource
    pub uri: String,
    /// Human-readable name
    pub name: String,
    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// MIME type of the resource
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    /// Optional annotations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Value>,
}

impl Resource {
    pub fn new(uri: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
            name: name.into(),
            description: None,
            mime_type: None,
            annotations: None,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_mime_type(mut self, mime_type: impl Into<String>) -> Self {
        self.mime_type = Some(mime_type.into());
        self
    }

    pub fn with_annotations(mut self, annotations: Value) -> Self {
        self.annotations = Some(annotations);
        self
    }
}

/// Parameters for resources/list request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListResourcesRequest {
    /// Optional cursor for pagination
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<Cursor>,
}

impl ListResourcesRequest {
    pub fn new() -> Self {
        Self { cursor: None }
    }

    pub fn with_cursor(mut self, cursor: Cursor) -> Self {
        self.cursor = Some(cursor);
        self
    }
}

impl Default for ListResourcesRequest {
    fn default() -> Self {
        Self::new()
    }
}

/// Response for resources/list
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListResourcesResponse {
    /// Available resources
    pub resources: Vec<Resource>,
    /// Optional cursor for next page
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<Cursor>,
}

impl ListResourcesResponse {
    pub fn new(resources: Vec<Resource>) -> Self {
        Self {
            resources,
            next_cursor: None,
        }
    }

    pub fn with_next_cursor(mut self, cursor: Cursor) -> Self {
        self.next_cursor = Some(cursor);
        self
    }
}

/// Parameters for resources/read request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadResourceRequest {
    /// URI of the resource to read
    pub uri: String,
}

impl ReadResourceRequest {
    pub fn new(uri: impl Into<String>) -> Self {
        Self { uri: uri.into() }
    }
}

/// Content types that can be returned by resources
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ResourceContent {
    /// Text content
    Text {
        text: String,
    },
    /// Binary content (base64 encoded)
    Blob {
        #[serde(rename = "blob")]
        data: String,
        #[serde(rename = "mimeType")]
        mime_type: String,
    },
}

impl ResourceContent {
    pub fn text(content: impl Into<String>) -> Self {
        Self::Text {
            text: content.into(),
        }
    }

    pub fn blob(data: impl Into<String>, mime_type: impl Into<String>) -> Self {
        Self::Blob {
            data: data.into(),
            mime_type: mime_type.into(),
        }
    }
}

/// Response for resources/read
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadResourceResponse {
    /// The resource content
    pub contents: Vec<ResourceContent>,
}

impl ReadResourceResponse {
    pub fn new(contents: Vec<ResourceContent>) -> Self {
        Self { contents }
    }

    pub fn single(content: ResourceContent) -> Self {
        Self::new(vec![content])
    }
}

/// Resource subscription parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceSubscription {
    /// URI of the resource to subscribe to
    pub uri: String,
}

impl ResourceSubscription {
    pub fn new(uri: impl Into<String>) -> Self {
        Self { uri: uri.into() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_creation() {
        let resource = Resource::new("file:///test.txt", "Test File")
            .with_description("A test file")
            .with_mime_type("text/plain");

        assert_eq!(resource.uri, "file:///test.txt");
        assert_eq!(resource.name, "Test File");
        assert!(resource.description.is_some());
        assert!(resource.mime_type.is_some());
    }

    #[test]
    fn test_resource_content() {
        let text_content = ResourceContent::text("Hello, world!");
        let blob_content = ResourceContent::blob("base64data", "image/png");

        assert!(matches!(text_content, ResourceContent::Text { .. }));
        assert!(matches!(blob_content, ResourceContent::Blob { .. }));
    }

    #[test]
    fn test_list_resources_response() {
        let resources = vec![
            Resource::new("file:///test1.txt", "Test 1"),
            Resource::new("file:///test2.txt", "Test 2"),
        ];

        let response = ListResourcesResponse::new(resources);
        assert_eq!(response.resources.len(), 2);
        assert!(response.next_cursor.is_none());
    }

    #[test]
    fn test_read_resource_response() {
        let content = ResourceContent::text("File contents");
        let response = ReadResourceResponse::single(content);

        assert_eq!(response.contents.len(), 1);
    }

    #[test]
    fn test_serialization() {
        let resource = Resource::new("file:///example.txt", "Example");
        let json = serde_json::to_string(&resource).unwrap();
        assert!(json.contains("file:///example.txt"));
        assert!(json.contains("Example"));

        let parsed: Resource = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.uri, "file:///example.txt");
    }
}