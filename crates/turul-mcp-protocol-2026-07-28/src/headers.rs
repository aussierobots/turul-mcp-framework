//! HTTP header name constants for MCP Streamable HTTP transport (SEP-2243,
//! changelog Minor #4).
//!
//! These header names are required on Streamable HTTP POST requests per
//! SEP-2243 ("HTTP Header Standardization for Streamable HTTP Transport").
//! They are transport-layer concerns — actual enforcement lives in
//! `turul-http-mcp-server` — but the **canonical spelling** lives here as
//! protocol constants so transport implementations can import a single source
//! of truth.

/// `MCP-Protocol-Version` — REQUIRED on every Streamable HTTP POST. Must match
/// `_meta.io.modelcontextprotocol/protocolVersion` in the request body
/// (servers MUST return `400 Bad Request` on mismatch per the
/// `RequestMetaObject` schema).
pub const HTTP_HEADER_PROTOCOL_VERSION: &str = "MCP-Protocol-Version";

/// `Mcp-Method` — REQUIRED per SEP-2243 on all requests and notifications.
/// Carries the JSON-RPC `method` string at the HTTP layer to enable
/// intelligent routing without body inspection. Servers MUST reject requests
/// where this header and the body's `method` disagree.
pub const HTTP_HEADER_METHOD: &str = "Mcp-Method";

/// `Mcp-Name` — REQUIRED per SEP-2243 for `tools/call` (`params.name`),
/// `resources/read` (`params.uri`), and `prompts/get` (`params.name`).
/// Servers MUST reject requests where this header and the body's value
/// disagree.
pub const HTTP_HEADER_NAME: &str = "Mcp-Name";

/// `Mcp-Param-{name}` — header name prefix for custom headers mirrored from
/// tool parameters (SEP-2243 §Custom Headers from Tool Parameters).
///
/// The `x-mcp-header` extension property inside a tool's `inputSchema`
/// designates a parameter for mirroring and supplies the `{name}` portion;
/// the resulting wire header is `Mcp-Param-{name}: {encoded-value}`. Values
/// that cannot be represented as plain ASCII header values are Base64-encoded
/// as `=?base64?{value}?=` (see [`MCP_PARAM_BASE64_PREFIX`] /
/// [`MCP_PARAM_BASE64_SUFFIX`]). Servers that process the body MUST validate
/// that decoded header values match the corresponding body arguments.
pub const HTTP_HEADER_PARAM_PREFIX: &str = "Mcp-Param-";

/// `x-mcp-header` — the JSON Schema extension property (inside a tool's
/// `inputSchema`) that designates a parameter for header mirroring. This is a
/// schema annotation key, NOT a wire header name — the wire header it
/// produces is [`HTTP_HEADER_PARAM_PREFIX`]`{name}`.
pub const X_MCP_HEADER_SCHEMA_KEY: &str = "x-mcp-header";

/// Sentinel prefix marking a Base64-encoded `Mcp-Param-*` value
/// (`=?base64?{Base64EncodedValue}?=`, case-sensitive, lowercase).
pub const MCP_PARAM_BASE64_PREFIX: &str = "=?base64?";

/// Sentinel suffix closing a Base64-encoded `Mcp-Param-*` value.
pub const MCP_PARAM_BASE64_SUFFIX: &str = "?=";

/// JSON-RPC error code for header-validation failures. Matches the schema's
/// `HEADER_MISMATCH` const and `HeaderMismatchError` interface. Returned with
/// HTTP `400 Bad Request` when a required standard header is missing/malformed
/// or a header value does not match the corresponding request-body value.
pub const ERROR_CODE_HEADER_MISMATCH: i64 = -32020;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_names_exact_spelling() {
        // Spec spelling — case matters per HTTP normalization rules.
        assert_eq!(HTTP_HEADER_PROTOCOL_VERSION, "MCP-Protocol-Version");
        assert_eq!(HTTP_HEADER_METHOD, "Mcp-Method");
        assert_eq!(HTTP_HEADER_NAME, "Mcp-Name");
        assert_eq!(HTTP_HEADER_PARAM_PREFIX, "Mcp-Param-");
        assert_eq!(X_MCP_HEADER_SCHEMA_KEY, "x-mcp-header");
    }

    #[test]
    fn base64_sentinel_exact_spelling() {
        // Markers are case-sensitive and MUST appear exactly as shown.
        assert_eq!(MCP_PARAM_BASE64_PREFIX, "=?base64?");
        assert_eq!(MCP_PARAM_BASE64_SUFFIX, "?=");
    }

    #[test]
    fn header_mismatch_code_matches_schema() {
        assert_eq!(ERROR_CODE_HEADER_MISMATCH, -32020);
        assert!((-32099..=-32000).contains(&ERROR_CODE_HEADER_MISMATCH));
    }
}

/// One `x-mcp-header` annotation discovered in a tool's `inputSchema`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpParamBinding {
    /// The `{name}` portion: the wire header is `Mcp-Param-{name}`.
    pub header_name: String,
    /// JSON-pointer path of the annotated property RELATIVE to the arguments
    /// object (e.g. `/region`, or `/nested/field` for nested annotations).
    pub argument_pointer: String,
    /// The annotated property's declared `type` (`string`/`integer`/`boolean`).
    pub schema_type: String,
}

/// Scan a tool `inputSchema` for `x-mcp-header` annotations (any nesting depth)
/// and validate the SEP-2243 constraints. A violation makes the WHOLE tool
/// definition invalid — Streamable HTTP clients MUST exclude such a tool from
/// `tools/list` results.
///
/// Constraints: non-empty; RFC 9110 `1*tchar` field-name syntax (which excludes
/// CR/LF and all control characters); case-insensitively unique within the
/// schema; only on `string`/`integer`/`boolean` properties (`number` is not
/// permitted).
pub fn scan_x_mcp_headers(
    input_schema: &serde_json::Value,
) -> Result<Vec<McpParamBinding>, String> {
    fn is_tchar(c: char) -> bool {
        c.is_ascii_alphanumeric() || "!#$%&'*+-.^_`|~".contains(c)
    }

    fn walk(
        properties: &serde_json::Value,
        pointer_prefix: &str,
        out: &mut Vec<McpParamBinding>,
    ) -> Result<(), String> {
        let Some(map) = properties.as_object() else {
            return Ok(());
        };
        for (prop_name, prop_schema) in map {
            let pointer = format!(
                "{pointer_prefix}/{}",
                prop_name.replace('~', "~0").replace('/', "~1")
            );
            if let Some(annotation) = prop_schema.get(X_MCP_HEADER_SCHEMA_KEY) {
                let name = annotation
                    .as_str()
                    .ok_or_else(|| format!("x-mcp-header on '{pointer}' must be a string"))?;
                if name.is_empty() {
                    return Err(format!("x-mcp-header on '{pointer}' must not be empty"));
                }
                if !name.chars().all(is_tchar) {
                    return Err(format!(
                        "x-mcp-header '{name}' on '{pointer}' violates field-name token syntax"
                    ));
                }
                let schema_type = prop_schema
                    .get("type")
                    .and_then(|t| t.as_str())
                    .unwrap_or_default()
                    .to_string();
                if !matches!(schema_type.as_str(), "string" | "integer" | "boolean") {
                    return Err(format!(
                        "x-mcp-header '{name}' on '{pointer}' targets type '{schema_type}' — \
                         only string/integer/boolean parameters are permitted"
                    ));
                }
                out.push(McpParamBinding {
                    header_name: name.to_string(),
                    argument_pointer: pointer.clone(),
                    schema_type,
                });
            }
            // Nested annotations: object properties may carry their own.
            if let Some(nested) = prop_schema.get("properties") {
                walk(nested, &pointer, out)?;
            }
        }
        Ok(())
    }

    let mut out = Vec::new();
    if let Some(properties) = input_schema.get("properties") {
        walk(properties, "", &mut out)?;
    }
    let mut seen = std::collections::HashSet::new();
    for binding in &out {
        if !seen.insert(binding.header_name.to_ascii_lowercase()) {
            return Err(format!(
                "x-mcp-header '{}' is not case-insensitively unique",
                binding.header_name
            ));
        }
    }
    Ok(out)
}

/// Encode a parameter value for an `Mcp-Param-*` header per SEP-2243.
///
/// `string` is used as-is, `integer` as its decimal representation (must be
/// within the JavaScript safe range), `boolean` as lowercase. Values that
/// cannot ride as plain ASCII (non-ASCII, control characters, leading/trailing
/// whitespace) — or that themselves match the Base64 sentinel pattern — are
/// Base64-encoded as `=?base64?{value}?=`. Returns `None` for value shapes the
/// contract does not permit (objects, arrays, floats, out-of-range integers).
pub fn encode_param_value(value: &serde_json::Value) -> Option<String> {
    const JS_SAFE_MAX: i64 = 9_007_199_254_740_991; // 2^53 - 1
    let plain = match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => {
            let i = n.as_i64()?;
            if !(-JS_SAFE_MAX..=JS_SAFE_MAX).contains(&i) {
                return None;
            }
            i.to_string()
        }
        _ => return None,
    };

    let needs_b64 = plain.chars().any(|c| !matches!(c, '\x20'..='\x7e'))
        || plain != plain.trim()
        || (plain.starts_with(MCP_PARAM_BASE64_PREFIX) && plain.ends_with(MCP_PARAM_BASE64_SUFFIX));
    if needs_b64 {
        use base64::Engine as _;
        let encoded = base64::engine::general_purpose::STANDARD.encode(plain.as_bytes());
        Some(format!(
            "{MCP_PARAM_BASE64_PREFIX}{encoded}{MCP_PARAM_BASE64_SUFFIX}"
        ))
    } else {
        Some(plain)
    }
}

/// Decode an `Mcp-Param-*` header value: unwrap the `=?base64?…?=` sentinel
/// when present, otherwise return the value verbatim.
pub fn decode_param_value(raw: &str) -> Result<String, String> {
    if let Some(inner) = raw
        .strip_prefix(MCP_PARAM_BASE64_PREFIX)
        .and_then(|r| r.strip_suffix(MCP_PARAM_BASE64_SUFFIX))
    {
        use base64::Engine as _;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(inner)
            .map_err(|e| format!("invalid Base64 in Mcp-Param value: {e}"))?;
        String::from_utf8(bytes).map_err(|e| format!("Mcp-Param value is not UTF-8: {e}"))
    } else {
        Ok(raw.to_string())
    }
}

#[cfg(test)]
mod param_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn encoding_matches_the_spec_examples() {
        // The five examples from §Value Encoding.
        assert_eq!(encode_param_value(&json!("us-west1")).unwrap(), "us-west1");
        assert_eq!(
            encode_param_value(&json!("Hello, 世界")).unwrap(),
            "=?base64?SGVsbG8sIOS4lueVjA==?="
        );
        assert_eq!(
            encode_param_value(&json!(" padded ")).unwrap(),
            "=?base64?IHBhZGRlZCA=?="
        );
        assert_eq!(
            encode_param_value(&json!("line1\nline2")).unwrap(),
            "=?base64?bGluZTEKbGluZTI=?="
        );
        assert_eq!(
            encode_param_value(&json!("=?base64?literal?=")).unwrap(),
            "=?base64?PT9iYXNlNjQ/bGl0ZXJhbD89?="
        );
    }

    #[test]
    fn integer_and_boolean_conversion() {
        assert_eq!(encode_param_value(&json!(42)).unwrap(), "42");
        assert_eq!(encode_param_value(&json!(-7)).unwrap(), "-7");
        assert_eq!(encode_param_value(&json!(true)).unwrap(), "true");
        assert_eq!(encode_param_value(&json!(false)).unwrap(), "false");
        // Floats and out-of-safe-range integers are not permitted.
        assert!(encode_param_value(&json!(1.5)).is_none());
        assert!(encode_param_value(&json!(9_007_199_254_740_992_i64)).is_none());
    }

    #[test]
    fn decode_round_trips() {
        for v in ["us-west1", "Hello, 世界", " padded ", "line1\nline2"] {
            let encoded = encode_param_value(&json!(v)).unwrap();
            assert_eq!(decode_param_value(&encoded).unwrap(), v);
        }
    }

    #[test]
    fn scan_finds_annotations_and_enforces_constraints() {
        let schema = json!({
            "type": "object",
            "properties": {
                "region": { "type": "string", "x-mcp-header": "Region" },
                "query":  { "type": "string" }
            }
        });
        let bindings = scan_x_mcp_headers(&schema).unwrap();
        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].header_name, "Region");
        assert_eq!(bindings[0].argument_pointer, "/region");

        // number type is not permitted.
        let bad = json!({"properties": {"n": {"type": "number", "x-mcp-header": "N"}}});
        assert!(scan_x_mcp_headers(&bad).is_err());

        // case-insensitive uniqueness.
        let dup = json!({"properties": {
            "a": {"type": "string", "x-mcp-header": "Region"},
            "b": {"type": "string", "x-mcp-header": "region"}
        }});
        assert!(scan_x_mcp_headers(&dup).is_err());

        // control characters / non-tchar.
        let ctrl = json!({"properties": {"a": {"type": "string", "x-mcp-header": "Re gion"}}});
        assert!(scan_x_mcp_headers(&ctrl).is_err());

        // nested depth.
        let nested = json!({"properties": {
            "outer": {"type": "object", "properties": {
                "inner": {"type": "boolean", "x-mcp-header": "Inner"}
            }}
        }});
        let bindings = scan_x_mcp_headers(&nested).unwrap();
        assert_eq!(bindings[0].argument_pointer, "/outer/inner");
    }

    /// Scoped claim only: `scan_x_mcp_headers` never EMITS an `Mcp-Param-*`
    /// header binding for an `x-mcp-header` annotation nested under `items`,
    /// composition keywords (`oneOf`/`anyOf`/`allOf`), or `$ref` — `walk` only
    /// recurses via a property's own `properties` map, so these are silently
    /// skipped (not an error) while a genuinely top-level property is found.
    ///
    /// This does NOT prove the separate SEP-2243 MUST — "a Streamable HTTP
    /// client MUST reject/exclude the WHOLE tool definition" when such a
    /// misplaced annotation is present. That requires first *detecting* the
    /// annotation to reject on, which `walk` cannot do (it never inspects
    /// `items`/composition/`$ref` at all, so it has no way to see a violation
    /// there to act on). That MUST is unimplemented in this workspace; do not
    /// read this test as covering it.
    #[test]
    fn x_mcp_header_emits_only_statically_reachable_properties() {
        // Array items — not statically reachable (any array element could
        // carry it, not a single named property).
        let via_items = json!({"properties": {
            "tags": {"type": "array", "items": {"type": "string", "x-mcp-header": "Tag"}}
        }});
        assert!(
            scan_x_mcp_headers(&via_items).unwrap().is_empty(),
            "x-mcp-header under `items` must not be found"
        );

        // allOf composition.
        let via_all_of = json!({"properties": {
            "region": {"allOf": [{"type": "string", "x-mcp-header": "Region"}]}
        }});
        assert!(
            scan_x_mcp_headers(&via_all_of).unwrap().is_empty(),
            "x-mcp-header under `allOf` must not be found"
        );

        // anyOf composition.
        let via_any_of = json!({"properties": {
            "region": {"anyOf": [{"type": "string", "x-mcp-header": "Region"}]}
        }});
        assert!(
            scan_x_mcp_headers(&via_any_of).unwrap().is_empty(),
            "x-mcp-header under `anyOf` must not be found"
        );

        // $ref indirection.
        let via_ref = json!({
            "properties": { "region": {"$ref": "#/$defs/Region"} },
            "$defs": { "Region": {"type": "string", "x-mcp-header": "Region"} }
        });
        assert!(
            scan_x_mcp_headers(&via_ref).unwrap().is_empty(),
            "x-mcp-header reached only via $ref must not be found"
        );

        // Control: a genuinely top-level property IS found.
        let top_level = json!({"properties": {
            "region": {"type": "string", "x-mcp-header": "Region"}
        }});
        let bindings = scan_x_mcp_headers(&top_level).unwrap();
        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].header_name, "Region");
    }
}
