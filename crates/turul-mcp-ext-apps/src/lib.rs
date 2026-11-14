//! Types for the MCP Apps extension (`io.modelcontextprotocol/ui`): UI views
//! that an MCP server serves as `ui://` resources and links from tools.
//!
//! Most users want [`turul-mcp-server`](https://docs.rs/turul-mcp-server); this
//! crate holds the extension's wire types for the 2026-07-28 spec lane. It
//! covers the client capability ([`UiClientCapabilities`]), tool metadata
//! ([`UiToolMeta`]) and UI resource metadata ([`UiResourceMeta`]). The
//! host-to-view iframe protocol is left to app and host SDKs.
//!
//! # Example
//!
//! Link a tool to its UI resource under `_meta.ui`:
//!
//! ```toml
//! [dependencies]
//! turul-mcp-ext-apps = "0.1"
//! ```
//!
//! ```rust
//! use turul_mcp_ext_apps::{META_KEY_UI, UiToolMeta, UiToolVisibility};
//!
//! let meta = UiToolMeta {
//!     resource_uri: Some("ui://weather/view.html".into()),
//!     visibility: Some(vec![UiToolVisibility::Model, UiToolVisibility::App]),
//! };
//! let tool_meta = meta.to_meta(); // pass to ToolBuilder::meta
//! assert_eq!(tool_meta[META_KEY_UI]["resourceUri"], "ui://weather/view.html");
//! ```
//!
//! # Where next
//!
//! - [`UiResourceMeta`] for CSP, sandbox permissions and border preference
//! - [`client_supports_html_views`] to check a client's declared capability
//! - [`capability()`] for the server's `capabilities.extensions` entry
//! - [MCP Apps specification](https://github.com/modelcontextprotocol/ext-apps)

#[cfg(feature = "protocol-2026-07-28")]
pub mod v2026_07_28;

#[cfg(feature = "protocol-2026-07-28")]
pub use v2026_07_28::{
    EXTENSION_IDENTIFIER, EmptyObject, MCP_APP_HTML_MIME, META_KEY_UI, UI_URI_SCHEME,
    UiClientCapabilities, UiResourceCsp, UiResourceMeta, UiResourcePermissions, UiToolMeta,
    UiToolVisibility, capability, client_supports_html_views, declared_by_client,
    declared_by_server,
};
