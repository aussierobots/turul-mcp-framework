//! JWT validation for OAuth 2.1 Resource Server.
//!
//! Signature verification, JWKS fetch/cache/refresh and the [`TokenClaims`]
//! shape are owned by [`turul_jwt_validator`]. This module re-exports that
//! surface and adds the framework's hardening policy on top.

use std::time::Duration;

pub use turul_jwt_validator::{JwksFetchErrorKind, JwtValidationError, JwtValidator, TokenClaims};

/// Re-exported so callers can name algorithms for
/// [`JwtValidator::with_algorithms`] without adding their own `jsonwebtoken`
/// dependency. It appears in that method's signature either way, so importing
/// it here is what makes the method callable rather than what creates the
/// coupling. Must track the same `jsonwebtoken` major as `turul-jwt-validator`.
pub use jsonwebtoken::Algorithm;

use crate::error::OAuthError;

/// Maximum age of a cached signing key before a `kid` hit is re-fetched anyway.
///
/// Bounds how long a key revoked at the authorization server stays usable.
pub const DEFAULT_MAX_AGE: Duration = Duration::from_secs(15 * 60);

/// How long keys may be served after a failed refresh, so a brief JWKS outage
/// degrades availability rather than rejecting every request.
///
/// Stale serving is a bounded overrun of [`DEFAULT_MAX_AGE`], so worst-case
/// revocation exposure is the sum of the two: 20 minutes.
pub const DEFAULT_STALE_WINDOW: Duration = Duration::from_secs(5 * 60);

/// Bounded retry for transient JWKS transport failures.
pub const DEFAULT_RETRY_ATTEMPTS: usize = 3;
/// Base delay for the retry backoff.
pub const DEFAULT_RETRY_BASE_DELAY: Duration = Duration::from_millis(100);

/// Build a [`JwtValidator`] carrying this framework's hardening policy.
///
/// `turul-jwt-validator` ships `max_age`, the stale window and retry all
/// disabled, so a bare [`JwtValidator::new`] validates tokens against a key it
/// may cache for the lifetime of the process. This applies the framework
/// defaults and rejects a plaintext `jwks_uri`.
///
/// [`crate::oauth_resource_server`] calls this, so the convenience path and
/// hand-built multi-AS deployments share one definition of the policy.
///
/// # Errors
///
/// [`OAuthError::InvalidConfiguration`] if `jwks_uri` is not `https` and is not
/// a loopback host.
pub fn hardened_validator(jwks_uri: &str, audience: &str) -> Result<JwtValidator, OAuthError> {
    require_secure_jwks_uri(jwks_uri)?;
    Ok(JwtValidator::new(jwks_uri, audience)
        .with_max_age(DEFAULT_MAX_AGE)
        .with_stale_window(DEFAULT_STALE_WINDOW)
        .with_retry(DEFAULT_RETRY_ATTEMPTS, DEFAULT_RETRY_BASE_DELAY))
}

/// Reject a JWKS URI that would fetch signing keys over plaintext.
///
/// Loopback is exempt so local development against a non-TLS authorization
/// server keeps working.
fn require_secure_jwks_uri(jwks_uri: &str) -> Result<(), OAuthError> {
    let parsed = url::Url::parse(jwks_uri).map_err(|e| {
        OAuthError::InvalidConfiguration(format!("jwks_uri is not a valid URL: {}", e))
    })?;

    if parsed.scheme() == "https" {
        return Ok(());
    }

    if is_loopback_host(parsed.host_str()) {
        return Ok(());
    }

    Err(OAuthError::InvalidConfiguration(format!(
        "jwks_uri must use https (loopback exempt), got: {}",
        jwks_uri
    )))
}

fn is_loopback_host(host: Option<&str>) -> bool {
    matches!(host, Some("localhost" | "127.0.0.1" | "::1" | "[::1]"))
}

impl From<JwtValidationError> for OAuthError {
    fn from(err: JwtValidationError) -> Self {
        match err {
            JwtValidationError::InvalidToken(msg) => OAuthError::InvalidToken(msg),
            JwtValidationError::TokenExpired => OAuthError::TokenExpired,
            JwtValidationError::InvalidAudience => OAuthError::InvalidAudience,
            JwtValidationError::InvalidIssuer => OAuthError::InvalidIssuer,
            JwtValidationError::UnsupportedAlgorithm(alg) => OAuthError::UnsupportedAlgorithm(alg),
            JwtValidationError::KeyNotFound(kid) => OAuthError::KeyNotFound(kid),
            JwtValidationError::DecodingError(msg) => OAuthError::DecodingError(msg),
            JwtValidationError::JwksFetchError { kind, message } => {
                OAuthError::JwksFetchError { kind, message }
            }
            // `JwtValidationError` is #[non_exhaustive]; a variant added upstream
            // must not silently become a success or a mis-mapped error.
            other => OAuthError::InvalidToken(other.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn https_jwks_uri_accepted() {
        assert!(hardened_validator("https://auth.example.com/jwks.json", "aud").is_ok());
    }

    #[test]
    fn plaintext_jwks_uri_rejected() {
        // `JwtValidator` is not Debug, so unwrap_err() is unavailable here.
        match hardened_validator("http://auth.example.com/jwks.json", "aud") {
            Err(OAuthError::InvalidConfiguration(m)) => {
                assert!(m.contains("must use https"), "unexpected message: {m}")
            }
            Err(other) => panic!("expected InvalidConfiguration, got: {other}"),
            Ok(_) => panic!("plaintext jwks_uri must be rejected"),
        }
    }

    #[test]
    fn loopback_plaintext_jwks_uri_accepted() {
        // The documented local-development shape must keep working.
        for uri in [
            "http://localhost:9000/.well-known/jwks.json",
            "http://127.0.0.1:9000/jwks.json",
        ] {
            assert!(
                hardened_validator(uri, "aud").is_ok(),
                "loopback must be exempt: {uri}"
            );
        }
    }

    #[test]
    fn malformed_jwks_uri_rejected() {
        assert!(matches!(
            hardened_validator("not a url", "aud"),
            Err(OAuthError::InvalidConfiguration(_))
        ));
    }

    #[test]
    fn jwks_fetch_error_preserves_kind() {
        let converted: OAuthError = JwtValidationError::JwksFetchError {
            kind: JwksFetchErrorKind::Timeout,
            message: "timed out".to_string(),
        }
        .into();
        assert!(
            matches!(
                converted,
                OAuthError::JwksFetchError {
                    kind: JwksFetchErrorKind::Timeout,
                    ..
                }
            ),
            "the typed fetch-failure discriminant must survive conversion, got: {converted}"
        );
    }

    #[test]
    fn expiry_and_audience_map_to_their_own_variants() {
        assert!(matches!(
            OAuthError::from(JwtValidationError::TokenExpired),
            OAuthError::TokenExpired
        ));
        assert!(matches!(
            OAuthError::from(JwtValidationError::InvalidAudience),
            OAuthError::InvalidAudience
        ));
    }
}
