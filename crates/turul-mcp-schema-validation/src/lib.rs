//! JSON Schema 2020-12 dialect validation for MCP tool `inputSchema`.
//!
//! 2026-07-28 (SEP-2106) opens `Tool.inputSchema` to the full JSON Schema
//! 2020-12 vocabulary (`oneOf`/`anyOf`/`allOf`/`$ref`/`$defs`/conditionals).
//! The spec states the MUST this crate satisfies:
//! "Clients and servers MUST validate schemas according to their declared or
//! default dialect and MUST handle unsupported dialects gracefully by
//! returning an appropriate error." [`validate_tool_input_schema`]'s dialect
//! check and 2020-12 meta-validation compile step are that MUST.
//!
//! Two additional checks in this crate are **framework security policy, not
//! a JSON Schema spec requirement**: rejecting a remote `$ref` (prevents
//! SSRF — fetching attacker-controlled schema content over the network) and
//! enforcing size/nesting/composition-depth bounds (prevents resource-
//! exhaustion DoS from an oversized or pathologically nested schema). Both
//! are deliberate hardening layered on top of the spec MUST, not mandated by
//! it.
//!
//! This crate is used by both trust boundaries: a server MUST NOT advertise
//! an invalid `inputSchema` (enforced at tool registration), and a client
//! MUST exclude a tool whose `inputSchema` is invalid from `tools/list`.

/// Maximum serialized size of a tool `inputSchema`, in bytes. Framework
/// security policy (DoS hardening), not a JSON Schema spec requirement.
pub const MAX_SCHEMA_BYTES: usize = 256 * 1024;

/// Reserved for a future resolved-`$ref`-chain bound. The current bounds walk
/// is deliberately cycle-safe: it traverses only the literal JSON document
/// tree and never resolves/follows a `$ref` (see [`validate_tool_input_schema`]
/// docs) — a legitimate cyclic local `$ref` (e.g. a recursive tree-node
/// schema) MUST pass. Because `$ref` targets are never resolved, there is no
/// "chain" to measure without reintroducing the cycle risk this bound would
/// otherwise guard against, so this constant is not currently exercised.
pub const MAX_REF_DEPTH: usize = 32;

/// Maximum nesting depth through composition keywords (`allOf`/`anyOf`/`oneOf`/
/// `not`/`if`/`then`/`else`) and structural keywords (`items`/`properties`).
/// Framework security policy (DoS hardening), not a JSON Schema spec requirement.
pub const MAX_COMPOSITION_DEPTH: usize = 32;

const DIALECT_2020_12: &str = "https://json-schema.org/draft/2020-12/schema";

/// Why a tool `inputSchema` failed validation.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SchemaValidationError {
    #[error("inputSchema is {bytes} bytes, exceeding the {limit}-byte limit")]
    TooLarge { bytes: usize, limit: usize },

    #[error("inputSchema {kind} nesting exceeds the limit of {limit}")]
    TooDeep { kind: &'static str, limit: usize },

    #[error(
        "inputSchema declares unsupported dialect '{0}' (only JSON Schema 2020-12 is supported)"
    )]
    UnsupportedDialect(String),

    #[error("inputSchema is not a well-formed JSON Schema 2020-12 document: {0}")]
    Malformed(String),

    #[error("remote $ref resolution is disabled by policy: '{0}'")]
    RemoteRef(String),
}

/// Validate a tool `inputSchema`: size bound, `$ref`/composition
/// bounds (checked over the literal JSON tree only — see below), remote-`$ref`
/// rejection, dialect check, then 2020-12 meta-validation. Checks run
/// cheapest-first so a pathological schema is rejected before the (relatively)
/// expensive compile step.
///
/// **Cycle safety**: the bounds walk traverses only the literal JSON document
/// tree (a finite structure by construction) — it does not resolve or follow
/// `$ref` targets. A `$ref` value is inspected as a string only (to detect a
/// remote reference); it is never dereferenced during this pass. This means a
/// legitimate cyclic local `$ref` (e.g. `{"$ref": "#/$defs/Node"}` inside its
/// own `$defs/Node` definition — a recursive tree-node schema) passes the
/// bounds check; `jsonschema` resolves and validates such references safely
/// at compile time in the step that follows.
pub fn validate_tool_input_schema(schema: &serde_json::Value) -> Result<(), SchemaValidationError> {
    let bytes = serde_json::to_vec(schema)
        .map(|v| v.len())
        .unwrap_or(usize::MAX);
    if bytes > MAX_SCHEMA_BYTES {
        return Err(SchemaValidationError::TooLarge {
            bytes,
            limit: MAX_SCHEMA_BYTES,
        });
    }

    check_bounds(schema, 0)?;
    check_dialect(schema)?;
    compile_2020_12_no_remote(schema)?;

    Ok(())
}

fn check_dialect(schema: &serde_json::Value) -> Result<(), SchemaValidationError> {
    match schema.get("$schema").and_then(|v| v.as_str()) {
        None => Ok(()),
        Some(dialect) if dialect.trim_end_matches('#') == DIALECT_2020_12 => Ok(()),
        Some(other) => Err(SchemaValidationError::UnsupportedDialect(other.to_string())),
    }
}

fn is_remote_ref(r: &str) -> bool {
    r.starts_with("http://") || r.starts_with("https://") || r.contains("://")
}

/// Literal-tree-only walk: recurses through `items`/`properties`/composition/
/// conditional keywords, never through a resolved `$ref` target. A `$ref`
/// value is checked for remoteness only.
fn check_bounds(node: &serde_json::Value, comp_depth: usize) -> Result<(), SchemaValidationError> {
    let Some(obj) = node.as_object() else {
        return Ok(());
    };

    if let Some(r) = obj.get("$ref").and_then(|v| v.as_str())
        && is_remote_ref(r)
    {
        return Err(SchemaValidationError::RemoteRef(r.to_string()));
    }

    for key in ["not", "if", "then", "else", "items"] {
        if let Some(sub) = obj.get(key) {
            let comp_depth = comp_depth + 1;
            if comp_depth > MAX_COMPOSITION_DEPTH {
                return Err(SchemaValidationError::TooDeep {
                    kind: "composition",
                    limit: MAX_COMPOSITION_DEPTH,
                });
            }
            check_bounds(sub, comp_depth)?;
        }
    }

    for key in ["allOf", "anyOf", "oneOf"] {
        if let Some(subs) = obj.get(key).and_then(|v| v.as_array()) {
            let comp_depth = comp_depth + 1;
            if comp_depth > MAX_COMPOSITION_DEPTH {
                return Err(SchemaValidationError::TooDeep {
                    kind: "composition",
                    limit: MAX_COMPOSITION_DEPTH,
                });
            }
            for sub in subs {
                check_bounds(sub, comp_depth)?;
            }
        }
    }

    if let Some(props) = obj.get("properties").and_then(|v| v.as_object()) {
        let comp_depth = comp_depth + 1;
        if comp_depth > MAX_COMPOSITION_DEPTH {
            return Err(SchemaValidationError::TooDeep {
                kind: "composition",
                limit: MAX_COMPOSITION_DEPTH,
            });
        }
        for sub in props.values() {
            check_bounds(sub, comp_depth)?;
        }
    }

    Ok(())
}

/// Retriever that refuses every lookup — remote `$ref` resolution is disabled
/// by policy, in addition to the crate-feature-level guard (no `resolve-http`/
/// `resolve-file`/`reqwest` features are enabled on the `jsonschema` dependency).
struct BlockingRetriever;

impl jsonschema::Retrieve for BlockingRetriever {
    fn retrieve(
        &self,
        uri: &jsonschema::Uri<String>,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        Err(format!("remote $ref resolution is disabled: {uri}").into())
    }
}

fn compile_2020_12_no_remote(schema: &serde_json::Value) -> Result<(), SchemaValidationError> {
    jsonschema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .with_retriever(BlockingRetriever)
        .build(schema)
        .map(|_| ())
        .map_err(|e| SchemaValidationError::Malformed(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn absent_schema_dialect_is_accepted() {
        let schema = json!({"type": "object", "properties": {"a": {"type": "string"}}});
        assert_eq!(validate_tool_input_schema(&schema), Ok(()));
    }

    #[test]
    fn canonical_2020_12_dialect_accepted_with_and_without_trailing_hash() {
        for uri in [
            DIALECT_2020_12,
            "https://json-schema.org/draft/2020-12/schema#",
        ] {
            let schema = json!({"$schema": uri, "type": "object"});
            assert_eq!(validate_tool_input_schema(&schema), Ok(()), "uri={uri}");
        }
    }

    #[test]
    fn unsupported_dialect_is_rejected() {
        let schema = json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object"
        });
        match validate_tool_input_schema(&schema) {
            Err(SchemaValidationError::UnsupportedDialect(d)) => {
                assert_eq!(d, "http://json-schema.org/draft-07/schema#");
            }
            other => panic!("expected UnsupportedDialect, got {other:?}"),
        }
    }

    #[test]
    fn malformed_schema_is_rejected() {
        let schema = json!({"type": 123});
        match validate_tool_input_schema(&schema) {
            Err(SchemaValidationError::Malformed(_)) => {}
            other => panic!("expected Malformed, got {other:?}"),
        }
    }

    #[test]
    fn oversized_schema_names_the_limit_in_its_message() {
        let big_description = "x".repeat(MAX_SCHEMA_BYTES + 1);
        let schema = json!({"type": "object", "description": big_description});
        let err = validate_tool_input_schema(&schema).unwrap_err();
        match err {
            SchemaValidationError::TooLarge { limit, .. } => assert_eq!(limit, MAX_SCHEMA_BYTES),
            other => panic!("expected TooLarge, got {other:?}"),
        }
        let msg = err.to_string();
        assert!(
            msg.contains(&MAX_SCHEMA_BYTES.to_string()),
            "message must name the exceeded limit: {msg}"
        );
    }

    #[test]
    fn excessive_composition_nesting_names_the_limit_in_its_message() {
        // Build allOf nested MAX_COMPOSITION_DEPTH + 1 deep: each level wraps
        // the next in {"allOf": [ ... ]}.
        let mut inner = json!({"type": "string"});
        for _ in 0..(MAX_COMPOSITION_DEPTH + 1) {
            inner = json!({"allOf": [inner]});
        }
        let err = validate_tool_input_schema(&inner).unwrap_err();
        match &err {
            SchemaValidationError::TooDeep { kind, limit } => {
                assert_eq!(*kind, "composition");
                assert_eq!(*limit, MAX_COMPOSITION_DEPTH);
            }
            other => panic!("expected TooDeep, got {other:?}"),
        }
        let msg = err.to_string();
        assert!(
            msg.contains("composition") && msg.contains(&MAX_COMPOSITION_DEPTH.to_string()),
            "message must name the kind and the exceeded limit: {msg}"
        );
    }

    #[test]
    fn local_ref_is_accepted_not_flagged_as_remote() {
        let schema = json!({
            "type": "object",
            "properties": {"a": {"$ref": "#/$defs/Foo"}},
            "$defs": {"Foo": {"type": "string"}}
        });
        assert_eq!(validate_tool_input_schema(&schema), Ok(()));
    }

    #[test]
    fn recursive_local_ref_schema_is_accepted() {
        // A tree-node schema referencing itself through `items` — a cyclic
        // local $ref that is legal JSON Schema. The bounds walk must not
        // follow it (cycle safety), so this must pass; jsonschema resolves
        // the recursion safely at compile time.
        let schema = json!({
            "type": "object",
            "properties": { "root": { "$ref": "#/$defs/Node" } },
            "$defs": {
                "Node": {
                    "type": "object",
                    "properties": {
                        "value": { "type": "string" },
                        "children": {
                            "type": "array",
                            "items": { "$ref": "#/$defs/Node" }
                        }
                    }
                }
            }
        });
        assert_eq!(validate_tool_input_schema(&schema), Ok(()));
    }

    #[test]
    fn remote_ref_is_rejected_and_names_the_ref_and_policy_reason() {
        let schema = json!({
            "type": "object",
            "properties": {"a": {"$ref": "https://evil.example/schema.json"}}
        });
        let err = validate_tool_input_schema(&schema).unwrap_err();
        match &err {
            SchemaValidationError::RemoteRef(uri) => {
                assert_eq!(uri, "https://evil.example/schema.json");
            }
            other => panic!("expected RemoteRef, got {other:?}"),
        }
        let msg = err.to_string();
        assert!(
            msg.contains("https://evil.example/schema.json"),
            "message must name the offending $ref: {msg}"
        );
        assert!(
            msg.contains("disabled") && msg.contains("policy"),
            "message must state this is a policy rejection, not a spec MUST: {msg}"
        );
    }
}
