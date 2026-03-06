//! Route registry for custom HTTP paths (e.g., `.well-known` endpoints)
//!
//! Provides exact-match routing with strict security validation.
//! All paths are matched as raw strings — no normalization is performed.

use async_trait::async_trait;
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::{Request, Response, StatusCode};
use std::sync::Arc;

/// Type-erased body used by route handlers.
///
/// Both `hyper::body::Incoming` (HTTP server) and `Full<Bytes>` (Lambda)
/// can be boxed into this type, making route handlers transport-portable.
pub type RouteBody = http_body_util::combinators::UnsyncBoxBody<Bytes, hyper::Error>;

/// Handler for a custom HTTP route
#[async_trait]
pub trait RouteHandler: Send + Sync {
    /// Handle an incoming HTTP request for this route
    async fn handle(&self, req: Request<RouteBody>) -> Response<RouteBody>;
}

/// Registry of custom HTTP routes with strict path validation
///
/// # Security Contract
///
/// - **Exact-match only** — no prefix matching, no wildcards, no regex
/// - **Case-sensitive** per RFC 8615
/// - **Reject before matching** — path traversal, double slashes, percent-encoded
///   separators, and null bytes are rejected with 400 Bad Request
/// - **No normalization** — malicious paths are rejected, not normalized
pub struct RouteRegistry {
    routes: Vec<(String, Arc<dyn RouteHandler>)>,
}

impl RouteRegistry {
    /// Create a new empty route registry
    pub fn new() -> Self {
        Self { routes: Vec::new() }
    }

    /// Add a route to the registry
    ///
    /// # Parameters
    ///
    /// - `path`: Exact path to match (e.g., `/.well-known/oauth-protected-resource`)
    /// - `handler`: Handler to invoke when the path matches
    pub fn add_route(&mut self, path: &str, handler: Arc<dyn RouteHandler>) {
        self.routes.push((path.to_string(), handler));
    }

    /// Check if the registry has any routes
    pub fn is_empty(&self) -> bool {
        self.routes.is_empty()
    }

    /// Match a request path against registered routes
    ///
    /// Returns `None` if the path is rejected by security checks or doesn't match.
    /// Returns the handler if an exact match is found.
    pub fn match_route(
        &self,
        path: &str,
    ) -> Result<Option<&Arc<dyn RouteHandler>>, RouteValidationError> {
        // Security validation — reject before matching
        validate_path(path)?;

        // Exact match only
        for (registered_path, handler) in &self.routes {
            if path == registered_path {
                return Ok(Some(handler));
            }
        }

        Ok(None)
    }
}

impl Default for RouteRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Error returned when a request path fails security validation
#[derive(Debug, Clone, PartialEq)]
pub enum RouteValidationError {
    /// Path contains path traversal sequences
    PathTraversal,
    /// Path contains double slashes
    DoubleSlash,
    /// Path contains percent-encoded separators or null bytes
    EncodedSeparator,
    /// Path contains null bytes
    NullByte,
}

impl std::fmt::Display for RouteValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PathTraversal => write!(f, "Path traversal detected"),
            Self::DoubleSlash => write!(f, "Double slash detected"),
            Self::EncodedSeparator => write!(f, "Percent-encoded separator detected"),
            Self::NullByte => write!(f, "Null byte detected"),
        }
    }
}

impl std::error::Error for RouteValidationError {}

impl RouteValidationError {
    /// Convert to an HTTP 400 Bad Request response
    pub fn into_response(self) -> Response<RouteBody> {
        Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .header("Content-Type", "text/plain")
            .body(
                Full::new(Bytes::from(format!("Bad Request: {}", self)))
                    .map_err(|never| match never {})
                    .boxed_unsync(),
            )
            .unwrap()
    }
}

/// Validate a request path against security rules.
///
/// Rejects:
/// - Path traversal: `/../`, `/./`, trailing `..`
/// - Double slashes: `//`
/// - Percent-encoded separators: `%2f`, `%2F` (slash), `%2e` (dot), `%00` (null byte)
/// - Double-encoded values: `%252f`, `%252e`
/// - Null bytes: `\0` anywhere in path
fn validate_path(path: &str) -> Result<(), RouteValidationError> {
    // Null byte check (raw)
    if path.contains('\0') {
        return Err(RouteValidationError::NullByte);
    }

    // Double slash check
    if path.contains("//") {
        return Err(RouteValidationError::DoubleSlash);
    }

    // Path traversal check
    if path.contains("/../")
        || path.contains("/./")
        || path.ends_with("/..")
        || path.ends_with("/.")
        || path == ".."
        || path == "."
    {
        return Err(RouteValidationError::PathTraversal);
    }

    // Percent-encoded separator/null check (case-insensitive)
    let lower = path.to_ascii_lowercase();
    if lower.contains("%2f") || lower.contains("%2e") || lower.contains("%00") {
        return Err(RouteValidationError::EncodedSeparator);
    }

    // Double-encoded check
    if lower.contains("%252f") || lower.contains("%252e") {
        return Err(RouteValidationError::EncodedSeparator);
    }

    // Malformed percent-encoding: % must be followed by exactly 2 hex digits
    let bytes = path.as_bytes();
    for i in 0..bytes.len() {
        if bytes[i] == b'%'
            && (i + 2 >= bytes.len()
                || !bytes[i + 1].is_ascii_hexdigit()
                || !bytes[i + 2].is_ascii_hexdigit())
        {
            return Err(RouteValidationError::EncodedSeparator);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    struct TestHandler {
        body: String,
    }

    #[async_trait]
    impl RouteHandler for TestHandler {
        async fn handle(&self, _req: Request<RouteBody>) -> Response<RouteBody> {
            Response::builder()
                .status(StatusCode::OK)
                .body(
                    Full::new(Bytes::from(self.body.clone()))
                        .map_err(|never| match never {})
                        .boxed_unsync(),
                )
                .unwrap()
        }
    }

    fn registry_with_well_known() -> RouteRegistry {
        let mut registry = RouteRegistry::new();
        registry.add_route(
            "/.well-known/oauth-protected-resource",
            Arc::new(TestHandler {
                body: r#"{"resource":"https://example.com/mcp"}"#.to_string(),
            }),
        );
        registry
    }

    #[test]
    fn test_route_registry_exact_match() {
        // T10: Exact match
        let registry = registry_with_well_known();
        let result = registry
            .match_route("/.well-known/oauth-protected-resource")
            .unwrap();
        assert!(result.is_some());
    }

    #[test]
    fn test_route_registry_no_prefix_match() {
        // T11: No prefix matching
        let registry = registry_with_well_known();
        let result = registry
            .match_route("/.well-known/oauth-protected-resource/extra")
            .unwrap();
        assert!(result.is_none());

        let result = registry.match_route("/.well-known").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_route_registry_case_sensitive() {
        // T12: Case sensitive
        let registry = registry_with_well_known();
        let result = registry
            .match_route("/.Well-Known/OAuth-Protected-Resource")
            .unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_route_registry_reject_path_traversal() {
        // T13: Path traversal rejection
        let registry = registry_with_well_known();

        assert!(matches!(
            registry.match_route("/../.well-known/oauth-protected-resource"),
            Err(RouteValidationError::PathTraversal)
        ));
        assert!(matches!(
            registry.match_route("/.well-known/../admin"),
            Err(RouteValidationError::PathTraversal)
        ));
        assert!(matches!(
            registry.match_route("/./test"),
            Err(RouteValidationError::PathTraversal)
        ));
        assert!(matches!(
            registry.match_route("/test/.."),
            Err(RouteValidationError::PathTraversal)
        ));
    }

    #[test]
    fn test_route_reject_percent_encoded_slash() {
        // T38: Percent-encoded slash
        let registry = registry_with_well_known();
        assert!(matches!(
            registry.match_route("/.well-known%2foauth-protected-resource"),
            Err(RouteValidationError::EncodedSeparator)
        ));
        assert!(matches!(
            registry.match_route("/.well-known%2Foauth-protected-resource"),
            Err(RouteValidationError::EncodedSeparator)
        ));
    }

    #[test]
    fn test_route_reject_double_encoding() {
        // T39: Double encoding
        let registry = registry_with_well_known();
        assert!(matches!(
            registry.match_route("/.well-known%252foauth"),
            Err(RouteValidationError::EncodedSeparator)
        ));
        assert!(matches!(
            registry.match_route("/.well-known%252etest"),
            Err(RouteValidationError::EncodedSeparator)
        ));
    }

    #[test]
    fn test_route_reject_null_byte() {
        // T40: Null byte
        let registry = registry_with_well_known();
        assert!(matches!(
            registry.match_route("/.well-known\x00/test"),
            Err(RouteValidationError::NullByte)
        ));
    }

    #[test]
    fn test_route_reject_double_slash() {
        // T41: Double slash
        let registry = registry_with_well_known();
        assert!(matches!(
            registry.match_route("//.well-known/oauth-protected-resource"),
            Err(RouteValidationError::DoubleSlash)
        ));
    }

    #[test]
    fn test_route_no_match_returns_none() {
        let registry = registry_with_well_known();
        let result = registry.match_route("/not-registered").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_route_reject_malformed_percent_encoding() {
        let registry = registry_with_well_known();
        // Trailing percent with no hex digits
        assert!(matches!(
            registry.match_route("/test%"),
            Err(RouteValidationError::EncodedSeparator)
        ));
        // Percent with only one hex digit
        assert!(matches!(
            registry.match_route("/test%2"),
            Err(RouteValidationError::EncodedSeparator)
        ));
        // Percent with non-hex characters
        assert!(matches!(
            registry.match_route("/test%ZZ"),
            Err(RouteValidationError::EncodedSeparator)
        ));
        assert!(matches!(
            registry.match_route("/test%G1"),
            Err(RouteValidationError::EncodedSeparator)
        ));
    }

    #[test]
    fn test_empty_registry() {
        let registry = RouteRegistry::new();
        assert!(registry.is_empty());
        let result = registry.match_route("/anything").unwrap();
        assert!(result.is_none());
    }
}
