//! OAuth 2.1 Resource Server support for Turul MCP framework
//!
//! This crate provides OAuth 2.1 Bearer token validation for MCP servers
//! acting as Resource Servers (RS). It does NOT implement an Authorization
//! Server — tokens are validated against an external AS via JWKS.
//!
//! # Architecture
//!
//! - **`OAuthResourceMiddleware`** — Pre-session middleware that validates
//!   Bearer tokens and injects claims into request extensions
//! - **`JwtValidator`** — JWT validation with JWKS caching and kid-miss refresh
//! - **`ProtectedResourceMetadata`** — RFC 9728 metadata document
//! - **`WellKnownOAuthHandler`** — Route handler for `/.well-known/oauth-protected-resource`
//!
//! # Usage
//!
//! ```rust,no_run
//! use std::sync::Arc;
//! use turul_mcp_oauth::{
//!     JwtValidator, OAuthResourceMiddleware, ProtectedResourceMetadata,
//!     WellKnownOAuthHandler, oauth_resource_server,
//! };
//!
//! let metadata = ProtectedResourceMetadata::new(
//!     "https://example.com/mcp",
//!     vec!["https://auth.example.com".to_string()],
//! );
//!
//! let (auth_middleware, routes) = oauth_resource_server(
//!     metadata,
//!     "https://auth.example.com/.well-known/jwks.json",
//! );
//!
//! // Register with McpServer::builder()
//! //   let mut builder = McpServer::builder().middleware(auth_middleware);
//! //   for (path, handler) in routes {
//! //       builder = builder.route(&path, handler);
//! //   }
//! ```

pub mod error;
pub mod jwt;
pub mod metadata;
pub mod middleware;
pub mod well_known;

pub use error::OAuthError;
pub use jwt::{JwtValidator, TokenClaims};
pub use metadata::ProtectedResourceMetadata;
pub use middleware::OAuthResourceMiddleware;
pub use well_known::WellKnownOAuthHandler;

use std::sync::Arc;

/// A route entry: `(path, handler)` pair for registration with the server builder.
pub type RouteEntry = (String, Arc<dyn turul_http_mcp_server::routes::RouteHandler>);

/// Convenience function to create both the OAuth middleware and well-known route handlers
///
/// Returns `(middleware, routes)` where `routes` contains path/handler pairs for all
/// RFC 9728 metadata endpoints. For resources with a path component (e.g.,
/// `https://example.com/mcp`), this registers both root-form and path-form endpoints:
///
/// - `/.well-known/oauth-protected-resource` (root form, always)
/// - `/.well-known/oauth-protected-resource/mcp` (path form, when resource has a path)
///
/// Register all routes with the server builder:
/// ```rust,ignore
/// let (middleware, routes) = oauth_resource_server(metadata, jwks_uri);
/// let mut builder = McpServer::builder().middleware(middleware);
/// for (path, handler) in routes {
///     builder = builder.route(&path, handler);
/// }
/// ```
pub fn oauth_resource_server(
    metadata: ProtectedResourceMetadata,
    jwks_uri: &str,
) -> (Arc<OAuthResourceMiddleware>, Vec<RouteEntry>) {
    let validator = Arc::new(JwtValidator::new(jwks_uri));
    let middleware = Arc::new(OAuthResourceMiddleware::new(validator, metadata.clone()));
    let handler: Arc<dyn turul_http_mcp_server::routes::RouteHandler> =
        Arc::new(WellKnownOAuthHandler::new(&metadata));

    // Register both root-form and path-form endpoints per RFC 9728 §3
    let routes: Vec<RouteEntry> = metadata
        .well_known_paths()
        .into_iter()
        .map(|path| (path, handler.clone()))
        .collect();

    (middleware, routes)
}
