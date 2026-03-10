# OAuth Endpoint Responsibilities & Flow

Reference for the demo Authorization Server endpoints, token lifecycle, and MCP interoperability.

> **Demo-grade reference.** This describes the endpoints implemented in the demo examples,
> not a complete OAuth 2.1 implementation.

## Authorization Code + PKCE Flow

```
  MCP Client                    Demo AS                     Turul MCP RS
  ──────────                    ───────                     ────────────
       │                            │                            │
       │  1. Call RS endpoint       │                            │
       │─────────────────────────────────────────────────────────→
       │                            │                            │
       │  2. 401 + WWW-Authenticate │     (no Bearer token)      │
       │←─────────────────────────────────────────────────────────│
       │     resource_metadata="https://rs/.well-known/..."      │
       │                            │                            │
       │  3. GET RS metadata        │                            │
       │─────────────────────────────────────────────────────────→
       │  ← { authorization_servers: ["https://as"] }            │
       │                            │                            │
       │  4. GET AS metadata        │                            │
       │───────────────────────────→│                            │
       │  ← { authorization_endpoint, token_endpoint, ... }     │
       │                            │                            │
       │  5. Generate PKCE:         │                            │
       │     code_verifier (random) │                            │
       │     code_challenge =       │                            │
       │       BASE64URL(SHA256(v)) │                            │
       │                            │                            │
       │  6. GET /authorize?        │                            │
       │     response_type=code     │                            │
       │     &client_id=...         │                            │
       │     &redirect_uri=...      │                            │
       │     &code_challenge=...    │                            │
       │     &code_challenge_method │                            │
       │       =S256                │                            │
       │     &scope=mcp:read        │                            │
       │     &resource=https://rs   │                            │
       │     &state=<random>        │                            │
       │───────────────────────────→│                            │
       │                            │                            │
       │  7. AS validates:          │                            │
       │     - client_id known      │                            │
       │     - redirect_uri exact   │                            │
       │     - scope allowed        │                            │
       │     - resource known       │                            │
       │     (demo: auto-approve)   │                            │
       │                            │                            │
       │  8. Redirect with code     │                            │
       │←───────────────────────────│                            │
       │     ?code=<code>&state=... │                            │
       │                            │                            │
       │  9. POST /token            │                            │
       │     grant_type=            │                            │
       │       authorization_code   │                            │
       │     &code=<code>           │                            │
       │     &redirect_uri=...      │                            │
       │     &client_id=...         │                            │
       │     &code_verifier=<v>     │                            │
       │     &resource=https://rs   │  ← MCP: required at /token │
       │───────────────────────────→│                            │
       │                            │                            │
       │  10. AS validates:         │                            │
       │     - code exists, unused  │                            │
       │     - client_id matches    │                            │
       │     - redirect_uri matches │                            │
       │     - resource matches     │                            │
       │       authorized resource  │                            │
       │     - SHA256(verifier) ==  │                            │
       │       stored challenge     │                            │
       │                            │                            │
       │  11. Token response        │                            │
       │←───────────────────────────│                            │
       │     { access_token (JWT),  │                            │
       │       refresh_token,       │                            │
       │       expires_in, scope }  │                            │
       │                            │                            │
       │  12. Call RS with Bearer   │                            │
       │─────────────────────────────────────────────────────────→
       │     Authorization: Bearer <JWT>                         │
       │                            │                            │
       │  13. RS validates JWT:     │                            │
       │     - signature via JWKS   │                            │
       │     - aud matches resource │                            │
       │     - iss matches AS       │                            │
       │     - exp not passed       │                            │
       │                            │                            │
       │  14. MCP response          │                            │
       │←─────────────────────────────────────────────────────────│
       │                            │                            │
```

## Endpoint Responsibilities

### GET /.well-known/oauth-authorization-server

| Responsibility | Detail |
|----------------|--------|
| **Purpose** | AS metadata discovery (RFC 8414) |
| **Validates** | Nothing — static document |
| **Returns** | JSON with issuer, endpoints, supported features |
| **Cache** | Can be cached aggressively (static per deployment) |

**Not to be confused with** `/.well-known/oauth-protected-resource` (RFC 9728), which is served by the Resource Server.

### GET /.well-known/jwks.json

| Responsibility | Detail |
|----------------|--------|
| **Purpose** | Public signing keys for token verification |
| **Validates** | Nothing — static key set |
| **Returns** | JWK Set with RSA/EC public keys |
| **Cache** | Cache with revalidation (keys rotate) |
| **Consumed by** | RS's `JwtValidator` (fetches and caches automatically) |

### GET /authorize

| Responsibility | Detail |
|----------------|--------|
| **Purpose** | Start authorization code flow |
| **Validates** | client_id, redirect_uri (exact match), scope (subset of allowed), resource (known RS), PKCE challenge |
| **Returns** | Redirect to redirect_uri with authorization code. (Demo example returns JSON instead of a real HTTP redirect — see note below.) |
| **Security** | PKCE required for public clients; state parameter for CSRF |

**Demo note:** The example `/authorize` handler returns `200 OK` JSON with a `redirect_to` URL and `code` field, rather than issuing an HTTP 302 redirect. This is intentional — the demo has no consent UI. A browser-compatible AS would render an HTML consent page, then redirect with `302 Location: redirect_uri?code=...&state=...`.

**Critical validations:**
- `redirect_uri` must **exactly match** a registered URI — no prefix, no wildcard
- `resource` must be a known RS — don't issue tokens for arbitrary audiences
- `scope` must be a subset of the client's allowed scopes — don't echo requests blindly

### POST /token

| Responsibility | Detail |
|----------------|--------|
| **Purpose** | Exchange code for tokens, or refresh |
| **Validates** | (code exchange) code, client_id, redirect_uri, PKCE verifier, resource (must match /authorize); (refresh) refresh_token validity |
| **Returns** | access_token (JWT), refresh_token (opaque), expires_in, scope |
| **Security** | Authorization codes are single-use; refresh tokens rotate on use |

### POST /register (optional, DCR)

| Responsibility | Detail |
|----------------|--------|
| **Purpose** | Dynamic Client Registration (RFC 7591) |
| **Validates** | redirect_uris (HTTP(S) only), auth method (public only in demo) |
| **Returns** | Generated client_id + granted metadata |
| **Security** | Demo has open registration — production would require authentication |

## Token Lifecycle

### Access Token (JWT)

```
Issued by AS          →  Validated by RS          →  Expires
 ┌──────────────┐        ┌──────────────────┐
 │ sub: user-id  │       │ Signature check   │
 │ iss: AS URL   │──JWT──│ aud == resource   │──→ Accept / Reject
 │ aud: RS URL   │       │ iss == expected   │
 │ scope: ...    │       │ exp > now         │
 │ exp: now+3600 │       └──────────────────┘
 │ kid: key-id   │
 └──────────────┘
```

- **Lifetime**: Short (1 hour in demo)
- **Audience**: Must match RS's expected audience exactly
- **Signature**: RS fetches JWKS from AS, matches by `kid`

### Refresh Token (opaque)

```
Issued alongside      →  Exchanged at AS          →  New tokens
access token               POST /token
                            grant_type=refresh_token
                            &refresh_token=<token>
```

- **Lifetime**: Longer (24 hours in demo)
- **Rotation**: Old token consumed, new one issued (prevents replay)
- **Not a JWT**: Just a random UUID stored server-side

## Client Models

| Model | Registration | Discovery | Demo support |
|-------|-------------|-----------|--------------|
| **Pre-registered** | At AS startup (config/hardcoded) | Client knows its client_id | Yes (default) |
| **DCR** | Runtime via `POST /register` (RFC 7591) | Client receives client_id from AS | Yes (optional) |
| **CIMD** | Client publishes metadata document; AS fetches it | AS discovers client metadata | No (future standard) |

Pre-registered is sufficient for most demo and local development scenarios. DCR is useful when testing with clients that aren't known at AS startup. CIMD is the emerging standards-preferred direction but is not implemented in the demo.

## Security Boundaries (demo vs production)

| Concern | Demo behavior | Production requirement |
|---------|--------------|----------------------|
| **Key management** | Static PEM file or ephemeral | HSM / KMS / Vault |
| **Client storage** | In-memory HashMap | Database with encryption |
| **Code storage** | In-memory, no cleanup | Time-limited, persistent, single-use enforced |
| **Consent** | Auto-approved (JSON response) | User-facing consent UI |
| **Rate limiting** | None | Per-client, per-IP |
| **Token revocation** | Not implemented | RFC 7009 endpoint |
| **Introspection** | Not implemented | RFC 7662 endpoint |
| **CSRF** | PKCE + state only | Full session-based CSRF |
| **Logging/audit** | stderr | Structured audit log |
| **OIDC** | Not implemented | id_token, /userinfo |

## Connecting to Turul RS

The demo AS issues tokens that `turul-mcp-oauth` validates. The connection points:

1. **JWKS**: RS fetches `http://localhost:9000/.well-known/jwks.json` to get signing keys
2. **Audience**: Tokens have `aud` matching the RS's `ProtectedResourceMetadata.resource`
3. **Issuer**: Tokens have `iss` matching the first entry in RS's `authorization_servers`

```rust
// In your Turul MCP RS:
let metadata = ProtectedResourceMetadata::new(
    "http://localhost:8080/mcp",                        // Must match token's aud
    vec!["http://localhost:9000".to_string()],           // Must match token's iss
)?;

let (auth_middleware, routes) = oauth_resource_server(
    metadata,
    "http://localhost:9000/.well-known/jwks.json",      // Demo AS's JWKS
)?;
```

The RS doesn't need to know anything about the AS's implementation — it validates tokens purely via JWKS signature verification and claim checks.
