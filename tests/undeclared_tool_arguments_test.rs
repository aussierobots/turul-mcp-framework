// A tool built with `#[derive(McpTool)]`, `#[mcp_tool]` or `tool!{}` refuses a
// `tools/call` argument it did not declare, and a present argument of the wrong type
// (optional ones included, which used to read as "not provided"). Per schema.ts
// (`CallToolResult.isError`, "Invalid tool input error") the refusal is a tool result
// with `isError: true` whose text names each offending argument, so the model can
// self-correct; it is not a JSON-RPC error. The advertised `inputSchema` carries
// `additionalProperties: false`. Declared-only calls are unchanged. See ADR-002
// § Input schema strictness.

use serde_json::{Value, json};
use turul_mcp_builders::prelude::*;
use turul_mcp_derive::{McpTool, mcp_tool, tool};
use turul_mcp_server::{McpResult, McpTool as McpToolTrait, SessionContext};

#[derive(McpTool, Clone, Default)]
#[tool(name = "derive_strict", description = "Derive tool")]
struct DeriveStrict {
    #[param(description = "Required")]
    a: String,
    #[param(description = "Optional")]
    limit: Option<u32>,
    #[param(description = "Optional list")]
    tags: Option<Vec<String>>,
    #[param(description = "Optional, T-typed", optional)]
    note: String,
}

impl DeriveStrict {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<String> {
        Ok(format!(
            "{}:{:?}:{:?}:{}",
            self.a, self.limit, self.tags, self.note
        ))
    }
}

#[mcp_tool(name = "attr_strict", description = "Attribute tool")]
async fn attr_strict(
    a: String,
    limit: Option<u32>,
    tags: Option<Vec<String>>,
) -> McpResult<String> {
    Ok(format!("{a}:{limit:?}:{tags:?}"))
}

#[derive(McpTool, Clone, Default)]
#[tool(name = "no_params", description = "No parameters")]
struct NoParams {}

impl NoParams {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<String> {
        Ok("ok".into())
    }
}

fn declarative() -> impl McpToolTrait {
    tool! {
        name: "decl_strict",
        description: "Declarative tool",
        params: {
            a: String => "Required",
        },
        execute: |a: String| async move { Ok::<_, &str>(a) }
    }
}

/// Assert `result` is an `isError: true` tool result whose text names every `keys` entry.
fn assert_refused_naming(result: McpResult<impl serde::Serialize>, keys: &[&str]) {
    let value =
        serde_json::to_value(result.expect("a refusal is a tool result, not an Err")).unwrap();
    assert_eq!(value["isError"], json!(true), "{value:#}");
    let text = value["content"][0]["text"].as_str().unwrap_or_default();
    for key in keys {
        assert!(text.contains(key), "text must name `{key}`: {text}");
    }
}

async fn check_tool<T: McpToolTrait + ToolDefinition>(tool: &T, ok_args: Value) {
    let schema = serde_json::to_value(tool.input_schema()).unwrap();
    assert_eq!(
        schema["additionalProperties"],
        json!(false),
        "inputSchema must close the object: {schema:#}"
    );

    tool.call(ok_args.clone(), None)
        .await
        .expect("declared-only call is unchanged");

    let mut with_bogus = ok_args.clone();
    with_bogus["bogus"] = json!(1);
    assert_refused_naming(tool.call(with_bogus, None).await, &["bogus"]);

    let mut with_two = ok_args;
    with_two["zeta"] = json!(1);
    with_two["alpha"] = json!(2);
    assert_refused_naming(tool.call(with_two, None).await, &["alpha", "zeta"]);
}

#[tokio::test]
async fn derive_tool_refuses_undeclared_argument() {
    check_tool(&DeriveStrict::default(), json!({"a": "x"})).await;
}

#[tokio::test]
async fn derive_tool_misspelled_optional_is_not_silently_defaulted() {
    assert_refused_naming(
        DeriveStrict::default()
            .call(json!({"a": "x", "limt": 5}), None)
            .await,
        &["limt"],
    );
}

#[tokio::test]
async fn derive_tool_accepts_every_declared_argument() {
    DeriveStrict::default()
        .call(json!({"a": "x", "limit": 5}), None)
        .await
        .expect("all declared keys accepted");
}

#[tokio::test]
async fn mcp_tool_attribute_refuses_undeclared_argument() {
    check_tool(&AttrStrictToolImpl, json!({"a": "x"})).await;
}

#[tokio::test]
async fn tool_macro_refuses_undeclared_argument() {
    check_tool(&declarative(), json!({"a": "x"})).await;
}

#[tokio::test]
async fn derive_optional_argument_of_wrong_type_is_refused() {
    let tool = DeriveStrict::default();
    for (key, bad) in [
        ("limit", json!("5")),
        ("limit", json!(-1)),
        ("limit", json!(true)),
        ("tags", json!("a")),
        ("tags", json!([1, 2])),
        ("note", json!(7)),
    ] {
        assert_refused_naming(tool.call(json!({"a": "x", key: bad}), None).await, &[key]);
    }
}

#[tokio::test]
async fn attr_optional_argument_of_wrong_type_is_refused() {
    let tool = AttrStrictToolImpl;
    for (key, bad) in [("limit", json!("5")), ("tags", json!([1]))] {
        assert_refused_naming(tool.call(json!({"a": "x", key: bad}), None).await, &[key]);
    }
}

#[tokio::test]
async fn missing_required_argument_is_a_tool_error_result() {
    assert_refused_naming(DeriveStrict::default().call(json!({}), None).await, &["a"]);
}

#[tokio::test]
async fn optional_argument_null_or_absent_or_right_type_is_accepted() {
    let tool = DeriveStrict::default();
    for args in [
        json!({"a": "x"}),
        json!({"a": "x", "limit": null, "tags": null, "note": null}),
        json!({"a": "x", "limit": 5, "tags": ["t"], "note": "n"}),
    ] {
        tool.call(args, None).await.expect("accepted");
    }
}

#[tokio::test]
async fn parameterless_tool_closes_schema_and_refuses_any_argument() {
    let tool = NoParams::default();
    let schema = serde_json::to_value(tool.input_schema()).unwrap();
    assert_eq!(schema["additionalProperties"], json!(false));
    for ok in [json!({}), json!(null)] {
        tool.call(ok, None).await.expect("no arguments is fine");
    }
    assert_refused_naming(tool.call(json!({"x": 1}), None).await, &["x"]);
}

#[tokio::test]
async fn null_arguments_name_the_missing_required_parameter() {
    let tool = DeriveStrict::default();
    assert_refused_naming(tool.call(json!(null), None).await, &["a"]);
}

// Nested strictness is the argument type's own serde contract: the framework closes the
// top-level object only (ADR-002 § Input schema strictness).
#[derive(
    serde::Deserialize, serde::Serialize, turul_mcp_builders::schemars::JsonSchema, Clone, Default,
)]
#[schemars(crate = "turul_mcp_builders::schemars")]
#[serde(deny_unknown_fields)]
struct Strict {
    city: String,
}

#[derive(
    serde::Deserialize, serde::Serialize, turul_mcp_builders::schemars::JsonSchema, Clone, Default,
)]
#[schemars(crate = "turul_mcp_builders::schemars")]
struct Lenient {
    city: String,
}

#[derive(McpTool, Clone, Default)]
#[allow(dead_code)]
#[tool(name = "nested_strict", description = "Nested argument types")]
struct NestedStrict {
    #[param(description = "Strict nested object")]
    strict: Option<Strict>,
    #[param(description = "Lenient nested object")]
    lenient: Option<Lenient>,
}

impl NestedStrict {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<String> {
        Ok("ok".into())
    }
}

#[tokio::test]
async fn nested_unknown_key_is_refused_only_when_the_type_denies_unknown_fields() {
    let tool = NestedStrict::default();
    assert_refused_naming(
        tool.call(json!({"strict": {"city": "a", "extra": 1}}), None)
            .await,
        &["strict"],
    );
    tool.call(json!({"lenient": {"city": "a", "extra": 1}}), None)
        .await
        .map(|r| assert_ne!(serde_json::to_value(r).unwrap()["isError"], json!(true)))
        .expect("a type that tolerates unknown fields keeps tolerating them");
}

// A type mismatch names the declared schema type and the received JSON type.
#[derive(McpTool, Clone, Default)]
#[allow(dead_code)]
#[tool(name = "typed_params", description = "One parameter per schema type")]
struct TypedParams {
    #[param(description = "string")]
    s: String,
    #[param(description = "number")]
    n: f64,
    #[param(description = "integer")]
    i: i64,
    #[param(description = "narrow integer")]
    u: u8,
    #[param(description = "boolean")]
    b: bool,
    #[param(description = "array")]
    v: Vec<String>,
    #[param(description = "optional number")]
    on: Option<f64>,
    #[param(description = "optional narrow integer")]
    ou: Option<usize>,
}

impl TypedParams {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<String> {
        Ok("ok".into())
    }
}

fn valid_typed_args() -> Value {
    json!({"s": "x", "n": 1.5, "i": 1, "u": 1, "b": true, "v": ["a"]})
}

#[tokio::test]
async fn type_mismatch_names_declared_and_received_json_type() {
    // (declared schema type, parameter, JSON values to send, with their JSON type)
    let sent = [
        (json!("x"), "string"),
        (json!(7), "integer"),
        (json!(1.5), "number"),
        (json!(true), "boolean"),
        (json!([1]), "array"),
        (json!({"k": 1}), "object"),
        (json!(null), "null"),
    ];
    // (parameter, declared type, JSON types that are valid for it)
    let params: [(&str, &str, &[&str]); 8] = [
        ("s", "string", &["string"]),
        ("n", "number", &["integer", "number"]),
        ("i", "integer", &["integer"]),
        ("u", "integer", &["integer"]),
        ("b", "boolean", &["boolean"]),
        ("v", "array", &["array", "string"]),
        ("on", "number", &["integer", "number", "null"]),
        ("ou", "integer", &["integer", "null"]),
    ];
    let tool = TypedParams::default();
    let mut checked = 0;
    for (param, declared, valid) in params {
        for (value, received) in &sent {
            if valid.contains(received) {
                continue;
            }
            let mut args = valid_typed_args();
            args[param] = value.clone();
            let result = serde_json::to_value(tool.call(args, None).await.unwrap()).unwrap();
            assert_eq!(result["isError"], json!(true), "{param}={value}: {result}");
            let text = result["content"][0]["text"].as_str().unwrap_or_default();
            assert!(
                text.contains(&format!("'{param}'"))
                    && text.contains(&format!("expected {declared}"))
                    && text.contains(&format!("got {received}")),
                "{param}={value} must say expected {declared}, got {received}: {text}"
            );
            checked += 1;
        }
    }
    assert!(checked >= 30, "table shrank: {checked} cases");
}

#[tokio::test]
async fn out_of_range_integer_is_not_reported_as_the_wrong_type() {
    let mut args = valid_typed_args();
    args["u"] = json!(300);
    let result = serde_json::to_value(TypedParams::default().call(args, None).await.unwrap()).unwrap();
    assert_eq!(result["isError"], json!(true));
    let text = result["content"][0]["text"].as_str().unwrap_or_default();
    assert!(
        text.contains("'u'") && text.contains("in range") && text.contains("got integer"),
        "{text}"
    );
}

#[tokio::test]
async fn missing_required_argument_says_missing_not_a_type_error() {
    let mut args = valid_typed_args();
    args.as_object_mut().unwrap().remove("n");
    let result = serde_json::to_value(TypedParams::default().call(args, None).await.unwrap()).unwrap();
    let text = result["content"][0]["text"].as_str().unwrap_or_default();
    assert!(text.contains("n") && text.to_lowercase().contains("missing"), "{text}");
}
