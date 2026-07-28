//! Result discrimination for MCP 2026-07-28.
//!
//! Every `Result` in 2026-07-28 carries a `resultType` discriminator that allows
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
/// `ResultType = "complete" | "input_required" | string`. The trailing open
/// `| string` arm (added in the finalized schema) means an unknown
/// discriminator MUST be tolerated rather than rejected — carried verbatim in
/// [`ResultType::Other`] so it round-trips.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ResultType {
    /// The request completed successfully and the result carries final content.
    Complete,
    /// The request requires additional input; the result is an
    /// [`InputRequiredResult`](crate::input_required::InputRequiredResult)
    /// with `inputRequests` and/or `requestState`.
    InputRequired,
    /// An unknown discriminator string, preserved per the open `| string` arm.
    Other(String),
}

impl Default for ResultType {
    /// Per schema backward-compat clause: missing `resultType` ⇒ `"complete"`.
    fn default() -> Self {
        Self::Complete
    }
}

impl ResultType {
    /// Wire string for this discriminator.
    pub fn as_str(&self) -> &str {
        match self {
            ResultType::Complete => "complete",
            ResultType::InputRequired => "input_required",
            ResultType::Other(s) => s,
        }
    }
}

impl std::fmt::Display for ResultType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Serialize for ResultType {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for ResultType {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(match s.as_str() {
            "complete" => ResultType::Complete,
            "input_required" => ResultType::InputRequired,
            _ => ResultType::Other(s),
        })
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
    fn accepts_unknown_discriminator_as_other() {
        // Finalized schema: `ResultType = "complete" | "input_required" | string`.
        // An unknown discriminator is preserved verbatim, not rejected.
        let r: ResultType = serde_json::from_str("\"partial\"").unwrap();
        assert_eq!(r, ResultType::Other("partial".to_string()));
    }

    #[test]
    fn other_round_trips_verbatim() {
        let v = serde_json::to_value(ResultType::Other("partial".to_string())).unwrap();
        assert_eq!(v, "partial");
        let back: ResultType = serde_json::from_value(v).unwrap();
        assert_eq!(back, ResultType::Other("partial".to_string()));
    }

    #[test]
    fn as_str_matches_wire_string() {
        assert_eq!(ResultType::Complete.as_str(), "complete");
        assert_eq!(ResultType::InputRequired.as_str(), "input_required");
        assert_eq!(ResultType::Other("x".to_string()).as_str(), "x");
    }
}
