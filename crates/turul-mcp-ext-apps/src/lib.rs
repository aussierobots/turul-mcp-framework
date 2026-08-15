//! # MCP Apps Extension (`io.modelcontextprotocol/ui`, SEP-1865)
//!
//! Spec-neutral host crate for the MCP-side surface of the Apps extension:
//! UI views embedded in AI chat hosts, served by MCP servers as `ui://`
//! resources and linked from tools via `_meta.ui`.
//!
//! Scope: this crate binds what an MCP **server** declares —
//! - the client capability shape under `capabilities.extensions`
//!   ([`UiClientCapabilities`], identifier [`EXTENSION_IDENTIFIER`])
//! - tool `_meta.ui` metadata ([`UiToolMeta`]: `resourceUri`, `visibility`)
//! - UI-resource `_meta.ui` metadata ([`UiResourceMeta`]: CSP, sandbox
//!   permissions, dedicated origin, border preference)
//!
//! The host↔view iframe protocol (`ui/*` methods over postMessage) belongs to
//! app/host SDKs and is deliberately not bound here.
//!
//! The Apps protocol versions independently of core MCP (current: 2026-01-26);
//! this crate's `v2026_07_28` module names the CORE spec lane it pairs with.
//! Schema provenance: `schema/README.md`.

#[cfg(feature = "protocol-2026-07-28")]
pub mod v2026_07_28;

#[cfg(feature = "protocol-2026-07-28")]
pub use v2026_07_28::{
    EXTENSION_IDENTIFIER, EmptyObject, MCP_APP_HTML_MIME, META_KEY_UI, UiClientCapabilities,
    UiResourceCsp, UiResourceMeta, UiResourcePermissions, UiToolMeta, UiToolVisibility,
    client_supports_html_views, declared_by_client,
};
