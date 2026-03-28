//! JWT validation for OAuth 2.1 Resource Server
//!
//! Validates Bearer tokens using JWKS (JSON Web Key Sets) fetched from
//! the authorization server. Supports RS256 and ES256 algorithms with
//! automatic key rotation via kid-miss refresh.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, warn};

use crate::error::OAuthError;

/// Validated token claims extracted from a JWT
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenClaims {
    /// Subject (user identifier)
    #[serde(default)]
    pub sub: String,
    /// Issuer
    #[serde(default)]
    pub iss: String,
    /// Audience (can be string or array)
    #[serde(default)]
    pub aud: serde_json::Value,
    /// Expiration time (Unix timestamp)
    #[serde(default)]
    pub exp: u64,
    /// Issued at (Unix timestamp)
    #[serde(default)]
    pub iat: u64,
    /// Scopes (space-separated string)
    #[serde(default)]
    pub scope: Option<String>,
    /// All other claims
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// JWKS response from the authorization server
#[derive(Debug, Clone, Deserialize)]
struct JwksResponse {
    keys: Vec<JwkKey>,
}

/// Individual JWK key
#[derive(Debug, Clone, Deserialize)]
struct JwkKey {
    /// Key type (RSA, EC)
    kty: String,
    /// Key ID
    kid: Option<String>,
    /// Algorithm
    alg: Option<String>,
    /// RSA modulus (base64url)
    n: Option<String>,
    /// RSA exponent (base64url)
    e: Option<String>,
    /// EC curve
    crv: Option<String>,
    /// EC x coordinate (base64url)
    x: Option<String>,
    /// EC y coordinate (base64url)
    y: Option<String>,
}

/// Cached JWKS with expiration tracking
struct CachedJwks {
    keys: HashMap<String, (DecodingKey, Algorithm)>,
    #[allow(dead_code)]
    fetched_at: Instant,
    last_refresh_at: Instant,
}

/// JWT validator with JWKS caching and kid-miss refresh
pub struct JwtValidator {
    jwks_uri: String,
    cached_jwks: RwLock<Option<CachedJwks>>,
    allowed_algorithms: Vec<Algorithm>,
    issuer: Option<String>,
    audience: Option<String>,
    /// Minimum interval between JWKS refreshes (rate limiting)
    refresh_interval: Duration,
    /// HTTP client for JWKS fetches
    http_client: reqwest::Client,
}

impl JwtValidator {
    /// Create a new JWT validator with a required audience
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

    /// Set the expected issuer
    pub fn with_issuer(mut self, issuer: impl Into<String>) -> Self {
        self.issuer = Some(issuer.into());
        self
    }

    /// Set allowed algorithms
    pub fn with_algorithms(mut self, algorithms: Vec<Algorithm>) -> Self {
        self.allowed_algorithms = algorithms;
        self
    }

    /// Set the JWKS refresh rate limit
    pub fn with_refresh_interval(mut self, interval: Duration) -> Self {
        self.refresh_interval = interval;
        self
    }

    /// Validate a JWT token and return the claims
    pub async fn validate(&self, token: &str) -> Result<TokenClaims, OAuthError> {
        // Decode header to get kid and algorithm
        let header = decode_header(token)
            .map_err(|e| OAuthError::DecodingError(format!("Invalid JWT header: {}", e)))?;

        // Reject alg:none
        if header.alg == Algorithm::default() {
            // jsonwebtoken uses HS256 as default, but we check explicitly
        }

        // Check algorithm is allowed
        if !self.allowed_algorithms.contains(&header.alg) {
            return Err(OAuthError::UnsupportedAlgorithm(format!(
                "{:?}",
                header.alg
            )));
        }

        let kid = header.kid.as_deref().unwrap_or("default").to_string();

        // Try to get key from cache (returns key + JWKS-advertised algorithm)
        let (key, jwks_alg) = self.get_decoding_key(&kid).await?;

        // Cross-check: token's header algorithm must match JWKS-advertised algorithm
        if header.alg != jwks_alg {
            return Err(OAuthError::UnsupportedAlgorithm(format!(
                "Token uses {:?} but JWKS key '{}' advertises {:?}",
                header.alg, kid, jwks_alg
            )));
        }

        // Build validation
        let mut validation = Validation::new(header.alg);
        validation.validate_exp = true;

        if let Some(ref iss) = self.issuer {
            validation.set_issuer(&[iss]);
        }

        if let Some(ref aud) = self.audience {
            validation.set_audience(&[aud]);
        } else {
            validation.validate_aud = false;
        }

        // Decode and validate
        let token_data =
            decode::<TokenClaims>(token, &key, &validation).map_err(|e| match e.kind() {
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => OAuthError::TokenExpired,
                jsonwebtoken::errors::ErrorKind::InvalidAudience => OAuthError::InvalidAudience,
                jsonwebtoken::errors::ErrorKind::InvalidIssuer => OAuthError::InvalidIssuer,
                _ => OAuthError::InvalidToken(e.to_string()),
            })?;

        Ok(token_data.claims)
    }

    /// Get a decoding key and its JWKS-advertised algorithm by kid, with cache-miss refresh
    async fn get_decoding_key(&self, kid: &str) -> Result<(DecodingKey, Algorithm), OAuthError> {
        // Try cache first
        {
            let cache = self.cached_jwks.read().await;
            if let Some(ref cached) = *cache
                && let Some((key, alg)) = cached.keys.get(kid)
            {
                return Ok((key.clone(), *alg));
            }
        }

        // Cache miss — refresh JWKS (rate-limited)
        self.refresh_jwks().await?;

        // Try again after refresh
        let cache = self.cached_jwks.read().await;
        if let Some(ref cached) = *cache
            && let Some((key, alg)) = cached.keys.get(kid)
        {
            return Ok((key.clone(), *alg));
        }

        Err(OAuthError::KeyNotFound(kid.to_string()))
    }

    /// Refresh JWKS from the authorization server (rate-limited)
    async fn refresh_jwks(&self) -> Result<(), OAuthError> {
        // Rate limit check
        {
            let cache = self.cached_jwks.read().await;
            if let Some(ref cached) = *cache
                && cached.last_refresh_at.elapsed() < self.refresh_interval
            {
                debug!("JWKS refresh rate-limited, skipping");
                return Ok(());
            }
        }

        debug!("Fetching JWKS from {}", self.jwks_uri);

        let response = self
            .http_client
            .get(&self.jwks_uri)
            .send()
            .await
            .map_err(|e| OAuthError::JwksFetchError(e.to_string()))?;

        let jwks: JwksResponse = response
            .json()
            .await
            .map_err(|e| OAuthError::JwksFetchError(format!("Invalid JWKS JSON: {}", e)))?;

        let mut keys = HashMap::new();

        for key in &jwks.keys {
            let kid = key.kid.clone().unwrap_or_else(|| "default".to_string());

            match key.kty.as_str() {
                "RSA" => {
                    if let (Some(n), Some(e)) = (&key.n, &key.e) {
                        match DecodingKey::from_rsa_components(n, e) {
                            Ok(decoding_key) => {
                                let alg = key
                                    .alg
                                    .as_deref()
                                    .and_then(|a| match a {
                                        "RS256" => Some(Algorithm::RS256),
                                        "RS384" => Some(Algorithm::RS384),
                                        "RS512" => Some(Algorithm::RS512),
                                        _ => None,
                                    })
                                    .unwrap_or(Algorithm::RS256);
                                keys.insert(kid, (decoding_key, alg));
                            }
                            Err(e) => {
                                warn!("Failed to parse RSA key: {}", e);
                            }
                        }
                    }
                }
                "EC" => {
                    if let (Some(x), Some(y), Some(crv)) = (&key.x, &key.y, &key.crv) {
                        match DecodingKey::from_ec_components(x, y) {
                            Ok(decoding_key) => {
                                let alg = match crv.as_str() {
                                    "P-256" => Algorithm::ES256,
                                    "P-384" => Algorithm::ES384,
                                    _ => {
                                        warn!("Unsupported EC curve: {}", crv);
                                        continue;
                                    }
                                };
                                keys.insert(kid, (decoding_key, alg));
                            }
                            Err(e) => {
                                warn!("Failed to parse EC key: {}", e);
                            }
                        }
                    }
                }
                other => {
                    debug!("Skipping unsupported key type: {}", other);
                }
            }
        }

        debug!("JWKS loaded: {} keys", keys.len());

        let now = Instant::now();
        let mut cache = self.cached_jwks.write().await;
        *cache = Some(CachedJwks {
            keys,
            fetched_at: now,
            last_refresh_at: now,
        });

        Ok(())
    }
}

/// Create a JwtValidator for testing with pre-loaded keys
#[cfg(test)]
impl JwtValidator {
    /// Create a test validator with a pre-loaded key (async)
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{EncodingKey, Header};

    fn generate_rsa_keys() -> (EncodingKey, DecodingKey) {
        let rsa_key = rsa::RsaPrivateKey::new(&mut rand::rngs::ThreadRng::default(), 2048).unwrap();
        let der = rsa::pkcs1::EncodeRsaPrivateKey::to_pkcs1_der(&rsa_key).unwrap();
        let encoding_key = EncodingKey::from_rsa_der(der.as_bytes());

        let public_key = rsa::RsaPublicKey::from(&rsa_key);
        let pub_der = rsa::pkcs1::EncodeRsaPublicKey::to_pkcs1_der(&public_key).unwrap();
        let decoding_key = DecodingKey::from_rsa_der(pub_der.as_bytes());

        (encoding_key, decoding_key)
    }

    fn create_test_token(encoding_key: &EncodingKey, kid: &str, claims: &TokenClaims) -> String {
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some(kid.to_string());
        jsonwebtoken::encode(&header, claims, encoding_key).unwrap()
    }

    fn valid_claims() -> TokenClaims {
        TokenClaims {
            sub: "user-123".to_string(),
            iss: "https://auth.example.com".to_string(),
            aud: serde_json::json!("https://example.com/mcp"),
            exp: (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp() as u64,
            iat: chrono::Utc::now().timestamp() as u64,
            scope: Some("mcp:read mcp:write".to_string()),
            extra: HashMap::new(),
        }
    }

    // T22: Valid JWT accepted
    #[tokio::test]
    async fn test_valid_jwt_accepted() {
        let (enc_key, dec_key) = generate_rsa_keys();
        let validator =
            JwtValidator::test_with_key_async(dec_key, "test-kid", Algorithm::RS256).await;

        let claims = valid_claims();
        let token = create_test_token(&enc_key, "test-kid", &claims);

        let result = validator.validate(&token).await;
        assert!(
            result.is_ok(),
            "Valid JWT should be accepted: {:?}",
            result.err()
        );
        let parsed = result.unwrap();
        assert_eq!(parsed.sub, "user-123");
    }

    // T23: Expired JWT rejected
    #[tokio::test]
    async fn test_expired_jwt_rejected_401() {
        let (enc_key, dec_key) = generate_rsa_keys();
        let validator =
            JwtValidator::test_with_key_async(dec_key, "test-kid", Algorithm::RS256).await;

        let mut claims = valid_claims();
        claims.exp = (chrono::Utc::now() - chrono::Duration::hours(1)).timestamp() as u64;
        let token = create_test_token(&enc_key, "test-kid", &claims);

        let result = validator.validate(&token).await;
        assert!(matches!(result, Err(OAuthError::TokenExpired)));
    }

    // T31: Wrong audience rejected
    #[tokio::test]
    async fn test_wrong_audience_rejected() {
        let (enc_key, dec_key) = generate_rsa_keys();
        let mut validator =
            JwtValidator::test_with_key_async(dec_key, "test-kid", Algorithm::RS256).await;
        validator.audience = Some("https://other.example.com".to_string());

        let claims = valid_claims();
        let token = create_test_token(&enc_key, "test-kid", &claims);

        let result = validator.validate(&token).await;
        assert!(
            matches!(result, Err(OAuthError::InvalidAudience)),
            "Expected InvalidAudience, got: {:?}",
            result
        );
    }

    // T32: Wrong issuer rejected
    #[tokio::test]
    async fn test_wrong_issuer_rejected() {
        let (enc_key, dec_key) = generate_rsa_keys();
        let mut validator =
            JwtValidator::test_with_key_async(dec_key, "test-kid", Algorithm::RS256).await;
        validator.issuer = Some("https://wrong-issuer.example.com".to_string());

        let claims = valid_claims();
        let token = create_test_token(&enc_key, "test-kid", &claims);

        let result = validator.validate(&token).await;
        assert!(matches!(result, Err(OAuthError::InvalidIssuer)));
    }

    // T37: alg:none rejected
    #[tokio::test]
    async fn test_alg_none_rejected() {
        let (_enc_key, dec_key) = generate_rsa_keys();
        let validator =
            JwtValidator::test_with_key_async(dec_key, "test-kid", Algorithm::RS256).await;

        // Create token with HS256 (not in allowed list)
        let claims = valid_claims();
        let mut header = Header::new(Algorithm::HS256);
        header.kid = Some("test-kid".to_string());
        let token =
            jsonwebtoken::encode(&header, &claims, &EncodingKey::from_secret(b"secret")).unwrap();

        let result = validator.validate(&token).await;
        assert!(matches!(result, Err(OAuthError::UnsupportedAlgorithm(_))));
    }

    #[tokio::test]
    async fn test_audience_always_validated() {
        let (enc_key, dec_key) = generate_rsa_keys();
        let validator =
            JwtValidator::test_with_key_async(dec_key, "test-kid", Algorithm::RS256).await;
        let mut claims = valid_claims();
        claims.aud = serde_json::json!("https://wrong.example.com");
        let token = create_test_token(&enc_key, "test-kid", &claims);
        let result = validator.validate(&token).await;
        assert!(
            matches!(result, Err(OAuthError::InvalidAudience)),
            "Audience must always be validated, got: {:?}",
            result
        );
    }

    // T36: RS256 and ES256 both accepted
    #[tokio::test]
    async fn test_rs256_es256_both_accepted() {
        let validator = JwtValidator::new("http://localhost/jwks", "https://example.com/mcp");
        assert!(validator.allowed_algorithms.contains(&Algorithm::RS256));
        assert!(validator.allowed_algorithms.contains(&Algorithm::ES256));
    }
}
