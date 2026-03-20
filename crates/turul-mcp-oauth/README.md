# turul-mcp-oauth

[![Crates.io](https://img.shields.io/crates/v/turul-mcp-oauth.svg)](https://crates.io/crates/turul-mcp-oauth)
[![Documentation](https://docs.rs/turul-mcp-oauth/badge.svg)](https://docs.rs/turul-mcp-oauth)

OAuth 2.1 Resource Server support for the Turul MCP framework.

## Overview

This crate provides Bearer token validation for MCP servers acting as OAuth 2.1 Resource Servers (RS). It does **not** implement an Authorization Server — tokens are validated against an external AS via JWKS.

Key components:

- **`OAuthResourceMiddleware`** — Pre-session middleware that validates Bearer tokens and injects claims into request extensions
- **`JwtValidator`** — JWT validation with JWKS caching, kid-miss refresh, and RS256/ES256 support
- **`ProtectedResourceMetadata`** — RFC 9728 metadata document with canonical URI validation
- **`WellKnownOAuthHandler`** — Route handler for `/.well-known/oauth-protected-resource`

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
turul-mcp-server = "0.3"
turul-mcp-oauth = "0.3"
```

Use the convenience function for single-AS deployments:

```rust,no_run
use std::sync::Arc;
use turul_mcp_oauth::{ProtectedResourceMetadata, oauth_resource_server};

let metadata = ProtectedResourceMetadata::new(
    "https://example.com/mcp",
    vec!["https://auth.example.com".to_string()],
).unwrap();

let (auth_middleware, routes) = oauth_resource_server(
    metadata,
    "https://auth.example.com/.well-known/jwks.json",
).unwrap();

// Register with McpServer::builder()
// let mut builder = McpServer::builder()
//     .name("my-server")
//     .middleware(auth_middleware);
// for (path, handler) in routes {
//     builder = builder.route(&path, handler);
// }
```

## How It Works

1. Client sends a request with `Authorization: Bearer <JWT>`
2. `OAuthResourceMiddleware` validates the token against the AS's JWKS endpoint
3. Validated claims are injected into `RequestContext.extensions` as `"oauth_claims"`
4. Tools access claims via `session.get_extension("oauth_claims")`
5. Invalid/missing tokens produce HTTP 401 with a `WWW-Authenticate` challenge

The middleware runs before session creation (`runs_before_session() == true`), so invalid tokens are rejected without allocating a session.

## RFC 9728 Metadata

The crate serves Protected Resource Metadata at the well-known endpoint, enabling clients to discover which Authorization Server protects this resource:

```bash
curl https://example.com/.well-known/oauth-protected-resource
```

For resources with a path component (e.g., `https://example.com/mcp`), both root-form and path-form endpoints are registered automatically:

- `/.well-known/oauth-protected-resource` (root form)
- `/.well-known/oauth-protected-resource/mcp` (path form)

## Manual Configuration

For multi-AS deployments or custom validation, construct the components directly:

```rust,no_run
use std::sync::Arc;
use turul_mcp_oauth::{JwtValidator, OAuthResourceMiddleware, ProtectedResourceMetadata};

let metadata = ProtectedResourceMetadata::new(
    "https://example.com/mcp",
    vec![
        "https://auth1.example.com".to_string(),
        "https://auth2.example.com".to_string(),
    ],
).unwrap();

let validator = Arc::new(
    JwtValidator::new(
        "https://auth1.example.com/.well-known/jwks.json",
        "https://example.com/mcp",
    )
    .with_issuer("https://auth1.example.com"),
);

let middleware = Arc::new(OAuthResourceMiddleware::new(validator, metadata));
```

## Example

See [`examples/oauth-resource-server`](https://github.com/aussierobots/turul-mcp-framework/tree/main/examples/oauth-resource-server) for a complete working server with CLI arguments for JWKS URI, resource identifier, and authorization server URL.

## Related Crates

- [`turul-mcp-server`](https://crates.io/crates/turul-mcp-server) — Core framework
- [`turul-http-mcp-server`](https://crates.io/crates/turul-http-mcp-server) — HTTP transport with middleware support
