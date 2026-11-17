//! The upstream examples and the vendored `schema.json` are the contract:
//! `validate()` must agree with the schema on every example.

use super::*;
use serde_json::{Value, json};

const SCHEMA_JSON: &str = include_str!("../../schema/schema.json");

const VALID: &[(&str, &str)] = &[
    (
        "minimal",
        include_str!("../../schema/examples/valid/minimal.json"),
    ),
    (
        "templated-remote",
        include_str!("../../schema/examples/valid/templated-remote.json"),
    ),
];

const INVALID: &[(&str, &str)] = &[
    (
        "bad-name-pattern",
        include_str!("../../schema/examples/invalid/bad-name-pattern.json"),
    ),
    (
        "date-versioned-schema",
        include_str!("../../schema/examples/invalid/date-versioned-schema.json"),
    ),
    (
        "missing-name",
        include_str!("../../schema/examples/invalid/missing-name.json"),
    ),
    (
        "missing-schema",
        include_str!("../../schema/examples/invalid/missing-schema.json"),
    ),
    (
        "wrong-schema-name",
        include_str!("../../schema/examples/invalid/wrong-schema-name.json"),
    ),
];

fn schema_validator() -> jsonschema::Validator {
    // schema.json holds only `$defs`; upstream's validate-examples.ts checks
    // each example against `#/$defs/ServerCard`, so this does too.
    let mut schema: Value = serde_json::from_str(SCHEMA_JSON).unwrap();
    schema["$ref"] = json!("#/$defs/ServerCard");
    jsonschema::validator_for(&schema).expect("vendored schema.json compiles")
}

/// `validate()`'s verdict on raw JSON: a document that does not even
/// deserialise (a required field missing) counts as invalid.
fn crate_accepts(raw: &str) -> bool {
    serde_json::from_str::<ServerCard>(raw).is_ok_and(|card| card.validate().is_ok())
}

#[test]
fn every_valid_example_passes_both_and_round_trips() {
    let schema = schema_validator();
    for (name, raw) in VALID {
        let value: Value = serde_json::from_str(raw).unwrap();
        assert!(
            schema.is_valid(&value),
            "{name}: schema rejects a valid example"
        );
        let card: ServerCard = serde_json::from_str(raw).unwrap();
        card.validate()
            .unwrap_or_else(|e| panic!("{name}: validate() rejects a valid example: {e}"));
        assert_eq!(
            serde_json::to_value(&card).unwrap(),
            value,
            "{name}: round trip"
        );
    }
}

#[test]
fn every_invalid_example_fails_both() {
    let schema = schema_validator();
    for (name, raw) in INVALID {
        let value: Value = serde_json::from_str(raw).unwrap();
        assert!(
            !schema.is_valid(&value),
            "{name}: schema accepts an invalid example"
        );
        assert!(
            !crate_accepts(raw),
            "{name}: the crate accepts an invalid example"
        );
    }
}

#[test]
fn validate_agrees_with_the_schema_on_each_constraint() {
    let schema = schema_validator();
    let base = json!({ "$schema": SCHEMA_URL, "name": "com.example/srv", "version": "1.0.0",
                       "description": "d" });
    let mutate = |f: &dyn Fn(&mut Value)| {
        let mut v = base.clone();
        f(&mut v);
        v
    };
    let cases: Vec<(&str, Value)> = vec![
        (
            "name without slash",
            mutate(&|v| v["name"] = json!("noslash")),
        ),
        (
            "name with two slashes",
            mutate(&|v| v["name"] = json!("a/b/c")),
        ),
        ("name too short", mutate(&|v| v["name"] = json!("a/"))),
        (
            "name too long",
            mutate(&|v| v["name"] = json!(format!("a/{}", "b".repeat(199)))),
        ),
        (
            "version too long",
            mutate(&|v| v["version"] = json!("1".repeat(256))),
        ),
        (
            "empty description",
            mutate(&|v| v["description"] = json!("")),
        ),
        (
            "description too long",
            mutate(&|v| v["description"] = json!("d".repeat(101))),
        ),
        ("empty title", mutate(&|v| v["title"] = json!(""))),
        (
            "title too long",
            mutate(&|v| v["title"] = json!("t".repeat(101))),
        ),
        (
            "remote url without scheme",
            mutate(&|v| {
                v["remotes"] = json!([{ "type": "streamable-http", "url": "example.com/mcp" }])
            }),
        ),
        (
            "remote url with a space",
            mutate(&|v| {
                v["remotes"] = json!([{ "type": "streamable-http", "url": "https://a b" }])
            }),
        ),
        (
            "schema url of another family",
            mutate(&|v| {
                v["$schema"] = json!(
                    "https://static.modelcontextprotocol.io/schemas/v2/server-card.schema.json"
                )
            }),
        ),
    ];
    for (what, value) in cases {
        assert!(!schema.is_valid(&value), "{what}: schema accepts it");
        assert!(
            !crate_accepts(&value.to_string()),
            "{what}: validate() accepts it"
        );
    }
    for (what, value) in [
        (
            "templated remote url",
            mutate(&|v| v["remotes"] = json!([{ "type": "sse", "url": "{base}/mcp" }])),
        ),
        (
            "100-char description",
            mutate(&|v| v["description"] = json!("d".repeat(100))),
        ),
        (
            "200-char name",
            mutate(&|v| v["name"] = json!(format!("a/{}", "b".repeat(198)))),
        ),
    ] {
        assert!(schema.is_valid(&value), "{what}: schema rejects it");
        assert!(
            crate_accepts(&value.to_string()),
            "{what}: validate() rejects it"
        );
    }
}

#[test]
fn version_ranges_are_refused() {
    for range in ["^1.2.3", "~1.2.3", ">=1.2.3", "1.x", "1.*"] {
        let card = ServerCard::new("com.example/srv", "d").with_version(range);
        assert!(card.validate().is_err(), "{range} accepted");
    }
    assert!(
        ServerCard::new("com.example/srv", "d")
            .with_version("2.1.0-alpha")
            .validate()
            .is_ok()
    );
}

#[test]
fn a_card_the_crate_builds_satisfies_the_schema() {
    let card = ServerCard::new("com.example/weather", "Weather forecasts by city")
        .with_version("1.2.0")
        .with_title("Weather")
        .with_remote(
            Remote::new(RemoteType::StreamableHttp, "https://mcp.example.com/mcp")
                .with_supported_protocol_versions(vec!["2026-07-28".into()]),
        );
    card.validate().unwrap();
    assert!(schema_validator().is_valid(&serde_json::to_value(&card).unwrap()));
}

#[test]
fn document_is_deterministic_and_etag_tracks_content() {
    let card = || {
        let mut meta = std::collections::HashMap::new();
        for k in ["z.example/a", "a.example/b", "m.example/c"] {
            meta.insert(k.to_string(), json!(k));
        }
        ServerCard::new("com.example/srv", "d")
            .with_version("1.0.0")
            .with_meta(meta)
    };
    let a = ServerCardDocument::new(&card()).unwrap();
    let b = ServerCardDocument::new(&card()).unwrap();
    assert_eq!(a.body(), b.body());
    assert_eq!(a.etag(), b.etag());
    let body = std::str::from_utf8(a.body()).unwrap();
    assert!(
        body.find("a.example/b") < body.find("m.example/c")
            && body.find("m.example/c") < body.find("z.example/a"),
        "_meta keys must serialise sorted: {body}"
    );
    let etag = a.etag();
    assert!(
        etag.starts_with('"') && etag.ends_with('"') && etag.len() == 34,
        "{etag}"
    );

    let other = ServerCardDocument::new(&card().with_version("1.0.1")).unwrap();
    assert_ne!(other.etag(), a.etag());
}

#[test]
fn document_refuses_an_invalid_card() {
    assert!(ServerCardDocument::new(&ServerCard::new("bad", "d").with_version("1")).is_err());
}
