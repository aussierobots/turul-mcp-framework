//! Test modules for turul-mcp-server crate
//!
//! This module contains comprehensive test suites for all aspects of the MCP server framework.

/// A spec-complete per-request `_meta` (RequestMetaObject). The 2026-07-28 core
/// requires it on every request's params; 2025-11-25 treats `_meta` as optional
/// and ignores the extra keys, so injecting it lets one test cover both specs.
#[allow(dead_code)]
pub(crate) fn request_meta() -> serde_json::Value {
    serde_json::json!({
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientInfo": { "name": "test", "version": "1.0.0" },
        "io.modelcontextprotocol/clientCapabilities": {}
    })
}

// Logging is deprecated and the logging builder/handler are gated out of the
// 2026-07-28 core; pagination's PaginatedResponse envelope was replaced by a
// top-level nextCursor. These suites exercise the 2025-11-25 shapes only.
#[cfg(feature = "protocol-2025-11-25")]
pub mod logging_builder_integration_tests;
pub mod notification_tests;
#[cfg(feature = "protocol-2025-11-25")]
pub mod pagination_integration_tests;
pub mod security_integration_tests;
#[cfg(feature = "protocol-2025-11-25")]
pub mod session_aware_logging_tests;
pub mod session_tests;
pub mod uri_template_tests;
