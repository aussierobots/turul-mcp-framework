//! Result discrimination for MCP DRAFT-2026-v1.
//!
//! Every `Result` in DRAFT-2026-v1 carries a `resultType` discriminator that allows
//! the client to determine how to parse the response — `"complete"` for normal
//! results, `"input_required"` for server-initiated multi-round-trip flows
//! (SEP-2322).
//!
//! Per schema:
//! > Servers implementing this protocol version MUST include this field.
//! > For backward compatibility, when a client receives a result from a
//! > server implementing an earlier protocol version (which does not include
//! > `resultType`), the client MUST treat the absent field as `"complete"`.
//!
//! Hence `ResultType::default() == ResultType::Complete` and the field is
//! deserialized with `#[serde(default)]` at every embedding site to honor the
//! backward-compat rule. Serialization always emits the field.

use serde::{Deserialize, Serialize};

/// Discriminator for [`Result`](crate::traits::RpcResult)-shaped responses.
///
/// `ResultType = "complete" | "input_required"`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ResultType {
    /// The request completed successfully and the result carries final content.
    Complete,
    /// The request requires additional input; the result is an
    /// [`InputRequiredResult`](crate::input_required::InputRequiredResult)
    /// with `inputRequests` and/or `requestState`.
    InputRequired,
}

impl Default for ResultType {
    /// Per schema backward-compat clause: missing `resultType` ⇒ `"complete"`.
    fn default() -> Self {
        Self::Complete
    }
}

impl ResultType {
    /// Wire string for this discriminator.
    pub const fn as_str(&self) -> &'static str {
        match self {
            ResultType::Complete => "complete",
            ResultType::InputRequired => "input_required",
        }
    }
}

impl std::fmt::Display for ResultType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complete_serializes_to_snake_case_string() {
        let v = serde_json::to_value(ResultType::Complete).unwrap();
        assert_eq!(v, "complete");
    }

    #[test]
    fn input_required_serializes_to_snake_case_string() {
        let v = serde_json::to_value(ResultType::InputRequired).unwrap();
        assert_eq!(v, "input_required");
    }

    #[test]
    fn complete_is_default_per_backward_compat_rule() {
        assert_eq!(ResultType::default(), ResultType::Complete);
    }

    #[test]
    fn parses_lowercase_strings() {
        let c: ResultType = serde_json::from_str("\"complete\"").unwrap();
        let i: ResultType = serde_json::from_str("\"input_required\"").unwrap();
        assert_eq!(c, ResultType::Complete);
        assert_eq!(i, ResultType::InputRequired);
    }

    #[test]
    fn rejects_unknown_discriminator() {
        let r: Result<ResultType, _> = serde_json::from_str("\"partial\"");
        assert!(r.is_err(), "unknown discriminator must fail to parse");
    }

    #[test]
    fn as_str_matches_wire_string() {
        assert_eq!(ResultType::Complete.as_str(), "complete");
        assert_eq!(ResultType::InputRequired.as_str(), "input_required");
    }
}
