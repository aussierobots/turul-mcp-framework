//! CORS (Cross-Origin Resource Sharing) support
//!
//! All CORS headers for the HTTP transport are centralized here.
//! `server.rs` applies [`CorsLayer::apply_cors_headers`] to every response
//! (including OPTIONS preflight), so individual handlers do not need to
//! set CORS headers themselves.

use hyper::HeaderMap;

// ── Canonical header values (single source of truth) ──────────────────

/// HTTP methods permitted by CORS preflight.
pub(crate) const CORS_ALLOW_METHODS: &str = "GET, POST, DELETE, OPTIONS";

/// Request headers a browser is allowed to send.
///
/// Derived from MCP 2025-11-25 transport requirements:
/// - `Content-Type`  — POST body is `application/json` (not CORS-safelisted)
/// - `Accept`        — content negotiation (`application/json`, `text/event-stream`)
/// - `Authorization`  — OAuth 2.1 Bearer tokens
/// - `Mcp-Session-Id` — client MUST send after initialization
/// - `MCP-Protocol-Version` — client MUST send on all requests
/// - `Last-Event-ID` — client SHOULD send for SSE stream resumption
pub(crate) const CORS_ALLOW_HEADERS: &str =
    "Content-Type, Accept, Authorization, Mcp-Session-Id, MCP-Protocol-Version, Last-Event-ID";

/// Response headers a browser is allowed to read.
///
/// `Mcp-Session-Id` — client must read the session ID from the initialize response.
pub(crate) const CORS_EXPOSE_HEADERS: &str = "Mcp-Session-Id";

/// Preflight cache duration in seconds (24 hours).
pub(crate) const CORS_MAX_AGE: &str = "86400";

// ── Public API ────────────────────────────────────────────────────────

/// CORS layer for adding appropriate headers to HTTP responses.
///
/// Applied by `server.rs` to **every** response when `enable_cors` is true,
/// including OPTIONS preflight responses. Individual handlers should not
/// set CORS headers — this layer is the single source of truth.
pub struct CorsLayer;

impl CorsLayer {
    /// Apply wildcard CORS headers to a response.
    ///
    /// Sets `Access-Control-Allow-Origin: *` (no credentials).
    pub fn apply_cors_headers(headers: &mut HeaderMap) {
        headers.insert("Access-Control-Allow-Origin", "*".parse().unwrap());
        headers.insert(
            "Access-Control-Allow-Methods",
            CORS_ALLOW_METHODS.parse().unwrap(),
        );
        headers.insert(
            "Access-Control-Allow-Headers",
            CORS_ALLOW_HEADERS.parse().unwrap(),
        );
        headers.insert(
            "Access-Control-Expose-Headers",
            CORS_EXPOSE_HEADERS.parse().unwrap(),
        );
        headers.insert("Access-Control-Max-Age", CORS_MAX_AGE.parse().unwrap());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_cors_headers() {
        let mut headers = HeaderMap::new();
        CorsLayer::apply_cors_headers(&mut headers);

        assert_eq!(headers.get("Access-Control-Allow-Origin").unwrap(), "*");

        let methods = headers
            .get("Access-Control-Allow-Methods")
            .unwrap()
            .to_str()
            .unwrap();
        assert!(methods.contains("GET"), "Must include GET");
        assert!(methods.contains("POST"), "Must include POST");
        assert!(methods.contains("DELETE"), "Must include DELETE");
        assert!(methods.contains("OPTIONS"), "Must include OPTIONS");

        let allowed = headers
            .get("Access-Control-Allow-Headers")
            .unwrap()
            .to_str()
            .unwrap();
        assert!(
            allowed.contains("Content-Type"),
            "Must include Content-Type"
        );
        assert!(allowed.contains("Accept"), "Must include Accept");
        assert!(
            allowed.contains("Authorization"),
            "Must include Authorization"
        );
        assert!(
            allowed.contains("Mcp-Session-Id"),
            "Must include Mcp-Session-Id"
        );
        assert!(
            allowed.contains("MCP-Protocol-Version"),
            "Must include MCP-Protocol-Version"
        );
        assert!(
            allowed.contains("Last-Event-ID"),
            "Must include Last-Event-ID"
        );

        assert_eq!(
            headers.get("Access-Control-Expose-Headers").unwrap(),
            "Mcp-Session-Id"
        );
        assert_eq!(
            headers.get("Access-Control-Max-Age").unwrap(),
            CORS_MAX_AGE
        );

        // Wildcard origin must NOT include credentials
        assert!(
            !headers.contains_key("Access-Control-Allow-Credentials"),
            "Wildcard origin must not set Allow-Credentials"
        );
    }

    /// Simulate the server.rs pipeline: options_response() returns a bare 200,
    /// then CorsLayer::apply_cors_headers() adds all CORS headers.
    #[test]
    fn test_options_response_then_cors_layer() {
        use crate::json_rpc_responses::options_response;

        let mut response = options_response();

        // Before CorsLayer: bare response with no CORS headers
        assert_eq!(response.status(), hyper::StatusCode::OK);
        assert!(
            response
                .headers()
                .get("Access-Control-Allow-Origin")
                .is_none(),
            "options_response() must not set CORS headers (CorsLayer handles that)"
        );
        assert!(
            response
                .headers()
                .get("Access-Control-Allow-Methods")
                .is_none(),
            "options_response() must not set CORS methods"
        );

        // Simulate server.rs:508 — apply CorsLayer
        CorsLayer::apply_cors_headers(response.headers_mut());

        // After CorsLayer: full CORS contract
        assert_eq!(
            response
                .headers()
                .get("Access-Control-Allow-Origin")
                .unwrap(),
            "*"
        );
        let methods = response
            .headers()
            .get("Access-Control-Allow-Methods")
            .unwrap()
            .to_str()
            .unwrap();
        assert!(methods.contains("DELETE"), "OPTIONS must expose DELETE");
        assert!(methods.contains("POST"), "OPTIONS must expose POST");

        let allowed = response
            .headers()
            .get("Access-Control-Allow-Headers")
            .unwrap()
            .to_str()
            .unwrap();
        assert!(allowed.contains("Authorization"));
        assert!(allowed.contains("Mcp-Session-Id"));
        assert!(allowed.contains("MCP-Protocol-Version"));
        assert!(allowed.contains("Last-Event-ID"));

        assert_eq!(
            response
                .headers()
                .get("Access-Control-Expose-Headers")
                .unwrap(),
            "Mcp-Session-Id"
        );
    }

    /// When enable_cors is false, server.rs skips CorsLayer.
    /// The bare OPTIONS response must have zero CORS headers.
    #[test]
    fn test_options_response_without_cors_has_no_cors_headers() {
        use crate::json_rpc_responses::options_response;

        let response = options_response();

        assert!(
            response
                .headers()
                .get("Access-Control-Allow-Origin")
                .is_none()
        );
        assert!(
            response
                .headers()
                .get("Access-Control-Allow-Methods")
                .is_none()
        );
        assert!(
            response
                .headers()
                .get("Access-Control-Allow-Headers")
                .is_none()
        );
        assert!(
            response
                .headers()
                .get("Access-Control-Expose-Headers")
                .is_none()
        );
    }
}
