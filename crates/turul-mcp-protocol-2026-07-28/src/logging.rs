//! MCP Logging Protocol Types
//!
//! # Deprecation status (DRAFT-2026-v1)
//!
//! Per SEP-2577, the Logging RPC surface (`notifications/message`) is
//! **deprecated** in this revision. New implementations SHOULD NOT adopt it.
//! Earliest removal: first revision released on or after **2027-07-28**.
//!
//! - Stdio transports: log to `stderr` instead.
//! - Other transports: use [OpenTelemetry](https://opentelemetry.io/) for
//!   observability — trace-context keys ride in `_meta` per SEP-414 (see
//!   `META_KEY_TRACEPARENT` etc. in [`crate::meta`]).
//!
//! The per-request log level mechanism ([`crate::meta::RequestMetaObject::log_level`])
//! is the replacement opt-in. `logging/setLevel` was REMOVED entirely in
//! DRAFT-2026-v1 — clients now declare desired level per-request.
//!
//! The wire-payload types ([`crate::notifications::LoggingMessageNotification`]
//! and [`crate::notifications::LoggingMessageNotificationParams`]) live in
//! [`crate::notifications`] alongside the other notification payloads. This
//! module carries only the wire-value enum [`LoggingLevel`] (still used by
//! the non-deprecated [`crate::meta::RequestMetaObject::log_level`] field).

use serde::{Deserialize, Serialize};

/// Logging levels (per MCP spec)
/// Maps to syslog message severities as specified in RFC-5424.
///
/// NOT deprecated: this enum is the wire-value type for both the deprecated
/// `notifications/message` surface AND the non-deprecated per-request
/// `_meta.io.modelcontextprotocol/logLevel` opt-in. Deprecating it would
/// cascade a warning into the replacement mechanism.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LoggingLevel {
    Debug,
    Info,
    Notice,
    Warning,
    Error,
    Critical,
    Alert,
    Emergency,
}

/// Type alias for compatibility (per MCP spec)
pub type LogLevel = LoggingLevel;

/// Convenience constructors for LoggingLevel
impl LoggingLevel {
    /// Get logging level priority (0 = debug, 7 = emergency)
    pub fn priority(&self) -> u8 {
        match self {
            LoggingLevel::Debug => 0,
            LoggingLevel::Info => 1,
            LoggingLevel::Notice => 2,
            LoggingLevel::Warning => 3,
            LoggingLevel::Error => 4,
            LoggingLevel::Critical => 5,
            LoggingLevel::Alert => 6,
            LoggingLevel::Emergency => 7,
        }
    }

    /// Check if this level should be logged at the given threshold
    pub fn should_log(&self, threshold: LoggingLevel) -> bool {
        self.priority() >= threshold.priority()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logging_level_priority() {
        assert_eq!(LoggingLevel::Debug.priority(), 0);
        assert_eq!(LoggingLevel::Emergency.priority(), 7);

        assert!(LoggingLevel::Error.should_log(LoggingLevel::Warning));
        assert!(!LoggingLevel::Info.should_log(LoggingLevel::Error));
    }
}
