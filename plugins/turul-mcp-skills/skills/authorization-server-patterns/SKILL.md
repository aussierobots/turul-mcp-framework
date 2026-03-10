---
name: authorization-server-patterns
description: >
  This skill should be used when the user asks about "authorization server",
  "OAuth AS", "token issuer", "PKCE", "authorization code flow",
  "oauth-authorization-server", "DCR", "dynamic client registration",
  "CIMD", "client metadata", "demo auth server", "token endpoint",
  "authorize endpoint", "JWKS signing key", "access token issuance",
  "refresh token", "authorization-server-patterns", or "build an auth server".
  Covers building a standalone demo OAuth 2.1 Authorization Server
  using standard Rust crates for use alongside Turul MCP Resource Servers.
  Demo-grade only — not production identity infrastructure.
---

# Authorization Server Patterns — Demo / Reference

> **This skill teaches demo-grade patterns only.** The examples here are
> useful for local development, demos, PoCs, and interoperability testing.
> They are **not** production identity infrastructure. For production,
> use a dedicated identity provider (Cognito, Auth0, Keycloak, Ory Hydra, etc.).

Build a standalone OAuth 2.1 Authorization Server (AS) in Rust for use alongside
Turul MCP Resource Servers. Turul provides Resource Server support via `turul-mcp-oauth` —
it does **not** include an Authorization Server framework. This skill fills the gap
for development and testing scenarios where you need a local AS.

## AS vs RS — Where Each Skill Fits

```
                          OAuth 2.1 Ecosystem
                          ────────────────────

  ┌─────────────────────────┐          ┌───────────────────────────────┐
  │  Authorization Server   │          │  Resource Server (MCP)        │
  │  (THIS SKILL)           │          │  (auth-patterns skill)        │
  │                         │          │                               │
  │  Issues tokens          │  JWT     │  Validates tokens             │
  │  Manages clients        │ ──────→  │  Serves RFC 9728 metadata     │
  │  Handles consent        │          │  Injects claims into tools    │
  │  Serves JWKS + AS meta  │          │  Uses turul-mcp-oauth         │
  │                         │          │                               │
  │  Built with: axum,      │          │  Built with: turul-mcp-server │
  │  jsonwebtoken, rsa      │          │                               │
  └─────────────────────────┘          └───────────────────────────────┘
         ↑                                          ↑
         │  Authorization code + PKCE               │  Bearer token
         │                                          │
  ┌──────┴──────────────────────────────────────────┴──┐
  │                      MCP Client                     │
  │  1. Discovers AS via RS's /.well-known/oauth-...    │
  │  2. Obtains token from AS                           │
  │  3. Calls RS with Bearer token                      │
  └─────────────────────────────────────────────────────┘
```

**The two skills are complementary:**
- This skill → build the token issuer (AS)
- `auth-patterns` → validate those tokens in your MCP server (RS)

## Required Endpoints

A minimal demo AS needs four endpoints:

| Endpoint | Method | Purpose | Spec |
|----------|--------|---------|------|
| `/.well-known/oauth-authorization-server` | GET | AS metadata discovery | RFC 8414 |
| `/.well-known/jwks.json` | GET | Public signing keys | RFC 7517 |
| `/authorize` | GET | Authorization + consent (PKCE) | RFC 6749, RFC 7636 |
| `/token` | POST | Token exchange + refresh | RFC 6749 |

**Optional:**

| Endpoint | Method | Purpose | Spec |
|----------|--------|---------|------|
| `/register` | POST | Dynamic Client Registration | RFC 7591 |

## Client Models

An AS needs to know which clients are allowed to request tokens. There are three approaches:

### Pre-Registered Clients (simplest)

Clients are configured in the AS at startup — hardcoded or loaded from config:

```rust
// Demo: hardcoded client registry
let clients = HashMap::from([(
    "demo-mcp-client".to_string(),
    ClientRecord {
        client_id: "demo-mcp-client".to_string(),
        redirect_uris: vec!["http://localhost:3000/callback".to_string()],
        allowed_scopes: vec!["mcp:read".to_string(), "mcp:write".to_string()],
        token_endpoint_auth_method: "none".to_string(), // public client
    },
)]);
```

**When to use:** demos, local development, known client set.

### Dynamic Client Registration (DCR)

Clients register themselves at runtime via `POST /register` (RFC 7591):

```rust
// POST /register
// Request:  { "redirect_uris": ["http://localhost:3000/callback"], ... }
// Response: { "client_id": "generated-uuid", "redirect_uris": [...], ... }
```

**When to use:** interoperability testing, environments where clients aren't known at AS startup.

### Client Identification via Metadata Document (CIMD)

CIMD (draft standard) inverts the model: instead of registering at the AS, clients publish their own metadata document at a well-known URL, and the AS fetches it during authorization. This is the standards-preferred direction for MCP client identification.

**This demo does not implement CIMD.** It's mentioned here so you know the landscape:

```
Client models (simplest → most dynamic):
├─ Pre-registered ── hardcoded or config-loaded (this skill)
├─ DCR ──────────── client self-registers at /register endpoint (this skill, optional)
└─ CIMD ─────────── client publishes metadata, AS fetches it (future standard, not implemented)
```

If you're building a production system, evaluate CIMD support in your chosen identity provider.

## Signing Key Management

The demo AS needs an RSA key pair: private key to sign JWTs, public key served via JWKS.

### Option A: Static Demo Key (recommended for demos)

Load a key from a PEM file. Tokens survive AS restarts:

```rust
use rsa::pkcs8::DecodePrivateKey;
use rsa::RsaPrivateKey;

let pem = std::fs::read_to_string("demo-key.pem")
    .expect("Place a demo RSA private key at demo-key.pem");
let private_key = RsaPrivateKey::from_pkcs8_pem(&pem)
    .expect("Invalid PEM key");
```

Generate a demo key once:
```bash
openssl genrsa -out demo-key.pem 2048
```

### Option B: Generated at Startup

Simpler but **all previously issued tokens become invalid on restart** because the signing key changes:

```rust
use rsa::RsaPrivateKey;
use rand::rngs::OsRng;

let private_key = RsaPrivateKey::new(&mut OsRng, 2048)
    .expect("Failed to generate RSA key");
// WARNING: Restarting the server invalidates all tokens issued
// by the previous instance. Fine for ephemeral demos, problematic
// for any test that spans server restarts.
```

The examples in this skill use **Option A** (static key) by default.

## Authorization Code + PKCE Flow

Public clients (no client secret) use PKCE to prevent authorization code interception:

```
1. Client generates code_verifier (random 43-128 chars)
2. Client computes code_challenge = BASE64URL(SHA256(code_verifier))

3. GET /authorize?
     response_type=code
     &client_id=demo-mcp-client
     &redirect_uri=http://localhost:3000/callback   ← exact match required
     &code_challenge=<challenge>
     &code_challenge_method=S256
     &scope=mcp:read mcp:write
     &resource=https://example.com/mcp              ← target RS
     &state=<random>

4. AS validates client_id, redirect_uri (exact match), scope, resource
5. AS issues authorization code, redirects to redirect_uri?code=<code>&state=<state>
   (Note: the demo example returns JSON with the code instead of a real HTTP
   redirect. A browser-compatible AS would render a consent page and redirect.)

6. POST /token
     grant_type=authorization_code
     &code=<code>
     &redirect_uri=http://localhost:3000/callback
     &client_id=demo-mcp-client
     &code_verifier=<verifier>                      ← proves possession
     &resource=https://example.com/mcp              ← must match /authorize

7. AS validates code, verifies SHA256(code_verifier) == stored challenge,
   verifies resource matches what was authorized
8. AS issues access_token (JWT) + refresh_token (opaque)
```

**See:** `references/oauth-endpoint-responsibilities.md` for the full endpoint flow diagram.

## Redirect URI Allowlisting

**Always use exact-match redirect URIs.** Never allow wildcards, prefix matching, or pattern matching:

```rust
// CORRECT — exact match
fn validate_redirect_uri(client: &ClientRecord, requested: &str) -> bool {
    client.redirect_uris.iter().any(|allowed| allowed == requested)
}

// WRONG — prefix match (allows attacker-controlled subpaths)
fn validate_redirect_uri(client: &ClientRecord, requested: &str) -> bool {
    client.redirect_uris.iter().any(|allowed| requested.starts_with(allowed))
}

// WRONG — no validation at all
fn validate_redirect_uri(_client: &ClientRecord, _requested: &str) -> bool {
    true  // Attacker redirects authorization code to their own server
}
```

For demos, pre-register `http://localhost:<port>/callback` with exact port.

## Token Issuance

### Access Token (JWT)

Short-lived, signed with the AS's private key:

```rust
use jsonwebtoken::{encode, Header, Algorithm, EncodingKey};

let claims = AccessTokenClaims {
    sub: user_id.clone(),
    iss: "https://demo-as.localhost".to_string(),
    aud: resource.clone(),        // Target RS — MUST match RS's expected audience
    scope: granted_scopes.join(" "),
    exp: now + 3600,              // 1 hour
    iat: now,
    jti: Uuid::new_v4().to_string(),
};

let token = encode(
    &Header::new(Algorithm::RS256),
    &claims,
    &EncodingKey::from_rsa_pem(&private_key_pem)?,
)?;
```

**The `aud` claim MUST match what the RS expects.** With `turul-mcp-oauth`'s `oauth_resource_server()`, the expected audience is the resource URL from `ProtectedResourceMetadata`.

### Refresh Token (opaque)

Not a JWT — just a random string stored server-side:

```rust
let refresh_token = Uuid::new_v4().to_string();
// Store: refresh_token → { client_id, user_id, scope, resource, expires_at }
```

### Audience and Scope Validation

The AS must validate both at `/authorize` AND `/token`:

```rust
// At /authorize: validate the requested resource is one this AS serves
fn validate_resource(requested: &str) -> bool {
    KNOWN_RESOURCES.contains(&requested.to_string())
}

// At /authorize: validate requested scopes are allowed for this client
fn validate_scopes(client: &ClientRecord, requested: &[String]) -> bool {
    requested.iter().all(|s| client.allowed_scopes.contains(s))
}

// At /token: MCP spec requires `resource` parameter here too.
// Validate it matches the resource authorized in the original /authorize request.
// Don't just silently use the stored resource — require the client to confirm it.
fn validate_token_resource(requested: &str, authorized: &str) -> bool {
    requested == authorized
}
```

**MCP-specific requirement:** The MCP authorization spec says clients MUST include `resource` in both authorization requests and token requests. The AS must validate that the `resource` at `/token` matches what was authorized at `/authorize`.

## AS Metadata (RFC 8414)

Served at `GET /.well-known/oauth-authorization-server`:

```rust
let metadata = serde_json::json!({
    "issuer": "https://demo-as.localhost",
    "authorization_endpoint": "https://demo-as.localhost/authorize",
    "token_endpoint": "https://demo-as.localhost/token",
    "jwks_uri": "https://demo-as.localhost/.well-known/jwks.json",
    "response_types_supported": ["code"],
    "grant_types_supported": ["authorization_code", "refresh_token"],
    "code_challenge_methods_supported": ["S256"],
    "scopes_supported": ["mcp:read", "mcp:write"],
    "token_endpoint_auth_methods_supported": ["none"],
    // Optional: DCR endpoint
    // "registration_endpoint": "https://demo-as.localhost/register",
});
```

**This is NOT the same as RFC 9728 Protected Resource Metadata.** The RS serves its own metadata at `/.well-known/oauth-protected-resource`. Clients discover the AS by reading the RS metadata first, then fetching the AS metadata.

## MCP Interoperability Notes

### Client Discovery Chain

An MCP client discovers authentication requirements through a chain:

```
1. Client calls RS → gets 401 with WWW-Authenticate header
2. WWW-Authenticate contains resource_metadata URL
3. Client fetches RS metadata (/.well-known/oauth-protected-resource)
   → learns authorization_servers list
4. Client fetches AS metadata (/.well-known/oauth-authorization-server)
   → learns authorization_endpoint, token_endpoint, etc.
5. Client runs PKCE flow against AS
6. Client calls RS with Bearer token
```

### Connecting Demo AS to Turul RS

```rust
// In your Turul MCP RS (auth-patterns skill):
let metadata = ProtectedResourceMetadata::new(
    "https://example.com/mcp",                          // resource
    vec!["https://demo-as.localhost".to_string()],       // points to YOUR demo AS
)?;

let (auth_middleware, routes) = oauth_resource_server(
    metadata,
    "https://demo-as.localhost/.well-known/jwks.json",  // YOUR demo AS's JWKS
)?;
```

The RS validates tokens using JWKS from the demo AS. The audience in issued tokens must match the RS's resource URL exactly.

### What the Demo AS Does NOT Cover

- **OIDC**: No `id_token`, no `/userinfo` endpoint
- **Token introspection** (RFC 7662): RS validates JWTs locally via JWKS
- **Token revocation** (RFC 7009): No revocation endpoint
- **CSRF protection**: PKCE + `state` parameter only (no server-side session)
- **Consent UI**: No HTML consent page — `/authorize` returns JSON with the authorization code instead of issuing an HTTP 302 redirect. A real AS would render a consent page and redirect the browser.

## Common Mistakes

1. **Treating this as production-ready** — These examples use in-memory stores, have no rate limiting, no CSRF protection beyond PKCE, and generate demo keys. Use Cognito, Auth0, Keycloak, or Ory Hydra for production.

2. **Over-broad redirect URIs** — Never allow wildcard or prefix-match redirect URIs. An attacker who controls a subpath of your allowed redirect can intercept authorization codes. Always exact-match.

3. **Trusting requested scopes blindly** — The client requests scopes; the AS must validate them against the client's allowed scopes. Don't echo back whatever the client asks for.

4. **Conflating AS metadata with Protected Resource Metadata** — AS metadata (RFC 8414) at `/.well-known/oauth-authorization-server` describes the AS. Protected Resource Metadata (RFC 9728) at `/.well-known/oauth-protected-resource` describes the RS. They are different documents served by different servers.

5. **Skipping resource/audience validation at /authorize and /token** — The `resource` parameter tells the AS which RS the token is for. The AS must validate this against its known resources and set the `aud` claim accordingly. Without this, tokens issued by your AS could be valid for unintended resource servers. This is especially important for MCP interop where multiple RS instances may share an AS.

6. **Assuming DCR is the only client model** — Pre-registered clients are simpler and sufficient for most demos. DCR adds complexity. CIMD is the emerging standards-preferred direction. Choose the model that fits your scenario.

7. **Generating signing keys at startup without documenting the consequence** — If you generate a new RSA key pair on every startup, all previously issued tokens become invalid. Either use a static demo key or clearly document this behavior.

## Beyond This Skill

**Validating tokens in your MCP server?** → See the `auth-patterns` skill for OAuth 2.1 RS, `JwtValidator`, and `turul-mcp-oauth` middleware.

**Middleware for token extraction?** → See the `middleware-patterns` skill for `McpMiddleware` trait and session injection.

**Lambda deployment with OAuth?** → See the `lambda-deployment` skill for `.route()` registration and `run_streaming()`.

**Production identity providers** → Use Cognito, Auth0, Keycloak, or Ory Hydra instead of these demo patterns.
