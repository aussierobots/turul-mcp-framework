//! MCP Resources Protocol Types
//!
//! This module defines the types used for the MCP resources functionality.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::meta::Cursor;

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
pub struct ListResourcesParams {
    /// Optional cursor for pagination
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<Cursor>,
    /// Meta information (optional _meta field inside params)
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<std::collections::HashMap<String, Value>>,
}

impl ListResourcesParams {
    pub fn new() -> Self {
        Self { 
            cursor: None,
            meta: None,
        }
    }

    pub fn with_cursor(mut self, cursor: Cursor) -> Self {
        self.cursor = Some(cursor);
        self
    }

    pub fn with_meta(mut self, meta: std::collections::HashMap<String, Value>) -> Self {
        self.meta = Some(meta);
        self
    }
}

impl Default for ListResourcesParams {
    fn default() -> Self {
        Self::new()
    }
}

/// Complete resources/list request (matches TypeScript ListResourcesRequest interface)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListResourcesRequest {
    /// Method name (always "resources/list")
    pub method: String,
    /// Request parameters
    pub params: ListResourcesParams,
}

impl ListResourcesRequest {
    pub fn new() -> Self {
        Self {
            method: "resources/list".to_string(),
            params: ListResourcesParams::new(),
        }
    }

    pub fn with_cursor(mut self, cursor: Cursor) -> Self {
        self.params = self.params.with_cursor(cursor);
        self
    }

    pub fn with_meta(mut self, meta: std::collections::HashMap<String, Value>) -> Self {
        self.params = self.params.with_meta(meta);
        self
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
    /// Meta information (follows MCP Result interface)
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        alias = "_meta",
        rename = "_meta"
    )]
    pub meta: Option<std::collections::HashMap<String, Value>>,
}

impl ListResourcesResponse {
    pub fn new(resources: Vec<Resource>) -> Self {
        Self {
            resources,
            next_cursor: None,
            meta: None,
        }
    }

    pub fn with_next_cursor(mut self, cursor: Cursor) -> Self {
        self.next_cursor = Some(cursor);
        self
    }

    pub fn with_meta(mut self, meta: std::collections::HashMap<String, Value>) -> Self {
        self.meta = Some(meta);
        self
    }
}

/// Parameters for resources/read request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadResourceParams {
    /// URI of the resource to read
    pub uri: String,
    /// Meta information (optional _meta field inside params)
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<std::collections::HashMap<String, Value>>,
}

impl ReadResourceParams {
    pub fn new(uri: impl Into<String>) -> Self {
        Self { 
            uri: uri.into(),
            meta: None,
        }
    }

    pub fn with_meta(mut self, meta: std::collections::HashMap<String, Value>) -> Self {
        self.meta = Some(meta);
        self
    }
}

/// Complete resources/read request (matches TypeScript ReadResourceRequest interface)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadResourceRequest {
    /// Method name (always "resources/read")
    pub method: String,
    /// Request parameters
    pub params: ReadResourceParams,
}

impl ReadResourceRequest {
    pub fn new(uri: impl Into<String>) -> Self {
        Self {
            method: "resources/read".to_string(),
            params: ReadResourceParams::new(uri),
        }
    }

    pub fn with_meta(mut self, meta: std::collections::HashMap<String, Value>) -> Self {
        self.params = self.params.with_meta(meta);
        self
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
    /// Meta information (follows MCP Result interface)
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        alias = "_meta",
        rename = "_meta"
    )]
    pub meta: Option<std::collections::HashMap<String, Value>>,
}

impl ReadResourceResponse {
    pub fn new(contents: Vec<ResourceContent>) -> Self {
        Self { 
            contents,
            meta: None,
        }
    }

    pub fn single(content: ResourceContent) -> Self {
        Self::new(vec![content])
    }

    pub fn with_meta(mut self, meta: std::collections::HashMap<String, Value>) -> Self {
        self.meta = Some(meta);
        self
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

// Trait implementations for resources

use crate::traits::*;
use std::collections::HashMap;

// Trait implementations for ListResourcesParams
impl Params for ListResourcesParams {}

impl HasListResourcesParams for ListResourcesParams {
    fn cursor(&self) -> Option<&Cursor> {
        self.cursor.as_ref()
    }
}

impl HasMetaParam for ListResourcesParams {
    fn meta(&self) -> Option<&std::collections::HashMap<String, Value>> {
        self.meta.as_ref()
    }
}

// Trait implementations for ListResourcesRequest
impl HasMethod for ListResourcesRequest {
    fn method(&self) -> &str {
        &self.method
    }
}

impl HasParams for ListResourcesRequest {
    fn params(&self) -> Option<&dyn Params> {
        Some(&self.params)
    }
}

// Trait implementations for ListResourcesResponse
impl HasData for ListResourcesResponse {
    fn data(&self) -> HashMap<String, Value> {
        let mut data = HashMap::new();
        data.insert("resources".to_string(), serde_json::to_value(&self.resources).unwrap_or(Value::Null));
        if let Some(ref next_cursor) = self.next_cursor {
            data.insert("nextCursor".to_string(), Value::String(next_cursor.as_str().to_string()));
        }
        data
    }
}

impl HasMeta for ListResourcesResponse {
    fn meta(&self) -> Option<HashMap<String, Value>> {
        self.meta.clone()
    }
}

impl RpcResult for ListResourcesResponse {}

impl ListResourcesResult for ListResourcesResponse {
    fn resources(&self) -> &Vec<Resource> {
        &self.resources
    }
    
    fn next_cursor(&self) -> Option<&Cursor> {
        self.next_cursor.as_ref()
    }
}

// Trait implementations for ReadResourceParams
impl Params for ReadResourceParams {}

impl HasReadResourceParams for ReadResourceParams {
    fn uri(&self) -> &String {
        &self.uri
    }
}

impl HasMetaParam for ReadResourceParams {
    fn meta(&self) -> Option<&std::collections::HashMap<String, Value>> {
        self.meta.as_ref()
    }
}

// Trait implementations for ReadResourceRequest
impl HasMethod for ReadResourceRequest {
    fn method(&self) -> &str {
        &self.method
    }
}

impl HasParams for ReadResourceRequest {
    fn params(&self) -> Option<&dyn Params> {
        Some(&self.params)
    }
}

// Trait implementations for ReadResourceResponse
impl HasData for ReadResourceResponse {
    fn data(&self) -> HashMap<String, Value> {
        let mut data = HashMap::new();
        data.insert("contents".to_string(), serde_json::to_value(&self.contents).unwrap_or(Value::Null));
        data
    }
}

impl HasMeta for ReadResourceResponse {
    fn meta(&self) -> Option<HashMap<String, Value>> {
        self.meta.clone()
    }
}

impl RpcResult for ReadResourceResponse {}

impl ReadResourceResult for ReadResourceResponse {
    fn contents(&self) -> &Vec<ResourceContent> {
        &self.contents
    }
}

// Trait implementations for ResourceSubscription
impl Params for ResourceSubscription {}

impl HasResourceUpdatedParams for ResourceSubscription {
    fn uri(&self) -> &String {
        &self.uri
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

    #[test]
    fn test_trait_implementations() {
        let request = ListResourcesRequest::new();
        assert!(request.params.cursor.is_none());
        
        let resources = vec![Resource::new("test://resource", "Test Resource")];
        let response = ListResourcesResponse::new(resources);
        assert_eq!(response.resources().len(), 1);
        assert!(response.next_cursor().is_none());
        
        let data = response.data();
        assert!(data.contains_key("resources"));
    }
}