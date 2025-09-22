//! Security Controls for MCP Server
//!
//! This module provides comprehensive security features including:
//! - Request rate limiting
//! - Resource access controls
//! - Input validation and sanitization
//! - Security middleware for handlers

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use regex::Regex;
use serde_json::Value;

use crate::SessionContext;
use turul_mcp_protocol::McpError;

/// Rate limiting configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum requests per window
    pub max_requests: u32,
    /// Time window duration
    pub window_duration: Duration,
    /// Burst allowance (temporary exceeding of rate limit)
    pub burst_size: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests: 100,
            window_duration: Duration::from_secs(60),
            burst_size: 10,
        }
    }
}

/// Rate limiter implementation using sliding window
#[derive(Debug)]
pub struct RateLimiter {
    config: RateLimitConfig,
    // Session ID -> (request_times, burst_count)
    session_buckets: Arc<Mutex<HashMap<String, (Vec<Instant>, u32)>>>,
}

impl RateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            session_buckets: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Check if request is allowed for the given session
    pub fn check_rate_limit(&self, session_id: &str) -> Result<(), McpError> {
        let mut buckets = self.session_buckets.lock().unwrap();
        let now = Instant::now();
        
        let (request_times, burst_count) = buckets
            .entry(session_id.to_string())
            .or_insert_with(|| (Vec::new(), 0));

        // Clean old requests outside the window
        request_times.retain(|&time| now.duration_since(time) < self.config.window_duration);

        // Allow request and record timestamp first
        request_times.push(now);
        
        // Check if we're over the limit after adding this request
        if request_times.len() > self.config.max_requests as usize {
            // Check burst allowance
            if *burst_count < self.config.burst_size {
                *burst_count += 1;
                return Ok(());
            }
            
            // Remove the request we just added since it's not allowed
            request_times.pop();
            
            return Err(McpError::param_out_of_range(
                "request_rate", 
                &format!("{} requests", request_times.len() + 1),
                &format!("max {} requests per {:?}", self.config.max_requests, self.config.window_duration)
            ));
        }
        
        // Reset burst count if we're below the limit
        if request_times.len() < (self.config.max_requests as f32 * 0.8) as usize {
            *burst_count = 0;
        }

        Ok(())
    }

    /// Clean up expired session data
    pub fn cleanup_expired_sessions(&self) {
        let mut buckets = self.session_buckets.lock().unwrap();
        let now = Instant::now();
        
        buckets.retain(|_, (request_times, _)| {
            request_times.retain(|&time| now.duration_since(time) < self.config.window_duration);
            !request_times.is_empty()
        });
    }
}

/// Resource access control levels
#[derive(Debug, Clone, PartialEq)]
pub enum AccessLevel {
    /// Public access - no restrictions
    Public,
    /// Session-based access - requires valid session
    SessionRequired,
    /// Custom validation function
    Custom(String), // Function name for custom validation
}

/// Resource access control configuration
#[derive(Debug, Clone)]
pub struct ResourceAccessControl {
    /// Access level requirement
    pub access_level: AccessLevel,
    /// Allowed URI patterns (regex)
    pub allowed_patterns: Vec<Regex>,
    /// Blocked URI patterns (regex) - takes precedence
    pub blocked_patterns: Vec<Regex>,
    /// Maximum resource size (bytes)
    pub max_size: Option<u64>,
    /// Allowed MIME types
    pub allowed_mime_types: Option<Vec<String>>,
}

impl Default for ResourceAccessControl {
    fn default() -> Self {
        Self {
            access_level: AccessLevel::SessionRequired,
            allowed_patterns: vec![
                Regex::new(r"^file:///[a-zA-Z0-9_/-]+\.(json|txt|md|html)$").unwrap(),
            ],
            blocked_patterns: vec![
                Regex::new(r"\.\.").unwrap(), // Directory traversal
                Regex::new(r"/etc/").unwrap(), // System files
                Regex::new(r"/proc/").unwrap(), // Process files
                Regex::new(r"\.exe$").unwrap(), // Executables
            ],
            max_size: Some(10 * 1024 * 1024), // 10MB default
            allowed_mime_types: Some(vec![
                "text/plain".to_string(),
                "text/markdown".to_string(),
                "application/json".to_string(),
                "text/html".to_string(),
                "image/png".to_string(),
                "image/jpeg".to_string(),
            ]),
        }
    }
}

impl ResourceAccessControl {
    /// Validate if a URI is allowed
    pub fn validate_uri(&self, uri: &str) -> Result<(), McpError> {
        // Check blocked patterns first (highest priority)
        for blocked_pattern in &self.blocked_patterns {
            if blocked_pattern.is_match(uri) {
                return Err(McpError::invalid_param_type(
                    "uri",
                    "URI not matching blocked patterns",
                    uri
                ));
            }
        }

        // Check allowed patterns
        if !self.allowed_patterns.is_empty() {
            let allowed = self.allowed_patterns
                .iter()
                .any(|pattern| pattern.is_match(uri));
            
            if !allowed {
                return Err(McpError::invalid_param_type(
                    "uri",
                    "URI matching allowed patterns",
                    uri
                ));
            }
        }

        Ok(())
    }

    /// Validate MIME type
    pub fn validate_mime_type(&self, mime_type: &str) -> Result<(), McpError> {
        if let Some(allowed_types) = &self.allowed_mime_types
            && !allowed_types.contains(&mime_type.to_string()) {
                return Err(McpError::invalid_param_type(
                    "mime_type",
                    "allowed MIME type",
                    mime_type
                ));
            }
        Ok(())
    }

    /// Validate content size
    pub fn validate_size(&self, size: u64) -> Result<(), McpError> {
        if let Some(max_size) = self.max_size
            && size > max_size {
                return Err(McpError::param_out_of_range(
                    "content_size",
                    &format!("{} bytes", size),
                    &format!("max {} bytes", max_size)
                ));
            }
        Ok(())
    }
}

/// Input validation and sanitization
pub struct InputValidator {
    /// Maximum JSON depth to prevent DoS
    max_json_depth: usize,
    /// Maximum string length
    max_string_length: usize,
    /// Maximum array/object size
    max_collection_size: usize,
}

impl Default for InputValidator {
    fn default() -> Self {
        Self {
            max_json_depth: 10,
            max_string_length: 1024 * 1024, // 1MB
            max_collection_size: 1000,
        }
    }
}

impl InputValidator {
    pub fn new(max_json_depth: usize, max_string_length: usize, max_collection_size: usize) -> Self {
        Self {
            max_json_depth,
            max_string_length,
            max_collection_size,
        }
    }

    /// Validate JSON input for security issues
    pub fn validate_json(&self, value: &Value) -> Result<(), McpError> {
        self.validate_json_recursive(value, 0)
    }

    fn validate_json_recursive(&self, value: &Value, depth: usize) -> Result<(), McpError> {
        if depth > self.max_json_depth {
            return Err(McpError::param_out_of_range(
                "json_depth",
                &format!("{}", depth),
                &format!("max {}", self.max_json_depth)
            ));
        }

        match value {
            Value::String(s) => {
                if s.len() > self.max_string_length {
                    return Err(McpError::param_out_of_range(
                        "string_length",
                        &format!("{}", s.len()),
                        &format!("max {}", self.max_string_length)
                    ));
                }
                
                // Check for potentially dangerous content
                if s.contains("../") || s.contains("..\\") {
                    return Err(McpError::invalid_param_type(
                        "string_content",
                        "string without directory traversal sequences",
                        s
                    ));
                }
            }
            Value::Array(arr) => {
                if arr.len() > self.max_collection_size {
                    return Err(McpError::param_out_of_range(
                        "array_size",
                        &format!("{}", arr.len()),
                        &format!("max {}", self.max_collection_size)
                    ));
                }
                
                for item in arr {
                    self.validate_json_recursive(item, depth + 1)?;
                }
            }
            Value::Object(obj) => {
                if obj.len() > self.max_collection_size {
                    return Err(McpError::param_out_of_range(
                        "object_size",
                        &format!("{}", obj.len()),
                        &format!("max {}", self.max_collection_size)
                    ));
                }
                
                for (key, val) in obj {
                    // Validate key
                    if key.len() > self.max_string_length {
                        return Err(McpError::param_out_of_range(
                            "object_key_length",
                            &format!("{}", key.len()),
                            &format!("max {}", self.max_string_length)
                        ));
                    }
                    
                    self.validate_json_recursive(val, depth + 1)?;
                }
            }
            _ => {} // Numbers, booleans, null are safe
        }

        Ok(())
    }

    /// Sanitize string input
    pub fn sanitize_string(&self, input: &str) -> String {
        input
            .chars()
            .filter(|c| c.is_ascii() && !c.is_control() || c.is_whitespace())
            .take(self.max_string_length)
            .collect()
    }
}

/// Security middleware for MCP handlers
pub struct SecurityMiddleware {
    rate_limiter: Option<RateLimiter>,
    resource_access_control: ResourceAccessControl,
    input_validator: InputValidator,
}

impl SecurityMiddleware {
    pub fn new() -> Self {
        Self {
            rate_limiter: Some(RateLimiter::new(RateLimitConfig::default())),
            resource_access_control: ResourceAccessControl::default(),
            input_validator: InputValidator::default(),
        }
    }

    /// Get reference to resource access control
    pub fn resource_access_control(&self) -> &ResourceAccessControl {
        &self.resource_access_control
    }

    pub fn with_rate_limiting(mut self, config: RateLimitConfig) -> Self {
        self.rate_limiter = Some(RateLimiter::new(config));
        self
    }

    pub fn without_rate_limiting(mut self) -> Self {
        self.rate_limiter = None;
        self
    }

    pub fn with_resource_access_control(mut self, config: ResourceAccessControl) -> Self {
        self.resource_access_control = config;
        self
    }

    pub fn with_input_validation(mut self, validator: InputValidator) -> Self {
        self.input_validator = validator;
        self
    }

    /// Validate a request before processing
    pub fn validate_request(
        &self,
        method: &str,
        params: Option<&Value>,
        session: Option<&SessionContext>,
    ) -> Result<(), McpError> {
        // Rate limiting check
        if let Some(rate_limiter) = &self.rate_limiter
            && let Some(session) = session {
                rate_limiter.check_rate_limit(&session.session_id)?;
            }

        // Input validation
        if let Some(params) = params {
            self.input_validator.validate_json(params)?;
        }

        // Method-specific security checks
        match method {
            "resources/read" => {
                if let Some(params) = params
                    && let Some(uri) = params.get("uri").and_then(|v| v.as_str()) {
                        self.resource_access_control.validate_uri(uri)?;
                    }

                // Check access level
                match self.resource_access_control.access_level {
                    AccessLevel::SessionRequired if session.is_none() => {
                        return Err(McpError::invalid_param_type(
                            "session",
                            "valid session context",
                            "none"
                        ));
                    }
                    _ => {}
                }
            }
            _ => {} // Other methods have minimal restrictions for now
        }

        Ok(())
    }

    /// Clean up expired data
    pub fn cleanup(&self) {
        if let Some(rate_limiter) = &self.rate_limiter {
            rate_limiter.cleanup_expired_sessions();
        }
    }
}

impl Default for SecurityMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_rate_limiter_basic() {
        let config = RateLimitConfig {
            max_requests: 3,
            window_duration: Duration::from_secs(60),
            burst_size: 1,
        };
        let limiter = RateLimiter::new(config);

        // First 3 requests should succeed
        assert!(limiter.check_rate_limit("session1").is_ok());
        assert!(limiter.check_rate_limit("session1").is_ok());
        assert!(limiter.check_rate_limit("session1").is_ok());

        // 4th request should succeed due to burst
        assert!(limiter.check_rate_limit("session1").is_ok());

        // 5th request should fail
        assert!(limiter.check_rate_limit("session1").is_err());
    }

    #[test]
    fn test_rate_limiter_different_sessions() {
        let config = RateLimitConfig {
            max_requests: 2,
            window_duration: Duration::from_secs(60),
            burst_size: 0,
        };
        let limiter = RateLimiter::new(config);

        // Different sessions should have independent limits
        assert!(limiter.check_rate_limit("session1").is_ok());
        assert!(limiter.check_rate_limit("session1").is_ok());
        assert!(limiter.check_rate_limit("session1").is_err());

        assert!(limiter.check_rate_limit("session2").is_ok());
        assert!(limiter.check_rate_limit("session2").is_ok());
        assert!(limiter.check_rate_limit("session2").is_err());
    }

    #[test]
    fn test_resource_access_control_uri_validation() {
        let access_control = ResourceAccessControl::default();

        // Valid URIs
        assert!(access_control.validate_uri("file:///data/test.json").is_ok());
        assert!(access_control.validate_uri("file:///docs/readme.txt").is_ok());

        // Invalid URIs (blocked patterns)
        assert!(access_control.validate_uri("file:///etc/passwd").is_err());
        assert!(access_control.validate_uri("file:///data/../etc/shadow").is_err());
        assert!(access_control.validate_uri("file:///app/malware.exe").is_err());
    }

    #[test]
    fn test_input_validator_json_depth() {
        let validator = InputValidator::new(3, 1000, 100);

        // Valid depth
        let valid_json = json!({
            "level1": {
                "level2": {
                    "level3": "value"
                }
            }
        });
        assert!(validator.validate_json(&valid_json).is_ok());

        // Excessive depth
        let deep_json = json!({
            "l1": { "l2": { "l3": { "l4": { "l5": "too deep" } } } }
        });
        assert!(validator.validate_json(&deep_json).is_err());
    }

    #[test]
    fn test_input_validator_string_length() {
        let validator = InputValidator::new(10, 10, 100);

        let valid_json = json!({"key": "short"});
        assert!(validator.validate_json(&valid_json).is_ok());

        let invalid_json = json!({"key": "this string is too long"});
        assert!(validator.validate_json(&invalid_json).is_err());
    }

    #[test]
    fn test_input_validator_directory_traversal() {
        let validator = InputValidator::default();

        let malicious_json = json!({"path": "../../../etc/passwd"});
        assert!(validator.validate_json(&malicious_json).is_err());

        let safe_json = json!({"path": "data/file.txt"});
        assert!(validator.validate_json(&safe_json).is_ok());
    }

    #[test]
    fn test_security_middleware_integration() {
        // Create a minimal session context for testing
        let session_id = "test-session".to_string();
        let session = SessionContext {
            session_id: session_id.clone(),
            get_state: Arc::new(|_| Box::pin(futures::future::ready(None))),
            set_state: Arc::new(|_, _| Box::pin(futures::future::ready(()))),
            remove_state: Arc::new(|_| Box::pin(futures::future::ready(None))),
            is_initialized: Arc::new(|| Box::pin(futures::future::ready(true))),
            send_notification: Arc::new(|_| Box::pin(futures::future::ready(()))),
            broadcaster: None,
        };
        
        let middleware = SecurityMiddleware::new();

        // Valid resource read request
        let params = json!({"uri": "file:///data/test.json"});
        assert!(middleware.validate_request("resources/read", Some(&params), Some(&session)).is_ok());

        // Invalid URI
        let bad_params = json!({"uri": "file:///etc/passwd"});
        assert!(middleware.validate_request("resources/read", Some(&bad_params), Some(&session)).is_err());

        // No session when required
        assert!(middleware.validate_request("resources/read", Some(&params), None).is_err());
    }

    #[test]
    fn test_mime_type_validation() {
        let access_control = ResourceAccessControl::default();

        assert!(access_control.validate_mime_type("application/json").is_ok());
        assert!(access_control.validate_mime_type("text/plain").is_ok());
        assert!(access_control.validate_mime_type("application/octet-stream").is_err());
        assert!(access_control.validate_mime_type("application/x-executable").is_err());
    }

    #[test]
    fn test_size_validation() {
        let access_control = ResourceAccessControl::default();

        assert!(access_control.validate_size(1024).is_ok()); // 1KB
        assert!(access_control.validate_size(1024 * 1024).is_ok()); // 1MB
        assert!(access_control.validate_size(20 * 1024 * 1024).is_err()); // 20MB - too large
    }
}