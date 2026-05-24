//! Content types for MCP 2025-11-25 specification
//!
//! This module contains the exact content type definitions from the MCP spec,
//! ensuring perfect compliance with the TypeScript schema definitions.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::meta::Annotations;

/// Text resource contents (matches TypeScript TextResourceContents exactly)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextResourceContents {
    /// The URI of this resource (REQUIRED by MCP spec)
    pub uri: String,
    /// The MIME type of this resource, if known
    #[serde(rename = "mimeType", skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    /// Meta information (REQUIRED by MCP spec)
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<HashMap<String, Value>>,
    /// The text content
    pub text: String,
}

/// Binary resource contents (matches TypeScript BlobResourceContents exactly)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlobResourceContents {
    /// The URI of this resource (REQUIRED by MCP spec)
    pub uri: String,
    /// The MIME type of this resource, if known
    #[serde(rename = "mimeType", skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    /// Meta information (REQUIRED by MCP spec)
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<HashMap<String, Value>>,
    /// Base64-encoded binary data
    pub blob: String,
}

/// Resource contents union type (matches TypeScript TextResourceContents | BlobResourceContents)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ResourceContents {
    /// Text content
    Text(TextResourceContents),
    /// Binary content
    Blob(BlobResourceContents),
}

/// Resource reference for resource links (matches TypeScript Resource interface)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceReference {
    /// The URI of this resource
    pub uri: String,
    /// A human-readable name for this resource
    pub name: String,
    /// A human-readable title for this resource
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// A description of what this resource represents or contains
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The MIME type of this resource, if known
    #[serde(rename = "mimeType", skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    /// Client annotations for this resource
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Annotations>,
    /// Additional metadata for this resource
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<HashMap<String, Value>>,
}

/// Content block union type matching MCP 2025-11-25 specification exactly
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    /// Text content
    #[serde(rename = "text")]
    Text {
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        annotations: Option<Annotations>,
        #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
        meta: Option<HashMap<String, Value>>,
    },
    /// Image content
    #[serde(rename = "image")]
    Image {
        /// Base64-encoded image data
        data: String,
        /// MIME type of the image
        #[serde(rename = "mimeType")]
        mime_type: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        annotations: Option<Annotations>,
        #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
        meta: Option<HashMap<String, Value>>,
    },
    /// Audio content
    #[serde(rename = "audio")]
    Audio {
        /// Base64-encoded audio data
        data: String,
        /// MIME type of the audio
        #[serde(rename = "mimeType")]
        mime_type: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        annotations: Option<Annotations>,
        #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
        meta: Option<HashMap<String, Value>>,
    },
    /// Resource link (ResourceLink from MCP spec)
    #[serde(rename = "resource_link")]
    ResourceLink {
        #[serde(flatten)]
        resource: ResourceReference,
        #[serde(skip_serializing_if = "Option::is_none")]
        annotations: Option<Annotations>,
        #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
        meta: Option<HashMap<String, Value>>,
    },
    /// Embedded resource (EmbeddedResource from MCP spec)
    #[serde(rename = "resource")]
    Resource {
        resource: ResourceContents,
        #[serde(skip_serializing_if = "Option::is_none")]
        annotations: Option<Annotations>,
        #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
        meta: Option<HashMap<String, Value>>,
    },
    /// Tool use content block (per MCP 2025-11-25 spec)
    #[serde(rename = "tool_use")]
    ToolUse {
        /// Unique identifier for this tool use
        id: String,
        /// Name of the tool being called
        name: String,
        /// Input arguments for the tool
        input: HashMap<String, Value>,
        #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
        meta: Option<HashMap<String, Value>>,
    },
    /// Tool result content block (per MCP 2025-11-25 spec)
    #[serde(rename = "tool_result")]
    ToolResult {
        /// ID of the tool use this result corresponds to
        #[serde(rename = "toolUseId")]
        tool_use_id: String,
        /// Content returned by the tool
        content: Vec<ContentBlock>,
        /// Structured content matching the tool's output schema
        #[serde(rename = "structuredContent", skip_serializing_if = "Option::is_none")]
        structured_content: Option<Value>,
        /// Whether the tool call resulted in an error
        #[serde(rename = "isError", skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
        #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
        meta: Option<HashMap<String, Value>>,
    },
}

impl ContentBlock {
    /// Create text content
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text {
            text: text.into(),
            annotations: None,
            meta: None,
        }
    }

    /// Create text content with annotations
    pub fn text_with_annotations(text: impl Into<String>, annotations: Annotations) -> Self {
        Self::Text {
            text: text.into(),
            annotations: Some(annotations),
            meta: None,
        }
    }

    /// Create image content
    pub fn image(data: impl Into<String>, mime_type: impl Into<String>) -> Self {
        Self::Image {
            data: data.into(),
            mime_type: mime_type.into(),
            annotations: None,
            meta: None,
        }
    }

    /// Create audio content
    pub fn audio(data: impl Into<String>, mime_type: impl Into<String>) -> Self {
        Self::Audio {
            data: data.into(),
            mime_type: mime_type.into(),
            annotations: None,
            meta: None,
        }
    }

    /// Create resource link
    pub fn resource_link(resource: ResourceReference) -> Self {
        Self::ResourceLink {
            resource,
            annotations: None,
            meta: None,
        }
    }

    /// Create embedded resource
    pub fn resource(resource: ResourceContents) -> Self {
        Self::Resource {
            resource,
            annotations: None,
            meta: None,
        }
    }

    /// Create tool use content block
    pub fn tool_use(
        id: impl Into<String>,
        name: impl Into<String>,
        input: HashMap<String, Value>,
    ) -> Self {
        Self::ToolUse {
            id: id.into(),
            name: name.into(),
            input,
            meta: None,
        }
    }

    /// Create tool result content block
    pub fn tool_result(tool_use_id: impl Into<String>, content: Vec<ContentBlock>) -> Self {
        Self::ToolResult {
            tool_use_id: tool_use_id.into(),
            content,
            structured_content: None,
            is_error: None,
            meta: None,
        }
    }

    /// Create tool result error content block
    pub fn tool_result_error(tool_use_id: impl Into<String>, content: Vec<ContentBlock>) -> Self {
        Self::ToolResult {
            tool_use_id: tool_use_id.into(),
            content,
            structured_content: None,
            is_error: Some(true),
            meta: None,
        }
    }

    /// Add annotations to any content block that supports them.
    /// ToolUse and ToolResult do not have annotations per spec — this is a no-op for those variants.
    pub fn with_annotations(mut self, annotations: Annotations) -> Self {
        match &mut self {
            ContentBlock::Text { annotations: a, .. }
            | ContentBlock::Image { annotations: a, .. }
            | ContentBlock::Audio { annotations: a, .. }
            | ContentBlock::ResourceLink { annotations: a, .. }
            | ContentBlock::Resource { annotations: a, .. } => {
                *a = Some(annotations);
            }
            // ToolUse and ToolResult don't have annotations per MCP spec
            ContentBlock::ToolUse { .. } | ContentBlock::ToolResult { .. } => {}
        }
        self
    }

    /// Add meta to any content block
    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        match &mut self {
            ContentBlock::Text { meta: m, .. }
            | ContentBlock::Image { meta: m, .. }
            | ContentBlock::Audio { meta: m, .. }
            | ContentBlock::ResourceLink { meta: m, .. }
            | ContentBlock::Resource { meta: m, .. }
            | ContentBlock::ToolUse { meta: m, .. }
            | ContentBlock::ToolResult { meta: m, .. } => {
                *m = Some(meta);
            }
        }
        self
    }
}

impl ResourceContents {
    /// Create text resource contents with required URI
    pub fn text(uri: impl Into<String>, text: impl Into<String>) -> Self {
        Self::Text(TextResourceContents {
            uri: uri.into(),
            mime_type: None,
            meta: None,
            text: text.into(),
        })
    }

    /// Create text resource contents with MIME type
    pub fn text_with_mime(
        uri: impl Into<String>,
        text: impl Into<String>,
        mime_type: impl Into<String>,
    ) -> Self {
        Self::Text(TextResourceContents {
            uri: uri.into(),
            mime_type: Some(mime_type.into()),
            meta: None,
            text: text.into(),
        })
    }

    /// Create blob resource contents with required URI
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

impl ResourceReference {
    /// Create resource reference
    pub fn new(uri: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
            name: name.into(),
            title: None,
            description: None,
            mime_type: None,
            annotations: None,
            meta: None,
        }
    }

    /// Add title
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Add description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Add MIME type
    pub fn with_mime_type(mut self, mime_type: impl Into<String>) -> Self {
        self.mime_type = Some(mime_type.into());
        self
    }

    /// Add annotations
    pub fn with_annotations(mut self, annotations: Annotations) -> Self {
        self.annotations = Some(annotations);
        self
    }

    /// Add meta information
    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.meta = Some(meta);
        self
    }
}

impl TextResourceContents {
    /// Create new text resource contents
    pub fn new(uri: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
            mime_type: None,
            meta: None,
            text: text.into(),
        }
    }

    /// Add MIME type
    pub fn with_mime_type(mut self, mime_type: impl Into<String>) -> Self {
        self.mime_type = Some(mime_type.into());
        self
    }

    /// Add meta information
    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.meta = Some(meta);
        self
    }
}

impl BlobResourceContents {
    /// Create new blob resource contents
    pub fn new(
        uri: impl Into<String>,
        blob: impl Into<String>,
        mime_type: impl Into<String>,
    ) -> Self {
        Self {
            uri: uri.into(),
            mime_type: Some(mime_type.into()),
            meta: None,
            blob: blob.into(),
        }
    }

    /// Add meta information
    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.meta = Some(meta);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_resource_reference_serialization_with_annotations_and_meta() {
        let mut meta = HashMap::new();
        meta.insert("version".to_string(), json!("1.0"));
        meta.insert("created_by".to_string(), json!("test"));

        let resource_ref = ResourceReference::new("file:///test/data.json", "test_data")
            .with_title("Test Data")
            .with_description("Sample data for testing")
            .with_mime_type("application/json")
            .with_annotations(Annotations::new().with_audience(vec!["user".to_string()]))
            .with_meta(meta);

        let resource_link = ContentBlock::resource_link(resource_ref);

        // Test serialization round-trip
        let json_str = serde_json::to_string(&resource_link).unwrap();
        let deserialized: ContentBlock = serde_json::from_str(&json_str).unwrap();

        // Verify structure - with #[serde(flatten)], ResourceReference fields get flattened
        if let ContentBlock::ResourceLink {
            resource,
            annotations,
            meta,
        } = deserialized
        {
            assert_eq!(resource.uri, "file:///test/data.json");
            assert_eq!(resource.name, "test_data");
            assert_eq!(resource.title, Some("Test Data".to_string()));
            assert_eq!(
                resource.description,
                Some("Sample data for testing".to_string())
            );
            assert_eq!(resource.mime_type, Some("application/json".to_string()));

            // With #[serde(flatten)], the ResourceReference annotations and meta get flattened
            // during serialization, but during deserialization, serde routes them to the
            // ContentBlock level since both structs have these fields.

            // ResourceReference level should be None after deserialization
            assert!(resource.annotations.is_none());
            assert!(resource.meta.is_none());

            // ContentBlock level should contain the annotations and meta
            assert!(annotations.is_some());
            assert_eq!(
                annotations.unwrap().audience,
                Some(vec!["user".to_string()])
            );

            assert!(meta.is_some());
            let cb_meta = meta.unwrap();
            assert_eq!(cb_meta.get("version"), Some(&json!("1.0")));
            assert_eq!(cb_meta.get("created_by"), Some(&json!("test")));
        } else {
            panic!("Expected ResourceLink variant");
        }
    }

    #[test]
    fn test_tool_use_content_block() {
        let mut input = HashMap::new();
        input.insert("query".to_string(), json!("test search"));

        let block = ContentBlock::tool_use("tu-123", "search", input);
        let json = serde_json::to_value(&block).unwrap();

        assert_eq!(json["type"], "tool_use");
        assert_eq!(json["id"], "tu-123");
        assert_eq!(json["name"], "search");
        assert_eq!(json["input"]["query"], "test search");
        assert!(json.get("_meta").is_none());

        // Round-trip
        let parsed: ContentBlock = serde_json::from_value(json).unwrap();
        if let ContentBlock::ToolUse {
            id, name, input, ..
        } = parsed
        {
            assert_eq!(id, "tu-123");
            assert_eq!(name, "search");
            assert_eq!(input.get("query"), Some(&json!("test search")));
        } else {
            panic!("Expected ToolUse variant");
        }
    }

    #[test]
    fn test_tool_result_content_block() {
        let block =
            ContentBlock::tool_result("tu-123", vec![ContentBlock::text("Search results here")]);
        let json = serde_json::to_value(&block).unwrap();

        assert_eq!(json["type"], "tool_result");
        assert_eq!(json["toolUseId"], "tu-123");
        assert!(json["content"].is_array());
        assert!(json.get("isError").is_none());
        assert!(json.get("structuredContent").is_none());

        // Round-trip
        let parsed: ContentBlock = serde_json::from_value(json).unwrap();
        if let ContentBlock::ToolResult {
            tool_use_id,
            content,
            is_error,
            ..
        } = parsed
        {
            assert_eq!(tool_use_id, "tu-123");
            assert_eq!(content.len(), 1);
            assert!(is_error.is_none());
        } else {
            panic!("Expected ToolResult variant");
        }
    }

    #[test]
    fn test_tool_result_error_content_block() {
        let block = ContentBlock::tool_result_error(
            "tu-456",
            vec![ContentBlock::text("Tool failed: not found")],
        );
        let json = serde_json::to_value(&block).unwrap();

        assert_eq!(json["type"], "tool_result");
        assert_eq!(json["isError"], true);
    }

    #[test]
    fn test_resource_reference_minimal() {
        let resource_ref = ResourceReference::new("file:///minimal.txt", "minimal");
        let resource_link = ContentBlock::resource_link(resource_ref);

        let json_str = serde_json::to_string(&resource_link).unwrap();
        let deserialized: ContentBlock = serde_json::from_str(&json_str).unwrap();

        if let ContentBlock::ResourceLink { resource, .. } = deserialized {
            assert_eq!(resource.uri, "file:///minimal.txt");
            assert_eq!(resource.name, "minimal");
            assert!(resource.title.is_none());
            assert!(resource.description.is_none());
            assert!(resource.mime_type.is_none());
            assert!(resource.annotations.is_none());
            assert!(resource.meta.is_none());
        } else {
            panic!("Expected ResourceLink variant");
        }
    }
}
