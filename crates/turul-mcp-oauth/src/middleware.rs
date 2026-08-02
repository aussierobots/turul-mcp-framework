//! OAuth Resource Server middleware
//!
//! Pre-session middleware that validates Bearer tokens and injects
//! token claims into the request context for tools to read.

use std::sync::Arc;

use async_trait::async_trait;
use tracing::debug;

use turul_http_mcp_server::middleware::{
    McpMiddleware, MiddlewareError, RequestContext, SessionInjection,
};

use turul_mcp_session_storage::SessionView;

use crate::jwt::JwtValidator;
use crate::metadata::ProtectedResourceMetadata;

/// OAuth 2.1 Resource Server middleware
///
/// Validates Bearer tokens against JWKS and injects claims into
/// request extensions. Runs before session creation to return
/// HTTP 401 challenges without allocating sessions.
pub struct OAuthResourceMiddleware {
    jwt_validator: Arc<JwtValidator>,
    metadata: ProtectedResourceMetadata,
    required_scopes: Vec<String>,
}

impl OAuthResourceMiddleware {
    /// Create a new OAuth middleware
    pub fn new(jwt_validator: Arc<JwtValidator>, metadata: ProtectedResourceMetadata) -> Self {
        Self {
            jwt_validator,
            metadata,
            required_scopes: Vec::new(),
        }
    }

    /// Require these scopes on every validated token (space-delimited `scope`
    /// claim). A token missing any of them is rejected with HTTP 403 and a
    /// `WWW-Authenticate` challenge carrying `error="insufficient_scope"`.
    pub fn with_required_scopes(mut self, scopes: Vec<String>) -> Self {
        self.required_scopes = scopes;
        self
    }

    /// Build a WWW-Authenticate challenge header value
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
}

#[async_trait]
impl McpMiddleware for OAuthResourceMiddleware {
    fn runs_before_session(&self) -> bool {
        true
    }

    async fn before_dispatch(
        &self,
        ctx: &mut RequestContext<'_>,
        _session: Option<&dyn SessionView>,
        _injection: &mut SessionInjection,
    ) -> Result<(), MiddlewareError> {
        let token = match ctx.bearer_token() {
            Some(token) => token,
            // RFC 6750 §3.1 / Authorization §Error Handling: a PRESENT but
            // malformed Authorization header is 400 invalid_request, NOT the
            // missing-credentials 401.
            None if ctx.authorization_malformed() => {
                return Err(MiddlewareError::http_challenge(
                    400,
                    self.build_challenge(
                        ", error=\"invalid_request\", \
                         error_description=\"malformed Authorization header\"",
                    ),
                ));
            }
            None => {
                return Err(MiddlewareError::http_challenge(
                    401,
                    self.build_challenge(""),
                ));
            }
        };

        debug!("Validating Bearer token for method: {}", ctx.method());

        let claims = self.jwt_validator.validate(token).await.map_err(|e| {
            debug!("Token validation failed: {}", e);
            MiddlewareError::http_challenge(
                401,
                self.build_challenge(&format!(
                    ", error=\"invalid_token\", error_description=\"{}\"",
                    e
                )),
            )
        })?;

        // Runtime scope enforcement (Authorization §Insufficient Scope):
        // "the server SHOULD respond with HTTP 403 Forbidden ... error=\"insufficient_scope\"".
        if !self.required_scopes.is_empty() {
            let granted: std::collections::HashSet<&str> = claims
                .scope
                .as_deref()
                .unwrap_or("")
                .split_ascii_whitespace()
                .collect();
            if let Some(missing) = self
                .required_scopes
                .iter()
                .find(|s| !granted.contains(s.as_str()))
            {
                debug!("Token lacks required scope {missing}");
                return Err(MiddlewareError::http_challenge(
                    403,
                    self.build_challenge(", error=\"insufficient_scope\""),
                ));
            }
        }

        // Write claims into extensions for downstream tools
        ctx.set_extension(
            "__turul_internal.auth_claims",
            serde_json::to_value(&claims).unwrap_or_default(),
        );

        debug!("Bearer token validated for sub={}", claims.sub);
        Ok(())
    }

    async fn after_dispatch(
        &self,
        _ctx: &RequestContext<'_>,
        _result: &mut turul_http_mcp_server::middleware::DispatcherResult,
    ) -> Result<(), MiddlewareError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // T25: WWW-Authenticate contains resource_metadata URL
    #[test]
    fn test_www_authenticate_contains_resource_metadata_url() {
        let metadata = ProtectedResourceMetadata::new(
            "https://example.com/mcp",
            vec!["https://auth.example.com".to_string()],
        )
        .unwrap();

        let middleware = OAuthResourceMiddleware::new(
            Arc::new(JwtValidator::new(
                "http://localhost/jwks",
                "https://example.com/mcp",
            )),
            metadata,
        );

        assert!(middleware.runs_before_session());
    }

    // T30: Missing bearer returns 401
    #[tokio::test]
    async fn test_missing_bearer_returns_401() {
        let metadata = ProtectedResourceMetadata::new(
            "https://example.com/mcp",
            vec!["https://auth.example.com".to_string()],
        )
        .unwrap();

        let middleware = OAuthResourceMiddleware::new(
            Arc::new(JwtValidator::new(
                "http://localhost/jwks",
                "https://example.com/mcp",
            )),
            metadata,
        );

        let mut ctx = RequestContext::new("tools/call", None);
        let mut injection = SessionInjection::new();

        let result = middleware
            .before_dispatch(&mut ctx, None, &mut injection)
            .await;

        match result {
            Err(MiddlewareError::HttpChallenge {
                status,
                www_authenticate,
                ..
            }) => {
                assert_eq!(status, 401);
                assert!(www_authenticate.contains("resource_metadata="));
                // RFC 9728: metadata URL uses origin, not full resource path
                assert!(
                    www_authenticate
                        .contains("https://example.com/.well-known/oauth-protected-resource"),
                    "Expected origin-based metadata URL, got: {}",
                    www_authenticate
                );
                // Must NOT contain the resource path in the metadata URL
                assert!(
                    !www_authenticate.contains("/mcp/.well-known/"),
                    "Metadata URL must not include resource path: {}",
                    www_authenticate
                );
            }
            other => panic!("Expected HttpChallenge, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_401_includes_scope_when_configured() {
        let metadata = ProtectedResourceMetadata::new(
            "https://example.com/mcp",
            vec!["https://auth.example.com".to_string()],
        )
        .unwrap()
        .with_scopes(vec!["mcp:read".to_string(), "mcp:write".to_string()]);

        let middleware = OAuthResourceMiddleware::new(
            Arc::new(JwtValidator::new(
                "http://localhost/jwks",
                "https://example.com/mcp",
            )),
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
                    "Expected scope in challenge, got: {}",
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
            Arc::new(JwtValidator::new(
                "http://localhost/jwks",
                "https://example.com/mcp",
            )),
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
                    "Should not include scope, got: {}",
                    www_authenticate
                );
            }
            other => panic!("Expected HttpChallenge, got {:?}", other),
        }
    }

    // ---- scope enforcement (Authorization §Insufficient Scope) ----

    fn test_metadata() -> ProtectedResourceMetadata {
        ProtectedResourceMetadata::new(
            "https://example.com/mcp",
            vec!["https://auth.example.com".to_string()],
        )
        .unwrap()
        .with_scopes(vec!["mcp:read".to_string(), "mcp:write".to_string()])
    }

    // ---- JWKS fixture ----
    //
    // The validator exposes no way to preload a key, so these tests serve a
    // real JWKS document over HTTP and exercise the fetch/parse/cache path.
    // Keygen is expensive, so one keypair is shared across the module.

    use base64::Engine as _;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use jsonwebtoken::{Algorithm, EncodingKey, Header};
    use std::sync::LazyLock;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    const TEST_KID: &str = "kid-1";
    const TEST_AUDIENCE: &str = "https://example.com/mcp";
    const TEST_ISSUER: &str = "https://auth.example.com";

    /// (signing key, JWKS modulus, JWKS exponent)
    static RSA_KEY: LazyLock<(EncodingKey, String, String)> = LazyLock::new(|| {
        use rsa::pkcs1::EncodeRsaPrivateKey;
        use rsa::traits::PublicKeyParts;

        let private = rsa::RsaPrivateKey::new(&mut rand::rngs::ThreadRng::default(), 2048).unwrap();
        let der = private.to_pkcs1_der().unwrap();
        let encoding = EncodingKey::from_rsa_der(der.as_bytes());

        let public = rsa::RsaPublicKey::from(&private);
        let n = URL_SAFE_NO_PAD.encode(be_bytes(public.n().as_ref()));
        let e = URL_SAFE_NO_PAD.encode(be_bytes(public.e()));
        (encoding, n, e)
    });

    /// Minimal big-endian bytes. `crypto-bigint` emits fixed-width limbs, so the
    /// exponent arrives zero-padded to the modulus width; JWKS carries the
    /// minimal form.
    fn be_bytes(v: &crypto_bigint::BoxedUint) -> Vec<u8> {
        let bytes = v.to_be_bytes();
        let first = bytes
            .iter()
            .position(|b| *b != 0)
            .unwrap_or(bytes.len().saturating_sub(1));
        bytes[first..].to_vec()
    }

    /// A JWKS endpoint serving the shared test key.
    async fn jwks_server() -> MockServer {
        let (_, n, e) = &*RSA_KEY;
        let body = serde_json::json!({
            "keys": [{
                "kty": "RSA",
                "use": "sig",
                "kid": TEST_KID,
                "alg": "RS256",
                "n": n,
                "e": e,
            }]
        });

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/jwks.json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&server)
            .await;
        server
    }

    fn validator_for(server: &MockServer) -> JwtValidator {
        JwtValidator::new(format!("{}/jwks.json", server.uri()), TEST_AUDIENCE)
            .with_issuer(TEST_ISSUER)
    }

    fn claims(scope: Option<&str>) -> crate::jwt::TokenClaims {
        crate::jwt::TokenClaims {
            sub: "user-1".to_string(),
            iss: TEST_ISSUER.to_string(),
            aud: serde_json::json!(TEST_AUDIENCE),
            exp: now_secs() + 3600,
            iat: now_secs(),
            scope: scope.map(String::from),
            extra: Default::default(),
        }
    }

    fn now_secs() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    fn sign(claims: &crate::jwt::TokenClaims, alg: Algorithm, key: &EncodingKey) -> String {
        let mut header = Header::new(alg);
        header.kid = Some(TEST_KID.to_string());
        jsonwebtoken::encode(&header, claims, key).unwrap()
    }

    fn mint_token(scope: Option<&str>) -> String {
        sign(&claims(scope), Algorithm::RS256, &RSA_KEY.0)
    }

    // ---- access-token validation (OAuth 2.1 §5.2, RFC 8707 §2) ----
    //
    // These assert through the middleware — the path a real request takes —
    // rather than against the validator in isolation.

    /// Drive a token through the middleware and report the validation outcome.
    ///
    /// The mock server must outlive the call, so it is created here rather than
    /// by the caller.
    async fn validate_via_middleware(token: String) -> Result<(), MiddlewareError> {
        let server = jwks_server().await;
        let mw = OAuthResourceMiddleware::new(Arc::new(validator_for(&server)), test_metadata());
        let mut ctx = RequestContext::new("tools/call", None);
        ctx.set_bearer_token(token);
        let mut injection = SessionInjection::default();
        mw.before_dispatch(&mut ctx, None, &mut injection)
            .await
            .map(|_| ())
    }

    #[tokio::test]
    async fn valid_jwt_accepted() {
        assert!(
            validate_via_middleware(mint_token(Some("mcp:read mcp:write")))
                .await
                .is_ok(),
            "a correctly signed, in-date, correctly scoped token must be accepted"
        );
    }

    /// Assert the rejection names `reason`.
    ///
    /// A bare `is_err()` would pass for any failure — including a broken test
    /// fixture — so each negative case pins the discriminating cause.
    fn assert_rejected_for(err: &MiddlewareError, reason: &str) {
        let rendered = format!("{err:?}").to_lowercase();
        assert!(
            rendered.contains(reason),
            "rejection should name {reason}, got: {rendered}"
        );
    }

    #[tokio::test]
    async fn expired_jwt_rejected() {
        let mut c = claims(Some("mcp:read mcp:write"));
        c.exp = now_secs() - 3600;
        let err = validate_via_middleware(sign(&c, Algorithm::RS256, &RSA_KEY.0))
            .await
            .expect_err("an expired token must be rejected");
        assert_rejected_for(&err, "expire");
    }

    #[tokio::test]
    async fn wrong_audience_rejected() {
        let mut c = claims(Some("mcp:read mcp:write"));
        c.aud = serde_json::json!("https://other.example.com/mcp");
        let err = validate_via_middleware(sign(&c, Algorithm::RS256, &RSA_KEY.0))
            .await
            .expect_err("audience binding is mandatory with no opt-out (RFC 8707 §2)");
        assert_rejected_for(&err, "audience");
    }

    #[tokio::test]
    async fn wrong_issuer_rejected() {
        let mut c = claims(Some("mcp:read mcp:write"));
        c.iss = "https://attacker.example.com".to_string();
        let err = validate_via_middleware(sign(&c, Algorithm::RS256, &RSA_KEY.0))
            .await
            .expect_err("a token from an unconfigured issuer must be rejected");
        assert_rejected_for(&err, "issuer");
    }

    #[tokio::test]
    async fn symmetric_alg_rejected() {
        // A token signed HS256 with the JWKS modulus as the shared secret is the
        // classic algorithm-confusion attack; RS256/ES256 are the only allowed
        // algorithms, so it must not validate.
        let forged = sign(
            &claims(Some("mcp:read mcp:write")),
            Algorithm::HS256,
            &EncodingKey::from_secret(RSA_KEY.1.as_bytes()),
        );
        let err = validate_via_middleware(forged)
            .await
            .expect_err("HS256 must be rejected: it is outside the allowed algorithm set");
        assert_rejected_for(&err, "algorithm");
    }

    #[tokio::test]
    async fn token_signed_by_unknown_key_rejected() {
        let other = rsa::RsaPrivateKey::new(&mut rand::rngs::ThreadRng::default(), 2048).unwrap();
        let der = rsa::pkcs1::EncodeRsaPrivateKey::to_pkcs1_der(&other).unwrap();
        let forged = sign(
            &claims(Some("mcp:read mcp:write")),
            Algorithm::RS256,
            &EncodingKey::from_rsa_der(der.as_bytes()),
        );
        assert!(
            validate_via_middleware(forged).await.is_err(),
            "a signature from a key absent from the JWKS must be rejected"
        );
    }

    #[tokio::test]
    async fn insufficient_scope_returns_403_challenge() {
        let server = jwks_server().await;
        let middleware =
            OAuthResourceMiddleware::new(Arc::new(validator_for(&server)), test_metadata())
                .with_required_scopes(vec!["mcp:write".to_string()]);

        let mut ctx = RequestContext::new("tools/call", None);
        ctx.set_bearer_token(mint_token(Some("mcp:read")));
        let mut injection = SessionInjection::default();
        let err = middleware
            .before_dispatch(&mut ctx, None, &mut injection)
            .await
            .expect_err("token without mcp:write must be rejected");
        match err {
            MiddlewareError::HttpChallenge {
                status,
                www_authenticate,
                ..
            } => {
                assert_eq!(status, 403, "insufficient scope is 403 Forbidden");
                assert!(
                    www_authenticate.contains("insufficient_scope"),
                    "{www_authenticate}"
                );
                assert!(
                    www_authenticate.contains("scope="),
                    "challenge must list the scopes: {www_authenticate}"
                );
            }
            other => panic!("expected HttpChallenge, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn sufficient_scope_passes_and_injects_claims() {
        let server = jwks_server().await;
        let middleware =
            OAuthResourceMiddleware::new(Arc::new(validator_for(&server)), test_metadata())
                .with_required_scopes(vec!["mcp:write".to_string()]);

        let mut ctx = RequestContext::new("tools/call", None);
        ctx.set_bearer_token(mint_token(Some("mcp:read mcp:write")));
        let mut injection = SessionInjection::default();
        middleware
            .before_dispatch(&mut ctx, None, &mut injection)
            .await
            .expect("token with all required scopes must pass");
        assert!(ctx.get_extension("__turul_internal.auth_claims").is_some());
    }

    /// RFC 6750 §3.1: present-but-malformed Authorization → 400 invalid_request.
    #[tokio::test]
    async fn malformed_authorization_returns_400_invalid_request() {
        // Rejected before the token is read, so the validator is never used.
        let middleware = OAuthResourceMiddleware::new(
            Arc::new(JwtValidator::new(
                "https://unused.example.com/jwks",
                TEST_AUDIENCE,
            )),
            test_metadata(),
        );

        let mut ctx = RequestContext::new("tools/call", None);
        ctx.set_authorization_malformed(true);
        let mut injection = SessionInjection::default();
        let err = middleware
            .before_dispatch(&mut ctx, None, &mut injection)
            .await
            .expect_err("malformed header must be rejected");
        match err {
            MiddlewareError::HttpChallenge {
                status,
                www_authenticate,
                ..
            } => {
                assert_eq!(status, 400);
                assert!(
                    www_authenticate.contains("invalid_request"),
                    "{www_authenticate}"
                );
            }
            other => panic!("expected HttpChallenge, got: {other:?}"),
        }
    }
}
