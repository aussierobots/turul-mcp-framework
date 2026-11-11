//! OAuth 2.1 resource-server support (Bearer token validation) for MCP servers.
//!
//! Add this crate alongside [`turul-mcp-server`](https://docs.rs/turul-mcp-server)
//! to protect an MCP endpoint: it validates tokens issued by an external authorization server via JWKS and
//! serves RFC 9728 Protected Resource Metadata. It does not implement an
//! authorization server.
//!
//! # Example
//!
//! ```toml
//! [dependencies]
//! turul-mcp-oauth = "0.4"
//! ```
//!
//! ```rust,no_run
//! use turul_mcp_oauth::{ProtectedResourceMetadata, oauth_resource_server};
//!
//! let metadata = ProtectedResourceMetadata::new(
//!     "https://example.com/mcp",
//!     vec!["https://auth.example.com".to_string()],
//! )?;
//! let (middleware, routes) = oauth_resource_server(
//!     metadata,
//!     "https://auth.example.com/.well-known/jwks.json",
//! )?;
//! // With turul-mcp-server: McpServer::builder().middleware(middleware), then
//! // builder.route(&path, handler) for each (path, handler) in `routes`.
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Components
//!
//! - [`OAuthResourceMiddleware`]: validates Bearer tokens (RFC 8707 audience
//!   binding) and injects claims into request extensions
//! - [`JwtValidator`]: JWKS-backed validation, owned by [`turul_jwt_validator`]
//!   and re-exported
//! - [`hardened_validator`]: a [`JwtValidator`] with this crate's key max-age,
//!   stale window and fetch retry; rejects a non-loopback plaintext `jwks_uri`.
//!   The bare constructor leaves those disabled
//! - [`ProtectedResourceMetadata`] and [`WellKnownOAuthHandler`]: serve
//!   `/.well-known/oauth-protected-resource`
//!
//! Client ID Metadata Documents and Dynamic Client Registration (RFC 7591) are
//! authorization-server and client concerns; this crate implements neither.
//!
//! # Where next
//!
//! - [`oauth_resource_server`]: the one-call setup, and multi-AS wiring notes
//! - [`turul-mcp-server`](https://docs.rs/turul-mcp-server): the server builder

pub mod error;
pub mod jwt;
pub mod metadata;
pub mod middleware;
pub mod well_known;

pub use error::OAuthError;
pub use jwt::{
    Algorithm, JwksFetchErrorKind, JwtValidationError, JwtValidator, TokenClaims,
    hardened_validator,
};
pub use metadata::ProtectedResourceMetadata;
pub use middleware::OAuthResourceMiddleware;
pub use well_known::WellKnownOAuthHandler;

use std::sync::Arc;

/// A route entry: `(path, handler)` pair for registration with the server builder.
pub type RouteEntry = (String, Arc<dyn turul_http_mcp_server::routes::RouteHandler>);

/// Convenience function to create both the OAuth middleware and well-known route handlers
///
/// Requires exactly one authorization server in the metadata. For multi-AS deployments,
/// build the validator with [`hardened_validator`] and wire `OAuthResourceMiddleware`
/// manually — not `JwtValidator::new`, which applies no hardening.
///
/// Returns `Ok((middleware, routes))` where `routes` contains path/handler pairs for all
/// RFC 9728 metadata endpoints. For resources with a path component (e.g.,
/// `https://example.com/mcp`), this registers both root-form and path-form endpoints:
///
/// - `/.well-known/oauth-protected-resource` (root form, always)
/// - `/.well-known/oauth-protected-resource/mcp` (path form, when resource has a path)
///
/// Register all routes with the server builder:
/// ```rust,ignore
/// let (middleware, routes) = oauth_resource_server(metadata, jwks_uri)?;
/// let mut builder = McpServer::builder().middleware(middleware);
/// for (path, handler) in routes {
///     builder = builder.route(&path, handler);
/// }
/// ```
pub fn oauth_resource_server(
    metadata: ProtectedResourceMetadata,
    jwks_uri: &str,
) -> Result<(Arc<OAuthResourceMiddleware>, Vec<RouteEntry>), OAuthError> {
    if metadata.authorization_servers.len() != 1 {
        return Err(OAuthError::InvalidConfiguration(format!(
            "oauth_resource_server requires exactly one authorization server, got {}; \
             for multi-AS deployments, build one with hardened_validator and wire \
             OAuthResourceMiddleware manually",
            metadata.authorization_servers.len()
        )));
    }
    let audience = &metadata.resource;
    let issuer = &metadata.authorization_servers[0];
    let validator = Arc::new(jwt::hardened_validator(jwks_uri, audience)?.with_issuer(issuer));
    let middleware = Arc::new(OAuthResourceMiddleware::new(validator, metadata.clone()));
    let handler: Arc<dyn turul_http_mcp_server::routes::RouteHandler> =
        Arc::new(WellKnownOAuthHandler::new(&metadata));

    // Register both root-form and path-form endpoints per RFC 9728 §3
    let routes: Vec<RouteEntry> = metadata
        .well_known_paths()
        .into_iter()
        .map(|path| (path, handler.clone()))
        .collect();

    Ok((middleware, routes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_as_ok() {
        let metadata = ProtectedResourceMetadata::new(
            "https://example.com/mcp",
            vec!["https://auth.example.com".to_string()],
        )
        .unwrap();
        assert!(
            oauth_resource_server(metadata, "https://auth.example.com/.well-known/jwks.json")
                .is_ok()
        );
    }

    #[test]
    fn test_multiple_as_rejected() {
        let metadata = ProtectedResourceMetadata::new(
            "https://example.com/mcp",
            vec![
                "https://a1.example.com".to_string(),
                "https://a2.example.com".to_string(),
            ],
        )
        .unwrap();
        assert!(
            oauth_resource_server(metadata, "https://auth.example.com/.well-known/jwks.json")
                .is_err()
        );
    }
}
