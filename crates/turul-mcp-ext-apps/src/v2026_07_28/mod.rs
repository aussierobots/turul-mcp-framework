//! SEP-1865 Apps extension MCP-side surface for the 2026-07-28 spec line.

pub mod capability;
pub mod types;

#[cfg(test)]
mod compliance_test;

pub use capability::{EXTENSION_IDENTIFIER, client_supports_html_views, declared_by_client};
pub use types::{
    EmptyObject, MCP_APP_HTML_MIME, META_KEY_UI, UiClientCapabilities, UiResourceCsp,
    UiResourceMeta, UiResourcePermissions, UiToolMeta, UiToolVisibility,
};
