//! Configuration types for MCP client

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Main client configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ClientConfig {
    /// Client identification information
    pub client_info: ClientInfo,

    /// Timeout configurations
    pub timeouts: TimeoutConfig,

    /// Retry configurations
    pub retry: RetryConfig,

    /// Connection configurations
    pub connection: ConnectionConfig,

    /// Logging configuration
    pub logging: LoggingConfig,
}

/// Client identification information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    /// Client name
    pub name: String,

    /// Client version
    pub version: String,

    /// Client description
    pub description: Option<String>,

    /// Vendor information
    pub vendor: Option<String>,

    /// Additional metadata
    pub metadata: Option<serde_json::Value>,
}

/// Timeout configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutConfig {
    /// Connection timeout
    #[serde(with = "duration_serde")]
    pub connect: Duration,

    /// Request timeout for individual operations
    #[serde(with = "duration_serde")]
    pub request: Duration,

    /// Long operation timeout (for streaming, etc.)
    #[serde(with = "duration_serde")]
    pub long_operation: Duration,

    /// Session initialization timeout
    #[serde(with = "duration_serde")]
    pub initialization: Duration,

    /// Heartbeat interval for keep-alive
    #[serde(with = "duration_serde")]
    pub heartbeat: Duration,
}

/// Retry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_attempts: u32,

    /// Initial retry delay
    #[serde(with = "duration_serde")]
    pub initial_delay: Duration,

    /// Maximum retry delay
    #[serde(with = "duration_serde")]
    pub max_delay: Duration,

    /// Exponential backoff multiplier
    pub backoff_multiplier: f64,

    /// Jitter factor (0.0 to 1.0)
    pub jitter: f64,

    /// Whether to enable exponential backoff
    pub exponential_backoff: bool,
}

/// Connection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    /// User agent string
    pub user_agent: Option<String>,

    /// Custom headers to include in requests
    pub headers: Option<std::collections::HashMap<String, String>>,

    /// Whether to follow redirects
    pub follow_redirects: bool,

    /// Maximum number of redirects to follow
    pub max_redirects: u32,

    /// Keep-alive settings
    pub keep_alive: bool,

    /// Connection pool settings
    pub pool_settings: PoolConfig,
}

/// Connection pool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolConfig {
    /// Maximum number of idle connections per host
    pub max_idle_per_host: u32,

    /// Idle connection timeout
    #[serde(with = "duration_serde")]
    pub idle_timeout: Duration,

    /// Connection lifetime
    #[serde(with = "duration_serde")]
    pub max_lifetime: Duration,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level
    pub level: String,

    /// Whether to log requests
    pub log_requests: bool,

    /// Whether to log responses
    pub log_responses: bool,

    /// Whether to log transport events
    pub log_transport: bool,

    /// Whether to redact sensitive information
    pub redact_sensitive: bool,
}

impl Default for ClientInfo {
    fn default() -> Self {
        Self {
            name: "mcp-client".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            description: Some("Rust MCP Client Library".to_string()),
            vendor: Some("MCP Framework".to_string()),
            metadata: None,
        }
    }
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            connect: Duration::from_secs(10),
            request: Duration::from_secs(30),
            long_operation: Duration::from_secs(300), // 5 minutes
            initialization: Duration::from_secs(15),
            heartbeat: Duration::from_secs(30),
        }
    }
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            backoff_multiplier: 2.0,
            jitter: 0.1,
            exponential_backoff: true,
        }
    }
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            user_agent: Some(format!("mcp-client/{}", env!("CARGO_PKG_VERSION"))),
            headers: None,
            follow_redirects: true,
            max_redirects: 5,
            keep_alive: true,
            pool_settings: PoolConfig::default(),
        }
    }
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_idle_per_host: 5,
            idle_timeout: Duration::from_secs(90),
            max_lifetime: Duration::from_secs(300),
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            log_requests: true,
            log_responses: true,
            log_transport: false,
            redact_sensitive: true,
        }
    }
}

impl RetryConfig {
    /// Calculate the delay for a given attempt number
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        if attempt == 0 {
            return Duration::from_millis(0);
        }

        let mut delay = self.initial_delay;

        if self.exponential_backoff && attempt > 1 {
            // Apply exponential backoff
            let multiplier = self.backoff_multiplier.powi((attempt - 1) as i32);
            delay = Duration::from_millis((delay.as_millis() as f64 * multiplier) as u64);
        }

        // Cap at max delay
        if delay > self.max_delay {
            delay = self.max_delay;
        }

        // Apply jitter
        if self.jitter > 0.0 {
            let jitter_ms = (delay.as_millis() as f64 * self.jitter) as u64;
            let random_offset = rand::random::<f64>() * jitter_ms as f64;
            delay = Duration::from_millis(delay.as_millis() as u64 + random_offset as u64);
        }

        // Ensure final delay never exceeds max_delay (even after jitter)
        if delay > self.max_delay {
            delay = self.max_delay;
        }

        delay
    }

    /// Check if an attempt should be retried
    pub fn should_retry(&self, attempt: u32) -> bool {
        attempt < self.max_attempts
    }
}

// Helper module for Duration serialization
mod duration_serde {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(duration.as_millis() as u64)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let millis = u64::deserialize(deserializer)?;
        Ok(Duration::from_millis(millis))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_delay_calculation() {
        let config = RetryConfig::default();

        // First attempt should have no delay
        assert_eq!(config.delay_for_attempt(0), Duration::from_millis(0));

        // Second attempt should have initial delay
        let delay1 = config.delay_for_attempt(1);
        assert!(delay1 >= config.initial_delay);

        // Third attempt should be longer with exponential backoff
        let delay2 = config.delay_for_attempt(2);
        assert!(delay2 > delay1);

        // Should not exceed max delay
        let large_delay = config.delay_for_attempt(20);
        assert!(large_delay <= config.max_delay);
    }

    #[test]
    fn test_retry_attempts() {
        let config = RetryConfig::default();

        assert!(config.should_retry(0));
        assert!(config.should_retry(1));
        assert!(config.should_retry(2));
        assert!(!config.should_retry(3)); // Default max is 3
    }

    #[test]
    fn test_config_serialization() {
        let config = ClientConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let _deserialized: ClientConfig = serde_json::from_str(&json).unwrap();
    }
}
