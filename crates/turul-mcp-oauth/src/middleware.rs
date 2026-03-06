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
}

impl OAuthResourceMiddleware {
    /// Create a new OAuth middleware
    pub fn new(jwt_validator: Arc<JwtValidator>, metadata: ProtectedResourceMetadata) -> Self {
        Self {
            jwt_validator,
            metadata,
        }
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
        let token = ctx.bearer_token().ok_or_else(|| {
            MiddlewareError::http_challenge(
                401,
                format!(
                    "Bearer realm=\"mcp\", resource_metadata=\"{}\"",
                    self.metadata.metadata_url(),
                ),
            )
        })?;

        debug!("Validating Bearer token for method: {}", ctx.method());

        let claims = self
            .jwt_validator
            .validate(token)
            .await
            .map_err(|e| {
                debug!("Token validation failed: {}", e);
                MiddlewareError::http_challenge(
                    401,
                    format!(
                        "Bearer realm=\"mcp\", error=\"invalid_token\", error_description=\"{}\", resource_metadata=\"{}\"",
                        e,
                        self.metadata.metadata_url(),
                    ),
                )
            })?;

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
        );

        let middleware = OAuthResourceMiddleware::new(
            Arc::new(JwtValidator::new("http://localhost/jwks")),
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
        );

        let middleware = OAuthResourceMiddleware::new(
            Arc::new(JwtValidator::new("http://localhost/jwks")),
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
}
