# ADR-021: OAuth 2.1 Resource Server Architecture

**Status**: Accepted

**Date**: 2026-03-06

## Context

MCP 2025-11-25 requires OAuth 2.1 support for protected resources. Clients need to
discover authorization servers, obtain Bearer tokens, and present them to MCP servers
acting as Resource Servers (RS). The specification references RFC 9728 (Protected
Resource Metadata) for discovery and RFC 6750 for Bearer token usage.

### Scope Decision: Resource Server Only

This implementation covers the **Resource Server** role exclusively. The framework
does NOT implement an Authorization Server — tokens are issued by an external AS and
validated via JWKS (JSON Web Key Sets). This decision reflects that:

1. Authorization Servers are complex, security-critical systems with their own
   standards (RFC 9126, RFC 7636, RFC 9207)
2. Most deployments use existing identity providers (Auth0, Okta, Keycloak, AWS Cognito)
3. The MCP spec requires RS behavior from servers, not AS behavior

## Design Decisions

### D1: MiddlewareError::HttpChallenge — Transport-Only HTTP Response

OAuth requires HTTP-level `WWW-Authenticate` headers on 401 responses. These don't
fit in JSON-RPC error responses. A new `MiddlewareError::HttpChallenge` variant
carries the HTTP status code and `WWW-Authenticate` header value.

**Key invariant**: `HttpChallenge` is handled exclusively at the transport level
(pre-session phase in `handle_client_message`), producing a raw HTTP response. It
never reaches `map_middleware_error_to_jsonrpc()`. An `unreachable!()` guard exists
in the JSON-RPC error mapper as a defensive assertion.

### D2: Request-Scoped Auth Context

Auth state is NOT persisted in session storage. Token claims live only for the
duration of one HTTP request in the in-memory `SessionContext.extensions` field.
This is a deliberate choice:

- Bearer tokens are validated per-request (RFC 6750 §2)
- Token claims may change between requests (rotation, scope changes)
- Session storage should not hold sensitive credential material

### D3: Auth Context Canonical Flow

One path, consistently documented:

```
Transport extracts Bearer → RequestContext.bearer_token
  ↓
Pre-session middleware validates token, writes claims
  ↓ writes to
RequestContext.extensions["__turul_internal.auth_claims"]
  ↓ transport copies to
json_rpc_server::SessionContext.extensions
  ↓ from_json_rpc_with_broadcaster() copies to
turul_mcp_server::SessionContext.extensions
  ↓ tool reads via
session.get_typed_extension::<TokenClaims>("__turul_internal.auth_claims")
```

Extensions are **never** persisted to session storage. The `__turul_internal.*`
namespace is reserved for framework use.

### D4: Two-Phase Middleware

OAuth auth MUST run before session creation to avoid allocating sessions for
unauthenticated requests. The `McpMiddleware` trait gains a
`runs_before_session() -> bool` method (default `false`). Pre-session middleware:

- Runs before session lookup/creation
- Receives `None` for the session parameter
- Can return `HttpChallenge` errors for HTTP-level responses
- Has zero overhead when no pre-session middleware is registered

### D5: Bearer Token Isolation

`Authorization: Bearer ...` is extracted into a dedicated `RequestContext` field.
Bearer-scheme headers NEVER enter the metadata map, even if the token itself is
malformed. Non-Bearer schemes (Basic, Digest) pass through to metadata normally.

### D6: Hardened Bearer Parsing

The Bearer token parser uses:
- Safe `splitn`-based parsing (no byte indexing)
- Case-insensitive scheme matching (RFC 7235 §2.1)
- OWS (optional whitespace) handling
- Rejection of all ASCII control characters
- Rejection of multi-token values (embedded whitespace)

This parser is shared between both transport layers (Streamable HTTP and legacy SSE).

### Route Registry Security

The `RouteRegistry` for serving `.well-known` paths enforces:
- Exact-match only (no prefix matching, no wildcards)
- Case-sensitive comparison (RFC 8615)
- Rejection of path traversal (`/../`, `/./`)
- Rejection of percent-encoded separators (`%2f`, `%2e`, `%00`)
- Rejection of double-encoded values (`%252f`)
- No normalization — malicious paths are rejected, not silently corrected

## Implementation

### New Crate: `turul-mcp-oauth`

- `OAuthResourceMiddleware` — Pre-session middleware implementing Bearer token
  validation and `WWW-Authenticate` challenge generation
- `JwtValidator` — RS256/ES256 JWT validation with JWKS caching, kid-miss refresh,
  and 60-second rate limiting on JWKS fetches
- `ProtectedResourceMetadata` — RFC 9728 metadata document (serializable)
- `WellKnownOAuthHandler` — Route handler for `/.well-known/oauth-protected-resource`
- `oauth_resource_server()` — Convenience function returning both middleware and
  route handler

### Framework Changes

- `MiddlewareError::HttpChallenge` variant in `turul-http-mcp-server`
- `RequestContext` gains `bearer_token` and `extensions` fields
- `McpMiddleware::runs_before_session()` default method
- `MiddlewareStack::execute_before_session()` for pre-session phase
- `RouteRegistry` and `RouteHandler` trait for custom HTTP routes
- `SessionContext.extensions` field threaded through JSON-RPC → framework layers
- Lambda parity: route support in `LambdaMcpHandler` and builder

### Usage

```rust
let metadata = ProtectedResourceMetadata::new(
    "https://example.com/mcp",
    vec!["https://auth.example.com".to_string()],
);

let (auth_middleware, well_known) = oauth_resource_server(
    metadata,
    "https://auth.example.com/.well-known/jwks.json",
);

McpServer::builder()
    .middleware(auth_middleware)
    .route("/.well-known/oauth-protected-resource", well_known)
    .build()?
```

## Consequences

### Positive

- MCP servers can participate in OAuth 2.1 flows as Resource Servers
- Pre-session middleware prevents session allocation for unauthenticated requests
- Request-scoped extensions provide a general-purpose mechanism for middleware→tool
  communication beyond OAuth
- Route registry enables custom HTTP endpoints without transport modifications
- Full Lambda parity from day one

### Negative

- Extensions are not type-safe at compile time (string-keyed `Value` map)
- JWKS refresh adds a network dependency at token validation time
- Pre-session middleware adds a phase to request processing (zero overhead when unused)

### Risks

- JWKS endpoint availability affects token validation
  (mitigated by caching + rate limiting)
- Token replay attacks if tokens have long expiration
  (mitigated by exp validation, out of scope for RS)
