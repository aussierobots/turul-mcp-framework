# OAuth 2.1 RS Compliance Fix — v0.3.10 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Fix P0-P2 compliance gaps in `turul-mcp-oauth` so the Resource Server role meets MCP 2025-11-25 Authorization spec requirements.

**Architecture:** Six targeted changes to the `turul-mcp-oauth` crate, two transport-layer header fixes, version bump, and release. No new crates or architectural changes — all fixes are within the existing OAuth RS foundation.

**Tech Stack:** Rust, `url` crate for URI parsing, `jsonwebtoken` for JWT validation, `hyper` for HTTP responses.

---

### Task 1: Add `url` dependency and new error variants

**Files:**
- Modify: `crates/turul-mcp-oauth/Cargo.toml:9-12` (add `url` dependency)
- Modify: `crates/turul-mcp-oauth/src/error.rs:7-41` (add variants)

**Step 1: Add `url` to dependencies**

In `crates/turul-mcp-oauth/Cargo.toml`, add under `[dependencies]`:
```toml
url = { workspace = true }
```

**Step 2: Add error variants**

In `crates/turul-mcp-oauth/src/error.rs`, add two new variants to `OAuthError`:
```rust
/// Resource or authorization server URI is not a valid canonical URI
InvalidResourceUri(String),
/// Configuration error (e.g., wrong number of authorization servers)
InvalidConfiguration(String),
```

Add matching arms to the `Display` impl:
```rust
Self::InvalidResourceUri(msg) => write!(f, "Invalid resource URI: {}", msg),
Self::InvalidConfiguration(msg) => write!(f, "Invalid configuration: {}", msg),
```

**Step 3: Verify it compiles**

Run: `cargo check -p turul-mcp-oauth`
Expected: Compiles with no errors.

**Step 4: Commit**

```
feat(oauth): add url dependency and new OAuthError variants for URI validation
```

---

### Task 2: Canonical URI validation in ProtectedResourceMetadata

**Files:**
- Modify: `crates/turul-mcp-oauth/src/metadata.rs:1-131` (fallible constructor + validation)

**Step 1: Write failing tests for URI validation**

Add these tests to the existing `mod tests` block in `metadata.rs`:

```rust
#[test]
fn test_valid_https_resource() {
    let m = ProtectedResourceMetadata::new(
        "https://example.com/mcp",
        vec!["https://auth.example.com".to_string()],
    );
    assert!(m.is_ok());
}

#[test]
fn test_valid_http_resource() {
    // http allowed for local dev
    let m = ProtectedResourceMetadata::new(
        "http://localhost:8080/mcp",
        vec!["http://localhost:9090".to_string()],
    );
    assert!(m.is_ok());
}

#[test]
fn test_reject_missing_scheme() {
    let m = ProtectedResourceMetadata::new(
        "example.com/mcp",
        vec!["https://auth.example.com".to_string()],
    );
    assert!(m.is_err());
}

#[test]
fn test_reject_fragment_in_resource() {
    let m = ProtectedResourceMetadata::new(
        "https://example.com/mcp#section",
        vec!["https://auth.example.com".to_string()],
    );
    assert!(m.is_err());
}

#[test]
fn test_reject_empty_authorization_servers() {
    let m = ProtectedResourceMetadata::new(
        "https://example.com/mcp",
        vec![],
    );
    assert!(m.is_err());
}

#[test]
fn test_reject_fragment_in_authorization_server() {
    let m = ProtectedResourceMetadata::new(
        "https://example.com/mcp",
        vec!["https://auth.example.com#frag".to_string()],
    );
    assert!(m.is_err());
}

#[test]
fn test_reject_non_http_scheme() {
    let m = ProtectedResourceMetadata::new(
        "ftp://example.com/mcp",
        vec!["https://auth.example.com".to_string()],
    );
    assert!(m.is_err());
}

#[test]
fn test_multiple_valid_authorization_servers() {
    let m = ProtectedResourceMetadata::new(
        "https://example.com/mcp",
        vec![
            "https://auth1.example.com".to_string(),
            "https://auth2.example.com".to_string(),
        ],
    );
    assert!(m.is_ok()); // Multiple AS allowed at metadata level
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p turul-mcp-oauth -- test_valid_https_resource test_reject_missing_scheme`
Expected: FAIL — `new()` returns `Self`, not `Result`.

**Step 3: Add URI validation helper and make constructor fallible**

At the top of `metadata.rs`, add `use url::Url;` and `use crate::error::OAuthError;`.

Add a private validation function:

```rust
/// Validate a URI is canonical per RFC 8707: absolute, http(s) scheme, no fragment.
fn validate_canonical_uri(uri: &str, field_name: &str) -> Result<(), OAuthError> {
    let parsed = Url::parse(uri).map_err(|e| {
        OAuthError::InvalidResourceUri(format!("{}: invalid URI '{}': {}", field_name, uri, e))
    })?;

    match parsed.scheme() {
        "https" | "http" => {}
        other => {
            return Err(OAuthError::InvalidResourceUri(format!(
                "{}: scheme must be https or http, got '{}'",
                field_name, other
            )));
        }
    }

    if parsed.host().is_none() {
        return Err(OAuthError::InvalidResourceUri(format!(
            "{}: authority (host) required",
            field_name
        )));
    }

    if parsed.fragment().is_some() {
        return Err(OAuthError::InvalidResourceUri(format!(
            "{}: fragment not allowed in canonical URI",
            field_name
        )));
    }

    Ok(())
}
```

Change the constructor signature and body:

```rust
pub fn new(
    resource: impl Into<String>,
    authorization_servers: Vec<String>,
) -> Result<Self, OAuthError> {
    let resource = resource.into();

    validate_canonical_uri(&resource, "resource")?;

    if authorization_servers.is_empty() {
        return Err(OAuthError::InvalidConfiguration(
            "authorization_servers must contain at least one entry".to_string(),
        ));
    }
    for uri in &authorization_servers {
        validate_canonical_uri(uri, "authorization_server")?;
    }

    Ok(Self {
        resource,
        authorization_servers,
        jwks_uri: None,
        scopes_supported: None,
        bearer_methods_supported: None,
        resource_documentation: None,
        resource_signing_alg_values_supported: None,
    })
}
```

**Step 4: Fix all existing callers of `ProtectedResourceMetadata::new()`**

Every existing call site that used `new()` infallibly now needs `.unwrap()` (tests) or `?` (production). Known call sites:

- `crates/turul-mcp-oauth/src/metadata.rs` — all existing tests: add `.unwrap()`
- `crates/turul-mcp-oauth/src/middleware.rs` — tests at lines 106, 126: add `.unwrap()`
- `crates/turul-mcp-oauth/src/well_known.rs` — tests at lines 53, 89: add `.unwrap()`
- `crates/turul-mcp-oauth/src/lib.rs` — doc example: update
- `examples/oauth-resource-server/src/main.rs` — line 112: add `?` or `.expect()`

Also fix the `test_resource_path_extraction` and `test_well_known_paths` tests — several use query strings and fragments in resource URIs. These now need to be valid URIs (no fragment). Tests that specifically tested fragment stripping in `resource_path()` should be removed or changed to test `validate_canonical_uri` rejection instead.

**Step 5: Run all tests**

Run: `cargo test -p turul-mcp-oauth`
Expected: All pass (new + existing).

**Step 6: Commit**

```
feat(oauth): validate canonical URIs in ProtectedResourceMetadata::new()

ProtectedResourceMetadata::new() is now fallible. Validates resource and
authorization_server URIs using url::Url: scheme must be http/https,
authority required, fragments forbidden. Empty AS list rejected.

BREAKING: new() returns Result<Self, OAuthError>.
```

---

### Task 3: Required audience in JwtValidator

**Files:**
- Modify: `crates/turul-mcp-oauth/src/jwt.rs:91-184` (constructor + validate)

**Step 1: Write failing test for audience-always-validated**

Add to the existing `mod tests` in `jwt.rs`:

```rust
// Audience is always validated — no way to construct without it
#[tokio::test]
async fn test_audience_always_validated() {
    let (enc_key, dec_key) = generate_rsa_keys();
    // Validator with audience set (now required)
    let validator =
        JwtValidator::test_with_key_async(dec_key, "test-kid", Algorithm::RS256).await;

    // Token with wrong audience
    let mut claims = valid_claims();
    claims.aud = serde_json::json!("https://wrong.example.com");
    let token = create_test_token(&enc_key, "test-kid", &claims);

    let result = validator.validate(&token).await;
    assert!(
        matches!(result, Err(OAuthError::InvalidAudience)),
        "Audience validation must be enforced by default, got: {:?}",
        result
    );
}
```

**Step 2: Run to verify it fails**

Run: `cargo test -p turul-mcp-oauth -- test_audience_always_validated`
Expected: FAIL — current default has `validate_aud = false`.

**Step 3: Change JwtValidator::new() to require audience**

```rust
pub fn new(jwks_uri: impl Into<String>, audience: impl Into<String>) -> Self {
    Self {
        jwks_uri: jwks_uri.into(),
        cached_jwks: RwLock::new(None),
        allowed_algorithms: vec![Algorithm::RS256, Algorithm::ES256],
        issuer: None,
        audience: Some(audience.into()),
        refresh_interval: Duration::from_secs(60),
        http_client: reqwest::Client::new(),
    }
}
```

Remove the `with_audience()` builder method (audience is always set via `new()`).

**Step 4: Fix test helper `test_with_key_async`**

Update to pass a default test audience:

```rust
pub(crate) async fn test_with_key_async(
    decoding_key: DecodingKey,
    kid: &str,
    alg: Algorithm,
) -> Self {
    let mut keys = HashMap::new();
    keys.insert(kid.to_string(), (decoding_key, alg));

    let validator = Self::new("http://localhost/jwks", "https://example.com/mcp");
    let mut cache = validator.cached_jwks.write().await;
    *cache = Some(CachedJwks {
        keys,
        fetched_at: Instant::now(),
        last_refresh_at: Instant::now(),
    });
    drop(cache);

    validator
}
```

**Step 5: Fix existing tests that relied on no audience validation**

- `test_valid_jwt_accepted`: update `valid_claims()` so `aud` matches `"https://example.com/mcp"` — it already does if we check.
- `test_wrong_audience_rejected`: uses `with_audience()` → change to construct with the "wrong" audience in `new()`, or set `validator.audience` directly since we're in `#[cfg(test)]`.
- `test_rs256_es256_both_accepted`: uses `JwtValidator::new(url)` → add audience arg.

For `test_wrong_audience_rejected`, the simplest approach: construct the validator with the wrong audience directly:

```rust
#[tokio::test]
async fn test_wrong_audience_rejected() {
    let (enc_key, dec_key) = generate_rsa_keys();
    let validator = {
        let mut v = JwtValidator::test_with_key_async(dec_key, "test-kid", Algorithm::RS256).await;
        v.audience = Some("https://other.example.com".to_string());
        v
    };
    // ... rest unchanged
}
```

For `test_wrong_issuer_rejected`, same pattern with `v.issuer = Some(...)`.

**Step 6: Run all jwt tests**

Run: `cargo test -p turul-mcp-oauth -- jwt`
Expected: All pass.

**Step 7: Commit**

```
feat(oauth): require audience parameter in JwtValidator::new()

JwtValidator::new() now takes (jwks_uri, audience). Audience validation
is always enabled — tokens without the expected audience are rejected
with OAuthError::InvalidAudience. Removes with_audience() builder.

BREAKING: JwtValidator::new() signature changed.
```

---

### Task 4: Enforce single-AS issuer policy in oauth_resource_server()

**Files:**
- Modify: `crates/turul-mcp-oauth/src/lib.rs:75-92` (convenience function)

**Step 1: Write failing tests**

Add a `#[cfg(test)]` module to `lib.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oauth_resource_server_single_as_ok() {
        let metadata = ProtectedResourceMetadata::new(
            "https://example.com/mcp",
            vec!["https://auth.example.com".to_string()],
        )
        .unwrap();

        let result = oauth_resource_server(
            metadata,
            "https://auth.example.com/.well-known/jwks.json",
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_oauth_resource_server_multiple_as_rejected() {
        let metadata = ProtectedResourceMetadata::new(
            "https://example.com/mcp",
            vec![
                "https://auth1.example.com".to_string(),
                "https://auth2.example.com".to_string(),
            ],
        )
        .unwrap();

        let result = oauth_resource_server(
            metadata,
            "https://auth.example.com/.well-known/jwks.json",
        );
        assert!(result.is_err());
    }
}
```

**Step 2: Run to verify they fail**

Run: `cargo test -p turul-mcp-oauth -- tests::test_oauth_resource_server`
Expected: FAIL — current function doesn't return `Result` and doesn't check AS count.

**Step 3: Update oauth_resource_server()**

```rust
pub fn oauth_resource_server(
    metadata: ProtectedResourceMetadata,
    jwks_uri: &str,
) -> Result<(Arc<OAuthResourceMiddleware>, Vec<RouteEntry>), OAuthError> {
    if metadata.authorization_servers.len() != 1 {
        return Err(OAuthError::InvalidConfiguration(format!(
            "oauth_resource_server requires exactly one authorization server, got {}; \
             for multi-AS deployments, construct JwtValidator and OAuthResourceMiddleware manually",
            metadata.authorization_servers.len()
        )));
    }

    let audience = &metadata.resource;
    let issuer = &metadata.authorization_servers[0];

    let validator = Arc::new(
        JwtValidator::new(jwks_uri, audience).with_issuer(issuer),
    );
    let middleware = Arc::new(OAuthResourceMiddleware::new(validator, metadata.clone()));
    let handler: Arc<dyn turul_http_mcp_server::routes::RouteHandler> =
        Arc::new(WellKnownOAuthHandler::new(&metadata));

    let routes: Vec<RouteEntry> = metadata
        .well_known_paths()
        .into_iter()
        .map(|path| (path, handler.clone()))
        .collect();

    Ok((middleware, routes))
}
```

**Step 4: Update the doc examples in lib.rs**

Update the `//!` module docs and the `/// ```rust,ignore` example to show `?` on the call.

**Step 5: Fix the example server**

In `examples/oauth-resource-server/src/main.rs:116-117`, change:
```rust
let (auth_middleware, routes) =
    turul_mcp_oauth::oauth_resource_server(metadata, &args.jwks_uri)?;
```

Also fix the `ProtectedResourceMetadata::new()` call at line 112 to add `?`.

**Step 6: Run tests and check example compiles**

Run: `cargo test -p turul-mcp-oauth && cargo check -p oauth-resource-server`
Expected: All pass, example compiles.

**Step 7: Commit**

```
feat(oauth): enforce single-AS issuer policy in oauth_resource_server()

oauth_resource_server() now returns Result and rejects metadata with
multiple authorization servers. Single AS is auto-wired as issuer.
Audience is derived from metadata.resource.

BREAKING: oauth_resource_server() returns Result.
```

---

### Task 5: Add scope to WWW-Authenticate challenges

**Files:**
- Modify: `crates/turul-mcp-oauth/src/middleware.rs:46-88` (before_dispatch)

**Step 1: Write failing test**

Add to `middleware.rs` tests:

```rust
#[tokio::test]
async fn test_401_includes_scope_when_configured() {
    let metadata = ProtectedResourceMetadata::new(
        "https://example.com/mcp",
        vec!["https://auth.example.com".to_string()],
    )
    .unwrap()
    .with_scopes(vec!["mcp:read".to_string(), "mcp:write".to_string()]);

    let middleware = OAuthResourceMiddleware::new(
        Arc::new(JwtValidator::new("http://localhost/jwks", "https://example.com/mcp")),
        metadata,
    );

    let mut ctx = RequestContext::new("tools/call", None);
    let mut injection = SessionInjection::new();

    let result = middleware
        .before_dispatch(&mut ctx, None, &mut injection)
        .await;

    match result {
        Err(MiddlewareError::HttpChallenge {
            www_authenticate, ..
        }) => {
            assert!(
                www_authenticate.contains("scope=\"mcp:read mcp:write\""),
                "Expected scope in WWW-Authenticate, got: {}",
                www_authenticate
            );
        }
        other => panic!("Expected HttpChallenge, got {:?}", other),
    }
}

#[tokio::test]
async fn test_401_omits_scope_when_not_configured() {
    let metadata = ProtectedResourceMetadata::new(
        "https://example.com/mcp",
        vec!["https://auth.example.com".to_string()],
    )
    .unwrap();

    let middleware = OAuthResourceMiddleware::new(
        Arc::new(JwtValidator::new("http://localhost/jwks", "https://example.com/mcp")),
        metadata,
    );

    let mut ctx = RequestContext::new("tools/call", None);
    let mut injection = SessionInjection::new();

    let result = middleware
        .before_dispatch(&mut ctx, None, &mut injection)
        .await;

    match result {
        Err(MiddlewareError::HttpChallenge {
            www_authenticate, ..
        }) => {
            assert!(
                !www_authenticate.contains("scope="),
                "Should not include scope when not configured, got: {}",
                www_authenticate
            );
        }
        other => panic!("Expected HttpChallenge, got {:?}", other),
    }
}
```

**Step 2: Run to verify they fail**

Run: `cargo test -p turul-mcp-oauth -- test_401_includes_scope`
Expected: FAIL — scope not in challenge header.

**Step 3: Update before_dispatch to include scope**

In `middleware.rs`, create a helper to build the challenge string:

```rust
fn build_challenge(&self, error_params: &str) -> String {
    let scope_param = self
        .metadata
        .scopes_supported
        .as_ref()
        .map(|scopes| format!(", scope=\"{}\"", scopes.join(" ")))
        .unwrap_or_default();

    format!(
        "Bearer realm=\"mcp\", resource_metadata=\"{}\"{}{}",
        self.metadata.metadata_url(),
        scope_param,
        error_params,
    )
}
```

Then in `before_dispatch`, replace the two inline `format!` calls:

Missing token:
```rust
let token = ctx.bearer_token().ok_or_else(|| {
    MiddlewareError::http_challenge(401, self.build_challenge(""))
})?;
```

Invalid token:
```rust
let claims = self
    .jwt_validator
    .validate(token)
    .await
    .map_err(|e| {
        debug!("Token validation failed: {}", e);
        MiddlewareError::http_challenge(
            401,
            self.build_challenge(&format!(
                ", error=\"invalid_token\", error_description=\"{}\"",
                e
            )),
        )
    })?;
```

**Step 4: Run tests**

Run: `cargo test -p turul-mcp-oauth`
Expected: All pass.

**Step 5: Commit**

```
feat(oauth): include scope in WWW-Authenticate when scopes configured

Per MCP spec (SHOULD), 401 challenges now include scope="scope1 scope2"
when scopes_supported is set on ProtectedResourceMetadata.
```

---

### Task 6: Cache-Control: no-store on challenge responses

**Files:**
- Modify: `crates/turul-http-mcp-server/src/streamable_http.rs:1939-1950`
- Modify: `crates/turul-http-mcp-server/src/session_handler.rs:405-410`

**Step 1: Add Cache-Control header to streamable_http challenge builder**

In `build_http_challenge_response()` at `streamable_http.rs:1939`, add after the `WWW-Authenticate` header:
```rust
.header("Cache-Control", "no-store")
```

**Step 2: Add Cache-Control header to session_handler challenge builder**

In `session_handler.rs:407`, add after the `WWW-Authenticate` header:
```rust
.header("Cache-Control", "no-store")
```

**Step 3: Add unit tests**

Add to `streamable_http.rs` tests (the existing `mod tests` block):

```rust
#[test]
fn test_challenge_response_has_cache_control() {
    let context = StreamableHttpContext {
        protocol_version: turul_mcp_protocol_2025_11_25::version::ProtocolVersion::V2025_11_25,
        headers: std::collections::HashMap::new(),
        session_id: None,
    };

    let response = build_http_challenge_response(
        401,
        "Bearer realm=\"mcp\"",
        None,
        &context,
    );

    assert_eq!(
        response.headers().get("Cache-Control").unwrap(),
        "no-store"
    );
}
```

**Step 4: Run tests**

Run: `cargo test -p turul-http-mcp-server -- test_challenge_response_has_cache_control`
Expected: PASS.

**Step 5: Commit**

```
feat(oauth): add Cache-Control: no-store to auth challenge responses

Per OAuth 2.1 Section 5.3, error responses SHOULD include
Cache-Control: no-store to prevent caching of auth challenges.
```

---

### Task 7: Update ADR-021

**Files:**
- Modify: `docs/adr/021-oauth-resource-server-architecture.md`

**Step 1: Add a "v0.3.10 Compliance Amendments" section**

Append before the `## Consequences` section:

```markdown
### v0.3.10 Compliance Amendments

**D7: Audience Validation Required by Default**

`JwtValidator::new(jwks_uri, audience)` — audience is a required parameter. The
MCP spec says servers MUST validate that tokens were specifically issued for them.
The `resource` URI from `ProtectedResourceMetadata` is used as the expected audience.

**D8: Single-AS Issuer Enforcement**

`oauth_resource_server()` requires exactly one authorization server in metadata
and automatically wires it as the expected JWT issuer. Multi-AS deployments must
construct `JwtValidator` and `OAuthResourceMiddleware` manually.

**D9: Canonical URI Validation**

`ProtectedResourceMetadata::new()` validates both `resource` and all
`authorization_servers` URIs using `url::Url`: scheme must be http/https,
authority required, fragments forbidden per RFC 8707.

**D10: Scope in WWW-Authenticate**

When `scopes_supported` is configured, the middleware includes
`scope="scope1 scope2"` in `WWW-Authenticate` challenges per RFC 6750 §3.

**D11: Cache-Control on Challenge Responses**

All 401/403 challenge responses include `Cache-Control: no-store` per
OAuth 2.1 §5.3.
```

**Step 2: Commit**

```
docs: update ADR-021 with v0.3.10 compliance amendments
```

---

### Task 8: Version bump to 0.3.10

**Files:**
- Modify: `Cargo.toml:110` (workspace version)
- Modify: `Cargo.toml:146-157` (all internal dep versions)
- Modify: `crates/turul-mcp-oauth/Cargo.toml:10-11` (internal dep versions)
- Modify: `examples/oauth-resource-server/src/main.rs:125` (.version string)

**Step 1: Update workspace version**

In root `Cargo.toml`, change:
```toml
version = "0.3.10"
```

**Step 2: Update all internal dependency versions**

In root `Cargo.toml` `[workspace.dependencies]`, change all `version = "0.3.9"` to `version = "0.3.10"`.

In `crates/turul-mcp-oauth/Cargo.toml`, update:
```toml
turul-http-mcp-server = { version = "0.3.10", path = "../turul-http-mcp-server" }
turul-mcp-session-storage = { version = "0.3.10", path = "../turul-mcp-session-storage", default-features = false }
```

**Step 3: Update example server version string**

In `examples/oauth-resource-server/src/main.rs`:
```rust
.version("0.3.10")
```

**Step 4: Verify workspace builds**

Run: `cargo check --workspace`
Expected: Clean compile.

**Step 5: Commit**

```
chore: bump workspace version to 0.3.10
```

---

### Task 9: Update CHANGELOG.md

**Files:**
- Modify: `CHANGELOG.md`

**Step 1: Add v0.3.10 entry**

Add at the top of the changelog (after the header), before the v0.3.9 entry:

```markdown
## [0.3.10] - 2026-03-07

### Breaking Changes

- `ProtectedResourceMetadata::new()` is now fallible (`-> Result<Self, OAuthError>`), validating resource and authorization server URIs as canonical per RFC 8707
- `JwtValidator::new()` now requires an `audience` parameter — audience validation is always enabled
- `oauth_resource_server()` now returns `Result` and rejects metadata with multiple authorization servers
- `JwtValidator::with_audience()` removed (audience is set via `new()`)

### Security

- **[P0]** Audience validation enforced by default in JwtValidator (MCP spec MUST)
- **[P1]** Issuer auto-wired from authorization server in oauth_resource_server()
- **[P1]** WWW-Authenticate challenges include `scope` when scopes_supported is configured (RFC 6750 §3)
- **[P2]** Canonical URI validation for resource and authorization_server URIs (RFC 8707)
- **[P2]** Cache-Control: no-store on all 401/403 challenge responses (OAuth 2.1 §5.3)
- **[P2]** Single-AS enforcement in oauth_resource_server() prevents silent [0] fallback
```

Update the comparison link at the bottom if one exists.

**Step 2: Commit**

```
docs: add CHANGELOG entry for v0.3.10
```

---

### Task 10: Full test suite and dry publish

**Step 1: Run full workspace tests**

Run: `cargo test --workspace`
Expected: All pass (0 failures).

**Step 2: Run clippy**

Run: `cargo clippy --workspace`
Expected: No warnings.

**Step 3: Dry publish (oauth crate)**

Run: `cargo publish -p turul-mcp-oauth --dry-run`
Expected: Passes validation.

**Step 4: Dry publish (all publishable crates in dependency order)**

Run each in order:
```bash
cargo publish -p turul-mcp-json-rpc-server --dry-run
cargo publish -p turul-mcp-protocol-2025-06-18 --dry-run
cargo publish -p turul-mcp-protocol-2025-11-25 --dry-run
cargo publish -p turul-mcp-protocol --dry-run
cargo publish -p turul-mcp-builders --dry-run
cargo publish -p turul-mcp-session-storage --dry-run
cargo publish -p turul-mcp-task-storage --dry-run
cargo publish -p turul-http-mcp-server --dry-run
cargo publish -p turul-mcp-derive --dry-run
cargo publish -p turul-mcp-server --dry-run
cargo publish -p turul-mcp-client --dry-run
cargo publish -p turul-mcp-aws-lambda --dry-run
cargo publish -p turul-mcp-oauth --dry-run
```

Expected: All pass.

**Step 5: Final commit (if any remaining changes)**

```
chore: v0.3.10 release preparation
```
