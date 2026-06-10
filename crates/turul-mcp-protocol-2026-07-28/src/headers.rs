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

/// `Mcp-Method` — REQUIRED per SEP-2243 on all requests and notifications.
/// Carries the JSON-RPC `method` string at the HTTP layer to enable
/// intelligent routing without body inspection. Servers MUST reject requests
/// where this header and the body's `method` disagree.
pub const HTTP_HEADER_METHOD: &str = "Mcp-Method";

/// `Mcp-Name` — REQUIRED per SEP-2243 for `tools/call` (`params.name`),
/// `resources/read` (`params.uri`), and `prompts/get` (`params.name`).
/// Servers MUST reject requests where this header and the body's value
/// disagree.
pub const HTTP_HEADER_NAME: &str = "Mcp-Name";

/// `Mcp-Param-{name}` — header name prefix for custom headers mirrored from
/// tool parameters (SEP-2243 §Custom Headers from Tool Parameters).
///
/// The `x-mcp-header` extension property inside a tool's `inputSchema`
/// designates a parameter for mirroring and supplies the `{name}` portion;
/// the resulting wire header is `Mcp-Param-{name}: {encoded-value}`. Values
/// that cannot be represented as plain ASCII header values are Base64-encoded
/// as `=?base64?{value}?=` (see [`MCP_PARAM_BASE64_PREFIX`] /
/// [`MCP_PARAM_BASE64_SUFFIX`]). Servers that process the body MUST validate
/// that decoded header values match the corresponding body arguments.
pub const HTTP_HEADER_PARAM_PREFIX: &str = "Mcp-Param-";

/// `x-mcp-header` — the JSON Schema extension property (inside a tool's
/// `inputSchema`) that designates a parameter for header mirroring. This is a
/// schema annotation key, NOT a wire header name — the wire header it
/// produces is [`HTTP_HEADER_PARAM_PREFIX`]`{name}`.
pub const X_MCP_HEADER_SCHEMA_KEY: &str = "x-mcp-header";

/// Sentinel prefix marking a Base64-encoded `Mcp-Param-*` value
/// (`=?base64?{Base64EncodedValue}?=`, case-sensitive, lowercase).
pub const MCP_PARAM_BASE64_PREFIX: &str = "=?base64?";

/// Sentinel suffix closing a Base64-encoded `Mcp-Param-*` value.
pub const MCP_PARAM_BASE64_SUFFIX: &str = "?=";

/// JSON-RPC error code for header-validation failures (`HeaderMismatch`,
/// implementation-defined server error range). Returned with HTTP
/// `400 Bad Request` when a required standard header is missing/malformed or
/// a header value does not match the corresponding request-body value.
/// Prose-only contract — the pinned schema defines no symbol for it.
pub const ERROR_CODE_HEADER_MISMATCH: i64 = -32001;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_names_exact_spelling() {
        // Spec spelling — case matters per HTTP normalization rules.
        assert_eq!(HTTP_HEADER_PROTOCOL_VERSION, "MCP-Protocol-Version");
        assert_eq!(HTTP_HEADER_METHOD, "Mcp-Method");
        assert_eq!(HTTP_HEADER_NAME, "Mcp-Name");
        assert_eq!(HTTP_HEADER_PARAM_PREFIX, "Mcp-Param-");
        assert_eq!(X_MCP_HEADER_SCHEMA_KEY, "x-mcp-header");
    }

    #[test]
    fn base64_sentinel_exact_spelling() {
        // Markers are case-sensitive and MUST appear exactly as shown.
        assert_eq!(MCP_PARAM_BASE64_PREFIX, "=?base64?");
        assert_eq!(MCP_PARAM_BASE64_SUFFIX, "?=");
    }

    #[test]
    fn header_mismatch_code_in_server_error_range() {
        assert_eq!(ERROR_CODE_HEADER_MISMATCH, -32001);
        assert!((-32099..=-32000).contains(&ERROR_CODE_HEADER_MISMATCH));
    }
}
