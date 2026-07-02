//! Content block types for MCP DRAFT-2026-v1.
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

/// Resource reference for resource links.
///
/// Mirrors the schema's `Resource` interface so `ContentBlock::ResourceLink`
/// (which the schema declares as `ResourceLink extends Resource`) round-trips
/// all spec-permitted fields including `size` and `icons`.
///
/// Carries the same fields as [`crate::resources::Resource`]; the parallel
/// type is preserved for now because `ContentBlock::ResourceLink` flattens
/// this body and we don't want to cascade the type swap through
/// `prompts.rs`/`lib.rs` re-exports in this slice. Collapsing onto a single
/// `Resource` struct is the cleaner end-state.
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
    /// Size of the raw resource content, in bytes. Hosts use this for file-size
    /// display and context-window estimation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    /// Display icons (from `Resource extends Icons`). Most consumers won't need this.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icons: Option<Vec<crate::icons::Icon>>,
    /// Client annotations for this resource
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Annotations>,
    /// Additional metadata for this resource
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<HashMap<String, Value>>,
}

/// Content block union — `text | image | audio | resource_link | resource | tool_use | tool_result`.
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
    /// Resource link (ResourceLink from MCP spec).
    ///
    /// Schema: `ResourceLink extends Resource` — exactly ONE `annotations?`
    /// and ONE `_meta?`, both inherited from `Resource`. The flattened
    /// [`ResourceReference`] is the single source of truth for both — do NOT
    /// add variant-level `annotations`/`meta` fields alongside it, which
    /// would emit the same key twice on the wire.
    #[serde(rename = "resource_link")]
    ResourceLink {
        #[serde(flatten)]
        resource: ResourceReference,
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
    /// Tool use content block.
    #[deprecated(
        since = "0.4.0",
        note = "Deprecated per SEP-2577 (DRAFT-2026-v1) with the Sampling surface. \
                Earliest removal: first release on/after 2027-07-28."
    )]
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
    /// Tool result content block.
    #[deprecated(
        since = "0.4.0",
        note = "Deprecated per SEP-2577 (DRAFT-2026-v1) with the Sampling surface. \
                Earliest removal: first release on/after 2027-07-28."
    )]
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
        Self::ResourceLink { resource }
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
    #[allow(deprecated)]
    #[deprecated(
        since = "0.4.0",
        note = "Deprecated per SEP-2577 (DRAFT-2026-v1) with the Sampling surface."
    )]
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
    #[allow(deprecated)]
    #[deprecated(
        since = "0.4.0",
        note = "Deprecated per SEP-2577 (DRAFT-2026-v1) with the Sampling surface."
    )]
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
    #[allow(deprecated)]
    #[deprecated(
        since = "0.4.0",
        note = "Deprecated per SEP-2577 (DRAFT-2026-v1) with the Sampling surface."
    )]
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
    #[allow(deprecated)] // matches the SEP-2577-deprecated variants
    pub fn with_annotations(mut self, annotations: Annotations) -> Self {
        match &mut self {
            ContentBlock::Text { annotations: a, .. }
            | ContentBlock::Image { annotations: a, .. }
            | ContentBlock::Audio { annotations: a, .. }
            | ContentBlock::Resource { annotations: a, .. } => {
                *a = Some(annotations);
            }
            ContentBlock::ResourceLink { resource } => {
                resource.annotations = Some(annotations);
            }
            // ToolUse and ToolResult don't have annotations per MCP spec
            ContentBlock::ToolUse { .. } | ContentBlock::ToolResult { .. } => {}
        }
        self
    }

    /// Add meta to any content block
    #[allow(deprecated)] // matches the SEP-2577-deprecated variants
    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        match &mut self {
            ContentBlock::Text { meta: m, .. }
            | ContentBlock::Image { meta: m, .. }
            | ContentBlock::Audio { meta: m, .. }
            | ContentBlock::Resource { meta: m, .. }
            | ContentBlock::ToolUse { meta: m, .. }
            | ContentBlock::ToolResult { meta: m, .. } => {
                *m = Some(meta);
            }
            ContentBlock::ResourceLink { resource } => {
                resource.meta = Some(meta);
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
            size: None,
            icons: None,
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

    /// Set the raw content size in bytes.
    pub fn with_size(mut self, size: u64) -> Self {
        self.size = Some(size);
        self
    }

    /// Attach display icons.
    pub fn with_icons(mut self, icons: Vec<crate::icons::Icon>) -> Self {
        self.icons = Some(icons);
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
#[allow(deprecated)] // exercises SEP-2577-deprecated surfaces
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_resource_reference_serialization_with_annotations_and_meta() {
        // Schema: `ResourceLink extends Resource` — exactly ONE `annotations?`
        // and ONE `_meta?`. `ContentBlock::ResourceLink` has no variant-level
        // copies of these fields, so `ResourceReference` (flattened) is the
        // single source of truth — no duplicate wire keys, no field asymmetry
        // on round-trip.
        let mut meta = HashMap::new();
        meta.insert("version".to_string(), json!("1.0"));
        meta.insert("created_by".to_string(), json!("test"));

        let resource_ref = ResourceReference::new("file:///test/data.json", "test_data")
            .with_title("Test Data")
            .with_description("Sample data for testing")
            .with_mime_type("application/json")
            .with_annotations(Annotations::new().with_audience(vec![crate::prompts::Role::User]))
            .with_meta(meta);

        let resource_link = ContentBlock::resource_link(resource_ref);

        // Outbound: exactly one `annotations` / `_meta` key on the wire.
        // MUST check the raw serialized text, not `serde_json::to_value()` —
        // `to_value()` builds a `Map` that cannot represent duplicate keys
        // (a second `serialize_entry` for the same key silently overwrites
        // the first in-memory), so counting keys on a `Value` can never
        // observe a duplicate-key defect even when the byte stream produced
        // by `to_string()`/`to_writer()` genuinely repeats the key.
        let json_str = serde_json::to_string(&resource_link).unwrap();
        assert_eq!(
            json_str.matches("\"annotations\":").count(),
            1,
            "must emit exactly one annotations key on the wire: {json_str}"
        );
        assert_eq!(
            json_str.matches("\"_meta\":").count(),
            1,
            "must emit exactly one _meta key on the wire: {json_str}"
        );

        // Round-trip.
        let deserialized: ContentBlock = serde_json::from_str(&json_str).unwrap();

        if let ContentBlock::ResourceLink { resource } = deserialized {
            assert_eq!(resource.uri, "file:///test/data.json");
            assert_eq!(resource.name, "test_data");
            assert_eq!(resource.title, Some("Test Data".to_string()));
            assert_eq!(
                resource.description,
                Some("Sample data for testing".to_string())
            );
            assert_eq!(resource.mime_type, Some("application/json".to_string()));

            // Annotations and meta survive on the ResourceReference itself —
            // no ContentBlock-level duplicate to route them to instead.
            assert_eq!(
                resource.annotations.unwrap().audience,
                Some(vec![crate::prompts::Role::User])
            );
            let resource_meta = resource.meta.unwrap();
            assert_eq!(resource_meta.get("version"), Some(&json!("1.0")));
            assert_eq!(resource_meta.get("created_by"), Some(&json!("test")));
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
    fn test_resource_link_round_trips_size_and_icons() {
        // Schema-anchor: `ResourceLink extends Resource`; `Resource` carries
        // `size?: number` and `icons?` via `extends Icons`. Both must survive
        // a wire round-trip when embedded inside `ContentBlock::ResourceLink`.
        use crate::icons::Icon;

        let icon = Icon::new("https://example.com/icon.svg");
        let resource_ref = ResourceReference::new("file:///big.bin", "big-binary")
            .with_size(1_048_576)
            .with_icons(vec![icon.clone()]);
        let block = ContentBlock::resource_link(resource_ref);

        // Outbound serialization carries both fields.
        let json = serde_json::to_value(&block).unwrap();
        assert_eq!(json["type"], "resource_link");
        assert_eq!(json["size"], 1_048_576);
        assert_eq!(json["icons"][0]["src"], "https://example.com/icon.svg");

        // Round-trip preserves them.
        let parsed: ContentBlock = serde_json::from_value(json).unwrap();
        if let ContentBlock::ResourceLink { resource, .. } = parsed {
            assert_eq!(resource.size, Some(1_048_576));
            assert_eq!(resource.icons.as_ref().map(|v| v.len()), Some(1));
        } else {
            panic!("Expected ResourceLink variant");
        }
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
