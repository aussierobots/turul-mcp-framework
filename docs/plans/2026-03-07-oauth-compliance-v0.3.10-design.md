# OAuth 2.1 Resource Server Compliance — v0.3.10

**Status**: Approved
**Date**: 2026-03-07

## Problem

Codex audit against MCP 2025-11-25 Authorization spec found 6 compliance gaps in
`turul-mcp-oauth` (P0-P2). The v0.3.9 implementation is architecturally sound but
defaults are not spec-compliant.

## Changes

### P0: Audience validation required by default

`JwtValidator::new(jwks_uri, audience)` — audience becomes a required parameter.
The MCP spec says servers MUST validate token audience. No opt-out.

### P1a: Issuer policy enforced at convenience layer

- `JwtValidator` keeps optional `with_issuer()` for advanced use.
- `oauth_resource_server()` enforces exactly one authorization server in metadata
  and calls `with_issuer(single_as)` automatically. Multiple AS → error.

### P1b: Scope in WWW-Authenticate

When `scopes_supported` is configured on metadata, the middleware includes
`scope="scope1 scope2"` in the `WWW-Authenticate` header per RFC 6750 Section 3.

### P2a: Canonical URI validation

`ProtectedResourceMetadata::new()` becomes fallible (`-> Result<Self, OAuthError>`).
Validates using `url::Url`:
- `resource`: absolute URI, scheme present (http/https), authority present, no fragment
- `authorization_servers`: non-empty, each absolute URI, no fragment

### P2b: Cache-Control on challenge responses

All 401/403 challenge responses include `Cache-Control: no-store` per OAuth 2.1 Section 5.3.

### P2c: 403 insufficient_scope support

`MiddlewareError::HttpChallenge` already supports status 403. Document the pattern
for tool-level scope checks returning 403 with `error="insufficient_scope"`.

## Non-goals

- Authorization Server implementation (out of scope)
- Client-side OAuth flows (PKCE, dynamic registration, etc.)
- Multi-AS JWKS federation (advanced use case, not needed for 0.3.10)

## Release bar for 0.3.10

1. Audience validated by default in JwtValidator and oauth_resource_server
2. Issuer policy enforced (single AS required, no silent [0] fallback)
3. ProtectedResourceMetadata.resource validated as canonical URI
4. Scope in WWW-Authenticate when configured
5. Cache-Control: no-store on challenge responses
6. Tests cover all new failure modes
7. ADR-021 updated
8. CHANGELOG.md updated
9. Version bumped workspace-wide
10. Dry publish passes
