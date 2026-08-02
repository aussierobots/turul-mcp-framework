//! OAuth error types

use std::fmt;

/// Errors from OAuth resource server operations
///
/// Non-exhaustive: variants track the upstream JWT validator's error surface,
/// which is itself non-exhaustive, so new discriminants must not break callers.
#[derive(Debug)]
#[non_exhaustive]
pub enum OAuthError {
    /// JWT validation failed
    InvalidToken(String),
    /// Token has expired
    TokenExpired,
    /// Wrong audience claim
    InvalidAudience,
    /// Wrong issuer claim
    InvalidIssuer,
    /// Algorithm not allowed
    UnsupportedAlgorithm(String),
    /// JWKS fetch or parse error, carrying the upstream discriminant so callers
    /// can distinguish a timeout from a 404 or an unparseable key set.
    JwksFetchError {
        kind: turul_jwt_validator::JwksFetchErrorKind,
        message: String,
    },
    /// Key not found in JWKS
    KeyNotFound(String),
    /// Token decoding error
    DecodingError(String),
    /// Resource URI validation failed
    InvalidResourceUri(String),
    /// Configuration error
    InvalidConfiguration(String),
}

impl fmt::Display for OAuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidToken(msg) => write!(f, "Invalid token: {}", msg),
            Self::TokenExpired => write!(f, "Token has expired"),
            Self::InvalidAudience => write!(f, "Invalid audience"),
            Self::InvalidIssuer => write!(f, "Invalid issuer"),
            Self::UnsupportedAlgorithm(alg) => write!(f, "Unsupported algorithm: {}", alg),
            Self::JwksFetchError { kind, message } => {
                write!(f, "JWKS fetch error ({:?}): {}", kind, message)
            }
            Self::KeyNotFound(kid) => write!(f, "Key not found: {}", kid),
            Self::DecodingError(msg) => write!(f, "Decoding error: {}", msg),
            Self::InvalidResourceUri(msg) => write!(f, "Invalid resource URI: {}", msg),
            Self::InvalidConfiguration(msg) => write!(f, "Invalid configuration: {}", msg),
        }
    }
}

impl std::error::Error for OAuthError {}
