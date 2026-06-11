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

    async fn hs256_validator() -> JwtValidator {
        use jsonwebtoken::{Algorithm, DecodingKey};
        JwtValidator::test_with_key_async(
            DecodingKey::from_secret(b"test-secret"),
            "kid-1",
            Algorithm::HS256,
        )
        .await
        .with_algorithms(vec![Algorithm::HS256])
    }

    fn mint_token(scope: Option<&str>) -> String {
        use jsonwebtoken::{Algorithm, EncodingKey, Header};
        let claims = crate::jwt::TokenClaims {
            sub: "user-1".to_string(),
            iss: "https://auth.example.com".to_string(),
            aud: serde_json::json!("https://example.com/mcp"),
            exp: (std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs())
                + 3600,
            iat: 0,
            scope: scope.map(String::from),
            extra: Default::default(),
        };
        let mut header = Header::new(Algorithm::HS256);
        header.kid = Some("kid-1".to_string());
        jsonwebtoken::encode(&header, &claims, &EncodingKey::from_secret(b"test-secret")).unwrap()
    }

    #[tokio::test]
    async fn insufficient_scope_returns_403_challenge() {
        let middleware =
            OAuthResourceMiddleware::new(Arc::new(hs256_validator().await), test_metadata())
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
        let middleware =
            OAuthResourceMiddleware::new(Arc::new(hs256_validator().await), test_metadata())
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
        let middleware =
            OAuthResourceMiddleware::new(Arc::new(hs256_validator().await), test_metadata());

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
