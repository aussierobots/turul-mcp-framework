---
name: auth-patterns
description: >
  This skill should be used when the user asks about "OAuth",
  "authentication", "authorization", "JWT", "Bearer",
  "JwtValidator", "oauth_resource_server", "ProtectedResourceMetadata",
  "turul-mcp-oauth", "API key auth", "auth middleware",
  "token validation", "WWW-Authenticate", "audience validation",
  "OAuthResourceMiddleware", "TokenClaims", "JWKS", "well-known",
  "oauth-protected-resource", "RFC 9728", "Bearer token",
  "auth-patterns", or "WellKnownOAuthHandler".
  Covers authentication and authorization patterns for MCP servers
  in the Turul MCP Framework (Rust): OAuth 2.1 Resource Server
  compliance, JWT validation, API key middleware, and Lambda
  authorizer integration.
---

# Auth Patterns — Turul MCP Framework

Add authentication to MCP servers. In OAuth terms, your MCP server plays the **Resource Server (RS)** role — it validates tokens, it doesn't issue them. But how you implement that validation depends on your deployment topology:

- **Direct validation in the MCP server** — `turul-mcp-oauth` middleware validates JWTs against JWKS (Patterns 1-2)
- **Gateway-level validation** — API Gateway authorizer validates tokens before your Lambda runs, passing claims downstream (Pattern 4)
- **Simple API keys** — Custom `McpMiddleware` checks a header value (Pattern 3)

These approaches can be combined. For example, a Gateway authorizer handles token validation at the edge while `turul-mcp-oauth` provides RFC 9728 metadata discovery and structured claims access inside the server.

## Decision Tree

```
How do you want to authenticate MCP clients?
│
├─ OAuth 2.1 / JWT tokens from an AS ──────→ turul-mcp-oauth (this skill, Pattern 1-2)
│   ├─ Single Authorization Server ────────→ oauth_resource_server() convenience function
│   └─ Multiple ASes or custom config ────→ Manual JwtValidator + OAuthResourceMiddleware
│
├─ API key in header ──────────────────────→ Custom McpMiddleware (Pattern 3)
│   └─ See middleware-patterns skill
│
└─ API Gateway authorizer (Lambda) ────────→ x-authorizer-* headers (Pattern 4)
    └─ See lambda-deployment skill
```

## Pattern 1: OAuth 2.1 RS — Single Authorization Server

The convenience function for the most common case: one AS, audience = resource URL.

```rust
// turul-mcp-server v0.3
use turul_mcp_oauth::{ProtectedResourceMetadata, oauth_resource_server};
use turul_mcp_server::prelude::*;

// 1. Define metadata (RFC 9728)
let metadata = ProtectedResourceMetadata::new(
    "https://example.com/mcp",                          // Resource URL (= audience)
    vec!["https://auth.example.com".to_string()],       // Authorization Server
)?
.with_scopes(vec!["mcp:read".to_string(), "mcp:write".to_string()]);

// 2. Create middleware + well-known routes
let (auth_middleware, routes) = oauth_resource_server(metadata, jwks_uri)?;

// 3. Wire into server
let mut builder = McpServer::builder()
    .name("my-server")
    .middleware(auth_middleware)    // Validates Bearer tokens
    .tool(MyTool::default());

for (path, handler) in routes {
    builder = builder.route(&path, handler);  // /.well-known/oauth-protected-resource
}

let server = builder.build()?;
```

**What `oauth_resource_server()` does for you:**
- Creates a `JwtValidator` with audience = resource URL, issuer = first AS
- Wraps it in `OAuthResourceMiddleware` (pre-session, so 401s don't allocate sessions)
- Registers `WellKnownOAuthHandler` for both root-form and path-form endpoints
- Returns `(Arc<OAuthResourceMiddleware>, Vec<RouteEntry>)` ready to wire in

**See:** `examples/oauth-resource-server.rs` for the full annotated version.

## Pattern 2: Manual JWT Validation

When you need custom audience, multiple ASes, or algorithm restrictions:

```rust
// turul-mcp-server v0.3
use std::sync::Arc;
use turul_mcp_oauth::{JwtValidator, OAuthResourceMiddleware, ProtectedResourceMetadata,
                       WellKnownOAuthHandler};

// Custom audience (not the resource URL)
let validator = Arc::new(
    JwtValidator::new(
        "https://auth.example.com/.well-known/jwks.json",
        "my-custom-audience",  // Audience is ALWAYS required
    )
    .with_issuer("https://auth.example.com")
    .with_algorithms(vec![
        jsonwebtoken::Algorithm::RS256,
        jsonwebtoken::Algorithm::ES256,
    ])
    .with_refresh_interval(std::time::Duration::from_secs(120)),
);

let metadata = ProtectedResourceMetadata::new(
    "https://example.com/mcp",
    vec!["https://auth.example.com".to_string()],
)?;

let middleware = Arc::new(OAuthResourceMiddleware::new(validator, metadata.clone()));
let well_known = Arc::new(WellKnownOAuthHandler::new(metadata.clone()));

let mut builder = McpServer::builder()
    .name("my-server")
    .middleware(middleware);

for path in metadata.well_known_paths() {
    builder = builder.route(&path, well_known.clone());
}
```

**See:** `references/jwt-validator-reference.md` for the full `JwtValidator` API.

## ProtectedResourceMetadata (RFC 9728)

The metadata document tells clients how to authenticate. It's served at `/.well-known/oauth-protected-resource`.

```rust
let metadata = ProtectedResourceMetadata::new(
    "https://example.com/mcp",                    // Resource identifier
    vec!["https://auth.example.com".to_string()], // Authorization Server(s)
)?
.with_jwks_uri("https://auth.example.com/.well-known/jwks.json")
.with_scopes(vec!["mcp:read".to_string(), "mcp:write".to_string()]);
```

**Validation rules (enforced at construction):**
- `resource` must be a canonical HTTP(S) URI (no fragments, no non-HTTP schemes)
- All `authorization_servers` must also be canonical HTTP(S) URIs
- At least one authorization server is required

**Well-known endpoints (auto-registered):**
- **Root form**: `/.well-known/oauth-protected-resource` (always)
- **Path form**: `/.well-known/oauth-protected-resource/mcp` (when resource has path `/mcp`)

The handler returns JSON with `Cache-Control: public, max-age=3600`.

## Audience Validation

Audience validation is **always required** — there's no opt-out.

```rust
// Audience is a required parameter in JwtValidator::new()
let validator = JwtValidator::new(jwks_uri, "my-audience");  // NOT Option<&str>
```

With `oauth_resource_server()`, the audience is automatically set to the resource URL from your metadata. With manual construction, you choose any audience string.

**Why mandatory**: Without audience validation, a token issued for `https://other-service.com` would be accepted by your MCP server. This is a critical security requirement per OAuth 2.1.

## Accessing Auth Claims in Tools

The middleware injects `TokenClaims` into request extensions. Tools read them via `SessionContext`:

```rust
// turul-mcp-server v0.3
use turul_mcp_oauth::TokenClaims;

#[derive(McpTool, Clone, Default)]
#[tool(name = "whoami", description = "Returns authenticated user identity")]
struct WhoAmITool {}

impl WhoAmITool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<serde_json::Value> {
        let session = session.ok_or_else(|| McpError::InvalidRequest {
            message: "Session required".to_string(),
        })?;

        // Current internal convention — no public constant yet
        let claims: TokenClaims = session
            .get_typed_extension("__turul_internal.auth_claims")
            .ok_or_else(|| McpError::InvalidRequest {
                message: "Not authenticated".to_string(),
            })?;

        Ok(serde_json::json!({
            "subject": claims.sub,
            "issuer": claims.iss,
            "scopes": claims.scope.map(|s| s.split_whitespace().collect::<Vec<_>>()),
        }))
    }
}
```

**TokenClaims fields:**

| Field | Type | Description |
|---|---|---|
| `sub` | `String` | Subject (user identifier) |
| `iss` | `String` | Issuer (Authorization Server URL) |
| `aud` | `serde_json::Value` | Audience (string or array) |
| `exp` | `u64` | Expiration (Unix timestamp) |
| `iat` | `u64` | Issued at (Unix timestamp) |
| `scope` | `Option<String>` | Space-separated scopes |
| `extra` | `HashMap<String, Value>` | All other claims |

**Extension key**: `"__turul_internal.auth_claims"` — this is the current internal convention used by `OAuthResourceMiddleware`. There is no public constant for it yet, so use this exact string. If the framework introduces a public constant in a future version, prefer that over the raw string.

## JWKS Caching & Key Rotation

`JwtValidator` caches JWKS keys in memory with automatic refresh:

1. **First request**: Fetches keys from `jwks_uri`, caches by `kid` (key ID)
2. **Subsequent requests**: Uses cached key (read lock, non-blocking)
3. **Key not found**: Refreshes JWKS once (handles AS key rotation)
4. **Rate limiting**: Minimum 60s between refreshes (configurable via `.with_refresh_interval()`)

**Key types supported**: RSA (RS256/384/512) and EC (ES256 P-256, ES384 P-384). Default algorithms: RS256, ES256.

**Cold start on Lambda**: JWKS keys are cached in the `OnceCell` handler — fetched once per container, reused across invocations. No repeated JWKS fetches on warm invocations.

## Pattern 3: API Key Middleware

For simple API key authentication without OAuth:

```rust
// turul-mcp-server v0.3
use turul_mcp_server::prelude::*;

struct ApiKeyMiddleware {
    valid_keys: Vec<String>,
}

#[async_trait::async_trait]
impl McpMiddleware for ApiKeyMiddleware {
    fn runs_before_session(&self) -> bool { true }

    async fn before_dispatch(
        &self,
        ctx: &mut RequestContext<'_>,
        _session: Option<&dyn SessionView>,
        _injection: &mut SessionInjection,
    ) -> Result<(), MiddlewareError> {
        let key = ctx.metadata()
            .get("x-api-key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| MiddlewareError::Unauthorized {
                message: "Missing X-API-Key header".into(),
            })?;

        if !self.valid_keys.contains(&key.to_string()) {
            return Err(MiddlewareError::Unauthorized {
                message: "Invalid API key".into(),
            });
        }

        Ok(())
    }
}
```

**When to use API keys instead of OAuth:**
- Internal services with pre-shared keys
- Simple authentication without an Authorization Server
- Development/testing environments

**See:** the `middleware-patterns` skill for the full `McpMiddleware` trait, error variants, and session injection.

## Pattern 4: Lambda API Gateway Authorizer

On Lambda, API Gateway can validate tokens before your MCP server runs. The authorizer populates `x-authorizer-*` headers automatically:

```rust
// Read authorizer claims in middleware or tools
let user_id = ctx.metadata()
    .get("x-authorizer-principalid")
    .and_then(|v| v.as_str());
```

The Lambda adapter converts camelCase authorizer fields to snake_case headers (`userId` → `x-authorizer-user_id`). Both V1 (REST API) and V2 (HTTP API) formats are supported.

**When to use Gateway authorizers instead of `turul-mcp-oauth`:**
- AWS-native deployments with Cognito or custom authorizers
- Token validation at the edge (before Lambda invocation)
- Reduced cold-start latency (no JWKS fetch in Lambda)

**Combining both**: You can use a Gateway authorizer for initial validation AND `turul-mcp-oauth` for additional claims extraction. The middleware runs after the Gateway authorizer has already validated the token.

**See:** the `lambda-deployment` skill for full API Gateway authorizer integration.

## Lambda + OAuth Deployment

Wire `turul-mcp-oauth` into a Lambda server. Register well-known routes via `.route()` on the builder — `handle_streaming()` checks the route registry before MCP dispatch, so no custom dispatch logic is needed:

```rust
// turul-mcp-server v0.3
use turul_mcp_aws_lambda::{LambdaMcpServerBuilder, run_streaming};
use turul_mcp_oauth::{ProtectedResourceMetadata, oauth_resource_server};

let metadata = ProtectedResourceMetadata::new(
    "https://api.example.com/mcp",
    vec!["https://auth.example.com".to_string()],
)?;

let (auth_middleware, routes) = oauth_resource_server(metadata, jwks_uri)?;

let mut builder = LambdaMcpServerBuilder::new()
    .name("oauth-lambda")
    .version("1.0.0")
    .middleware(auth_middleware)
    .tool(MyTool::default())
    .storage(storage)
    .sse(true);

// Register well-known routes — handle_streaming() checks these before MCP dispatch
for (path, handler) in routes {
    builder = builder.route(&path, handler);
}

let server = builder.build().await?;
let handler = server.handler().await?;

// run_streaming() works here — well-known routes are in the route registry
run_streaming(handler).await
```

**See:** `examples/lambda-oauth-server.rs` for the full annotated version.

## Cargo.toml

```toml
[dependencies]
turul-mcp-server = { version = "0.3" }
turul-mcp-oauth = { version = "0.3" }

# For Lambda deployment:
turul-mcp-aws-lambda = { version = "0.3", features = ["streaming", "dynamodb"] }
```

The `turul-mcp-oauth` crate has no feature flags — all functionality is always available.

## OAuthError Variants

| Variant | When | HTTP |
|---|---|---|
| `InvalidToken(String)` | JWT decode or validation failed | 401 |
| `TokenExpired` | `exp` claim is in the past | 401 |
| `InvalidAudience` | `aud` claim doesn't match required audience | 401 |
| `InvalidIssuer` | `iss` claim doesn't match configured issuer | 401 |
| `UnsupportedAlgorithm(String)` | JWT uses algorithm not in allowed list | 401 |
| `JwksFetchError(String)` | HTTP error fetching JWKS endpoint | 401 |
| `KeyNotFound(String)` | `kid` not found in JWKS after refresh | 401 |
| `DecodingError(String)` | JWT header malformed | 401 |
| `InvalidResourceUri(String)` | Non-canonical URI in metadata | (construction) |
| `InvalidConfiguration(String)` | Invalid metadata or validator config | (construction) |

All runtime errors produce HTTP 401 with a `WWW-Authenticate` header:
```
Bearer realm="mcp", resource_metadata="https://example.com/.well-known/oauth-protected-resource", error="invalid_token", error_description="..."
```

When `scopes_supported` is configured, `scope="mcp:read mcp:write"` is included in the header.

## Common Mistakes

1. **Forgetting audience validation is mandatory** — `JwtValidator::new()` requires an audience string. There's no way to skip it. This is by design per OAuth 2.1.

2. **Using `oauth_resource_server()` with multiple ASes** — The convenience function takes only the first AS from `authorization_servers`. For multi-AS setups, construct `JwtValidator` and `OAuthResourceMiddleware` manually.

3. **Wrong extension key for claims** — Claims are currently stored at `"__turul_internal.auth_claims"` (no public constant yet). Use this exact string. If the framework introduces a public constant, prefer that.

4. **Not registering well-known routes** — `oauth_resource_server()` returns routes that must be registered via `.route()`. Without them, clients can't discover your Authorization Server per RFC 9728.

5. **Expecting `turul-mcp-oauth` to issue tokens** — The crate is a Resource Server only. Token issuance is the Authorization Server's job (Cognito, Auth0, Keycloak, etc.).

6. **JWKS endpoint hammering** — The validator rate-limits refresh to once per 60s by default. If you set a very short refresh interval, you risk being rate-limited by the AS.

7. **Manually dispatching well-known routes in `run_streaming_with()`** — You don't need custom dispatch for `.well-known` routes. Register them via `.route()` on `LambdaMcpServerBuilder` — `handle_streaming()` checks the route registry before MCP dispatch. Use `run_streaming_with()` only when you need pre-dispatch logic that isn't route-based (e.g., request logging, custom health checks).

## Beyond This Skill

**Middleware details?** → See the `middleware-patterns` skill for `McpMiddleware` trait, error variants, and session injection.

**Lambda deployment?** → See the `lambda-deployment` skill for `LambdaMcpServerBuilder`, cold-start caching, and streaming modes.

**Error handling in tools?** → See the `error-handling-patterns` skill for `McpError` variants and decision tree.

**JWT Validator API?** → See `references/jwt-validator-reference.md` for the full API surface.

**Need a demo Authorization Server?** → See the `authorization-server-patterns` skill for building a standalone demo AS with PKCE, JWKS, and MCP interop.
