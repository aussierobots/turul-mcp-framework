# JwtValidator API Reference

API reference for `turul_mcp_oauth::JwtValidator`. The public API (construction, builder methods, `validate()`) is stable. Internal behaviors like caching strategy, refresh rate limiting, and key type support reflect the current implementation and may evolve.

## Construction

```rust
use turul_mcp_oauth::JwtValidator;

let validator = JwtValidator::new(jwks_uri, audience);
```

**Required parameters:**
- `jwks_uri: &str` — URL of the JWKS endpoint (e.g., `https://auth.example.com/.well-known/jwks.json`)
- `audience: &str` — Required audience claim value. **No opt-out** — audience validation is always enforced.

## Builder Methods

| Method | Signature | Default | Notes |
|---|---|---|---|
| `.with_issuer()` | `fn with_issuer(self, issuer: &str) -> Self` | None | When set, `iss` claim is validated against this value. |
| `.with_algorithms()` | `fn with_algorithms(self, algs: Vec<Algorithm>) -> Self` | `[RS256, ES256]` | Allowed JWT signing algorithms. |
| `.with_refresh_interval()` | `fn with_refresh_interval(self, interval: Duration) -> Self` | 60 seconds | Minimum time between JWKS endpoint refreshes. |

## Validation

```rust
let claims: TokenClaims = validator.validate(token).await?;
```

**Parameters:**
- `token: &str` — Raw JWT string (without `Bearer` prefix)

**Returns:** `Result<TokenClaims, OAuthError>`

## Validation Rules

| Check | Always | Configurable |
|---|---|---|
| Token expiration (`exp`) | Yes | No |
| Audience (`aud`) | Yes | No (required in constructor) |
| Issuer (`iss`) | Only if `.with_issuer()` called | Yes |
| Algorithm | Yes (against allowed list) | Yes (`.with_algorithms()`) |
| Signature | Yes (against JWKS public key) | No |

## JWKS Caching Behavior

The following describes the current caching implementation. The public contract is: keys are cached, refreshed on cache miss, and rate-limited to prevent endpoint hammering. The exact flow below reflects current internals.

```
validate(token) called
    │
    ▼
Extract `kid` from JWT header
    │
    ▼
Lookup kid in cache (read lock)
    ├─ Found ───────→ Use cached key → validate
    └─ Not found ──→ Check rate limit
                         ├─ < 60s since last refresh → KeyNotFound error
                         └─ >= 60s → Refresh from JWKS URI
                              │
                              ▼
                         Fetch HTTP GET jwks_uri
                              │
                              ▼
                         Parse JWKS, cache all keys by kid
                              │
                              ▼
                         Lookup kid again
                              ├─ Found ───→ validate
                              └─ Not found → KeyNotFound error
```

**Key behaviors:**
- Cache is per-`JwtValidator` instance (in-memory `HashMap` behind `RwLock`)
- Refresh fetches ALL keys, not just the missing one
- Rate limiting prevents AS endpoint hammering during key rotation
- On Lambda, cache lives in the `OnceCell` handler — shared across warm invocations

## Supported Key Types

| JWK `kty` | Supported Algorithms | Key Components |
|---|---|---|
| `RSA` | RS256, RS384, RS512 | `n` (modulus), `e` (exponent) |
| `EC` | ES256 (P-256), ES384 (P-384) | `x`, `y`, `crv` (curve name) |

**Algorithm selection for EC keys:**
- `crv: "P-256"` → ES256
- `crv: "P-384"` → ES384
- Other curves → warning logged, key skipped

**Algorithm cross-check:** The algorithm advertised in the JWKS key entry is compared against the algorithm in the JWT header. Mismatches produce `UnsupportedAlgorithm`.

## TokenClaims

```rust
pub struct TokenClaims {
    pub sub: String,                               // Subject (user identifier)
    pub iss: String,                               // Issuer
    pub aud: serde_json::Value,                    // Audience (string or array)
    pub exp: u64,                                  // Expiration (Unix timestamp)
    pub iat: u64,                                  // Issued at (Unix timestamp)
    pub scope: Option<String>,                     // Space-separated scopes
    pub extra: HashMap<String, serde_json::Value>, // All other claims
}
```

**`aud` field:** Can be a JSON string (`"my-audience"`) or JSON array (`["aud1", "aud2"]`). The validator checks the required audience against both forms.

**`extra` field:** Captures all non-standard claims via `#[serde(flatten)]`. Custom claims like `email`, `name`, `roles`, etc. appear here.

## Error Types

| OAuthError Variant | Cause | Retryable? |
|---|---|---|
| `InvalidToken(String)` | Signature invalid, claims malformed | No (token issue) |
| `TokenExpired` | `exp` claim in the past | No (get new token) |
| `InvalidAudience` | `aud` doesn't match required value | No (wrong audience) |
| `InvalidIssuer` | `iss` doesn't match configured value | No (wrong issuer) |
| `UnsupportedAlgorithm(String)` | JWT alg not in allowed list | No (AS config issue) |
| `JwksFetchError(String)` | Network error fetching JWKS | Yes (transient) |
| `KeyNotFound(String)` | `kid` not in JWKS after refresh | Maybe (key rotation lag) |
| `DecodingError(String)` | JWT header can't be decoded | No (malformed token) |

## Usage with OAuthResourceMiddleware

```rust
use std::sync::Arc;
use turul_mcp_oauth::{JwtValidator, OAuthResourceMiddleware, ProtectedResourceMetadata};

let validator = Arc::new(JwtValidator::new(jwks_uri, audience));
let metadata = ProtectedResourceMetadata::new(resource, auth_servers)?;
let middleware = Arc::new(OAuthResourceMiddleware::new(validator, metadata));

// Register as McpMiddleware
let server = McpServer::builder()
    .middleware(middleware)
    .build()?;
```

The middleware calls `validator.validate(bearer_token)` in `before_dispatch()`, then injects `TokenClaims` into request extensions at key `"__turul_internal.auth_claims"`.

## Usage with oauth_resource_server()

```rust
use turul_mcp_oauth::{ProtectedResourceMetadata, oauth_resource_server};

let metadata = ProtectedResourceMetadata::new(resource, auth_servers)?;
let (middleware, routes) = oauth_resource_server(metadata, jwks_uri)?;
```

The convenience function creates a `JwtValidator` with:
- `audience` = `metadata.resource` (the resource URL)
- `issuer` = `metadata.authorization_servers[0]` (first AS only)
- Default algorithms (RS256, ES256)
- Default refresh interval (60s)

For different audience, multiple issuers, or custom algorithms — construct `JwtValidator` manually.
