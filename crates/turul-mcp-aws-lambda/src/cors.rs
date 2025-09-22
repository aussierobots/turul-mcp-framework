//! CORS (Cross-Origin Resource Sharing) support for Lambda MCP servers
//!
//! This module provides CORS header injection for Lambda responses, since Tower
//! middleware cannot be used in the Lambda execution environment.

use std::collections::HashSet;

use http::{HeaderValue, Method};
use lambda_http::{Body as LambdaBody, Response as LambdaResponse};
use tracing::debug;

use crate::error::{LambdaError, Result};

/// CORS configuration for Lambda MCP servers
#[derive(Debug, Clone)]
pub struct CorsConfig {
    /// Allowed origins for CORS requests
    /// Use "*" to allow all origins (not recommended for production)
    pub allowed_origins: Vec<String>,

    /// Allowed HTTP methods
    pub allowed_methods: Vec<Method>,

    /// Allowed request headers
    pub allowed_headers: Vec<String>,

    /// Whether to allow credentials (cookies, authorization headers)
    pub allow_credentials: bool,

    /// Maximum age for preflight cache (in seconds)
    pub max_age: Option<u32>,

    /// Headers to expose to the client
    pub expose_headers: Vec<String>,
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            allowed_origins: vec!["*".to_string()],
            allowed_methods: vec![Method::GET, Method::POST, Method::DELETE, Method::OPTIONS],
            allowed_headers: vec![
                "Content-Type".to_string(),
                "Accept".to_string(),
                "Authorization".to_string(),
                "Mcp-Session-Id".to_string(),
                "Mcp-Protocol-Version".to_string(),
                "Last-Event-ID".to_string(),
            ],
            allow_credentials: false,
            max_age: Some(86400), // 24 hours
            expose_headers: vec![
                "Mcp-Session-Id".to_string(),
                "Mcp-Protocol-Version".to_string(),
            ],
        }
    }
}

impl CorsConfig {
    /// Create a CORS config that allows all origins (for development)
    pub fn allow_all() -> Self {
        Self::default()
    }

    /// Create a CORS config for specific origins
    pub fn for_origins(origins: Vec<String>) -> Self {
        Self {
            allowed_origins: origins,
            ..Default::default()
        }
    }

    /// Create a CORS config from environment variables
    pub fn from_env() -> Self {
        let allowed_origins = std::env::var("MCP_CORS_ORIGINS")
            .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_else(|_| vec!["*".to_string()]);

        let allow_credentials = std::env::var("MCP_CORS_CREDENTIALS")
            .map(|s| s.parse().unwrap_or(false))
            .unwrap_or(false);

        let max_age = std::env::var("MCP_CORS_MAX_AGE")
            .ok()
            .and_then(|s| s.parse().ok());

        Self {
            allowed_origins,
            allow_credentials,
            max_age,
            ..Default::default()
        }
    }
}

/// Inject CORS headers into a Lambda response
///
/// This function adds the appropriate CORS headers based on the configuration
/// and the incoming request's Origin header.
pub fn inject_cors_headers(
    response: &mut LambdaResponse<LambdaBody>,
    config: &CorsConfig,
    request_origin: Option<&str>,
) -> Result<()> {
    debug!("Injecting CORS headers for origin: {:?}", request_origin);

    // Determine allowed origin
    let allowed_origin = determine_allowed_origin(config, request_origin);

    if let Some(origin) = allowed_origin {
        response.headers_mut().insert(
            "Access-Control-Allow-Origin",
            HeaderValue::from_str(&origin)
                .map_err(|e| LambdaError::Cors(format!("Invalid origin: {}", e)))?,
        );
    }

    // Add allowed methods
    let methods_str = config
        .allowed_methods
        .iter()
        .map(|m| m.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    response.headers_mut().insert(
        "Access-Control-Allow-Methods",
        HeaderValue::from_str(&methods_str)
            .map_err(|e| LambdaError::Cors(format!("Invalid methods: {}", e)))?,
    );

    // Add allowed headers
    if !config.allowed_headers.is_empty() {
        let headers_str = config.allowed_headers.join(", ");
        response.headers_mut().insert(
            "Access-Control-Allow-Headers",
            HeaderValue::from_str(&headers_str)
                .map_err(|e| LambdaError::Cors(format!("Invalid headers: {}", e)))?,
        );
    }

    // Add exposed headers
    if !config.expose_headers.is_empty() {
        let expose_str = config.expose_headers.join(", ");
        response.headers_mut().insert(
            "Access-Control-Expose-Headers",
            HeaderValue::from_str(&expose_str)
                .map_err(|e| LambdaError::Cors(format!("Invalid expose headers: {}", e)))?,
        );
    }

    // Add credentials if allowed
    if config.allow_credentials {
        response.headers_mut().insert(
            "Access-Control-Allow-Credentials",
            HeaderValue::from_static("true"),
        );
    }

    // Add max age for preflight requests
    if let Some(max_age) = config.max_age {
        response.headers_mut().insert(
            "Access-Control-Max-Age",
            HeaderValue::from_str(&max_age.to_string())
                .map_err(|e| LambdaError::Cors(format!("Invalid max age: {}", e)))?,
        );
    }

    debug!("CORS headers injected successfully");
    Ok(())
}

/// Create a CORS preflight response
///
/// Handles OPTIONS requests that browsers send before making actual CORS requests.
pub fn create_preflight_response(
    config: &CorsConfig,
    request_origin: Option<&str>,
) -> Result<LambdaResponse<LambdaBody>> {
    debug!("Creating CORS preflight response");

    let mut response = LambdaResponse::builder()
        .status(200)
        .body(LambdaBody::Empty)
        .map_err(LambdaError::Http)?;

    inject_cors_headers(&mut response, config, request_origin)?;

    Ok(response)
}

/// Determine the allowed origin based on configuration and request
fn determine_allowed_origin(config: &CorsConfig, request_origin: Option<&str>) -> Option<String> {
    // If wildcard is configured, return it
    if config.allowed_origins.contains(&"*".to_string()) {
        return Some("*".to_string());
    }

    // If no origin in request, no CORS header needed
    let request_origin = request_origin?;

    // Check if the request origin is in the allowed list
    if config.allowed_origins.contains(&request_origin.to_string()) {
        Some(request_origin.to_string())
    } else {
        // Origin not allowed, don't set CORS header
        None
    }
}

/// Validate CORS configuration
pub fn validate_config(config: &CorsConfig) -> Result<()> {
    // Check for wildcard with credentials (security issue)
    if config.allow_credentials && config.allowed_origins.contains(&"*".to_string()) {
        return Err(LambdaError::Cors(
            "Cannot use wildcard origin (*) with credentials enabled".to_string(),
        ));
    }

    // Validate origins are proper URLs or wildcards
    for origin in &config.allowed_origins {
        if origin != "*" && !origin.starts_with("http://") && !origin.starts_with("https://") {
            return Err(LambdaError::Cors(format!(
                "Invalid origin format: {}",
                origin
            )));
        }
    }

    // Check for duplicate headers
    let headers_set: HashSet<_> = config.allowed_headers.iter().collect();
    if headers_set.len() != config.allowed_headers.len() {
        return Err(LambdaError::Cors(
            "Duplicate headers in allowed_headers".to_string(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use lambda_http::Body;

    #[test]
    fn test_default_config() {
        let config = CorsConfig::default();
        assert!(config.allowed_origins.contains(&"*".to_string()));
        assert!(config.allowed_methods.contains(&Method::GET));
        assert!(config.allowed_methods.contains(&Method::POST));
        assert!(config.allowed_headers.contains(&"Content-Type".to_string()));
    }

    #[test]
    fn test_config_validation() {
        let mut config = CorsConfig::default();
        assert!(validate_config(&config).is_ok());

        // Test invalid wildcard with credentials
        config.allow_credentials = true;
        assert!(validate_config(&config).is_err());

        // Test invalid origin format
        config.allow_credentials = false;
        config.allowed_origins = vec!["invalid-origin".to_string()];
        assert!(validate_config(&config).is_err());
    }

    #[tokio::test]
    async fn test_cors_headers_injection() {
        let config = CorsConfig::default();
        let mut response = LambdaResponse::builder()
            .status(200)
            .body(Body::Empty)
            .unwrap();

        inject_cors_headers(&mut response, &config, Some("https://example.com")).unwrap();

        assert_eq!(
            response.headers().get("access-control-allow-origin"),
            Some(&HeaderValue::from_static("*"))
        );

        assert!(
            response
                .headers()
                .contains_key("access-control-allow-methods")
        );
        assert!(
            response
                .headers()
                .contains_key("access-control-allow-headers")
        );
    }

    #[tokio::test]
    async fn test_preflight_response() {
        let config = CorsConfig::default();
        let response = create_preflight_response(&config, Some("https://example.com")).unwrap();

        assert_eq!(response.status(), 200);
        assert!(
            response
                .headers()
                .contains_key("access-control-allow-origin")
        );
        assert!(
            response
                .headers()
                .contains_key("access-control-allow-methods")
        );
    }
}
