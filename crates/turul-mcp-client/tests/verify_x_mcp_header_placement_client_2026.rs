//! Client-side `x-mcp-header` placement rules and tool-schema exclusion.
//!
//! Drives a real `McpClient` (2026-07-28) over wiremock through
//! `parse_list_tools` (crates/turul-mcp-client/src/protocol/v2026_07_28.rs).
//! The x-mcp-header positional rule is an ALLOWLIST: an annotation is valid
//! ONLY when every step from the schema root to it is a `properties/<name>`
//! step; ANYTHING else (applicators, $defs referenced or not) makes the tool
//! invalid and it MUST be excluded from tools/list. The four instance-data
//! keywords (const/default/enum/examples) carry literal JSON, not subschemas,
//! so a coincidental "x-mcp-header" key inside them must NOT trip exclusion.
//!
//! Each completeness/precision case is proven at BOTH layers:
//!  (wire) the real client's list_tools() excludes/keeps the tool, and
//!  (unit) find_misplaced_x_mcp_header(inputSchema) flags/does-not-flag it —
//! the unit assertion is unambiguous even where a surviving tool would trip a
//! downstream 2025-11-25 remap quirk.

use serde_json::{Value, json};
use turul_mcp_client::config::ClientConfig;
use turul_mcp_client::transport::http::HttpTransport;
use turul_mcp_client::{McpClient, McpVersion};
use turul_mcp_protocol_2026_07_28::headers::find_misplaced_x_mcp_header;
use wiremock::matchers::{body_partial_json, header, method};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn meta_match(rpc_method: &str) -> Value {
    json!({
        "method": rpc_method,
        "params": { "_meta": { "io.modelcontextprotocol/protocolVersion": "2026-07-28" } }
    })
}

async fn mount_2026_result(server: &MockServer, rpc_method: &str, result: Value) {
    Mock::given(method("POST"))
        .and(header("Mcp-Method", rpc_method))
        .and(header("MCP-Protocol-Version", "2026-07-28"))
        .and(body_partial_json(meta_match(rpc_method)))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json")
                .set_body_json(json!({ "jsonrpc": "2.0", "id": "x", "result": result })),
        )
        .mount(server)
        .await;
}

async fn start_2026_server() -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(header("Mcp-Method", "server/discover"))
        .and(header("MCP-Protocol-Version", "2026-07-28"))
        .and(body_partial_json(json!({"method": "server/discover"})))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json")
                .set_body_json(json!({
                    "jsonrpc": "2.0", "id": "d",
                    "result": {
                        "resultType": "complete", "ttlMs": 0, "cacheScope": "public",
                        "supportedVersions": ["2026-07-28"], "capabilities": {},
                        "_meta": { "io.modelcontextprotocol/serverInfo": { "name": "mock-2026", "version": "1.0.0" } }
                    }
                })),
        )
        .mount(&server)
        .await;
    server
}

async fn connect_2026(server: &MockServer) -> McpClient {
    let url = format!("{}/mcp", server.uri());
    let transport = Box::new(HttpTransport::new(&url).unwrap());
    let client = McpClient::new(transport, ClientConfig::default());
    client.connect().await.expect("connect to 2026 server");
    assert_eq!(
        client.negotiated_version().await,
        Some(McpVersion::V2026_07_28)
    );
    client
}

/// Mount the given `(name, inputSchema)` tools and return the surviving names.
async fn survivors(tools: Vec<(&str, Value)>) -> Vec<String> {
    let server = start_2026_server().await;
    let arr: Vec<Value> = tools
        .iter()
        .map(|(n, s)| json!({ "name": n, "inputSchema": s }))
        .collect();
    mount_2026_result(
        &server,
        "tools/list",
        json!({ "resultType": "complete", "ttlMs": 0, "cacheScope": "public", "tools": arr }),
    )
    .await;
    let client = connect_2026(&server).await;
    client
        .list_tools()
        .await
        .expect("tools/list")
        .into_iter()
        .map(|t| t.name)
        .collect()
}

/// Unit-level detector: is this inputSchema flagged as having a misplaced
/// x-mcp-header?
fn flagged(input_schema: &Value) -> bool {
    find_misplaced_x_mcp_header(input_schema).is_some()
}

fn good_schema() -> Value {
    json!({ "type": "object", "properties": { "q": { "type": "string" } } })
}

// ================= schema-level exclusion from tools/list ==================

#[tokio::test]
async fn unsupported_dialect_tool_excluded() {
    let bad = json!({
        "type": "object",
        "$schema": "http://json-schema.org/draft-07/schema#",
        "properties": { "a": { "type": "string" } }
    });
    let names = survivors(vec![("keep_me", good_schema()), ("draft07_tool", bad)]).await;
    assert!(names.contains(&"keep_me".to_string()), "{names:?}");
    assert!(
        !names.contains(&"draft07_tool".to_string()),
        "unsupported dialect excluded: {names:?}"
    );
}

#[tokio::test]
async fn remote_ref_tool_excluded() {
    let bad = json!({
        "type": "object",
        "properties": { "a": { "$ref": "https://attacker.example/evil.json" } }
    });
    let names = survivors(vec![("keep_me", good_schema()), ("remote_ref_tool", bad)]).await;
    assert!(names.contains(&"keep_me".to_string()), "{names:?}");
    assert!(
        !names.contains(&"remote_ref_tool".to_string()),
        "remote $ref excluded: {names:?}"
    );
}

// ========== COMPLETENESS — annotation off the properties chain =============
// Each: (unit) detector flags it AND (wire) tool excluded, good tool survives.

async fn assert_excluded(case: &str, bad_schema: Value) {
    assert!(
        flagged(&bad_schema),
        "[{case}] detector unit MUST flag the misplaced annotation: {bad_schema}"
    );
    let names = survivors(vec![("good", good_schema()), ("bad", bad_schema)]).await;
    assert!(
        names.contains(&"good".to_string()),
        "[{case}] valid tool must survive: {names:?}"
    );
    assert!(
        !names.contains(&"bad".to_string()),
        "[{case}] misplaced-annotation tool MUST be excluded: {names:?}"
    );
}

#[tokio::test]
async fn prefix_items_excluded() {
    assert_excluded(
        "prefixItems",
        json!({
            "type": "object",
            "properties": { "pair": { "type": "array",
                "prefixItems": [ { "type": "string", "x-mcp-header": "Sneaky" } ] } }
        }),
    )
    .await;
}

#[tokio::test]
async fn pattern_properties_excluded() {
    assert_excluded(
        "patternProperties",
        json!({
            "type": "object",
            "properties": { "cfg": { "type": "object",
                "patternProperties": { "^x-": { "type": "string", "x-mcp-header": "Sneaky" } } } }
        }),
    )
    .await;
}

#[tokio::test]
async fn additional_properties_excluded() {
    assert_excluded(
        "additionalProperties",
        json!({
            "type": "object",
            "properties": { "cfg": { "type": "object",
                "additionalProperties": { "type": "string", "x-mcp-header": "Sneaky" } } }
        }),
    )
    .await;
}

#[tokio::test]
async fn contains_excluded() {
    assert_excluded(
        "contains",
        json!({
            "type": "object",
            "properties": { "list": { "type": "array",
                "contains": { "type": "string", "x-mcp-header": "Sneaky" } } }
        }),
    )
    .await;
}

#[tokio::test]
async fn property_names_excluded() {
    assert_excluded(
        "propertyNames",
        json!({
            "type": "object",
            "properties": { "cfg": { "type": "object",
                "propertyNames": { "type": "string", "x-mcp-header": "Sneaky" } } }
        }),
    )
    .await;
}

#[tokio::test]
async fn unevaluated_properties_excluded() {
    assert_excluded(
        "unevaluatedProperties",
        json!({
            "type": "object",
            "properties": { "cfg": { "type": "object",
                "unevaluatedProperties": { "type": "string", "x-mcp-header": "Sneaky" } } }
        }),
    )
    .await;
}

#[tokio::test]
async fn dependent_schemas_excluded() {
    assert_excluded(
        "dependentSchemas",
        json!({
            "type": "object",
            "properties": { "a": { "type": "string" } },
            "dependentSchemas": { "a": { "properties": {
                "b": { "type": "string", "x-mcp-header": "Sneaky" } } } }
        }),
    )
    .await;
}

#[tokio::test]
async fn defs_referenced_excluded() {
    assert_excluded(
        "$defs (referenced via $ref)",
        json!({
            "type": "object",
            "properties": { "region": { "$ref": "#/$defs/Region" } },
            "$defs": { "Region": { "type": "string", "x-mcp-header": "Region" } }
        }),
    )
    .await;
}

#[tokio::test]
async fn defs_unreferenced_excluded() {
    // Nothing $ref's #/$defs/Region — still off the pure-properties chain, so
    // the tool is invalid and MUST be excluded.
    assert_excluded(
        "$defs (unreferenced)",
        json!({
            "type": "object",
            "properties": { "region": { "type": "string" } },
            "$defs": { "Region": { "type": "string", "x-mcp-header": "Region" } }
        }),
    )
    .await;
}

// ================= PRECISION — no false positives (KEPT) ===================
// Each: (unit) detector does NOT flag AND (wire) tool survives.

async fn assert_kept(case: &str, schema: Value) {
    assert!(
        !flagged(&schema),
        "[{case}] detector unit MUST NOT flag a valid placement: {schema}"
    );
    let names = survivors(vec![("kept", schema)]).await;
    assert!(
        names.contains(&"kept".to_string()),
        "[{case}] validly-placed tool must survive: {names:?}"
    );
}

#[tokio::test]
async fn precision_pure_properties_nesting_kept() {
    assert_kept(
        "pure properties/outer/properties/inner",
        json!({
            "type": "object",
            "properties": { "outer": { "type": "object", "properties": {
                "inner": { "type": "string", "x-mcp-header": "Inner" } } } }
        }),
    )
    .await;
}

#[tokio::test]
async fn precision_const_kept() {
    assert_kept(
        "const carries literal x-mcp-header key",
        json!({
            "type": "object",
            "properties": { "region": { "type": "object",
                "const": { "x-mcp-header": "NotAnAnnotation" } } }
        }),
    )
    .await;
}

#[tokio::test]
async fn precision_default_kept() {
    assert_kept(
        "default carries literal x-mcp-header key",
        json!({
            "type": "object",
            "properties": { "region": { "type": "object",
                "default": { "x-mcp-header": "NotAnAnnotation" } } }
        }),
    )
    .await;
}

#[tokio::test]
async fn precision_enum_kept() {
    assert_kept(
        "enum member carries literal x-mcp-header key",
        json!({
            "type": "object",
            "properties": { "region": { "type": "object",
                "enum": [ { "x-mcp-header": "NotAnAnnotation" } ] } }
        }),
    )
    .await;
}

#[tokio::test]
async fn precision_examples_kept() {
    assert_kept(
        "examples member carries literal x-mcp-header key",
        json!({
            "type": "object",
            "properties": { "region": { "type": "object",
                "examples": [ { "x-mcp-header": "NotAnAnnotation" } ] } }
        }),
    )
    .await;
}
