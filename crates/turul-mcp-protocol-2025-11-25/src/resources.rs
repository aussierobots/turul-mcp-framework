//! MCP Resources Protocol Types
//!
//! This module defines the types used for the MCP resources functionality.

use crate::meta::Cursor;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

// ===========================================

/// A template description for resources available on the server
/// ResourceTemplate extends BaseMetadata
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceTemplate {
    /// Programmatic identifier (from BaseMetadata)
    pub name: String,
    /// Human-readable display name (from BaseMetadata)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// A URI template (according to RFC 6570) that can be used to construct resource URIs (format: uri-template)
    #[serde(rename = "uriTemplate")]
    pub uri_template: String,
    /// A description of what this template is for
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The MIME type for all resources that match this template
    #[serde(rename = "mimeType", skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    /// Optional annotations for the client
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<crate::meta::Annotations>,
    /// Optional icons for display. Most implementations do not need icons.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icons: Option<Vec<crate::icons::Icon>>,
    /// See General fields: _meta for notes on _meta usage
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<HashMap<String, Value>>,
}

impl ResourceTemplate {
    pub fn new(name: impl Into<String>, uri_template: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            title: None,
            uri_template: uri_template.into(),
            description: None,
            mime_type: None,
            annotations: None,
            icons: None,
            meta: None,
        }
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_mime_type(mut self, mime_type: impl Into<String>) -> Self {
        self.mime_type = Some(mime_type.into());
        self
    }

    pub fn with_annotations(mut self, annotations: crate::meta::Annotations) -> Self {
        self.annotations = Some(annotations);
        self
    }

    pub fn with_icons(mut self, icons: Vec<crate::icons::Icon>) -> Self {
        self.icons = Some(icons);
        self
    }

    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.meta = Some(meta);
        self
    }
}

/// A resource descriptor (matches TypeScript Resource interface)
/// Resource extends BaseMetadata, so it includes name and title fields
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Resource {
    /// The URI of this resource (format: uri)
    pub uri: String,
    /// Programmatic identifier (from BaseMetadata)
    pub name: String,
    /// Human-readable display name (from BaseMetadata)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// A description of what this resource represents
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The MIME type of this resource, if known
    #[serde(rename = "mimeType", skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    /// The size of the raw resource content, in bytes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    /// Optional annotations for the client
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<crate::meta::Annotations>,
    /// Optional icons for display. Most implementations do not need icons.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icons: Option<Vec<crate::icons::Icon>>,
    /// See General fields: _meta for notes on _meta usage
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<HashMap<String, Value>>,
}

impl Resource {
    pub fn new(uri: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
            name: name.into(),
            title: None,
            description: None,
            mime_type: None,
            size: None,
            annotations: None,
            icons: None,
            meta: None,
        }
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_mime_type(mut self, mime_type: impl Into<String>) -> Self {
        self.mime_type = Some(mime_type.into());
        self
    }

    pub fn with_size(mut self, size: u64) -> Self {
        self.size = Some(size);
        self
    }

    pub fn with_annotations(mut self, annotations: crate::meta::Annotations) -> Self {
        self.annotations = Some(annotations);
        self
    }

    pub fn with_icons(mut self, icons: Vec<crate::icons::Icon>) -> Self {
        self.icons = Some(icons);
        self
    }

    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.meta = Some(meta);
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

impl Default for ListResourcesRequest {
    fn default() -> Self {
        Self::new()
    }
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

/// Result for resources/list (per MCP spec)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListResourcesResult {
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

impl ListResourcesResult {
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

/// The contents of a specific resource or sub-resource (base interface)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceContents {
    /// The URI of this resource (format: uri)
    pub uri: String,
    /// The MIME type of this resource, if known
    #[serde(rename = "mimeType", skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    /// See General fields: _meta for notes on _meta usage
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<HashMap<String, Value>>,
}

/// Text resource contents
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextResourceContents {
    /// The URI of this resource (format: uri)
    pub uri: String,
    /// The MIME type of this resource, if known
    #[serde(rename = "mimeType", skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    /// See General fields: _meta for notes on _meta usage
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<HashMap<String, Value>>,
    /// The text of the item. This must only be set if the item can actually be represented as text (not binary data)
    pub text: String,
}

/// Blob resource contents
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlobResourceContents {
    /// The URI of this resource (format: uri)
    pub uri: String,
    /// The MIME type of this resource, if known
    #[serde(rename = "mimeType", skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    /// See General fields: _meta for notes on _meta usage
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<HashMap<String, Value>>,
    /// A base64-encoded string representing the binary data of the item (format: byte)
    pub blob: String,
}

/// Union type for resource contents (matches TypeScript union)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ResourceContent {
    Text(TextResourceContents),
    Blob(BlobResourceContents),
}

impl ResourceContent {
    pub fn text(uri: impl Into<String>, text: impl Into<String>) -> Self {
        Self::Text(TextResourceContents {
            uri: uri.into(),
            mime_type: Some("text/plain".to_string()),
            meta: None,
            text: text.into(),
        })
    }

    pub fn json(uri: impl Into<String>, json: impl Into<String>) -> Self {
        Self::Text(TextResourceContents {
            uri: uri.into(),
            mime_type: Some("application/json".to_string()),
            meta: None,
            text: json.into(),
        })
    }

    pub fn blob(
        uri: impl Into<String>,
        blob: impl Into<String>,
        mime_type: impl Into<String>,
    ) -> Self {
        Self::Blob(BlobResourceContents {
            uri: uri.into(),
            mime_type: Some(mime_type.into()),
            meta: None,
            blob: blob.into(),
        })
    }
}

/// Parameters for resources/templates/list request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListResourceTemplatesParams {
    /// Optional cursor for pagination
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<Cursor>,
    /// Meta information (optional _meta field inside params)
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<HashMap<String, Value>>,
}

impl Default for ListResourceTemplatesParams {
    fn default() -> Self {
        Self::new()
    }
}

impl ListResourceTemplatesParams {
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

    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.meta = Some(meta);
        self
    }
}

/// Complete resources/templates/list request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListResourceTemplatesRequest {
    /// Method name (always "resources/templates/list")
    pub method: String,
    /// Request parameters
    pub params: ListResourceTemplatesParams,
}

impl Default for ListResourceTemplatesRequest {
    fn default() -> Self {
        Self::new()
    }
}

impl ListResourceTemplatesRequest {
    pub fn new() -> Self {
        Self {
            method: "resources/templates/list".to_string(),
            params: ListResourceTemplatesParams::new(),
        }
    }
}

/// Result for resources/templates/list
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListResourceTemplatesResult {
    /// Available resource templates
    #[serde(rename = "resourceTemplates")]
    pub resource_templates: Vec<ResourceTemplate>,
    /// Optional cursor for next page
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<Cursor>,
    /// Meta information
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<HashMap<String, Value>>,
}

impl ListResourceTemplatesResult {
    pub fn new(resource_templates: Vec<ResourceTemplate>) -> Self {
        Self {
            resource_templates,
            next_cursor: None,
            meta: None,
        }
    }

    pub fn with_next_cursor(mut self, cursor: Cursor) -> Self {
        self.next_cursor = Some(cursor);
        self
    }

    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.meta = Some(meta);
        self
    }
}

/// Parameters for resources/subscribe request (per MCP spec)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscribeParams {
    /// Resource URI to subscribe to (format: uri)
    pub uri: String,
    /// Meta information (optional _meta field inside params)
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<HashMap<String, Value>>,
}

/// Complete resources/subscribe request (per MCP spec)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscribeRequest {
    /// Method name (always "resources/subscribe")
    pub method: String,
    /// Request parameters
    pub params: SubscribeParams,
}

impl SubscribeRequest {
    pub fn new(uri: impl Into<String>) -> Self {
        Self {
            method: "resources/subscribe".to_string(),
            params: SubscribeParams {
                uri: uri.into(),
                meta: None,
            },
        }
    }

    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.params.meta = Some(meta);
        self
    }
}

/// Parameters for resources/unsubscribe request (per MCP spec)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnsubscribeParams {
    /// Resource URI to unsubscribe from (format: uri)
    pub uri: String,
    /// Meta information (optional _meta field inside params)
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<HashMap<String, Value>>,
}

/// Complete resources/unsubscribe request (per MCP spec)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnsubscribeRequest {
    /// Method name (always "resources/unsubscribe")
    pub method: String,
    /// Request parameters
    pub params: UnsubscribeParams,
}

impl UnsubscribeRequest {
    pub fn new(uri: impl Into<String>) -> Self {
        Self {
            method: "resources/unsubscribe".to_string(),
            params: UnsubscribeParams {
                uri: uri.into(),
                meta: None,
            },
        }
    }

    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.params.meta = Some(meta);
        self
    }
}

/// Result for resources/read (per MCP spec)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadResourceResult {
    /// The resource content (TextResourceContents | BlobResourceContents)[]
    pub contents: Vec<ResourceContent>,
    /// Meta information (follows MCP Result interface)
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<HashMap<String, Value>>,
}

impl ReadResourceResult {
    pub fn new(contents: Vec<ResourceContent>) -> Self {
        Self {
            contents,
            meta: None,
        }
    }

    pub fn single(content: ResourceContent) -> Self {
        Self::new(vec![content])
    }

    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.meta = Some(meta);
        self
    }
}

// Add trait implementations for ReadResourceResult
impl HasData for ReadResourceResult {
    fn data(&self) -> HashMap<String, Value> {
        let mut data = HashMap::new();
        data.insert(
            "contents".to_string(),
            serde_json::to_value(&self.contents).unwrap_or(Value::Null),
        );
        data
    }
}

impl HasMeta for ReadResourceResult {
    fn meta(&self) -> Option<HashMap<String, Value>> {
        self.meta.clone()
    }
}

impl crate::traits::RpcResult for ReadResourceResult {}

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

// Trait implementations for all params types
impl Params for ListResourcesParams {}
impl Params for ReadResourceParams {}
impl Params for SubscribeParams {}
impl Params for UnsubscribeParams {}
impl Params for ListResourceTemplatesParams {}

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

// Additional trait implementations for ListResourceTemplatesResult
impl HasData for ListResourceTemplatesResult {
    fn data(&self) -> HashMap<String, Value> {
        let mut data = HashMap::new();
        data.insert(
            "resourceTemplates".to_string(),
            serde_json::to_value(&self.resource_templates).unwrap_or(Value::Null),
        );
        if let Some(ref next_cursor) = self.next_cursor {
            data.insert(
                "nextCursor".to_string(),
                Value::String(next_cursor.as_str().to_string()),
            );
        }
        data
    }
}

impl HasMeta for ListResourceTemplatesResult {
    fn meta(&self) -> Option<HashMap<String, Value>> {
        self.meta.clone()
    }
}

impl crate::traits::RpcResult for ListResourceTemplatesResult {}

impl crate::traits::ListResourceTemplatesResult for ListResourceTemplatesResult {
    fn resource_templates(&self) -> &Vec<ResourceTemplate> {
        &self.resource_templates
    }

    fn next_cursor(&self) -> Option<&Cursor> {
        self.next_cursor.as_ref()
    }
}

// Trait implementations for ListResourcesResult
impl HasData for ListResourcesResult {
    fn data(&self) -> HashMap<String, Value> {
        let mut data = HashMap::new();
        data.insert(
            "resources".to_string(),
            serde_json::to_value(&self.resources).unwrap_or(Value::Null),
        );
        if let Some(ref next_cursor) = self.next_cursor {
            data.insert(
                "nextCursor".to_string(),
                Value::String(next_cursor.as_str().to_string()),
            );
        }
        data
    }
}

impl HasMeta for ListResourcesResult {
    fn meta(&self) -> Option<HashMap<String, Value>> {
        self.meta.clone()
    }
}

// RpcResult automatically implemented via blanket impl (HasMeta + HasData)
impl crate::traits::RpcResult for ListResourcesResult {}

impl crate::traits::ListResourcesResult for ListResourcesResult {
    fn resources(&self) -> &Vec<Resource> {
        &self.resources
    }

    fn next_cursor(&self) -> Option<&Cursor> {
        self.next_cursor.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::ListResourcesResult;

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
        let text_content = ResourceContent::text("file:///test.txt", "Hello, world!");
        let blob_content = ResourceContent::blob("file:///image.png", "base64data", "image/png");

        assert!(matches!(text_content, ResourceContent::Text(_)));
        assert!(matches!(blob_content, ResourceContent::Blob(_)));
    }

    #[test]
    fn test_list_resources_response() {
        let resources = vec![
            Resource::new("file:///test1.txt", "Test 1"),
            Resource::new("file:///test2.txt", "Test 2"),
        ];

        let response = super::ListResourcesResult::new(resources);
        assert_eq!(response.resources.len(), 2);
        assert!(response.next_cursor.is_none());
    }

    #[test]
    fn test_read_resource_response() {
        let content = ResourceContent::text("file:///test.txt", "File contents");
        let response = ReadResourceResult::single(content);

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
        let response = super::ListResourcesResult::new(resources);
        assert_eq!(response.resources().len(), 1);
        assert!(response.next_cursor().is_none());

        let data = response.data();
        assert!(data.contains_key("resources"));
    }
}
