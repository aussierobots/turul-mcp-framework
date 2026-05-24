//! HTTP header name constants for MCP Streamable HTTP transport (SEP-2243,
//! changelog Minor #4).
//!
//! These header names are required on Streamable HTTP POST requests per
//! SEP-2243 ("HTTP Header Standardization for Streamable HTTP Transport").
//! They are transport-layer concerns — actual enforcement lives in
//! `turul-http-mcp-server` — but the **canonical spelling** lives here as
//! protocol constants so transport implementations can import a single source
//! of truth.

/// `MCP-Protocol-Version` — REQUIRED on every Streamable HTTP POST. Must match
/// `_meta.io.modelcontextprotocol/protocolVersion` in the request body
/// (servers MUST return `400 Bad Request` on mismatch per the
/// `RequestMetaObject` schema).
pub const HTTP_HEADER_PROTOCOL_VERSION: &str = "MCP-Protocol-Version";

/// `Mcp-Method` — REQUIRED per SEP-2243. Carries the JSON-RPC `method` string
/// at the HTTP layer to enable intelligent routing without body inspection.
/// Servers MUST reject requests where this header and the body's `method`
/// disagree.
pub const HTTP_HEADER_METHOD: &str = "Mcp-Method";

/// `Mcp-Name` — REQUIRED per SEP-2243. Carries the target tool/resource/prompt
/// name when present (e.g. for `tools/call`, the value of `params.name`).
/// Servers MUST reject requests where this header and the body's name disagree.
pub const HTTP_HEADER_NAME: &str = "Mcp-Name";

/// `x-mcp-header` — header prefix for custom headers exposed to tool
/// implementations via tool parameters (SEP-2243). Tools can advertise a list
/// of custom headers they read from the request and clients can populate them
/// via `x-mcp-header-<custom-name>: <value>`. The full custom-header name
/// substituted into the request is `x-mcp-header-` + the lowercased tool-defined
/// suffix.
pub const HTTP_HEADER_CUSTOM_PREFIX: &str = "x-mcp-header-";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_names_exact_spelling() {
        // Spec spelling — case matters per HTTP normalization rules.
        assert_eq!(HTTP_HEADER_PROTOCOL_VERSION, "MCP-Protocol-Version");
        assert_eq!(HTTP_HEADER_METHOD, "Mcp-Method");
        assert_eq!(HTTP_HEADER_NAME, "Mcp-Name");
        assert_eq!(HTTP_HEADER_CUSTOM_PREFIX, "x-mcp-header-");
    }
}
