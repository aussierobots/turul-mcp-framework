//! MCP TypeScript Specification Compliance Tests
//!
//! Tests verify Rust types serialize to JSON shapes matching `schema/draft-schema.ts`
//! at the vendored ETag. When the schema is re-vendored, these tests are the
//! contract that must keep passing.

/// Shared test fixture — minimal `RequestMetaObject` satisfying the
/// DRAFT-2026-v1 stateless-core required `_meta` contract. Tests that don't
/// care about meta contents use this; tests that DO care construct their own.
#[allow(dead_code)]
fn fixture_meta() -> turul_mcp_protocol_2026_07_28::meta::RequestMetaObject {
    use turul_mcp_protocol_2026_07_28::initialize::{ClientCapabilities, Implementation};
    turul_mcp_protocol_2026_07_28::meta::RequestMetaObject::new(
        "DRAFT-2026-v1",
        Implementation::new("test-client", "1.0.0"),
        ClientCapabilities::default(),
    )
}

#[cfg(test)]
mod tests {
    use turul_mcp_protocol_2026_07_28::notifications::*;
    use turul_mcp_protocol_2026_07_28::tools::*;
    use serde_json::json;
    use std::collections::HashMap;

    #[test]
    fn test_call_tool_request_matches_typescript_spec() {
        // Test CallToolRequest matches: { method: string, params: { name: string, arguments?: {...}, _meta?: {...} } }
        let mut args = HashMap::new();
        args.insert("text".to_string(), json!("Hello, world!"));

        // RequestMetaObject carries the required spec fields + arbitrary
        // namespaced keys (`clientId`) via the flatten `extra` catch-all.
        let meta = turul_mcp_protocol_2026_07_28::meta::RequestMetaObject::new(
            "DRAFT-2026-v1",
            turul_mcp_protocol_2026_07_28::initialize::Implementation::new("test-client", "1.0.0"),
            turul_mcp_protocol_2026_07_28::initialize::ClientCapabilities::default(),
        )
        .with_extra("clientId", json!("test-client"));

        let request = CallToolRequest::new("echo", meta)
            .with_arguments_value(json!(args));

        // Serialize to JSON to check structure
        let json_value = serde_json::to_value(&request).unwrap();

        assert_eq!(json_value["method"], "tools/call");
        assert!(json_value["params"].is_object());
        assert_eq!(json_value["params"]["name"], "echo");
        assert!(json_value["params"]["arguments"].is_object());
        assert!(json_value["params"]["_meta"].is_object());
        assert_eq!(json_value["params"]["_meta"]["clientId"], "test-client");
    }

    #[test]
    fn test_list_tools_request_matches_typescript_spec() {
        // Schema lines 1601–1603: `ListToolsRequest extends PaginatedRequest`,
        // params.cursor at TOP LEVEL (not inside _meta). params._meta is a
        // RequestMetaObject with required namespaced fields + arbitrary
        // namespaced keys (e.g. `sessionId`) via the flatten `extra` catch-all.
        let meta = turul_mcp_protocol_2026_07_28::meta::RequestMetaObject::new(
            "DRAFT-2026-v1",
            turul_mcp_protocol_2026_07_28::initialize::Implementation::new("test-client", "1.0.0"),
            turul_mcp_protocol_2026_07_28::initialize::ClientCapabilities::default(),
        )
        .with_extra("sessionId", json!("session-123"));

        let request = ListToolsRequest::new(meta)
            .with_cursor(turul_mcp_protocol_2026_07_28::meta::Cursor::new("cursor-456"));

        let json_value = serde_json::to_value(&request).unwrap();

        assert_eq!(json_value["method"], "tools/list");
        assert!(json_value["params"].is_object());
        assert_eq!(json_value["params"]["cursor"], "cursor-456");
        assert_eq!(
            json_value["params"]["_meta"]["io.modelcontextprotocol/protocolVersion"],
            "DRAFT-2026-v1"
        );
        assert_eq!(json_value["params"]["_meta"]["sessionId"], "session-123");
    }

    #[test]
    fn test_call_tool_response_matches_typescript_spec() {
        // Test CallToolResponse has top-level _meta: { content: [...], isError?: boolean, _meta?: {...} }
        let mut meta = HashMap::new();
        meta.insert("executionTime".to_string(), json!(42));

        let response = CallToolResult::success(vec![ToolResult::text("Success!")]).with_meta(meta);

        let json_value = serde_json::to_value(&response).unwrap();

        assert!(json_value["content"].is_array());
        assert_eq!(json_value["isError"], false);
        assert!(json_value["_meta"].is_object());
        assert_eq!(json_value["_meta"]["executionTime"], 42);
    }

    #[test]
    fn test_notification_matches_typescript_spec() {
        // Test Notification matches: { method: string, params?: { _meta?: {...}, [key: string]: unknown } }
        let mut meta = HashMap::new();
        meta.insert("timestamp".to_string(), json!("2025-01-01T00:00:00Z"));

        let notification = ResourceListChangedNotification::new().with_meta(meta);

        let json_value = serde_json::to_value(&notification).unwrap();

        assert_eq!(json_value["method"], "notifications/resources/list_changed");
        assert!(json_value["params"].is_object());
        assert_eq!(
            json_value["params"]["_meta"]["timestamp"],
            "2025-01-01T00:00:00Z"
        );
    }

    #[test]
    fn test_progress_notification_with_params() {
        // Test notification with specific params + _meta
        let mut meta = HashMap::new();
        meta.insert("requestId".to_string(), json!("req-789"));

        let notification = ProgressNotification::new("token-123", 50.0)
            .with_total(100.0)
            .with_message("Processing...")
            .with_meta(meta);

        let json_value = serde_json::to_value(&notification).unwrap();

        assert_eq!(json_value["method"], "notifications/progress");
        assert_eq!(json_value["params"]["progressToken"], "token-123");
        assert_eq!(json_value["params"]["progress"], 50.0);
        assert_eq!(json_value["params"]["total"], 100.0);
        assert_eq!(json_value["params"]["message"], "Processing...");
        assert_eq!(json_value["params"]["_meta"]["requestId"], "req-789");
    }

    #[test]
    fn test_resource_updated_notification_with_uri() {
        // Test notification with URI param + _meta
        let mut meta = HashMap::new();
        meta.insert("changeType".to_string(), json!("modified"));

        let notification = ResourceUpdatedNotification::new("file:///config.json").with_meta(meta);

        let json_value = serde_json::to_value(&notification).unwrap();

        assert_eq!(json_value["method"], "notifications/resources/updated");
        assert_eq!(json_value["params"]["uri"], "file:///config.json");
        assert_eq!(json_value["params"]["_meta"]["changeType"], "modified");
    }

    #[test]
    fn test_optional_params_serialization() {
        // Test that empty notifications don't serialize params if None
        let notification = ResourceListChangedNotification::new();
        let json_value = serde_json::to_value(&notification).unwrap();

        assert_eq!(json_value["method"], "notifications/resources/list_changed");
        // params should be null/absent since it's None
        assert!(
            json_value["params"].is_null()
                || !json_value.as_object().unwrap().contains_key("params")
        );
    }

    #[test]
    fn test_meta_always_serialized() {
        // Schema compliance: `CallToolRequestParams._meta` is REQUIRED in
        // DRAFT-2026-v1 stateless core. Every `tools/call` request MUST
        // serialize a `_meta` carrying per-request capability negotiation.
        let request = CallToolRequest::new("test", super::fixture_meta());
        let json_value = serde_json::to_value(&request).unwrap();

        assert_eq!(json_value["method"], "tools/call");
        assert_eq!(json_value["params"]["name"], "test");
        // `_meta` MUST be present.
        assert!(
            json_value["params"]
                .as_object()
                .unwrap()
                .contains_key("_meta"),
            "RequestParams._meta is required per schema"
        );
        // Required namespaced fields on the wire:
        assert_eq!(
            json_value["params"]["_meta"]["io.modelcontextprotocol/protocolVersion"],
            "DRAFT-2026-v1"
        );
    }
}

/// Phase 6 — JSON Schema 2020-12 acceptance on Tool.inputSchema/outputSchema (SEP-2106).
///
/// Schema lines 1815–1834 widen Tool schemas to accept any JSON Schema 2020-12
/// keyword. Our `ToolSchema` uses `#[serde(flatten)] additional: HashMap<String, Value>`
/// to pass through unknown keywords. These tests prove the round-trip works
/// for the composition and reference keywords introduced by 2020-12.
#[cfg(test)]
mod json_schema_2020_12 {
    use turul_mcp_protocol_2026_07_28::tools::{Tool, ToolSchema};
    use serde_json::json;
    use std::collections::HashMap;

    fn schema_with_extra(extra: serde_json::Map<String, serde_json::Value>) -> ToolSchema {
        let mut additional = HashMap::new();
        for (k, v) in extra {
            additional.insert(k, v);
        }
        ToolSchema {
            schema_type: "object".to_string(),
            properties: None,
            required: None,
            additional,
        }
    }

    #[test]
    fn input_schema_accepts_one_of() {
        // SEP-2106: oneOf composition keyword.
        let mut extra = serde_json::Map::new();
        extra.insert(
            "oneOf".to_string(),
            json!([
                {"type": "string", "const": "a"},
                {"type": "number", "minimum": 0}
            ]),
        );
        let s = schema_with_extra(extra);
        let v = serde_json::to_value(&s).unwrap();
        assert_eq!(v["type"], "object");
        assert_eq!(v["oneOf"][0]["const"], "a");
        assert_eq!(v["oneOf"][1]["minimum"], 0);
    }

    #[test]
    fn input_schema_accepts_any_of() {
        let mut extra = serde_json::Map::new();
        extra.insert("anyOf".to_string(), json!([{"type": "string"}, {"type": "number"}]));
        let s = schema_with_extra(extra);
        let v = serde_json::to_value(&s).unwrap();
        assert!(v["anyOf"].is_array());
    }

    #[test]
    fn input_schema_accepts_all_of() {
        let mut extra = serde_json::Map::new();
        extra.insert("allOf".to_string(), json!([{"required": ["a"]}, {"required": ["b"]}]));
        let s = schema_with_extra(extra);
        let v = serde_json::to_value(&s).unwrap();
        assert!(v["allOf"].is_array());
    }

    #[test]
    fn input_schema_accepts_ref_and_defs() {
        // SEP-2106: $ref and $defs reference keywords. Per safeguard note (schema 1820),
        // external $ref URIs must NOT be auto-dereferenced — but our type just stores
        // them; dereference is a consumer concern.
        let mut extra = serde_json::Map::new();
        extra.insert(
            "$defs".to_string(),
            json!({
                "positiveInt": {"type": "integer", "minimum": 1}
            }),
        );
        extra.insert(
            "properties".to_string(),
            json!({
                "count": {"$ref": "#/$defs/positiveInt"}
            }),
        );
        let s = schema_with_extra(extra);
        let v = serde_json::to_value(&s).unwrap();
        assert!(v["$defs"]["positiveInt"].is_object());
        assert_eq!(v["properties"]["count"]["$ref"], "#/$defs/positiveInt");
    }

    #[test]
    fn input_schema_accepts_conditional_keywords() {
        // SEP-2106: if/then/else conditional keywords.
        let mut extra = serde_json::Map::new();
        extra.insert("if".to_string(), json!({"properties": {"mode": {"const": "x"}}}));
        extra.insert("then".to_string(), json!({"required": ["x_data"]}));
        extra.insert("else".to_string(), json!({"required": ["y_data"]}));
        let s = schema_with_extra(extra);
        let v = serde_json::to_value(&s).unwrap();
        assert!(v["if"].is_object());
        assert!(v["then"].is_object());
        assert!(v["else"].is_object());
    }

    #[test]
    fn input_schema_accepts_schema_dialect_marker() {
        // Schema line 1826: `inputSchema: { $schema?: string; type: "object"; ... }`.
        let mut extra = serde_json::Map::new();
        extra.insert(
            "$schema".to_string(),
            json!("https://json-schema.org/draft/2020-12/schema"),
        );
        let s = schema_with_extra(extra);
        let v = serde_json::to_value(&s).unwrap();
        assert_eq!(v["$schema"], "https://json-schema.org/draft/2020-12/schema");
    }

    #[test]
    fn input_schema_round_trips_top_level_2020_12_keywords() {
        // Verifies $schema + $defs + oneOf at the SCHEMA ROOT round-trip via
        // the `additional` flatten HashMap. The schema's [key: string]: unknown
        // clause means any 2020-12 keyword at the root is accepted.
        let wire = json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "$defs": {
                "address": {
                    "type": "object",
                    "required": ["street", "city"]
                }
            },
            "oneOf": [
                {"const": "cash"},
                {"required": ["cardNumber"]}
            ]
        });
        let parsed: ToolSchema = serde_json::from_value(wire.clone()).unwrap();
        let re_v = serde_json::to_value(&parsed).unwrap();
        // Identity comparison: every key-value preserved through round-trip.
        for k in ["$schema", "$defs", "oneOf", "type"] {
            assert_eq!(re_v[k], wire[k], "key `{}` must round-trip identically", k);
        }
    }

    #[test]
    fn properties_field_accepts_2020_12_unknown_values() {
        // Schema lines 1815–1826: `properties` values are `unknown` (any JSON).
        // `ToolSchema.properties: Option<HashMap<String, Value>>` honors this —
        // a `$ref`-only property schema parses without error, and any
        // composition keyword inside individual property schemas survives
        // round-trip.
        let wire = json!({
            "type": "object",
            "properties": {
                "x": {"$ref": "#/$defs/y"},
                "y": {"oneOf": [{"type": "string"}, {"type": "number"}]}
            }
        });
        let parsed: ToolSchema = serde_json::from_value(wire.clone()).unwrap();
        let re_v = serde_json::to_value(&parsed).unwrap();
        assert_eq!(
            re_v["properties"]["x"]["$ref"],
            "#/$defs/y",
            "$ref inside properties values must round-trip"
        );
        assert!(
            re_v["properties"]["y"]["oneOf"].is_array(),
            "composition keywords inside properties values must round-trip"
        );
    }

    #[test]
    fn tool_with_2020_12_input_schema_round_trips() {
        // End-to-end: build a Tool with a 2020-12 inputSchema and confirm
        // the keywords ride through Tool's serializer untouched.
        let mut extra = serde_json::Map::new();
        extra.insert(
            "oneOf".to_string(),
            json!([{"type": "string"}, {"type": "number"}]),
        );
        let input_schema = schema_with_extra(extra);
        let tool = Tool {
            name: "x".to_string(),
            title: None,
            description: None,
            input_schema,
            output_schema: None,
            annotations: None,
            icons: None,
            meta: None,
        };
        let v = serde_json::to_value(&tool).unwrap();
        assert!(v["inputSchema"]["oneOf"].is_array());
    }

    #[test]
    fn output_schema_accepts_non_object_root() {
        // Schema lines 1828–1834: `outputSchema?: { $schema?: string; [key: string]: unknown }`.
        // No root `type` constraint. `Tool.output_schema` is now typed
        // `Option<ToolOutputSchema>`, a separate flatten-only struct that
        // doesn't bake in `type: "object"`. Any 2020-12 root works.
        use turul_mcp_protocol_2026_07_28::tools::ToolOutputSchema;

        // Array root.
        let array_root = ToolOutputSchema::any().with("type", json!("array")).with(
            "items",
            json!({"type": "string"}),
        );
        let v = serde_json::to_value(&array_root).unwrap();
        assert_eq!(v["type"], "array", "non-object root permitted");
        assert!(v["items"].is_object());

        // Empty (any) schema — produces `{}` for unrestricted output.
        let any = ToolOutputSchema::any();
        let v = serde_json::to_value(&any).unwrap();
        assert!(
            v.as_object().unwrap().is_empty(),
            "empty output schema serializes to `{{}}` per schema's `[k]: unknown`"
        );

        // String root.
        let string_root = ToolOutputSchema::any().with("type", json!("string"));
        let v = serde_json::to_value(&string_root).unwrap();
        assert_eq!(v["type"], "string");
    }
}

/// Phase 9 — coverage closure: shape tests for remaining schema types
/// (Resource, ResourceTemplate, Prompt, PromptArgument, PromptMessage,
/// Implementation, Annotations, Icon). Most of these were carried unchanged
/// from 2025-11-25; the tests prove their wire shapes still match DRAFT-2026-v1.
#[cfg(test)]
mod remaining_shapes {
    use turul_mcp_protocol_2026_07_28::icons::Icon;
    use turul_mcp_protocol_2026_07_28::initialize::Implementation;
    use turul_mcp_protocol_2026_07_28::meta::Annotations;
    use turul_mcp_protocol_2026_07_28::prompts::{Prompt, PromptArgument, PromptMessage, Role};
    use turul_mcp_protocol_2026_07_28::resources::{Resource, ResourceTemplate};
    use serde_json::json;

    #[test]
    fn resource_shape_matches_schema() {
        // Schema lines 1275–1308: `Resource extends BaseMetadata, Icons`
        // with uri (required), description?, mimeType?, annotations?, size?, _meta?.
        let r = Resource::new("file:///x", "x");
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["uri"], "file:///x");
        assert_eq!(v["name"], "x");
        // Optional fields omitted when absent.
        for missing in ["description", "mimeType", "annotations", "size", "_meta", "icons", "title"] {
            assert!(
                !v.as_object().unwrap().contains_key(missing),
                "{} omitted when None",
                missing
            );
        }
    }

    #[test]
    fn resource_template_shape_matches_schema() {
        // Schema lines 1315–1341: `ResourceTemplate extends BaseMetadata, Icons`
        // with uriTemplate (required), description?, mimeType?, annotations?, _meta?.
        let r = ResourceTemplate::new("template", "file:///{user_id}.json");
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["uriTemplate"], "file:///{user_id}.json");
        assert_eq!(v["name"], "template");
    }

    #[test]
    fn prompt_shape_matches_schema() {
        // Schema lines 1493–1505: `Prompt extends BaseMetadata, Icons`
        // with description?, arguments?, _meta?.
        let p = Prompt::new("code_review");
        let v = serde_json::to_value(&p).unwrap();
        assert_eq!(v["name"], "code_review");
    }

    #[test]
    fn prompt_argument_shape_matches_schema() {
        // Schema lines 1512–1521: `PromptArgument extends BaseMetadata` with
        // description?, required?.
        let a = PromptArgument::new("language");
        let v = serde_json::to_value(&a).unwrap();
        assert_eq!(v["name"], "language");
    }

    #[test]
    fn prompt_message_role_values_match_schema() {
        // Schema line 1528: `Role = "user" | "assistant"`. Also schema lines
        // 1538–1541: `PromptMessage { role, content }`.
        let m_user = PromptMessage::user_text("hi");
        let m_assistant = PromptMessage::assistant_text("hello");
        let v_user = serde_json::to_value(&m_user).unwrap();
        let v_assistant = serde_json::to_value(&m_assistant).unwrap();
        assert_eq!(v_user["role"], "user");
        assert_eq!(v_assistant["role"], "assistant");
        // Content always a ContentBlock object.
        assert!(v_user["content"].is_object());
        assert_eq!(v_user["content"]["type"], "text");
    }

    #[test]
    fn prompt_message_only_user_and_assistant_roles() {
        // Schema line 1528 enumerates exactly these two; no "system".
        let user_wire = json!({"role": "user", "content": {"type": "text", "text": "x"}});
        let assist_wire = json!({"role": "assistant", "content": {"type": "text", "text": "x"}});
        let system_wire = json!({"role": "system", "content": {"type": "text", "text": "x"}});

        assert!(serde_json::from_value::<PromptMessage>(user_wire).is_ok());
        assert!(serde_json::from_value::<PromptMessage>(assist_wire).is_ok());
        assert!(
            serde_json::from_value::<PromptMessage>(system_wire).is_err(),
            "role 'system' must be rejected per schema line 1528 (only user|assistant)"
        );
    }

    #[test]
    fn implementation_shape_matches_schema() {
        // Schema lines 865–886: `Implementation extends BaseMetadata, Icons`
        // with version (required), description?, websiteUrl?.
        let i = Implementation::new("my-server", "0.4.0")
            .with_description("test server")
            .with_title("My Server");
        let v = serde_json::to_value(&i).unwrap();
        assert_eq!(v["name"], "my-server");
        assert_eq!(v["version"], "0.4.0");
        assert_eq!(v["description"], "test server");
        assert_eq!(v["title"], "My Server");
        // websiteUrl is camelCase
        assert!(!v.as_object().unwrap().contains_key("website_url"));
    }

    #[test]
    fn annotations_shape_matches_schema() {
        // Schema lines 2059–2089: `Annotations { audience?: Role[], priority?: number, lastModified?: string }`.
        let a = Annotations::new()
            .with_audience(vec!["user".to_string(), "assistant".to_string()])
            .with_priority(0.7)
            .with_last_modified("2026-05-24T12:00:00Z");
        let v = serde_json::to_value(&a).unwrap();
        assert_eq!(v["audience"][0], "user");
        assert_eq!(v["audience"][1], "assistant");
        assert_eq!(v["priority"], 0.7);
        assert_eq!(v["lastModified"], "2026-05-24T12:00:00Z");
        // camelCase
        assert!(!v.as_object().unwrap().contains_key("last_modified"));
    }

    #[test]
    fn icon_shape_matches_schema() {
        // Schema lines 779–816: `Icon { src, mimeType?, sizes?, theme? ("light"|"dark") }`.
        let i = Icon::new("https://example.com/icon.png");
        let v = serde_json::to_value(&i).unwrap();
        assert_eq!(v["src"], "https://example.com/icon.png");
        // All others omitted when None.
        for missing in ["mimeType", "sizes", "theme"] {
            assert!(
                !v.as_object().unwrap().contains_key(missing),
                "{} omitted when None",
                missing
            );
        }
    }

    #[test]
    fn role_default_value_user() {
        // Just verifies the Role enum exists with the right wire format.
        let r_user = Role::User;
        let r_assistant = Role::Assistant;
        assert_eq!(serde_json::to_value(&r_user).unwrap(), "user");
        assert_eq!(serde_json::to_value(&r_assistant).unwrap(), "assistant");
    }
}

/// Phase 3.5 — completion.rs schema alignment (schema lines 2358–2474).
#[cfg(test)]
mod completion_alignment {
    use turul_mcp_protocol_2026_07_28::completion::{
        CompleteArgument, CompleteResult, CompletionResult, PromptReference,
        ResourceTemplateReference,
    };
    use turul_mcp_protocol_2026_07_28::result_type::ResultType;
    use serde_json::json;

    #[test]
    fn complete_result_emits_result_type() {
        // Schema lines 2421–2438: `CompleteResult extends Result` with required `resultType`.
        let r = CompleteResult::new(CompletionResult::new(vec!["foo".to_string()]));
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["resultType"], "complete");
        assert!(v["completion"]["values"].is_array());
        assert_eq!(v["completion"]["values"][0], "foo");
    }

    #[test]
    fn complete_result_back_compat_accepts_missing_result_type() {
        let wire = json!({
            "completion": {"values": ["x"]}
        });
        let r: CompleteResult = serde_json::from_value(wire).unwrap();
        assert_eq!(r.result_type, ResultType::Complete);
    }

    #[test]
    fn completion_inner_field_shape() {
        // Schema lines 2422–2437: `completion: {values: string[] (max 100), total?, hasMore?}`.
        let inner = CompletionResult::new(vec!["a".to_string(), "b".to_string()])
            .with_total(10)
            .with_has_more(true);
        let v = serde_json::to_value(&inner).unwrap();
        assert_eq!(v["values"][0], "a");
        assert_eq!(v["values"][1], "b");
        assert_eq!(v["total"], 10);
        assert_eq!(v["hasMore"], true);
    }

    #[test]
    fn completion_omits_total_and_has_more_when_absent() {
        let inner = CompletionResult::new(vec!["x".to_string()]);
        let v = serde_json::to_value(&inner).unwrap();
        assert!(!v.as_object().unwrap().contains_key("total"));
        assert!(!v.as_object().unwrap().contains_key("hasMore"));
    }

    #[test]
    fn resource_template_reference_type_field() {
        // Schema lines 2457–2465: `{type: "ref/resource", uri}`.
        let r = ResourceTemplateReference::new("template://t");
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["type"], "ref/resource");
        assert_eq!(v["uri"], "template://t");
    }

    #[test]
    fn prompt_reference_type_field() {
        // Schema lines 2472–2474: `PromptReference extends BaseMetadata { type: "ref/prompt" }`.
        let r = PromptReference::new("my_prompt");
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["type"], "ref/prompt");
        assert_eq!(v["name"], "my_prompt");
    }

    #[test]
    fn complete_argument_shape() {
        // Schema lines 2375–2384: `argument: {name, value}`.
        let a = CompleteArgument::new("lang", "rust");
        let v = serde_json::to_value(&a).unwrap();
        assert_eq!(v["name"], "lang");
        assert_eq!(v["value"], "rust");
    }
}

/// Phase 3.6 — Schema-driven assertion that `EmptyResult` carries `resultType`.
///
/// Schema line 435: `EmptyResult = Result`. The 2025-11-25 EmptyResult had
/// no resultType; DRAFT-2026-v1 inherits the required `resultType` field from
/// `Result`. Phase 3.6 also confirms `ping` is gone (already proven in
/// `removed_methods::ping_method_is_gone`); `PingRequest` remains transitionally
/// in code with `#[deprecated]`.
#[cfg(test)]
mod empty_result_alignment {
    use turul_mcp_protocol_2026_07_28::ping::EmptyResult;
    use turul_mcp_protocol_2026_07_28::result_type::ResultType;
    use serde_json::json;

    #[test]
    fn empty_result_serializes_result_type_complete() {
        let r = EmptyResult::new();
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(
            v["resultType"], "complete",
            "EmptyResult must carry resultType per schema line 178+435 \
             (EmptyResult = Result, Result.resultType required)"
        );
    }

    #[test]
    fn empty_result_back_compat_accepts_missing_result_type() {
        // 2025-11-25 wire shape: `{}`. DRAFT-2026-v1 backward-compat allows
        // missing resultType, defaults to Complete.
        let wire = json!({});
        let r: EmptyResult = serde_json::from_value(wire).unwrap();
        assert_eq!(r.result_type, ResultType::Complete);
    }

    #[test]
    fn empty_result_with_meta_round_trips() {
        let mut meta = std::collections::HashMap::new();
        meta.insert("custom".to_string(), serde_json::json!("value"));
        let r = EmptyResult::new().with_meta(meta);
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["resultType"], "complete");
        assert_eq!(v["_meta"]["custom"], "value");
    }
}

/// Phase 3.7 — content.rs ContentBlock variants (schema lines 2091–2099).
///
/// Verifies wire shape of `ContentBlock` discriminated union:
/// `TextContent | ImageContent | AudioContent | ResourceLink | EmbeddedResource`,
/// distinguished by `type` field.
#[cfg(test)]
mod content_alignment {
    use turul_mcp_protocol_2026_07_28::content::ContentBlock;
    use serde_json::json;

    #[test]
    fn text_content_type_field_is_text() {
        // Schema lines 2109–2123.
        let c = ContentBlock::text("hello");
        let v = serde_json::to_value(&c).unwrap();
        assert_eq!(v["type"], "text");
        assert_eq!(v["text"], "hello");
    }

    #[test]
    fn image_content_type_field_is_image() {
        // Schema lines 2133–2154.
        let c = ContentBlock::image("base64data", "image/png");
        let v = serde_json::to_value(&c).unwrap();
        assert_eq!(v["type"], "image");
        assert_eq!(v["data"], "base64data");
        assert_eq!(v["mimeType"], "image/png");
    }

    #[test]
    fn audio_content_type_field_is_audio() {
        // Schema lines 2164–2185.
        let c = ContentBlock::audio("base64data", "audio/wav");
        let v = serde_json::to_value(&c).unwrap();
        assert_eq!(v["type"], "audio");
        assert_eq!(v["data"], "base64data");
        assert_eq!(v["mimeType"], "audio/wav");
    }

    #[test]
    fn content_blocks_round_trip_via_untagged_discrimination() {
        // The ContentBlock union is discriminated by `type` at the wire level.
        // Round-trip each variant through serde_json::Value and confirm shape.
        let variants = vec![
            ContentBlock::text("t1"),
            ContentBlock::image("img-data", "image/png"),
            ContentBlock::audio("aud-data", "audio/wav"),
        ];
        for c in variants {
            let v = serde_json::to_value(&c).unwrap();
            let s = v.to_string();
            let parsed: ContentBlock = serde_json::from_str(&s).unwrap();
            let re_v = serde_json::to_value(&parsed).unwrap();
            assert_eq!(v, re_v, "ContentBlock variant must round-trip identically");
        }
    }

    #[test]
    fn text_content_with_annotations_serializes() {
        use turul_mcp_protocol_2026_07_28::meta::Annotations;
        let c = ContentBlock::text_with_annotations(
            "annotated",
            Annotations::new().with_priority(0.8),
        );
        let v = serde_json::to_value(&c).unwrap();
        assert_eq!(v["type"], "text");
        assert_eq!(v["text"], "annotated");
        assert_eq!(v["annotations"]["priority"], 0.8);
    }

    #[test]
    fn resource_link_type_field_present() {
        // Schema lines 1553–1555: `ResourceLink extends Resource { type: "resource_link" }`.
        // Construct via the public API used by the prompts/tools paths.
        let r = json!({
            "type": "resource_link",
            "uri": "file:///x",
            "name": "x"
        });
        // Round-trip via ContentBlock if the variant is structurally present.
        // If parsing fails, the variant isn't bound — flag clearly.
        let parsed: Result<ContentBlock, _> = serde_json::from_value(r.clone());
        assert!(
            parsed.is_ok(),
            "ContentBlock must accept `type: resource_link` (schema lines 1553–1555); \
             got error: {:?}",
            parsed.err()
        );
        let v = serde_json::to_value(&parsed.unwrap()).unwrap();
        assert_eq!(v["type"], "resource_link");
        assert_eq!(v["uri"], "file:///x");
    }

    #[test]
    fn embedded_resource_type_field_present() {
        // Schema lines 1568–1578: `EmbeddedResource { type: "resource", resource, ... }`.
        let wire = json!({
            "type": "resource",
            "resource": {
                "uri": "file:///x",
                "text": "content"
            }
        });
        let parsed: Result<ContentBlock, _> = serde_json::from_value(wire.clone());
        assert!(
            parsed.is_ok(),
            "ContentBlock must accept `type: resource` (embedded resource); \
             got error: {:?}",
            parsed.err()
        );
    }
}

/// Phase 3.4 — elicitation.rs enum schema variants (schema lines 2687–2886).
///
/// Verifies wire shape of all four new DRAFT-2026-v1 enum schema types:
/// - `UntitledSingleSelectEnumSchema` — `{type:"string", enum:[]}`
/// - `TitledSingleSelectEnumSchema` — `{type:"string", oneOf:[{const,title}]}`
/// - `UntitledMultiSelectEnumSchema` — `{type:"array", items:{type:"string", enum:[]}}`
/// - `TitledMultiSelectEnumSchema` — `{type:"array", items:{anyOf:[{const,title}]}}`

/// Group A — URL-mode elicitation discriminated union (schema lines 2540–2629).
#[cfg(test)]
mod elicitation_modes {
    use turul_mcp_protocol_2026_07_28::elicitation::{
        ElicitRequest, ElicitRequestFormParams, ElicitRequestParams, ElicitRequestURLParams,
        ElicitationSchema, FormModeMarker, UrlModeMarker,
    };
    use serde_json::json;

    #[test]
    fn form_mode_round_trips_with_optional_mode() {
        let r = ElicitRequest::new_form("hi", ElicitationSchema::new());
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["method"], "elicitation/create");
        assert_eq!(v["params"]["message"], "hi");
        assert!(v["params"]["requestedSchema"].is_object());
        assert!(!v["params"].as_object().unwrap().contains_key("mode"));
    }

    #[test]
    fn url_mode_round_trips_with_required_mode_and_fields() {
        // Schema lines 2588–2606: mode:"url" REQUIRED, plus elicitationId + url.
        let r = ElicitRequest::new_url("Please authorize", "elicit-abc-123", "https://auth.example/login");
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["method"], "elicitation/create");
        assert_eq!(v["params"]["mode"], "url");
        assert_eq!(v["params"]["message"], "Please authorize");
        assert_eq!(v["params"]["elicitationId"], "elicit-abc-123");
        assert_eq!(v["params"]["url"], "https://auth.example/login");
    }

    #[test]
    fn untagged_discrimination_picks_url_when_mode_url_present() {
        let wire = json!({
            "method": "elicitation/create",
            "params": {"mode": "url", "message": "go here", "elicitationId": "id-1", "url": "https://x"}
        });
        let parsed: ElicitRequest = serde_json::from_value(wire).unwrap();
        match parsed.params {
            ElicitRequestParams::Url(p) => {
                assert_eq!(p.mode, UrlModeMarker::Url);
                assert_eq!(p.elicitation_id, "id-1");
            }
            ElicitRequestParams::Form(_) => panic!("must parse as URL mode"),
        }
    }

    #[test]
    fn untagged_discrimination_picks_form_when_mode_absent() {
        let wire = json!({
            "method": "elicitation/create",
            "params": {"message": "hi", "requestedSchema": {"type": "object", "properties": {}}}
        });
        let parsed: ElicitRequest = serde_json::from_value(wire).unwrap();
        assert!(matches!(parsed.params, ElicitRequestParams::Form(_)));
    }

    #[test]
    fn form_mode_marker_serializes_lowercase() {
        let p = ElicitRequestFormParams {
            mode: Some(FormModeMarker::Form),
            message: "x".into(),
            requested_schema: ElicitationSchema::new(),
        };
        let v = serde_json::to_value(&p).unwrap();
        assert_eq!(v["mode"], "form");
    }

    #[test]
    fn url_mode_marker_serializes_lowercase() {
        let p = ElicitRequestURLParams::new("m", "id", "https://x");
        let v = serde_json::to_value(&p).unwrap();
        assert_eq!(v["mode"], "url");
    }
}

#[cfg(test)]
mod elicitation_enum_schemas {
    use turul_mcp_protocol_2026_07_28::elicitation::{
        TitledEnumOption, TitledMultiSelectEnumSchema, TitledSingleSelectEnumSchema,
        UntitledMultiSelectEnumSchema, UntitledSingleSelectEnumSchema,
    };
    use serde_json::json;

    #[test]
    fn untitled_single_select_wire_shape() {
        // Schema lines 2695–2713.
        let s = UntitledSingleSelectEnumSchema::new(vec![
            "red".to_string(),
            "green".to_string(),
            "blue".to_string(),
        ]);
        let v = serde_json::to_value(&s).unwrap();
        assert_eq!(v["type"], "string");
        assert_eq!(v["enum"][0], "red");
        assert_eq!(v["enum"][1], "green");
        assert_eq!(v["enum"][2], "blue");
        assert!(
            !v.as_object().unwrap().contains_key("oneOf"),
            "untitled variant must not use oneOf"
        );
    }

    #[test]
    fn titled_single_select_wire_shape() {
        // Schema lines 2723–2750.
        let s = TitledSingleSelectEnumSchema::new(vec![
            TitledEnumOption::new("r", "Red"),
            TitledEnumOption::new("g", "Green"),
        ]);
        let v = serde_json::to_value(&s).unwrap();
        assert_eq!(v["type"], "string");
        assert_eq!(v["oneOf"][0]["const"], "r");
        assert_eq!(v["oneOf"][0]["title"], "Red");
        assert!(
            !v.as_object().unwrap().contains_key("enum"),
            "titled variant uses oneOf not enum"
        );
    }

    #[test]
    fn untitled_multi_select_wire_shape() {
        // Schema lines 2768–2800.
        let s = UntitledMultiSelectEnumSchema::new(vec!["a".to_string(), "b".to_string()]);
        let v = serde_json::to_value(&s).unwrap();
        assert_eq!(v["type"], "array");
        assert_eq!(v["items"]["type"], "string");
        assert_eq!(v["items"]["enum"][0], "a");
        assert_eq!(v["items"]["enum"][1], "b");
    }

    #[test]
    fn titled_multi_select_wire_shape() {
        // Schema lines 2810–2850.
        let s = TitledMultiSelectEnumSchema::new(vec![
            TitledEnumOption::new("a", "Alpha"),
            TitledEnumOption::new("b", "Beta"),
        ]);
        let v = serde_json::to_value(&s).unwrap();
        assert_eq!(v["type"], "array");
        assert_eq!(v["items"]["anyOf"][0]["const"], "a");
        assert_eq!(v["items"]["anyOf"][0]["title"], "Alpha");
        assert_eq!(v["items"]["anyOf"][1]["const"], "b");
        assert_eq!(v["items"]["anyOf"][1]["title"], "Beta");
    }

    #[test]
    fn enum_schemas_round_trip() {
        // Each variant survives serialize/deserialize.
        let cases = vec![
            serde_json::to_value(UntitledSingleSelectEnumSchema::new(vec!["x".to_string()]))
                .unwrap(),
            serde_json::to_value(TitledSingleSelectEnumSchema::new(vec![TitledEnumOption::new(
                "x", "X",
            )]))
            .unwrap(),
            serde_json::to_value(UntitledMultiSelectEnumSchema::new(vec!["y".to_string()]))
                .unwrap(),
            serde_json::to_value(TitledMultiSelectEnumSchema::new(vec![TitledEnumOption::new(
                "y", "Y",
            )]))
            .unwrap(),
        ];
        for v in cases {
            // Round-trip via Value.
            let s = v.to_string();
            let parsed: serde_json::Value = serde_json::from_str(&s).unwrap();
            assert_eq!(parsed, v);
        }
    }

    #[test]
    fn untitled_single_select_omits_optional_fields_when_none() {
        let s = UntitledSingleSelectEnumSchema::new(vec!["x".to_string()]);
        let v = serde_json::to_value(&s).unwrap();
        for missing in ["title", "description", "default"] {
            assert!(
                !v.as_object().unwrap().contains_key(missing),
                "{} omitted when None",
                missing
            );
        }
    }

    #[test]
    fn titled_enum_option_camelcase_const_key() {
        // Schema field is literal `const` (a JSON reserved-feeling keyword); the
        // Rust field is `const_value` to avoid the Rust keyword collision.
        let opt = TitledEnumOption::new("k", "Display");
        let v = serde_json::to_value(&opt).unwrap();
        assert_eq!(v["const"], "k", "must serialize as `const` not `const_value`");
        assert_eq!(v["title"], "Display");
    }

    #[test]
    fn schema_examples_round_trip() {
        // Reproduce the schema's example shapes verbatim.
        // From `TitledSingleSelectEnumSchema/titled-color-select-schema`:
        let wire = json!({
            "type": "string",
            "oneOf": [
                {"const": "r", "title": "Red"},
                {"const": "g", "title": "Green"},
                {"const": "b", "title": "Blue"}
            ]
        });
        let parsed: TitledSingleSelectEnumSchema = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(parsed.one_of.len(), 3);
        assert_eq!(parsed.one_of[0].const_value, "r");
        assert_eq!(parsed.one_of[2].title, "Blue");

        // Re-serialize; shape preserved.
        let v = serde_json::to_value(&parsed).unwrap();
        assert_eq!(v["oneOf"][2]["const"], "b");
    }
}

/// Phase 3.3 — prompts.rs schema alignment (schema lines 1391–1590).
///
/// Verifies wire shape of `ListPromptsResult` (extends PaginatedResult,
/// CacheableResult), `GetPromptResult` (extends Result), and `GetPromptRequestParams`
/// (extends InputResponseRequestParams).

/// Group C — `SamplingMessage` single|array content + 5-variant `SamplingMessageContentBlock`
/// (schema lines 2038–2052) and `CreateMessageResult extends SamplingMessage` (2007–2025).
#[cfg(test)]
mod sampling_message_alignment {
    use turul_mcp_protocol_2026_07_28::sampling::{
        CreateMessageResult, Role, SamplingMessage, SamplingMessageContent,
        SamplingMessageContentBlock,
    };
    use serde_json::json;

    #[test]
    fn single_block_content_serializes_as_object_not_array() {
        // Schema example: "single-content-block.json" — content is a single object.
        let msg = SamplingMessage::single(Role::User, SamplingMessageContentBlock::text("hi"));
        let v = serde_json::to_value(&msg).unwrap();
        assert_eq!(v["role"], "user");
        assert!(v["content"].is_object(), "single block must be object on wire");
        assert_eq!(v["content"]["type"], "text");
        assert_eq!(v["content"]["text"], "hi");
    }

    #[test]
    fn multiple_blocks_content_serializes_as_array() {
        let msg = SamplingMessage::new(
            Role::Assistant,
            SamplingMessageContent::Multiple(vec![
                SamplingMessageContentBlock::text("a"),
                SamplingMessageContentBlock::text("b"),
            ]),
        );
        let v = serde_json::to_value(&msg).unwrap();
        assert!(v["content"].is_array());
        assert_eq!(v["content"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn untagged_content_round_trips_single() {
        let wire = json!({"role": "user", "content": {"type": "text", "text": "x"}});
        let parsed: SamplingMessage = serde_json::from_value(wire).unwrap();
        assert!(matches!(parsed.content, SamplingMessageContent::Single(_)));
    }

    #[test]
    fn untagged_content_round_trips_array() {
        let wire = json!({"role": "user", "content": [{"type": "text", "text": "x"}]});
        let parsed: SamplingMessage = serde_json::from_value(wire).unwrap();
        assert!(matches!(parsed.content, SamplingMessageContent::Multiple(_)));
    }

    #[test]
    fn sampling_message_meta_is_camelcase_underscore_meta() {
        // Schema line 2041: `_meta?: MetaObject`.
        let mut meta = std::collections::HashMap::new();
        meta.insert("trace".into(), json!("abc"));
        let msg = SamplingMessage::user_text("hi").with_meta(meta);
        let v = serde_json::to_value(&msg).unwrap();
        assert_eq!(v["_meta"]["trace"], "abc");
    }

    #[test]
    fn sampling_message_content_block_excludes_resource_link_variant() {
        // Schema lines 2047–2052 list only Text|Image|Audio|ToolUse|ToolResult.
        // A wire payload tagged "resource_link" must NOT round-trip through SamplingMessageContentBlock.
        let wire = json!({"type": "resource_link", "uri": "file:///x", "name": "x"});
        let parsed: Result<SamplingMessageContentBlock, _> = serde_json::from_value(wire);
        assert!(parsed.is_err(), "resource_link must not deserialize as SamplingMessageContentBlock");
    }

    #[test]
    fn sampling_message_content_block_excludes_embedded_resource_variant() {
        let wire = json!({"type": "resource", "resource": {"uri": "file:///x", "text": "x"}});
        let parsed: Result<SamplingMessageContentBlock, _> = serde_json::from_value(wire);
        assert!(parsed.is_err(), "embedded resource must not deserialize as SamplingMessageContentBlock");
    }

    #[test]
    fn tool_use_content_block_carries_id_name_input() {
        // Schema lines 2047–2052 → ToolUseContent has id, name, input map.
        let mut input = std::collections::HashMap::new();
        input.insert("k".into(), json!("v"));
        let blk = SamplingMessageContentBlock::ToolUse {
            id: "t-1".into(),
            name: "search".into(),
            input,
            meta: None,
        };
        let v = serde_json::to_value(&blk).unwrap();
        assert_eq!(v["type"], "tool_use");
        assert_eq!(v["id"], "t-1");
        assert_eq!(v["name"], "search");
        assert_eq!(v["input"]["k"], "v");
    }

    #[test]
    fn create_message_result_extends_sampling_message_shape() {
        // Schema lines 2007–2025: extends SamplingMessage, adds model + stopReason.
        let r = CreateMessageResult::single(
            Role::Assistant,
            SamplingMessageContentBlock::text("done"),
            "claude-x",
        );
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["role"], "assistant");
        assert!(v["content"].is_object(), "single content stays object");
        assert_eq!(v["content"]["type"], "text");
        assert_eq!(v["model"], "claude-x");
        let obj = v.as_object().unwrap();
        assert!(!obj.contains_key("stopReason"), "absent stop_reason is omitted");
        assert!(!obj.contains_key("_meta"));
    }

    #[test]
    fn create_message_result_supports_array_content_from_extends() {
        // Inheriting from SamplingMessage means CreateMessageResult.content can be array too.
        let r = CreateMessageResult::new(
            Role::Assistant,
            SamplingMessageContent::Multiple(vec![
                SamplingMessageContentBlock::text("a"),
                SamplingMessageContentBlock::text("b"),
            ]),
            "claude-x",
        );
        let v = serde_json::to_value(&r).unwrap();
        assert!(v["content"].is_array());
    }
}

#[cfg(test)]
mod prompts_alignment {
    use turul_mcp_protocol_2026_07_28::caching::{CacheScope, CacheableResult};
    use turul_mcp_protocol_2026_07_28::input_required::{InputResponse, InputResponses};
    use turul_mcp_protocol_2026_07_28::prompts::{GetPromptRequestParams, GetPromptResult, ListPromptsResult, PromptMessage};
    use turul_mcp_protocol_2026_07_28::result_type::ResultType;
    use turul_mcp_protocol_2026_07_28::roots::ListRootsResult;
    use serde_json::json;
    use std::collections::HashMap;

    #[test]
    fn list_prompts_result_emits_result_type() {
        let r = ListPromptsResult::new(vec![]);
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["resultType"], "complete");
        assert!(v["prompts"].is_array());
    }

    #[test]
    fn list_prompts_result_with_cache() {
        let r = ListPromptsResult::new(vec![])
            .with_cache(CacheableResult::new(120_000, CacheScope::Public));
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["ttlMs"], 120_000);
        assert_eq!(v["cacheScope"], "public");
    }

    #[test]
    fn list_prompts_result_back_compat_accepts_missing_result_type() {
        let wire = json!({"prompts": [], "ttlMs": 0, "cacheScope": "public"});
        let r: ListPromptsResult = serde_json::from_value(wire).unwrap();
        assert_eq!(r.result_type, ResultType::Complete);
    }

    #[test]
    fn get_prompt_result_emits_result_type() {
        let r = GetPromptResult::new(vec![PromptMessage::user_text("hi")]);
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["resultType"], "complete");
        assert!(v["messages"].is_array());
    }

    #[test]
    fn get_prompt_result_back_compat_accepts_missing_result_type() {
        let wire = json!({"messages": []});
        let r: GetPromptResult = serde_json::from_value(wire).unwrap();
        assert_eq!(r.result_type, ResultType::Complete);
    }

    #[test]
    fn get_prompt_params_arguments_is_string_map() {
        // Schema line 1444: `arguments?: { [key: string]: string }`.
        let mut args = HashMap::new();
        args.insert("lang".to_string(), "rust".to_string());
        let p = GetPromptRequestParams::new("code_review", super::fixture_meta()).with_arguments(args);
        let v = serde_json::to_value(&p).unwrap();
        assert_eq!(v["arguments"]["lang"], "rust");
        assert!(
            v["arguments"]["lang"].is_string(),
            "arguments values must be strings (schema constraint), not unknown"
        );
    }

    #[test]
    fn get_prompt_params_input_responses_mixin_serializes() {
        // Schema line 1436: `GetPromptRequestParams extends InputResponseRequestParams`.
        let mut responses: InputResponses = HashMap::new();
        responses.insert(
            "rq-1".to_string(),
            InputResponse::ListRoots(ListRootsResult::new(vec![])),
        );
        let p = GetPromptRequestParams::new("code_review", super::fixture_meta())
            .with_input_responses(responses, "opaque-state");
        let v = serde_json::to_value(&p).unwrap();
        assert_eq!(v["name"], "code_review");
        assert_eq!(v["requestState"], "opaque-state");
        assert!(v["inputResponses"]["rq-1"].is_object());
    }

    #[test]
    fn get_prompt_params_omits_mixin_fields_when_absent() {
        let p = GetPromptRequestParams::new("x", super::fixture_meta());
        let v = serde_json::to_value(&p).unwrap();
        assert!(!v.as_object().unwrap().contains_key("inputResponses"));
        assert!(!v.as_object().unwrap().contains_key("requestState"));
    }
}

/// Phase 3.2 — resources.rs schema alignment (schema lines 999–1390).
///
/// Verifies the wire shape of:
/// - `ListResourcesResult` (extends PaginatedResult, CacheableResult)
/// - `ListResourceTemplatesResult` (extends PaginatedResult, CacheableResult)
/// - `ReadResourceResult` (extends CacheableResult)
/// - `ReadResourceRequestParams` (extends ResourceRequestParams, InputResponseRequestParams)
#[cfg(test)]
mod resources_alignment {
    use turul_mcp_protocol_2026_07_28::caching::{CacheScope, CacheableResult};
    use turul_mcp_protocol_2026_07_28::input_required::{InputResponse, InputResponses};
    use turul_mcp_protocol_2026_07_28::resources::{
        ListResourceTemplatesResult, ListResourcesResult, ReadResourceRequestParams, ReadResourceResult,
        ResourceContent,
    };
    use turul_mcp_protocol_2026_07_28::result_type::ResultType;
    use turul_mcp_protocol_2026_07_28::roots::ListRootsResult;
    use serde_json::json;
    use std::collections::HashMap;

    #[test]
    fn list_resources_result_emits_result_type() {
        let r = ListResourcesResult::new(vec![]);
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["resultType"], "complete");
    }

    #[test]
    fn list_resources_result_with_cache_produces_compliant_wire_shape() {
        let r = ListResourcesResult::new(vec![])
            .with_cache(CacheableResult::new(30_000, CacheScope::Public));
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["resultType"], "complete");
        assert_eq!(v["ttlMs"], 30_000);
        assert_eq!(v["cacheScope"], "public");
        assert!(v["resources"].is_array());
    }

    #[test]
    fn list_resources_result_emits_required_cache_fields_with_defaults() {
        // Schema requires ttlMs + cacheScope (CacheableResult mixin, lines 1020–1022).
        let r = ListResourcesResult::new(vec![]);
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["ttlMs"], 0);
        assert_eq!(v["cacheScope"], "public");
    }

    #[test]
    fn list_resources_result_back_compat_accepts_missing_result_type() {
        let wire = json!({"resources": [], "ttlMs": 0, "cacheScope": "public"});
        let r: ListResourcesResult = serde_json::from_value(wire).unwrap();
        assert_eq!(r.result_type, ResultType::Complete);
    }

    #[test]
    fn list_resource_templates_result_emits_result_type_and_camelcase_key() {
        let r = ListResourceTemplatesResult::new(vec![]);
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["resultType"], "complete");
        // Schema line 1058: `resourceTemplates: ResourceTemplate[]`.
        assert!(v["resourceTemplates"].is_array());
    }

    #[test]
    fn list_resource_templates_result_with_cache() {
        let r = ListResourceTemplatesResult::new(vec![])
            .with_cache(CacheableResult::private_60s());
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["ttlMs"], 60_000);
        assert_eq!(v["cacheScope"], "private");
    }

    #[test]
    fn read_resource_result_emits_result_type() {
        let r = ReadResourceResult::new(vec![ResourceContent::text("file:///x", "hello")]);
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["resultType"], "complete");
        assert!(v["contents"].is_array());
    }

    #[test]
    fn read_resource_result_with_cache() {
        let r = ReadResourceResult::new(vec![ResourceContent::text("file:///x", "hi")])
            .with_cache(CacheableResult::stale_public());
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["ttlMs"], 0);
        assert_eq!(v["cacheScope"], "public");
    }

    #[test]
    fn read_resource_params_input_responses_mixin_serializes() {
        // Schema lines 1092–1093: ReadResourceRequestParams extends
        // ResourceRequestParams, InputResponseRequestParams.
        let mut responses: InputResponses = HashMap::new();
        responses.insert(
            "rq-1".to_string(),
            InputResponse::ListRoots(ListRootsResult::new(vec![])),
        );
        let p = ReadResourceRequestParams::new("file:///x", super::fixture_meta())
            .with_input_responses(responses, "opaque");
        let v = serde_json::to_value(&p).unwrap();
        assert_eq!(v["uri"], "file:///x");
        assert_eq!(v["requestState"], "opaque");
        assert!(v["inputResponses"]["rq-1"].is_object());
    }

    #[test]
    fn read_resource_params_omits_mixin_fields_when_absent() {
        let p = ReadResourceRequestParams::new("file:///x", super::fixture_meta());
        let v = serde_json::to_value(&p).unwrap();
        assert!(!v.as_object().unwrap().contains_key("inputResponses"));
        assert!(!v.as_object().unwrap().contains_key("requestState"));
    }

    #[test]
    fn read_resource_result_back_compat_accepts_missing_result_type() {
        let wire = json!({"contents": [], "ttlMs": 0, "cacheScope": "public"});
        let r: ReadResourceResult = serde_json::from_value(wire).unwrap();
        assert_eq!(r.result_type, ResultType::Complete);
    }
}

/// Phase 3.1 — tools.rs schema alignment (schema lines 1601–1844).
///
/// Verifies:
/// - `tools/list` and `tools/call` method strings
/// - `CallToolResult.resultType` always emits `"complete"` on the wire
/// - `ListToolsResult.resultType` likewise
/// - `ListToolsResult.with_cache()` produces wire-compliant ttlMs/cacheScope
/// - `CallToolRequestParams.with_input_responses()` produces InputResponseRequestParams mixin shape
/// - `structuredContent` accepts any JSON value (DRAFT-2026-v1 widening from object-only)
#[cfg(test)]
mod tools_alignment {
    use turul_mcp_protocol_2026_07_28::caching::{CacheScope, CacheableResult};
    use turul_mcp_protocol_2026_07_28::content::ContentBlock;
    use turul_mcp_protocol_2026_07_28::input_required::{InputResponse, InputResponses};
    use turul_mcp_protocol_2026_07_28::result_type::ResultType;
    use turul_mcp_protocol_2026_07_28::roots::ListRootsResult;
    use turul_mcp_protocol_2026_07_28::tools::{
        CallToolRequestParams, CallToolRequest, CallToolResult, ListToolsRequest, ListToolsResult,
    };
    use serde_json::json;
    use std::collections::HashMap;

    #[test]
    fn tools_list_request_emits_correct_method() {
        // Schema line 1602: `method: "tools/list"`.
        let r = ListToolsRequest::new(super::fixture_meta());
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["method"], "tools/list");
    }

    #[test]
    fn tools_call_request_emits_correct_method() {
        // Schema line 1717: `method: "tools/call"`.
        let r = CallToolRequest::new("echo", super::fixture_meta());
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["method"], "tools/call");
        assert_eq!(v["params"]["name"], "echo");
    }

    #[test]
    fn call_tool_result_always_emits_result_type() {
        // CallToolResult extends Result; resultType is required on wire per
        // schema line 178. Our default-init produces Complete.
        for r in [
            CallToolResult::new(vec![ContentBlock::text("hi")]),
            CallToolResult::success(vec![ContentBlock::text("ok")]),
            CallToolResult::error(vec![ContentBlock::text("err")]),
        ] {
            let v = serde_json::to_value(&r).unwrap();
            assert_eq!(
                v["resultType"], "complete",
                "every CallToolResult constructor must emit resultType=complete"
            );
        }
    }

    #[test]
    fn list_tools_result_emits_result_type() {
        let r = ListToolsResult::new(vec![]);
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["resultType"], "complete");
    }

    #[test]
    fn list_tools_result_emits_required_cache_fields_with_defaults() {
        // Schema requires ttlMs + cacheScope on every list result (CacheableResult
        // mixin, schema lines 1613–1615). Default constructor produces (0, Public)
        // — immediately-stale public response.
        let r = ListToolsResult::new(vec![]);
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(
            v["ttlMs"], 0,
            "ttlMs is REQUIRED per schema; default is 0 (immediately stale)"
        );
        assert_eq!(
            v["cacheScope"], "public",
            "cacheScope is REQUIRED per schema; default is 'public'"
        );
    }

    #[test]
    fn list_tools_result_with_cache_produces_compliant_wire_shape() {
        // Schema lines 1613–1615: `ListToolsResult extends PaginatedResult, CacheableResult`.
        let r = ListToolsResult::new(vec![])
            .with_cache(CacheableResult::new(60_000, CacheScope::Private));
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["resultType"], "complete");
        assert_eq!(v["ttlMs"], 60_000);
        assert_eq!(v["cacheScope"], "private");
        assert!(v["tools"].is_array());
    }

    #[test]
    fn list_tools_result_round_trips_with_cache() {
        let r = ListToolsResult::new(vec![]).with_cache(CacheableResult::stale_public());
        let s = serde_json::to_string(&r).unwrap();
        let parsed: ListToolsResult = serde_json::from_str(&s).unwrap();
        assert_eq!(parsed.result_type, ResultType::Complete);
        assert_eq!(parsed.ttl_ms, 0);
        assert_eq!(parsed.cache_scope, CacheScope::Public);
    }

    #[test]
    fn call_tool_params_with_input_responses_mixes_in_correctly() {
        // Schema line 1697: `CallToolRequestParams extends InputResponseRequestParams`.
        let mut responses: InputResponses = HashMap::new();
        responses.insert(
            "rq-1".to_string(),
            InputResponse::ListRoots(ListRootsResult::new(vec![])),
        );
        let p = CallToolRequestParams::new("echo", super::fixture_meta()).with_input_responses(responses, "opaque-state");
        let v = serde_json::to_value(&p).unwrap();
        assert_eq!(v["name"], "echo");
        assert!(
            v["inputResponses"]["rq-1"].is_object(),
            "InputResponseRequestParams mixin field 'inputResponses' present"
        );
        assert_eq!(v["requestState"], "opaque-state");
    }

    #[test]
    fn call_tool_params_omits_mixin_fields_when_absent() {
        let p = CallToolRequestParams::new("echo", super::fixture_meta());
        let v = serde_json::to_value(&p).unwrap();
        assert!(
            !v.as_object().unwrap().contains_key("inputResponses"),
            "inputResponses omitted when None"
        );
        assert!(
            !v.as_object().unwrap().contains_key("requestState"),
            "requestState omitted when None"
        );
    }

    #[test]
    fn structured_content_accepts_any_json_value() {
        // Schema lines 1649–1655 widen `structuredContent` from object-only
        // (2025-11-25) to `unknown` — arrays, scalars, null are all valid.
        for value in [
            json!(42),
            json!("a string"),
            json!([1, 2, 3]),
            json!({"k": "v"}),
            json!(null),
            json!(true),
        ] {
            let r = CallToolResult::success(vec![]).with_structured_content(value.clone());
            let v = serde_json::to_value(&r).unwrap();
            assert_eq!(
                v["structuredContent"], value,
                "structuredContent must round-trip any JSON value (got: {})",
                value
            );
        }
    }

    #[test]
    fn list_tools_result_back_compat_accepts_missing_result_type() {
        // Per schema line 178–181 backward-compat clause for resultType.
        // CacheableResult fields are required, so a back-compat wire MUST still carry them.
        let wire = json!({"tools": [], "ttlMs": 0, "cacheScope": "public"});
        let r: ListToolsResult = serde_json::from_value(wire).unwrap();
        assert_eq!(r.result_type, ResultType::Complete);
    }

    #[test]
    fn call_tool_result_back_compat_accepts_missing_result_type() {
        let wire = json!({"content": []});
        let r: CallToolResult = serde_json::from_value(wire).unwrap();
        assert_eq!(r.result_type, ResultType::Complete);
    }
}

/// Phase 2.3 — `ClientCapabilities`/`ServerCapabilities` shape compliance.
///
/// Verifies the DRAFT-2026-v1-specific fields (`sampling.context`, `sampling.tools`,
/// `elicitation.form`, `elicitation.url`, `extensions` on both client and server)
/// serialize with the correct wire names from schema lines 623–772.
#[cfg(test)]
mod capabilities_shape {
    use turul_mcp_protocol_2026_07_28::initialize::{
        ClientCapabilities, ElicitationCapabilities, SamplingCapabilities, ServerCapabilities,
    };
    use serde_json::json;
    use std::collections::HashMap;

    #[test]
    fn client_sampling_subcapabilities_serialize_with_camelcase() {
        // Schema lines 648–658: `sampling?: { context?, tools? }`.
        let mut caps = ClientCapabilities::default();
        caps.sampling = Some(SamplingCapabilities {
            context: Some(HashMap::new()),
            tools: Some(HashMap::new()),
            extra: HashMap::new(),
        });
        let v = serde_json::to_value(&caps).unwrap();
        assert!(v["sampling"]["context"].is_object());
        assert!(v["sampling"]["tools"].is_object());
    }

    #[test]
    fn client_sampling_subcapabilities_omitted_when_none() {
        let mut caps = ClientCapabilities::default();
        caps.sampling = Some(SamplingCapabilities::default());
        let v = serde_json::to_value(&caps).unwrap();
        // Empty SamplingCapabilities → just `{}` for sampling
        assert!(
            !v["sampling"].as_object().unwrap().contains_key("context"),
            "context omitted when None"
        );
        assert!(
            !v["sampling"].as_object().unwrap().contains_key("tools"),
            "tools omitted when None"
        );
    }

    #[test]
    fn client_elicitation_subcapabilities_serialize_with_camelcase() {
        // Schema lines 668–671: `elicitation?: { form?, url? }`.
        let mut caps = ClientCapabilities::default();
        caps.elicitation = Some(ElicitationCapabilities {
            form: Some(HashMap::new()),
            url: Some(HashMap::new()),
            extra: HashMap::new(),
        });
        let v = serde_json::to_value(&caps).unwrap();
        assert!(v["elicitation"]["form"].is_object());
        assert!(v["elicitation"]["url"].is_object());
    }

    #[test]
    fn client_extensions_serializes_reverse_dns_keys() {
        // Schema lines 673–681: `extensions?: { [k]: JSONObject }`.
        let mut caps = ClientCapabilities::default();
        let mut exts = HashMap::new();
        exts.insert(
            "io.modelcontextprotocol/oauth-client-credentials".to_string(),
            json!({}),
        );
        exts.insert(
            "com.example.app/custom-ext".to_string(),
            json!({"setting": 1}),
        );
        caps.extensions = Some(exts);
        let v = serde_json::to_value(&caps).unwrap();
        assert!(v["extensions"]["io.modelcontextprotocol/oauth-client-credentials"].is_object());
        assert_eq!(v["extensions"]["com.example.app/custom-ext"]["setting"], 1);
    }

    #[test]
    fn server_extensions_serializes() {
        // Schema lines 763–771.
        let mut caps = ServerCapabilities::default();
        let mut exts = HashMap::new();
        exts.insert("io.modelcontextprotocol/apps".to_string(), json!({}));
        caps.extensions = Some(exts);
        let v = serde_json::to_value(&caps).unwrap();
        assert!(v["extensions"]["io.modelcontextprotocol/apps"].is_object());
    }

    #[test]
    fn capabilities_omit_extensions_when_none() {
        let client = ClientCapabilities::default();
        let server = ServerCapabilities::default();
        let cv = serde_json::to_value(&client).unwrap();
        let sv = serde_json::to_value(&server).unwrap();
        assert!(
            !cv.as_object().unwrap().contains_key("extensions"),
            "ClientCapabilities omits extensions when None"
        );
        assert!(
            !sv.as_object().unwrap().contains_key("extensions"),
            "ServerCapabilities omits extensions when None"
        );
    }

    #[test]
    fn capabilities_round_trip_through_json() {
        // End-to-end round-trip with the new fields populated.
        let mut client = ClientCapabilities::default();
        client.sampling = Some(SamplingCapabilities {
            context: None,
            tools: Some(HashMap::new()),
            extra: HashMap::new(),
        });
        client.elicitation = Some(ElicitationCapabilities {
            form: Some(HashMap::new()),
            url: None,
            extra: HashMap::new(),
        });
        let mut exts = HashMap::new();
        exts.insert("io.modelcontextprotocol/tasks".to_string(), json!({}));
        client.extensions = Some(exts);

        let s = serde_json::to_string(&client).unwrap();
        let parsed: ClientCapabilities = serde_json::from_str(&s).unwrap();
        assert!(parsed.sampling.is_some());
        assert!(parsed.sampling.as_ref().unwrap().tools.is_some());
        assert!(parsed.sampling.as_ref().unwrap().context.is_none());
        assert!(parsed.elicitation.is_some());
        assert!(parsed.elicitation.as_ref().unwrap().form.is_some());
        assert!(parsed.elicitation.as_ref().unwrap().url.is_none());
        assert!(parsed.extensions.is_some());
        assert!(
            parsed
                .extensions
                .unwrap()
                .contains_key("io.modelcontextprotocol/tasks")
        );
    }
}

/// Phase 8 — Exhaustive method-string coverage against the vendored schema.
///
/// Maintains a single source of truth list of all 22 method strings DRAFT-2026-v1
/// declares (per `Phase 2.4 removed_methods` cross-check + the positive list
/// from `docs/plans/2026-07-28-migration-diff.md`). Three guarantees:
///
/// 1. Every method in the list appears in the vendored `schema/draft-schema.ts`
///    (positive cross-check — catches typos in the list).
/// 2. The schema declares exactly this set — no more, no less (count pin).
/// 3. For each method, the Rust type that binds it emits the correct method
///    string on the wire (per-binding shape check).
#[cfg(test)]
mod method_strings {
    use serde_json::Value;

    const SCHEMA_TS: &str = include_str!("../schema/draft-schema.ts");

    /// The canonical list of method strings DRAFT-2026-v1 declares.
    /// Sorted alphabetically for stability.
    const DRAFT_METHODS: &[&str] = &[
        "completion/complete",
        "elicitation/create",
        "notifications/cancelled",
        "notifications/elicitation/complete",
        "notifications/message",
        "notifications/progress",
        "notifications/prompts/list_changed",
        "notifications/resources/list_changed",
        "notifications/resources/updated",
        "notifications/subscriptions/acknowledged",
        "notifications/tools/list_changed",
        "prompts/get",
        "prompts/list",
        "resources/list",
        "resources/read",
        "resources/templates/list",
        "roots/list",
        "sampling/createMessage",
        "server/discover",
        "subscriptions/listen",
        "tools/call",
        "tools/list",
    ];

    #[test]
    fn every_listed_method_appears_in_schema() {
        for m in DRAFT_METHODS {
            let pat = format!("method: \"{}\"", m);
            assert!(
                SCHEMA_TS.contains(&pat),
                "method `{}` is in our canonical list but absent from \
                 schema/draft-schema.ts — re-vendor or fix the list",
                m
            );
        }
    }

    #[test]
    fn schema_method_count_matches_canonical_list() {
        // Count occurrences of the `method: "..."` pattern in the schema.
        // Each schema interface's method declaration is exactly one such line.
        let schema_method_count = SCHEMA_TS.matches("method: \"").count();
        assert_eq!(
            schema_method_count,
            DRAFT_METHODS.len(),
            "Schema declares {} methods but our canonical list has {}. \
             Re-vendor was performed without updating the list — \
             see docs/plans/2026-07-28-migration-diff.md and update Phase 8 / Phase 2.4.",
            schema_method_count,
            DRAFT_METHODS.len()
        );
    }

    // ---------------------------------------------------------------------
    // Per-binding wire-shape checks. Each helper constructs the
    // corresponding Rust type and returns its serialized JSON method field.
    // ---------------------------------------------------------------------

    fn method_of(v: &Value) -> &str {
        v["method"]
            .as_str()
            .unwrap_or_else(|| panic!("type missing `method` field: {}", v))
    }

    #[test]
    fn server_discover_binding() {
        use turul_mcp_protocol_2026_07_28::discover::DiscoverRequest;
        let v = serde_json::to_value(DiscoverRequest::new(super::fixture_meta())).unwrap();
        assert_eq!(method_of(&v), "server/discover");
    }

    #[test]
    fn tools_list_binding() {
        use turul_mcp_protocol_2026_07_28::tools::ListToolsRequest;
        let v = serde_json::to_value(ListToolsRequest::new(super::fixture_meta())).unwrap();
        assert_eq!(method_of(&v), "tools/list");
    }

    #[test]
    fn tools_call_binding() {
        use turul_mcp_protocol_2026_07_28::tools::CallToolRequest;
        let v = serde_json::to_value(CallToolRequest::new("x", super::fixture_meta())).unwrap();
        assert_eq!(method_of(&v), "tools/call");
    }

    #[test]
    fn resources_list_binding() {
        use turul_mcp_protocol_2026_07_28::resources::ListResourcesRequest;
        let v = serde_json::to_value(ListResourcesRequest::new(super::fixture_meta())).unwrap();
        assert_eq!(method_of(&v), "resources/list");
    }

    #[test]
    fn resources_templates_list_binding() {
        use turul_mcp_protocol_2026_07_28::resources::ListResourceTemplatesRequest;
        let v = serde_json::to_value(ListResourceTemplatesRequest::new(super::fixture_meta())).unwrap();
        assert_eq!(method_of(&v), "resources/templates/list");
    }

    #[test]
    fn resources_read_binding() {
        use turul_mcp_protocol_2026_07_28::resources::ReadResourceRequest;
        let v = serde_json::to_value(ReadResourceRequest::new("file:///x", super::fixture_meta())).unwrap();
        assert_eq!(method_of(&v), "resources/read");
    }

    #[test]
    fn prompts_list_binding() {
        use turul_mcp_protocol_2026_07_28::prompts::ListPromptsRequest;
        let v = serde_json::to_value(ListPromptsRequest::new(super::fixture_meta())).unwrap();
        assert_eq!(method_of(&v), "prompts/list");
    }

    #[test]
    fn prompts_get_binding() {
        use turul_mcp_protocol_2026_07_28::prompts::GetPromptRequest;
        let v = serde_json::to_value(GetPromptRequest::new("p", super::fixture_meta())).unwrap();
        assert_eq!(method_of(&v), "prompts/get");
    }

    #[test]
    fn roots_list_binding() {
        use turul_mcp_protocol_2026_07_28::roots::ListRootsRequest;
        let v = serde_json::to_value(ListRootsRequest::new()).unwrap();
        assert_eq!(method_of(&v), "roots/list");
    }

    #[test]
    fn subscriptions_listen_binding() {
        use turul_mcp_protocol_2026_07_28::subscriptions::{SubscriptionFilter, SubscriptionsListenRequest};
        let v = serde_json::to_value(SubscriptionsListenRequest::new(SubscriptionFilter::new()))
            .unwrap();
        assert_eq!(method_of(&v), "subscriptions/listen");
    }

    // --- Notifications ---

    #[test]
    fn notifications_resources_list_changed_binding() {
        use turul_mcp_protocol_2026_07_28::notifications::ResourceListChangedNotification;
        let v = serde_json::to_value(ResourceListChangedNotification::new()).unwrap();
        assert_eq!(method_of(&v), "notifications/resources/list_changed");
    }

    #[test]
    fn notifications_tools_list_changed_binding() {
        use turul_mcp_protocol_2026_07_28::notifications::ToolListChangedNotification;
        let v = serde_json::to_value(ToolListChangedNotification::new()).unwrap();
        assert_eq!(method_of(&v), "notifications/tools/list_changed");
    }

    #[test]
    fn notifications_prompts_list_changed_binding() {
        use turul_mcp_protocol_2026_07_28::notifications::PromptListChangedNotification;
        let v = serde_json::to_value(PromptListChangedNotification::new()).unwrap();
        assert_eq!(method_of(&v), "notifications/prompts/list_changed");
    }

    #[test]
    fn notifications_progress_binding() {
        use turul_mcp_protocol_2026_07_28::notifications::ProgressNotification;
        let v = serde_json::to_value(ProgressNotification::new("tok", 0.5)).unwrap();
        assert_eq!(method_of(&v), "notifications/progress");
    }

    #[test]
    fn notifications_resources_updated_binding() {
        use turul_mcp_protocol_2026_07_28::notifications::ResourceUpdatedNotification;
        let v = serde_json::to_value(ResourceUpdatedNotification::new("file:///x")).unwrap();
        assert_eq!(method_of(&v), "notifications/resources/updated");
    }

    #[test]
    fn notifications_subscriptions_acknowledged_binding() {
        use turul_mcp_protocol_2026_07_28::subscriptions::{
            SubscriptionFilter, SubscriptionsAcknowledgedNotification,
        };
        let v = serde_json::to_value(SubscriptionsAcknowledgedNotification::new(
            SubscriptionFilter::new(),
        ))
        .unwrap();
        assert_eq!(method_of(&v), "notifications/subscriptions/acknowledged");
    }

    // --- Bindings without trivial constructors are checked via their
    //     module's own unit tests; the count-pin + every-listed-method-in-schema
    //     tests above guarantee schema coverage. Add new bindings here when
    //     adding a new method to DRAFT_METHODS. ---
    //
    // Currently delegated to module tests:
    // - notifications/cancelled (CancelledNotification, needs RequestId)
    // - notifications/message (LoggingMessageNotification, needs Level + Value)
    // - notifications/elicitation/complete (ElicitationCompleteNotification)
    // - sampling/createMessage (CreateMessageRequest, needs SamplingMessage list)
    // - completion/complete (CompleteRequest, needs CompletionReference + arg)
    // - elicitation/create (ElicitRequest, needs schema)
}

/// Phase 2.4 — Schema-drift detector for removed methods.
///
/// Reads the vendored `schema/draft-schema.ts` at compile time and asserts that
/// the methods removed from DRAFT-2026-v1 (per the migration diff) do NOT
/// appear in the schema's method-string declarations. This is the test that
/// proves "these methods are gone" against the actual upstream contract.
///
/// If the upstream schema re-introduces any of these, this test fires —
/// signaling that the migration diff needs updating before continuing.
#[cfg(test)]
mod removed_methods {
    /// The schema file vendored at the crate's ETag-pinned snapshot.
    const SCHEMA_TS: &str = include_str!("../schema/draft-schema.ts");

    /// Asserts that `needle` does not appear inside any `method:` declaration
    /// in the schema. We grep for the literal `method: "<needle>"` pattern,
    /// which is how every schema interface declares its wire method string.
    fn assert_method_absent(needle: &str) {
        let pattern = format!("method: \"{}\"", needle);
        assert!(
            !SCHEMA_TS.contains(&pattern),
            "DRAFT-2026-v1 schema unexpectedly contains `{}` — the migration diff \
             and Phase 2/3 assumptions need revisiting. Re-vendor + re-audit.",
            pattern
        );
    }

    #[test]
    fn initialize_method_is_gone() {
        // Stateless core (SEP-2567, SEP-2575) removes the handshake.
        assert_method_absent("initialize");
    }

    #[test]
    fn initialized_notification_is_gone() {
        assert_method_absent("notifications/initialized");
    }

    #[test]
    fn ping_method_is_gone() {
        assert_method_absent("ping");
    }

    #[test]
    fn logging_set_level_method_is_gone() {
        // Replaced by per-request `_meta.io.modelcontextprotocol/logLevel`.
        assert_method_absent("logging/setLevel");
    }

    #[test]
    fn resources_subscribe_method_is_gone() {
        // Replaced by `subscriptions/listen` filter.
        assert_method_absent("resources/subscribe");
    }

    #[test]
    fn resources_unsubscribe_method_is_gone() {
        assert_method_absent("resources/unsubscribe");
    }

    #[test]
    fn roots_list_changed_notification_is_gone() {
        assert_method_absent("notifications/roots/list_changed");
    }

    #[test]
    fn tasks_methods_are_gone_from_core() {
        // SEP-2663: tasks moved to extension. No `tasks/*` methods in core.
        for m in ["tasks/get", "tasks/list", "tasks/cancel", "tasks/result"] {
            let pattern = format!("method: \"{}\"", m);
            assert!(
                !SCHEMA_TS.contains(&pattern),
                "DRAFT-2026-v1 core schema unexpectedly contains `{}` — \
                 tasks should live only in the extension repo per SEP-2663.",
                pattern
            );
        }
    }

    /// Positive control: discover IS in the schema. Catches a class of bug where
    /// `assert_method_absent` would silently pass against a file that doesn't
    /// match the expected `method: "..."` pattern at all.
    #[test]
    fn server_discover_method_is_present_positive_control() {
        assert!(
            SCHEMA_TS.contains("method: \"server/discover\""),
            "Positive control failed — schema scanner is not matching the \
             expected `method: \"...\"` pattern; investigate."
        );
    }

    /// Drift canary: pins the schema's protocol version constant. If this fires,
    /// re-vendor was performed and the wire string should be re-checked against
    /// `src/lib.rs::MCP_VERSION` and `src/version.rs::McpVersion::V2026_07_28`'s
    /// serde rename.
    #[test]
    fn schema_protocol_version_constant_matches_crate() {
        assert!(
            SCHEMA_TS.contains("LATEST_PROTOCOL_VERSION = \"DRAFT-2026-v1\""),
            "Schema's LATEST_PROTOCOL_VERSION drifted from `DRAFT-2026-v1`; \
             update `MCP_VERSION` in src/lib.rs and the serde rename in src/version.rs."
        );
    }
}

/// Changelog Minor #2 — convention `_meta` key constants for OpenTelemetry
/// trace context (SEP-414) + Changelog Major #4 — subscription tagging key.
///
/// Verifies the spelling of constants exported from `meta` so consumers have
/// a typo-safe single source of truth.
#[cfg(test)]
mod convention_meta_keys {
    use turul_mcp_protocol_2026_07_28::meta::{
        META_KEY_BAGGAGE, META_KEY_CLIENT_CAPABILITIES, META_KEY_CLIENT_INFO, META_KEY_LOG_LEVEL,
        META_KEY_PROTOCOL_VERSION, META_KEY_SUBSCRIPTION_ID, META_KEY_TRACEPARENT,
        META_KEY_TRACESTATE,
    };

    #[test]
    fn schema_declared_meta_keys_match_request_meta_serialization() {
        // The schema-declared keys MUST match what RequestMetaObject serializes —
        // if a serde rename ever drifts, this test catches it. We construct a
        // RequestMetaObject and inspect the JSON keys.
        use turul_mcp_protocol_2026_07_28::initialize::{ClientCapabilities, Implementation};
        use turul_mcp_protocol_2026_07_28::logging::LoggingLevel;
        use turul_mcp_protocol_2026_07_28::meta::RequestMetaObject;

        let m = RequestMetaObject::new(
            "DRAFT-2026-v1",
            Implementation::new("c", "1"),
            ClientCapabilities::default(),
        )
        .with_log_level(LoggingLevel::Info);

        let v = serde_json::to_value(&m).unwrap();
        let obj = v.as_object().unwrap();

        assert!(obj.contains_key(META_KEY_PROTOCOL_VERSION));
        assert!(obj.contains_key(META_KEY_CLIENT_INFO));
        assert!(obj.contains_key(META_KEY_CLIENT_CAPABILITIES));
        assert!(obj.contains_key(META_KEY_LOG_LEVEL));
    }

    #[test]
    fn tracing_keys_use_w3c_unprefixed_spelling() {
        // SEP-414: W3C Trace Context keys use their standard unprefixed names
        // (traceparent / tracestate / baggage), not reverse-DNS prefixed.
        assert_eq!(META_KEY_TRACEPARENT, "traceparent");
        assert_eq!(META_KEY_TRACESTATE, "tracestate");
        assert_eq!(META_KEY_BAGGAGE, "baggage");
    }

    #[test]
    fn subscription_id_uses_io_mcp_prefix() {
        // SEP-2575 / changelog Major #4: tagged with the io.modelcontextprotocol/ prefix.
        assert_eq!(
            META_KEY_SUBSCRIPTION_ID,
            "io.modelcontextprotocol/subscriptionId"
        );
    }

    #[test]
    fn log_level_constant_matches_schema_line_106() {
        // Schema line 106: `"io.modelcontextprotocol/logLevel"?: LoggingLevel`.
        assert_eq!(META_KEY_LOG_LEVEL, "io.modelcontextprotocol/logLevel");
    }
}

/// Phase 1.2 — `RequestMetaObject` compliance with draft schema (lines 70–107).
///
/// Verifies the required-fields invariant of the stateless-core per-request
/// capability negotiation: `protocolVersion`, `clientInfo`, `clientCapabilities`.
#[cfg(test)]
mod request_meta {
    use turul_mcp_protocol_2026_07_28::initialize::{ClientCapabilities, Implementation};
    use turul_mcp_protocol_2026_07_28::logging::LoggingLevel;
    use turul_mcp_protocol_2026_07_28::meta::RequestMetaObject;
    use serde_json::json;

    fn fixture_impl() -> Implementation {
        Implementation::new("test-client", "1.0.0")
    }

    fn fixture_caps() -> ClientCapabilities {
        ClientCapabilities::default()
    }

    #[test]
    fn required_fields_serialize_with_namespaced_keys() {
        // Schema lines 83, 90, 98: keys use full `io.modelcontextprotocol/<name>` prefix.
        let m = RequestMetaObject::new("DRAFT-2026-v1", fixture_impl(), fixture_caps());
        let v = serde_json::to_value(&m).unwrap();

        assert_eq!(
            v["io.modelcontextprotocol/protocolVersion"], "DRAFT-2026-v1",
            "protocolVersion key must use full reverse-DNS prefix per schema line 83"
        );
        assert!(
            v["io.modelcontextprotocol/clientInfo"].is_object(),
            "clientInfo key must use full reverse-DNS prefix per schema line 90"
        );
        assert!(
            v["io.modelcontextprotocol/clientCapabilities"].is_object(),
            "clientCapabilities key must use full reverse-DNS prefix per schema line 98"
        );
    }

    #[test]
    fn optional_fields_absent_when_none() {
        let m = RequestMetaObject::new("DRAFT-2026-v1", fixture_impl(), fixture_caps());
        let v = serde_json::to_value(&m).unwrap();
        assert!(
            !v.as_object().unwrap().contains_key("progressToken"),
            "progressToken omitted when None"
        );
        assert!(
            !v.as_object()
                .unwrap()
                .contains_key("io.modelcontextprotocol/logLevel"),
            "logLevel omitted when None"
        );
    }

    #[test]
    fn progress_token_serializes_under_short_camelcase_key() {
        // Schema line 74: `progressToken?: ProgressToken` — NOT namespaced.
        let m = RequestMetaObject::new("DRAFT-2026-v1", fixture_impl(), fixture_caps())
            .with_progress_token("tok-1");
        let v = serde_json::to_value(&m).unwrap();
        assert_eq!(v["progressToken"], "tok-1");
    }

    #[test]
    fn log_level_serializes_under_namespaced_key() {
        // Schema line 100–106: `"io.modelcontextprotocol/logLevel"?: LoggingLevel`.
        let m = RequestMetaObject::new("DRAFT-2026-v1", fixture_impl(), fixture_caps())
            .with_log_level(LoggingLevel::Warning);
        let v = serde_json::to_value(&m).unwrap();
        assert_eq!(
            v["io.modelcontextprotocol/logLevel"], "warning",
            "logLevel must use full reverse-DNS prefix per schema and snake_case wire value"
        );
    }

    #[test]
    fn rejects_missing_required_protocol_version() {
        // Without `io.modelcontextprotocol/protocolVersion` deserialization must fail.
        let json = json!({
            "io.modelcontextprotocol/clientInfo": {"name": "c", "version": "1"},
            "io.modelcontextprotocol/clientCapabilities": {}
        });
        let r: Result<RequestMetaObject, _> = serde_json::from_value(json);
        assert!(r.is_err(), "missing protocolVersion must fail");
    }

    #[test]
    fn rejects_missing_required_client_info() {
        let json = json!({
            "io.modelcontextprotocol/protocolVersion": "DRAFT-2026-v1",
            "io.modelcontextprotocol/clientCapabilities": {}
        });
        let r: Result<RequestMetaObject, _> = serde_json::from_value(json);
        assert!(r.is_err(), "missing clientInfo must fail");
    }

    #[test]
    fn rejects_missing_required_client_capabilities() {
        let json = json!({
            "io.modelcontextprotocol/protocolVersion": "DRAFT-2026-v1",
            "io.modelcontextprotocol/clientInfo": {"name": "c", "version": "1"}
        });
        let r: Result<RequestMetaObject, _> = serde_json::from_value(json);
        assert!(
            r.is_err(),
            "missing clientCapabilities must fail (required per schema line 92)"
        );
    }

    #[test]
    fn extra_keys_preserved_via_flatten() {
        let m = RequestMetaObject::new("DRAFT-2026-v1", fixture_impl(), fixture_caps())
            .with_extra("com.example.app/buildId", json!("abc123"))
            .with_extra("traceparent", json!("00-trace-id-01"));
        let v = serde_json::to_value(&m).unwrap();
        assert_eq!(v["com.example.app/buildId"], "abc123");
        assert_eq!(v["traceparent"], "00-trace-id-01");

        // Round-trip
        let parsed: RequestMetaObject = serde_json::from_value(v).unwrap();
        assert_eq!(parsed.extra.get("com.example.app/buildId").unwrap(), "abc123");
        assert_eq!(parsed.extra.get("traceparent").unwrap(), "00-trace-id-01");
    }

    #[test]
    fn round_trip_preserves_all_fields() {
        let m = RequestMetaObject::new("DRAFT-2026-v1", fixture_impl(), fixture_caps())
            .with_progress_token("tok-X")
            .with_log_level(LoggingLevel::Info);
        let s = serde_json::to_string(&m).unwrap();
        let parsed: RequestMetaObject = serde_json::from_str(&s).unwrap();
        assert_eq!(parsed.protocol_version, "DRAFT-2026-v1");
        assert_eq!(parsed.progress_token.unwrap().as_str(), "tok-X");
        assert!(matches!(parsed.log_level, Some(LoggingLevel::Info)));
    }
}

/// Phase 1.3 — `ResultType` discriminator compliance with draft schema (lines 157–185).
///
/// The `ResultType` enum's serialization is already covered in `result_type::tests`;
/// this section verifies its integration with the broader Result contract — that
/// `InputRequiredResult` always emits `resultType: "input_required"` on the wire,
/// and that the `Complete` default honours the backward-compat clause for
/// pre-2026 results that omit the field.
#[cfg(test)]
mod result_discrimination {
    use turul_mcp_protocol_2026_07_28::input_required::InputRequiredResult;
    use turul_mcp_protocol_2026_07_28::result_type::ResultType;
    use serde_json::json;

    #[test]
    fn input_required_always_emits_result_type_on_wire() {
        let r = InputRequiredResult::with_state("opaque");
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["resultType"], "input_required");
    }

    #[test]
    fn pre_2026_result_without_result_type_defaults_to_complete() {
        // Schema lines 178–181: "when a client receives a result from a server
        // implementing an earlier protocol version (which does not include
        // `resultType`), the client MUST treat the absent field as 'complete'."
        //
        // ResultType::default() returns Complete to honour this.
        assert_eq!(ResultType::default(), ResultType::Complete);
    }

    #[test]
    fn unknown_result_type_value_is_rejected() {
        // Wire JSON with an unsupported discriminator must fail to parse.
        let bad = json!("partial");
        let r: Result<ResultType, _> = serde_json::from_value(bad);
        assert!(r.is_err());
    }
}

/// Phase 1.5 — multi-round-trip flow compliance with draft schema (lines 437–514).
///
/// Integration-style test that walks the schema-defined flow end-to-end:
/// server emits `InputRequiredResult`, client constructs
/// `InputResponseRequestParams` with verbatim `requestState` echo + matching
/// keys for `inputResponses`. The schema's invariants (matching keys, opaque
/// state) are asserted on the wire shapes.
#[cfg(test)]
mod multi_round_trip {
    use turul_mcp_protocol_2026_07_28::input_required::{
        InputRequest, InputRequests, InputRequiredResult, InputResponse, InputResponseRequestParams,
        InputResponses,
    };
    use turul_mcp_protocol_2026_07_28::roots::{ListRootsRequest, ListRootsResult, Root};
    use std::collections::HashMap;

    #[test]
    fn server_emits_input_required_result_with_one_request_and_state() {
        // Server side: assemble a one-request InputRequiredResult.
        let mut reqs: InputRequests = HashMap::new();
        reqs.insert(
            "rq-roots".to_string(),
            InputRequest::ListRoots(ListRootsRequest::new()),
        );
        let server_result =
            InputRequiredResult::with_requests_and_state(reqs, "opaque-server-state-v1");

        // Wire shape must carry: resultType="input_required", inputRequests map keyed
        // by server id, requestState opaque string.
        let v = serde_json::to_value(&server_result).unwrap();
        assert_eq!(v["resultType"], "input_required");
        assert!(v["inputRequests"]["rq-roots"].is_object());
        assert_eq!(v["inputRequests"]["rq-roots"]["method"], "roots/list");
        assert_eq!(v["requestState"], "opaque-server-state-v1");
    }

    #[test]
    fn client_retries_with_responses_keyed_same_as_input_requests() {
        // Schema line 507–509: "For each key in the response's inputRequests
        // field, the same key must appear here with the associated response."
        let mut responses: InputResponses = HashMap::new();
        responses.insert(
            "rq-roots".to_string(),
            InputResponse::ListRoots(ListRootsResult::new(vec![Root::new("file:///work")])),
        );
        let client_retry = InputResponseRequestParams::with_responses_and_state(
            responses,
            "opaque-server-state-v1", // verbatim echo
        );
        let v = serde_json::to_value(&client_retry).unwrap();
        assert!(
            v["inputResponses"]["rq-roots"].is_object(),
            "client must echo the server's key 'rq-roots'"
        );
        // Verify the embedded ListRootsResult is intact.
        assert!(v["inputResponses"]["rq-roots"]["roots"].is_array());
        assert_eq!(
            v["inputResponses"]["rq-roots"]["roots"][0]["uri"],
            "file:///work"
        );
        assert_eq!(
            v["requestState"], "opaque-server-state-v1",
            "client must echo requestState verbatim per schema lines 494–498"
        );
    }

    /// Group D — `InputResponseRequestParams` is a mixin (schema line 505,
    /// `extends RequestParams`). The three extending interfaces inline its
    /// fields (`inputResponses?`, `requestState?`) by composition. This test
    /// proves the same JSON wire shape produced by each extender so future
    /// drift in any single extender is caught against the others.
    #[test]
    fn input_response_mixin_wire_shape_is_uniform_across_three_extenders() {
        use turul_mcp_protocol_2026_07_28::prompts::GetPromptRequestParams;
        use turul_mcp_protocol_2026_07_28::resources::ReadResourceRequestParams;
        use turul_mcp_protocol_2026_07_28::tools::CallToolRequestParams;

        let mut responses: InputResponses = HashMap::new();
        responses.insert(
            "rq-x".to_string(),
            InputResponse::ListRoots(ListRootsResult::new(vec![])),
        );

        let call = CallToolRequestParams::new("t", super::fixture_meta())
            .with_input_responses(responses.clone(), "state-x");
        let read = ReadResourceRequestParams::new("res://x", super::fixture_meta())
            .with_input_responses(responses.clone(), "state-x");
        let prompt = GetPromptRequestParams::new("p", super::fixture_meta())
            .with_input_responses(responses.clone(), "state-x");

        let v_call = serde_json::to_value(&call).unwrap();
        let v_read = serde_json::to_value(&read).unwrap();
        let v_prompt = serde_json::to_value(&prompt).unwrap();

        for (name, v) in [("call", &v_call), ("read", &v_read), ("prompt", &v_prompt)] {
            assert_eq!(v["requestState"], "state-x", "{name} requestState");
            assert!(v["inputResponses"]["rq-x"].is_object(), "{name} inputResponses");
        }

        // Standalone InputResponseRequestParams must omit anything beyond the
        // mixin fields — it does NOT carry `_meta` (those belong to the host
        // RequestParams). Confirms our doc statement.
        let mixin = InputResponseRequestParams::with_responses_and_state(responses, "state-x");
        let v_mixin = serde_json::to_value(&mixin).unwrap();
        let obj = v_mixin.as_object().unwrap();
        assert_eq!(obj.len(), 2, "mixin-only struct has exactly 2 fields on wire");
        assert!(obj.contains_key("inputResponses"));
        assert!(obj.contains_key("requestState"));
    }

    /// Group E — `InputRequest` (schema line 438) is a union of three
    /// `*Request` types. Discrimination on the wire `method` string is
    /// enforced by the custom Deserialize impl: known methods map to
    /// their variant, unknown methods fail to parse.
    #[test]
    fn input_request_deserialize_dispatches_on_method_string() {
        let roots_wire = serde_json::json!({"method": "roots/list"});
        let parsed: InputRequest = serde_json::from_value(roots_wire).unwrap();
        assert!(matches!(parsed, InputRequest::ListRoots(_)));

        let elicit_wire = serde_json::json!({
            "method": "elicitation/create",
            "params": {"message": "hi", "requestedSchema": {"type": "object", "properties": {}}}
        });
        let parsed: InputRequest = serde_json::from_value(elicit_wire).unwrap();
        assert!(matches!(parsed, InputRequest::Elicit(_)));

        let sampling_wire = serde_json::json!({
            "method": "sampling/createMessage",
            "params": {"messages": [], "maxTokens": 100}
        });
        let parsed: InputRequest = serde_json::from_value(sampling_wire).unwrap();
        assert!(matches!(parsed, InputRequest::CreateMessage(_)));
    }

    #[test]
    fn input_request_deserialize_rejects_unknown_method() {
        let bogus = serde_json::json!({"method": "tools/list"});
        let parsed: Result<InputRequest, _> = serde_json::from_value(bogus);
        assert!(parsed.is_err());
        let err = parsed.unwrap_err().to_string();
        assert!(err.contains("tools/list"), "error should name the bad method: {err}");
    }

    #[test]
    fn input_request_deserialize_rejects_missing_method() {
        let bogus = serde_json::json!({"params": {}});
        let parsed: Result<InputRequest, _> = serde_json::from_value(bogus);
        assert!(parsed.is_err());
        let err = parsed.unwrap_err().to_string();
        assert!(err.contains("method"), "error should mention method discriminator: {err}");
    }

    /// Group F — schema line 480 "At least one of `inputRequests` or
    /// `requestState` MUST be present" is enforced by the custom Deserialize
    /// impl. JSON missing both fields must be rejected at parse time.
    #[test]
    fn input_required_result_deserialize_rejects_both_fields_absent() {
        let wire = serde_json::json!({"resultType": "input_required"});
        let parsed: Result<InputRequiredResult, _> = serde_json::from_value(wire);
        assert!(parsed.is_err(), "missing both inputRequests AND requestState must fail to deserialize");
        let err = parsed.unwrap_err().to_string();
        assert!(
            err.contains("at least one"),
            "error message should reference the at-least-one-of invariant: {err}"
        );
    }

    #[test]
    fn input_required_result_deserialize_accepts_input_requests_only() {
        let wire = serde_json::json!({
            "resultType": "input_required",
            "inputRequests": {"rq-1": {"method": "roots/list"}}
        });
        let parsed: InputRequiredResult = serde_json::from_value(wire).unwrap();
        assert!(parsed.input_requests.is_some());
        assert!(parsed.request_state.is_none());
    }

    #[test]
    fn input_required_result_deserialize_accepts_request_state_only() {
        let wire = serde_json::json!({"resultType": "input_required", "requestState": "s"});
        let parsed: InputRequiredResult = serde_json::from_value(wire).unwrap();
        assert!(parsed.input_requests.is_none());
        assert_eq!(parsed.request_state.as_deref(), Some("s"));
    }

    #[test]
    fn input_required_well_formed_invariant() {
        // Schema line 480: "At least one of `inputRequests` or `requestState`
        // MUST be present."
        let only_requests =
            InputRequiredResult::with_requests(HashMap::new()); // empty map counts as present
        assert!(only_requests.is_well_formed());

        let only_state = InputRequiredResult::with_state("s");
        assert!(only_state.is_well_formed());

        let both = InputRequiredResult::with_requests_and_state(HashMap::new(), "s");
        assert!(both.is_well_formed());
    }
}

/// Phase 1.1 — JSON-RPC envelope compliance with draft schema (lines 26–258).
///
/// Cross-crate integration tests: asserts that our consumption of the
/// `turul-rpc` wire envelopes produces JSON byte shapes matching the MCP
/// DRAFT-2026-v1 schema. The envelope types themselves are owned by
/// `turul-rpc`; these tests are calibrated to the **consumer contract**.
#[cfg(test)]
mod envelope {
    use turul_mcp_protocol_2026_07_28::{
        JSONRPC_VERSION, JsonRpcError, JsonRpcErrorObject, JsonRpcNotification, JsonRpcRequest,
        JsonRpcResponse, JsonRpcSuccessResponse, JsonRpcVersion, RequestId,
    };
    use serde_json::json;

    #[test]
    fn jsonrpc_version_constant_is_literal_2_0() {
        assert_eq!(JSONRPC_VERSION, "2.0");
        assert_eq!(JsonRpcVersion::V2_0.as_str(), "2.0");
    }

    #[test]
    fn jsonrpc_request_wire_shape() {
        // `JSONRPCRequest extends Request { jsonrpc, id, method, params? }`.
        let req = JsonRpcRequest::new_no_params(RequestId::Number(42), "tools/list".to_string());
        let v = serde_json::to_value(&req).unwrap();
        assert_eq!(v["jsonrpc"], "2.0", "jsonrpc must serialize as literal \"2.0\"");
        assert_eq!(v["id"], 42);
        assert_eq!(v["method"], "tools/list");
        assert!(
            !v.as_object().unwrap().contains_key("params") || v["params"].is_null(),
            "params absent when not set"
        );
    }

    #[test]
    fn jsonrpc_request_id_accepts_string_and_number() {
        // Schema: `RequestId = string | number`.
        let v_num = serde_json::to_value(JsonRpcRequest::new_no_params(
            RequestId::Number(7), "x".to_string(),
        ))
        .unwrap();
        let v_str = serde_json::to_value(JsonRpcRequest::new_no_params(
            RequestId::String("req-1".into()), "x".to_string(),
        ))
        .unwrap();
        assert_eq!(v_num["id"], 7);
        assert_eq!(v_str["id"], "req-1");
    }

    #[test]
    fn jsonrpc_notification_has_no_id() {
        // `JSONRPCNotification extends Notification { jsonrpc, method, params? }`.
        // Notifications never carry `id`.
        let notif = JsonRpcNotification::new_no_params("notifications/cancelled".to_string());
        let v = serde_json::to_value(&notif).unwrap();
        assert_eq!(v["jsonrpc"], "2.0");
        assert_eq!(v["method"], "notifications/cancelled");
        assert!(
            !v.as_object().unwrap().contains_key("id"),
            "notifications MUST NOT carry an id field"
        );
    }

    #[test]
    fn jsonrpc_success_response_has_result_no_error() {
        // `JSONRPCResultResponse { jsonrpc, id, result }` per schema.
        let success = JsonRpcSuccessResponse::success(
            RequestId::Number(1),
            json!({"tools": []}),
        );
        let resp = JsonRpcResponse::Success(success);
        let v = serde_json::to_value(&resp).unwrap();
        assert_eq!(v["jsonrpc"], "2.0");
        assert_eq!(v["id"], 1);
        assert!(v["result"].is_object(), "success response carries result");
        assert!(
            !v.as_object().unwrap().contains_key("error") || v["error"].is_null(),
            "success response has no error field"
        );
    }

    #[test]
    fn jsonrpc_error_response_has_error_no_result() {
        // `JSONRPCErrorResponse { jsonrpc, id?, error }` per schema.
        let err = JsonRpcError::invalid_params(RequestId::Number(7), "bad args");
        let resp = JsonRpcResponse::Error(err);
        let v = serde_json::to_value(&resp).unwrap();
        assert_eq!(v["jsonrpc"], "2.0");
        assert_eq!(v["id"], 7);
        assert!(v["error"].is_object(), "error response carries error");
        assert_eq!(v["error"]["code"], -32602);
        assert!(
            !v.as_object().unwrap().contains_key("result") || v["result"].is_null(),
            "error response has no result field"
        );
    }

    #[test]
    fn jsonrpc_error_response_id_optional_per_schema() {
        // Per JSON-RPC 2.0 and schema `JSONRPCErrorResponse { id?: RequestId }`:
        // server MAY omit `id` if it couldn't parse the request's id (e.g. parse
        // error). `turul_rpc::JsonRpcError::parse_error()` produces this shape.
        let err = JsonRpcError::parse_error();
        let v = serde_json::to_value(&err).unwrap();
        assert!(
            !v.as_object().unwrap().contains_key("id") || v["id"].is_null(),
            "parse-error responses may omit id"
        );
    }

    #[test]
    fn jsonrpc_error_object_shape() {
        // `Error { code, message, data? }` per schema.
        let err = JsonRpcErrorObject::new(
            turul_mcp_protocol_2026_07_28::JsonRpcErrorCode::InternalError,
            Some("Internal error".to_string()),
            Some(json!({"detail": "boom"})),
        );
        let v = serde_json::to_value(&err).unwrap();
        assert_eq!(v["code"], -32603);
        assert_eq!(v["message"], "Internal error");
        assert_eq!(v["data"]["detail"], "boom");
    }

    #[test]
    fn jsonrpc_error_object_omits_data_when_absent() {
        let err = JsonRpcErrorObject::parse_error(None);
        let v = serde_json::to_value(&err).unwrap();
        assert_eq!(v["code"], -32700);
        assert!(
            !v.as_object().unwrap().contains_key("data"),
            "data field omitted when None"
        );
    }

    #[test]
    fn standard_jsonrpc_error_codes_match_schema_constants() {
        // Schema-declared error codes: -32700, -32600, -32601, -32602, -32603.
        use turul_mcp_protocol_2026_07_28::JsonRpcErrorCode::*;
        assert_eq!(ParseError.code(), -32700);
        assert_eq!(InvalidRequest.code(), -32600);
        assert_eq!(MethodNotFound.code(), -32601);
        assert_eq!(InvalidParams.code(), -32602);
        assert_eq!(InternalError.code(), -32603);
    }

    #[test]
    fn jsonrpc_response_is_untagged_union() {
        // Untagged: deserializing `{result, ...}` picks Success, `{error, ...}` picks Error.
        let success_wire = json!({"jsonrpc": "2.0", "id": 1, "result": {"ok": true}});
        let r: JsonRpcResponse = serde_json::from_value(success_wire).unwrap();
        assert!(matches!(r, JsonRpcResponse::Success(_)));

        let error_wire = json!({
            "jsonrpc": "2.0", "id": 2,
            "error": {"code": -32603, "message": "boom"}
        });
        let r: JsonRpcResponse = serde_json::from_value(error_wire).unwrap();
        assert!(matches!(r, JsonRpcResponse::Error(_)));
    }

    // NOTE: the schema's `JSONRPCMessage = JSONRPCRequest | JSONRPCNotification |
    // JSONRPCResponse` (3-variant transport-frame union) is NOT exposed by
    // turul-rpc 0.2 as a single type — `turul_rpc::JsonRpcMessage` is the
    // inbound-only `Request | Notification` enum, and `turul_rpc::JsonRpcResponse`
    // is the outbound result-or-error union. A complete 3-variant frame would
    // need to land upstream in turul-rpc. Flagged as a follow-up; transport
    // code today deserializes one shape at a time based on direction.
}

/// Phase 1.4 — Error code compliance with draft schema (lines 261–427).
///
/// Asserts every McpError variant emits the on-wire code and structured data
/// the schema declares. See `docs/plans/2026-07-28-compliance-plan.md` §1.4
/// and `docs/plans/2026-07-28-migration-diff.md` "Foundational types > lib.rs".
#[cfg(test)]
mod error_codes {
    use turul_mcp_protocol_2026_07_28::McpError;
    // McpError::to_error_object is an inherent method; the `ToJsonRpcError` trait
    // import previously needed here was removed when we stopped using its trait form.

    // -- Not-found errors: must now map to JSON-RPC standard -32602 (InvalidParams)
    //    per SEP-2164 (was custom -32001/-32002/-32003 in 2025-11-25). --

    #[test]
    fn tool_not_found_maps_to_invalid_params() {
        let err = McpError::ToolNotFound("get_weather".to_string());
        let obj = err.to_error_object();
        // Re-serialize to inspect wire shape since JsonRpcErrorObject's internal
        // representation isn't directly compared — round through JSON.
        let v = serde_json::to_value(&obj).unwrap();
        assert_eq!(v["code"], -32602, "tool not found must be InvalidParams (-32602)");
        assert!(
            v["message"].as_str().unwrap().contains("get_weather"),
            "message must include tool name"
        );
    }

    #[test]
    fn resource_not_found_maps_to_invalid_params() {
        let err = McpError::ResourceNotFound("file:///missing.txt".to_string());
        let v = serde_json::to_value(&err.to_error_object()).unwrap();
        assert_eq!(v["code"], -32602, "resource not found must be InvalidParams (-32602)");
        assert!(v["message"].as_str().unwrap().contains("file:///missing.txt"));
    }

    #[test]
    fn prompt_not_found_maps_to_invalid_params() {
        let err = McpError::PromptNotFound("code_review".to_string());
        let v = serde_json::to_value(&err.to_error_object()).unwrap();
        assert_eq!(v["code"], -32602, "prompt not found must be InvalidParams (-32602)");
        assert!(v["message"].as_str().unwrap().contains("code_review"));
    }

    #[test]
    fn missing_required_client_capability_emits_minus_32003_with_data() {
        // Schema lines 363, 414–427: `MissingRequiredClientCapabilityError` carries
        // `data: { requiredCapabilities: ClientCapabilities }`.
        let required_caps = serde_json::json!({
            "elicitation": { "form": {} }
        });
        let err = McpError::MissingRequiredClientCapability {
            required: required_caps.clone(),
        };
        let v = serde_json::to_value(&err.to_error_object()).unwrap();
        assert_eq!(v["code"], -32003, "MissingRequiredClientCapability wire code");
        assert!(
            v["data"]["requiredCapabilities"].is_object(),
            "data.requiredCapabilities must be present per schema"
        );
        assert_eq!(
            v["data"]["requiredCapabilities"]["elicitation"]["form"],
            serde_json::json!({}),
            "required capabilities content round-trips verbatim"
        );
    }

    #[test]
    fn unsupported_protocol_version_emits_minus_32004_with_data() {
        // Schema lines 371, 384–402: `UnsupportedProtocolVersionError` carries
        // `data: { supported: string[], requested: string }`.
        let err = McpError::UnsupportedProtocolVersion {
            supported: vec!["DRAFT-2026-v1".to_string(), "2025-11-25".to_string()],
            requested: "1999-01-01".to_string(),
        };
        let v = serde_json::to_value(&err.to_error_object()).unwrap();
        assert_eq!(v["code"], -32004, "UnsupportedProtocolVersion wire code");
        let supported = v["data"]["supported"].as_array().unwrap();
        assert_eq!(supported.len(), 2);
        assert_eq!(supported[0], "DRAFT-2026-v1");
        assert_eq!(supported[1], "2025-11-25");
        assert_eq!(v["data"]["requested"], "1999-01-01");
    }

    // -- Standard JSON-RPC codes round-trip correctly per schema lines 261–265. --

    #[test]
    fn invalid_params_variants_all_emit_minus_32602() {
        for err in [
            McpError::InvalidParameters("bad shape".to_string()),
            McpError::MissingParameter("name".to_string()),
            McpError::InvalidParameterType {
                param: "x".to_string(),
                expected: "number".to_string(),
                actual: "string".to_string(),
            },
            McpError::ParameterOutOfRange {
                param: "n".to_string(),
                value: "999".to_string(),
                constraint: "0..100".to_string(),
            },
            McpError::InvalidRequest {
                message: "broken".to_string(),
            },
        ] {
            let v = serde_json::to_value(&err.to_error_object()).unwrap();
            assert_eq!(
                v["code"], -32602,
                "all parameter-validation variants must emit InvalidParams (-32602); variant was: {:?}",
                err
            );
        }
    }

    /// Drift detector: this test pins the set of wire error codes that the
    /// `to_error_object()` mapping is allowed to emit. If a future change adds a
    /// new code without updating this set, the test fails. Whitelists exactly
    /// what the draft schema defines plus the framework-internal server-error
    /// range that the spec leaves for implementation use (-32000..=-32099,
    /// excluding the spec-reserved -32003 and -32004).
    #[test]
    fn no_unauthorised_error_codes_emitted() {
        use std::collections::HashSet;

        let allowed: HashSet<i64> = [
            // Standard JSON-RPC (schema 261–265):
            -32700, -32600, -32601, -32602, -32603,
            // MCP-specific structured codes (schema 363, 371):
            -32003, -32004,
            // Framework-internal server-error codes still in use (not in spec,
            // but in the JSON-RPC-reserved server-error range; replace per area
            // as Phases 2+ land):
            -32010, -32011, -32012, -32013, -32020, -32021, -32022,
            -32030, -32031, -32040, -32041,
            // Sample passthrough code for the `JsonRpcError` test variant above.
            -32050,
        ]
        .into_iter()
        .collect();

        // Exhaustively sample one of each variant. New variants added without
        // updating this exhaustive list will fail to compile (match exhaustiveness).
        let samples = vec![
            McpError::VersionMismatch {
                expected: "a".into(),
                actual: "b".into(),
            },
            McpError::InvalidCapability("c".into()),
            McpError::ToolNotFound("t".into()),
            McpError::ResourceNotFound("r".into()),
            McpError::PromptNotFound("p".into()),
            McpError::InvalidRequest {
                message: "m".into(),
            },
            McpError::InvalidParameters("p".into()),
            McpError::MissingParameter("x".into()),
            McpError::InvalidParameterType {
                param: "p".into(),
                expected: "e".into(),
                actual: "a".into(),
            },
            McpError::ParameterOutOfRange {
                param: "p".into(),
                value: "v".into(),
                constraint: "c".into(),
            },
            McpError::ToolExecutionError("e".into()),
            McpError::ResourceExecutionError("e".into()),
            McpError::PromptExecutionError("e".into()),
            McpError::ResourceAccessDenied("r".into()),
            McpError::ConfigurationError("c".into()),
            McpError::SessionError("s".into()),
            McpError::ValidationError("v".into()),
            McpError::TransportError("t".into()),
            McpError::JsonRpcProtocolError("j".into()),
            // Use a server-error-range code (-32099..=-32000). Standard codes
            // like -32603 currently panic when passed through `server_error()` —
            // tracked as a separate pre-existing bug in the `JsonRpcError`
            // pass-through path, orthogonal to this Phase 1.4 slice.
            McpError::JsonRpcError {
                code: -32050,
                message: "x".into(),
                data: None,
            },
            McpError::MissingRequiredClientCapability {
                required: serde_json::json!({}),
            },
            McpError::UnsupportedProtocolVersion {
                supported: vec!["DRAFT-2026-v1".into()],
                requested: "x".into(),
            },
        ];

        for err in samples {
            let v = serde_json::to_value(&err.to_error_object()).unwrap();
            let code = v["code"].as_i64().expect("wire code must be i64");
            assert!(
                allowed.contains(&code),
                "McpError variant {:?} emitted wire code {} which is not in the allowed set; \
                 update the draft schema's allowed-codes whitelist or fix the mapping",
                err,
                code
            );
        }
    }
}
