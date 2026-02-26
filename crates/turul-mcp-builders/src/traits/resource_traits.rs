//! Framework traits for MCP resource construction
//!
//! **IMPORTANT**: These are framework features, NOT part of the MCP specification.
//! The MCP specification defines concrete types only.

use serde_json::Value;
use std::collections::HashMap;

// Import protocol types (spec-defined)
use turul_mcp_protocol::Resource;
use turul_mcp_protocol::meta::Annotations;

pub trait HasResourceMetadata {
    /// Programmatic identifier (fallback display name)
    fn name(&self) -> &str;

    /// Human-readable display name (UI contexts)
    fn title(&self) -> Option<&str> {
        None
    }
}

/// Resource description trait
pub trait HasResourceDescription {
    fn description(&self) -> Option<&str> {
        None
    }
}

/// Resource URI trait
pub trait HasResourceUri {
    fn uri(&self) -> &str;
}

/// Resource MIME type trait
pub trait HasResourceMimeType {
    fn mime_type(&self) -> Option<&str> {
        None
    }
}

/// Resource size trait
pub trait HasResourceSize {
    fn size(&self) -> Option<u64> {
        None
    }
}

/// Resource annotations trait
pub trait HasResourceAnnotations {
    fn annotations(&self) -> Option<&Annotations> {
        None
    }
}

/// Resource-specific meta trait (separate from RPC _meta)
pub trait HasResourceMeta {
    fn resource_meta(&self) -> Option<&HashMap<String, Value>> {
        None
    }
}

/// **Complete MCP Resource Creation** - Build readable resources that clients can access.
///
/// This trait represents a **complete, working MCP resource** that can be registered with a server
/// and accessed by clients. When you implement the required metadata traits, you automatically
/// get `ResourceDefinition` for free via blanket implementation.
///
/// ## What This Enables
///
/// Resources implementing `ResourceDefinition` become **full MCP citizens** that are:
/// - 🔍 **Discoverable** via `resources/list` requests
/// - 📖 **Readable** via `resources/read` requests
/// - 🎯 **Template-capable** with URI variable substitution
/// - 📡 **Protocol-ready** for JSON-RPC communication
///
/// ## Complete Working Example
///
/// ```rust
/// use turul_mcp_protocol::Resource;
/// use turul_mcp_protocol::meta::Annotations;
/// use turul_mcp_builders::prelude::*;  // Import framework traits
/// use std::collections::HashMap;
///
/// // This struct will automatically implement ResourceDefinition!
/// struct ApiDataFeed {
///     uri: String,
/// }
///
/// impl ApiDataFeed {
///     fn new() -> Self {
///         Self {
///             uri: "https://api.example.com/data/{dataset}".to_string(),
///         }
///     }
/// }
///
/// impl HasResourceMetadata for ApiDataFeed {
///     fn name(&self) -> &str { "api_data" }
///     fn title(&self) -> Option<&str> { Some("Live API Data Feed") }
/// }
///
/// impl HasResourceDescription for ApiDataFeed {
///     fn description(&self) -> Option<&str> {
///         Some("Real-time data feed from external API")
///     }
/// }
///
/// impl HasResourceUri for ApiDataFeed {
///     fn uri(&self) -> &str { &self.uri }
/// }
///
/// impl HasResourceMimeType for ApiDataFeed {
///     fn mime_type(&self) -> Option<&str> { Some("application/json") }
/// }
///
/// impl HasResourceSize for ApiDataFeed {
///     fn size(&self) -> Option<u64> { None }
/// }
///
/// impl HasResourceAnnotations for ApiDataFeed {
///     fn annotations(&self) -> Option<&Annotations> { None }
/// }
///
/// impl HasResourceMeta for ApiDataFeed {
///     fn resource_meta(&self) -> Option<&HashMap<String, serde_json::Value>> { None }
/// }
///
/// // 🎉 ApiDataFeed now automatically implements ResourceDefinition!
/// let resource = ApiDataFeed::new();
/// assert_eq!(resource.name(), "api_data");
/// assert_eq!(resource.mime_type(), Some("application/json"));
/// ```
///
/// ## Usage Patterns
///
/// ### Easy: Use Derive Macros (see turul-mcp-derive crate)
/// ```rust
/// // Example of manual implementation without macros
/// use turul_mcp_protocol::Resource;
/// use turul_mcp_protocol::meta::Annotations;
/// use turul_mcp_builders::prelude::*;  // Import framework traits
/// use std::collections::HashMap;
///
/// struct LogFiles;
///
/// impl HasResourceMetadata for LogFiles {
///     fn name(&self) -> &str { "logs" }
///     fn title(&self) -> Option<&str> { None }
/// }
///
/// impl HasResourceDescription for LogFiles {
///     fn description(&self) -> Option<&str> { None }
/// }
///
/// impl HasResourceUri for LogFiles {
///     fn uri(&self) -> &str { "file:///var/log/{service}.log" }
/// }
///
/// impl HasResourceMimeType for LogFiles {
///     fn mime_type(&self) -> Option<&str> { None }
/// }
///
/// impl HasResourceSize for LogFiles {
///     fn size(&self) -> Option<u64> { None }
/// }
///
/// impl HasResourceAnnotations for LogFiles {
///     fn annotations(&self) -> Option<&Annotations> { None }
/// }
///
/// impl HasResourceMeta for LogFiles {
///     fn resource_meta(&self) -> Option<&HashMap<String, serde_json::Value>> { None }
/// }
///
/// let resource = LogFiles;
/// assert_eq!(resource.name(), "logs");
/// ```
///
/// ### Advanced: Manual Implementation (shown above)
/// Perfect when you need dynamic URIs, custom metadata, or complex content handling.
///
/// ## Real-World Resource Ideas
///
/// - **File Resources**: Config files, logs, documentation, data exports
/// - **API Resources**: REST endpoints, GraphQL schemas, webhook data
/// - **Data Resources**: Database views, CSV exports, JSON feeds, report data
/// - **System Resources**: Process info, system stats, environment configs
/// - **Template Resources**: Dynamic content with `{variable}` substitution
///
/// ## URI Template Power
///
/// Resources support powerful URI templating:
/// ```text
/// Static:   "file:///config.json"              → Single resource
/// Template: "file:///logs/{service}.log"       → Multiple resources
/// Complex:  "api://data/{type}/{id}?fmt={fmt}" → Full parameterization
/// ```
///
/// ## How It Works in MCP
///
/// 1. **Registration**: Server registers your resource during startup
/// 2. **Discovery**: Client calls `resources/list` → sees available resources
/// 3. **Template Resolution**: Client expands URI templates with parameters
/// 4. **Reading**: Client calls `resources/read` with resolved URI
/// 5. **Content Delivery**: Your resource returns actual content
/// 6. **Response**: Framework serializes content back to client
///
/// The framework handles URI template parsing, parameter validation, and protocol serialization!
pub trait ResourceDefinition:
    HasResourceMetadata +       // name, title (from BaseMetadata)
    HasResourceDescription +    // description
    HasResourceUri +           // uri
    HasResourceMimeType +      // mimeType
    HasResourceSize +          // size
    HasResourceAnnotations +   // annotations
    HasResourceMeta +          // _meta (resource-specific)
    super::icon_traits::HasIcons + // icons (MCP 2025-11-25)
    Send +
    Sync
{
    /// Display name precedence: title > name (matches TypeScript spec)
    fn display_name(&self) -> &str {
        self.title().unwrap_or_else(|| self.name())
    }

    /// Convert to concrete Resource struct for protocol serialization
    fn to_resource(&self) -> Resource {
        Resource {
            uri: self.uri().to_string(),
            name: self.name().to_string(),
            title: self.title().map(String::from),
            description: self.description().map(String::from),
            mime_type: self.mime_type().map(String::from),
            size: self.size(),
            annotations: self.annotations().cloned(),
            icons: self.icons().cloned(),
            meta: self.resource_meta().cloned(),
        }
    }
}
impl<T> ResourceDefinition for T where
    T: HasResourceMetadata
        + HasResourceDescription
        + HasResourceUri
        + HasResourceMimeType
        + HasResourceSize
        + HasResourceAnnotations
        + HasResourceMeta
        + super::icon_traits::HasIcons
        + Send
        + Sync
{
}
