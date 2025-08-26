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

/// Parameters for resources/subscribe request (per MCP spec)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscribeParams {
    /// Resource URI to subscribe to
    pub uri: String,
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
            },
        }
    }
}

/// Parameters for resources/unsubscribe request (per MCP spec)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnsubscribeParams {
    /// Resource URI to unsubscribe from
    pub uri: String,
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
            },
        }
    }
}

/// Result for resources/read (per MCP spec)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadResourceResult {
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

// Trait implementations for protocol compliance
use crate::traits::Params;
impl Params for SubscribeParams {}
impl Params for UnsubscribeParams {}

// Trait implementations for ListResourcesResult
impl HasData for ListResourcesResult {
    fn data(&self) -> HashMap<String, Value> {
        let mut data = HashMap::new();
        data.insert("resources".to_string(), serde_json::to_value(&self.resources).unwrap_or(Value::Null));
        if let Some(ref next_cursor) = self.next_cursor {
            data.insert("nextCursor".to_string(), Value::String(next_cursor.as_str().to_string()));
        }
        data
    }
}

impl HasMeta for ListResourcesResult {
    fn meta(&self) -> Option<HashMap<String, Value>> {
        self.meta.clone()
    }
}

// ===========================================
// === Fine-Grained Resource Traits ===
// ===========================================

/// Trait for basic resource metadata (uri, name, title)
pub trait HasResourceMetadata {
    /// The URI identifier for this resource
    fn uri(&self) -> &str;
    
    /// Human-readable name
    fn name(&self) -> &str;
    
    /// Optional title for display
    fn title(&self) -> Option<&str> {
        None
    }
}

/// Trait for resource description
pub trait HasResourceDescription {
    /// Description of the resource
    fn description(&self) -> Option<&str>;
}

/// Trait for resource content information
pub trait HasResourceContent {
    /// Optional MIME type hint
    fn mime_type(&self) -> Option<&str>;
    
    /// Content encoding information
    fn encoding(&self) -> Option<&str> {
        None
    }
    
    /// Content size hint if known
    fn content_size(&self) -> Option<u64> {
        None
    }
}

/// Trait for resource access capabilities
pub trait HasResourceAccess {
    /// Whether this resource supports real-time subscriptions
    fn supports_subscriptions(&self) -> bool {
        false
    }
    
    /// Whether this resource requires authentication
    fn requires_auth(&self) -> bool {
        false
    }
    
    /// Access permissions level
    fn access_level(&self) -> ResourceAccessLevel {
        ResourceAccessLevel::Read
    }
}

/// Trait for resource annotations and custom metadata
pub trait HasResourceAnnotations {
    /// Optional annotations for client hints
    fn annotations(&self) -> Option<&serde_json::Value>;
}

/// Trait for resource-specific metadata
pub trait HasResourceMeta {
    /// Optional resource-specific metadata
    fn resource_meta(&self) -> Option<&HashMap<String, serde_json::Value>> {
        None
    }
}

/// Access levels for resources
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceAccessLevel {
    /// Read-only access
    Read,
    /// Read and subscribe access
    ReadSubscribe,
    /// Full access (if resource supports modifications)
    Full,
}

/// Composed resource definition trait (automatically implemented via blanket impl)
pub trait ResourceDefinition: 
    HasResourceMetadata + 
    HasResourceDescription + 
    HasResourceContent + 
    HasResourceAccess + 
    HasResourceAnnotations + 
    HasResourceMeta 
{
    /// Convert this resource definition to a protocol Resource struct
    fn to_resource(&self) -> Resource {
        let mut resource = Resource::new(self.uri(), self.name());
        
        if let Some(description) = self.description() {
            resource = resource.with_description(description);
        }
        
        if let Some(mime_type) = self.mime_type() {
            resource = resource.with_mime_type(mime_type);
        }
        
        if let Some(annotations) = self.annotations() {
            resource = resource.with_annotations(annotations.clone());
        }
        
        resource
    }
}

// Blanket implementation: any type implementing the fine-grained traits automatically gets ResourceDefinition
impl<T> ResourceDefinition for T 
where 
    T: HasResourceMetadata + HasResourceDescription + HasResourceContent + HasResourceAccess + HasResourceAnnotations + HasResourceMeta 
{}

impl RpcResult for ListResourcesResult {}

impl crate::traits::ListResourcesResult for ListResourcesResult {
    fn resources(&self) -> &Vec<Resource> {
        &self.resources
    }
    
    fn next_cursor(&self) -> Option<&Cursor> {
        self.next_cursor.as_ref()
    }
}

// Trait implementations for ReadResourceParams
impl Params for ReadResourceParams {}
impl Params for SubscribeRequest {}
impl Params for UnsubscribeRequest {}

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

// Trait implementations for ReadResourceResult
impl HasData for ReadResourceResult {
    fn data(&self) -> HashMap<String, Value> {
        let mut data = HashMap::new();
        data.insert("contents".to_string(), serde_json::to_value(&self.contents).unwrap_or(Value::Null));
        data
    }
}

impl HasMeta for ReadResourceResult {
    fn meta(&self) -> Option<HashMap<String, Value>> {
        self.meta.clone()
    }
}

impl RpcResult for ReadResourceResult {}

impl crate::traits::ReadResourceResult for ReadResourceResult {
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

        let response = super::ListResourcesResult::new(resources);
        assert_eq!(response.resources.len(), 2);
        assert!(response.next_cursor.is_none());
    }

    #[test]
    fn test_read_resource_response() {
        let content = ResourceContent::text("File contents");
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