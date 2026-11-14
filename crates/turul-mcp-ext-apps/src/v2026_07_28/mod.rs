//! SEP-1865 Apps extension MCP-side surface for the 2026-07-28 spec line.

pub mod capability;
pub mod types;

#[cfg(test)]
mod compliance_test;

pub use capability::{
    EXTENSION_IDENTIFIER, capability, client_supports_html_views, declared_by_client,
    declared_by_server,
};
pub use types::{
    EmptyObject, MCP_APP_HTML_MIME, META_KEY_UI, UI_URI_SCHEME, UiClientCapabilities,
    UiResourceCsp, UiResourceMeta, UiResourcePermissions, UiToolMeta, UiToolVisibility,
};
