//! Wire-shape tests against the vendored `schema/skills.mdx`, and the
//! owner-routing guard (ADR-033 §Tests).

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use turul_mcp_protocol_2026_07_28::json_rpc::PaginatedRequestParams;

use super::types::{
    GetSkillRequestParams, GetSkillResult, ListSkillsResult, METHOD_SKILLS_GET, METHOD_SKILLS_LIST,
    SkillResources,
};

const SPEC: &str = include_str!("../../schema/skills.mdx");

/// Every ```json block in the pinned specification.
fn json_examples() -> Vec<Value> {
    let mut out = Vec::new();
    let mut rest = SPEC;
    while let Some(start) = rest.find("```json\n") {
        let body = &rest[start + 8..];
        let end = body.find("\n```").expect("closed fence");
        out.push(serde_json::from_str(&body[..end]).expect("spec example is JSON"));
        rest = &body[end..];
    }
    out
}

fn round_trip<T: Serialize + DeserializeOwned>(v: &Value) {
    let typed: T = serde_json::from_value(v.clone()).expect("deserializes");
    assert_eq!(&serde_json::to_value(typed).unwrap(), v);
}

/// Every skills/list and skills/get example in the spec round-trips through the
/// typed wire structs unchanged.
#[test]
fn spec_examples_round_trip() {
    let mut seen = [0usize; 4];
    for ex in json_examples() {
        match ex["method"].as_str() {
            Some(METHOD_SKILLS_LIST) => {
                round_trip::<PaginatedRequestParams>(&ex["params"]);
                seen[0] += 1;
            }
            Some(METHOD_SKILLS_GET) => {
                round_trip::<GetSkillRequestParams>(&ex["params"]);
                seen[1] += 1;
            }
            _ => {}
        }
        if ex["result"].get("skills").is_some() {
            round_trip::<ListSkillsResult>(&ex["result"]);
            seen[2] += 1;
        }
        if ex["result"].get("skill").is_some() {
            round_trip::<GetSkillResult>(&ex["result"]);
            seen[3] += 1;
        }
    }
    assert!(seen.iter().all(|&n| n >= 1), "examples matched: {seen:?}");
}

/// The spec example listing includes a `"dynamic"` entry and manifest entries.
#[test]
fn resources_dynamic_and_manifest_parse() {
    let list = json_examples()
        .into_iter()
        .find(|e| e["result"].get("skills").is_some())
        .unwrap();
    let r: ListSkillsResult = serde_json::from_value(list["result"].clone()).unwrap();
    assert!(
        r.skills
            .iter()
            .any(|s| s.resources == SkillResources::Dynamic)
    );
    assert!(
        r.skills
            .iter()
            .any(|s| matches!(s.resources, SkillResources::Manifest(_)))
    );
}

/// :231 — any `resources` value other than an array or `"dynamic"` is invalid.
#[test]
fn resources_other_values_rejected() {
    for bad in [
        json!("static"),
        json!({}),
        json!(null),
        json!(1),
        json!(true),
    ] {
        assert!(
            serde_json::from_value::<SkillResources>(bad.clone()).is_err(),
            "{bad}"
        );
    }
    let missing =
        json!({ "uri": "skill://a/SKILL.md", "frontmatter": {"name":"a","description":"d"} });
    assert!(serde_json::from_value::<super::types::Skill>(missing).is_err());
}

/// ADR-033 §1: every caller routes through the owner constructors — no
/// `skill://` or `sha256:` literal is formatted in non-test code outside
/// `uri.rs` and `manifest.rs`, in this crate, the server's ext_skills module or the client.
#[test]
fn owners_route_through_constructors() {
    let here = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = vec![here.join("src/lib.rs")];
    for e in std::fs::read_dir(here.join("src/v2026_07_28")).unwrap() {
        files.push(e.unwrap().path());
    }
    // Consumers hold no such literal anywhere, tests included: scanned whole.
    let mut whole = Vec::new();
    for consumer in [
        "../turul-mcp-server/src/ext_skills.rs",
        "../turul-mcp-client/src/client.rs",
    ] {
        let path = here.join(consumer);
        assert!(path.exists(), "{consumer} moved; update this guard");
        whole.push(path);
    }
    let crate_files = files.into_iter().map(|f| (f, false));
    for (f, scan_whole) in crate_files.chain(whole.into_iter().map(|f| (f, true))) {
        let name = f.file_name().unwrap().to_string_lossy().to_string();
        if !scan_whole
            && matches!(
                name.as_str(),
                "uri.rs" | "manifest.rs" | "compliance_test.rs"
            )
        {
            continue;
        }
        let text = std::fs::read_to_string(&f).unwrap();
        // A crate file's unit tests are its trailing `mod tests`, which may quote URIs.
        let code = if scan_whole {
            text.as_str()
        } else {
            text.split("#[cfg(test)]\nmod tests {").next().unwrap()
        };
        let code: String = code
            .lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        for needle in ["\"skill://", "\"sha256:"] {
            assert!(!code.contains(needle), "{} formats {needle}", f.display());
        }
    }
}
