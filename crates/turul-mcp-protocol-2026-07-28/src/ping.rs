//! `EmptyResult` and `EmptyParams` — utility shapes for empty payloads.
//!
//! The DRAFT-2026-v1 schema declares `EmptyResult = Result` as a TypeScript
//! type alias. Rust has no analog for aliasing a structural interface, so
//! `EmptyResult` is a concrete struct mirroring the `Result` shape:
//! a required `resultType` discriminator and an optional `_meta`.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// `EmptyResult` — the `Result` shape with no extra fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmptyResult {
    /// Required `resultType` discriminator inherited from `Result`.
    #[serde(default)]
    pub result_type: crate::result_type::ResultType,

    /// Loose `_meta` per the `MetaObject` rules.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        alias = "_meta",
        rename = "_meta"
    )]
    pub meta: Option<HashMap<String, Value>>,
}

impl EmptyResult {
    pub fn new() -> Self {
        Self {
            result_type: crate::result_type::ResultType::Complete,
            meta: None,
        }
    }

    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.meta = Some(meta);
        self
    }
}

impl Default for EmptyResult {
    fn default() -> Self {
        Self::new()
    }
}

// Trait implementations for EmptyResult
use crate::traits::{HasData, HasMeta, HasResultType, RpcResult};

impl HasData for EmptyResult {
    fn data(&self) -> HashMap<String, Value> {
        HashMap::new()
    }
}

impl HasMeta for EmptyResult {
    fn meta(&self) -> Option<&crate::meta::MetaObject> {
        self.meta.as_ref()
    }
}

impl HasResultType for EmptyResult {
    fn result_type(&self) -> crate::result_type::ResultType {
        self.result_type
    }
}

impl RpcResult for EmptyResult {}

// Trait implementations for protocol compliance
use crate::traits::Params;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EmptyParams;

impl Params for EmptyParams {}

// Note: PingRequest contains method field which is handled at the request level
// The actual ping params would be EmptyParams in the params field

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // `test_ping_request` removed: PingRequest deleted with `ping` (not in DRAFT-2026-v1 schema).

    #[test]
    fn test_empty_result() {
        let result = EmptyResult::new();
        assert!(result.meta.is_none());

        let meta = HashMap::from([("test".to_string(), json!("value"))]);
        let result_with_meta = EmptyResult::new().with_meta(meta.clone());
        assert_eq!(result_with_meta.meta, Some(meta));
    }

    #[test]
    fn test_empty_result_serialization() {
        let result = EmptyResult::new();
        let json = serde_json::to_value(&result).unwrap();

        // Required `resultType` discriminator must appear on the wire.
        assert_eq!(json["resultType"], "complete");

        let meta = HashMap::from([("progressToken".to_string(), json!("test-123"))]);
        let result_with_meta = EmptyResult::new().with_meta(meta);
        let json_with_meta = serde_json::to_value(&result_with_meta).unwrap();
        assert!(json_with_meta["_meta"].is_object());
        assert_eq!(json_with_meta["resultType"], "complete");
    }
}
