//! OAuth error types

use std::fmt;

/// Errors from OAuth resource server operations
#[derive(Debug)]
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
    /// JWKS fetch or parse error
    JwksFetchError(String),
    /// Key not found in JWKS
    KeyNotFound(String),
    /// Token decoding error
    DecodingError(String),
}

impl fmt::Display for OAuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidToken(msg) => write!(f, "Invalid token: {}", msg),
            Self::TokenExpired => write!(f, "Token has expired"),
            Self::InvalidAudience => write!(f, "Invalid audience"),
            Self::InvalidIssuer => write!(f, "Invalid issuer"),
            Self::UnsupportedAlgorithm(alg) => write!(f, "Unsupported algorithm: {}", alg),
            Self::JwksFetchError(msg) => write!(f, "JWKS fetch error: {}", msg),
            Self::KeyNotFound(kid) => write!(f, "Key not found: {}", kid),
            Self::DecodingError(msg) => write!(f, "Decoding error: {}", msg),
        }
    }
}

impl std::error::Error for OAuthError {}
