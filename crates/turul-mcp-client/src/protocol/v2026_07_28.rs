//! 2026-07-28 request building and response parsing for client operations.
//!
//! Stateless core: every request carries `_meta` (per-request capability
//! negotiation — `protocolVersion` + `clientInfo` + `clientCapabilities`), and
//! list/read results carry `resultType` plus the `CacheableResult` mixin
//! (`ttlMs`/`cacheScope`). Results are mapped back to the version-neutral public
//! types `McpClient` exposes (which are the 2025-11-25 alias types); 2026-only
//! fields that the public types don't carry are dropped on the way out.

use crate::error::McpClientResult;
use turul_mcp_protocol_2026_07_28 as p;

/// Build the per-request `_meta` object required by the 2026 stateless core.
pub(crate) fn request_meta(client_name: &str, client_version: &str) -> p::meta::RequestMetaObject {
    p::meta::RequestMetaObject::new(
        p::MCP_VERSION,
        p::initialize::Implementation::new(client_name.to_string(), client_version.to_string()),
        p::initialize::ClientCapabilities::default(),
    )
}

/// `tools/list` request params, carrying the required `_meta`.
pub(crate) fn list_tools_params(meta: &p::meta::RequestMetaObject) -> serde_json::Value {
    serde_json::json!({ "_meta": meta })
}

/// Parse a 2026 `tools/list` result into the public `Tool` list.
///
/// The 2026 `ListToolsResult` wraps the tools with `resultType`/`ttlMs`/
/// `cacheScope`; each 2026 `Tool` is mapped to the public alias `Tool` via JSON
/// (the core `name`/`description`/`inputSchema` fields overlap).
pub(crate) fn parse_list_tools(
    result: &serde_json::Value,
) -> McpClientResult<Vec<turul_mcp_protocol::Tool>> {
    let parsed: p::tools::ListToolsResult = serde_json::from_value(result.clone())?;
    parsed
        .tools
        .into_iter()
        .map(|t| {
            let as_value = serde_json::to_value(&t)?;
            let public: turul_mcp_protocol::Tool = serde_json::from_value(as_value)?;
            Ok(public)
        })
        .collect()
}
