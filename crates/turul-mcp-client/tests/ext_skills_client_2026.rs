#![cfg(all(
    feature = "ext-skills",
    any(feature = "client-bilingual", feature = "client-2026-07-28-only")
))]
//! Skills extension client — the executable contract of ADR-033 §6 (host
//! clauses C-1 to C-8). A wiremock peer serves crafted and tampered entries;
//! one test runs against the real `turul-mcp-server`.

use serde_json::{Value, json};
use turul_mcp_client::transport::HttpTransport;
use turul_mcp_client::{McpClient, McpClientError};
use turul_mcp_ext_skills::{
    SkillEntry, SkillFile, SkillName, SkillPath, SkillResource, SkillResources,
};
use wiremock::matchers::{body_partial_json, method};
use wiremock::{Mock, MockServer, ResponseTemplate};

const SKILL_MD: &str = "---\nname: demo\ndescription: Does demo.\nlicense: MIT\n---\nSteps.\n";
const SKILL_URI: &str = "skill://demo/SKILL.md";
const GUIDE_URI: &str = "skill://demo/GUIDE.md";
const GUIDE: &str = "# Guide\n";

/// The entry the peer serves: built by the owner, so digests are real.
fn entry() -> Value {
    let e = SkillEntry::from_files(
        SkillPath::new([], SkillName::parse("demo").unwrap()).unwrap(),
        vec![
            SkillFile::new("SKILL.md", SKILL_MD),
            SkillFile::new("GUIDE.md", GUIDE),
        ],
    )
    .unwrap();
    serde_json::to_value(e.to_wire()).unwrap()
}

fn rpc(result: Value) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(json!({ "jsonrpc": "2.0", "id": 1, "result": result }))
}

fn cache() -> Value {
    json!({ "resultType": "complete", "ttlMs": 0, "cacheScope": "public" })
}

fn merged(a: Value, b: Value) -> Value {
    let mut a = a;
    a.as_object_mut()
        .unwrap()
        .extend(b.as_object().unwrap().clone());
    a
}

/// A peer that declares (or not) the extension, serves `skill` from
/// `skills/get` and `skills/list`, and answers `resources/read` from `reads`.
async fn peer(declare: bool, skill: Value, reads: &[(&str, Value)]) -> MockServer {
    let server = MockServer::start().await;
    let mut caps = json!({ "resources": {} });
    if declare {
        caps["extensions"] = json!({ "io.modelcontextprotocol/skills": {} });
    }
    Mock::given(method("POST"))
        .and(body_partial_json(json!({ "method": "server/discover" })))
        .respond_with(rpc(merged(
            cache(),
            json!({
                "supportedVersions": ["2026-07-28"],
                "capabilities": caps,
                "_meta": { "io.modelcontextprotocol/serverInfo": { "name": "peer", "version": "1" } }
            }),
        )))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(body_partial_json(json!({ "method": "skills/get" })))
        .respond_with(rpc(merged(cache(), json!({ "skill": skill.clone() }))))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(body_partial_json(json!({ "method": "skills/list" })))
        .respond_with(rpc(merged(cache(), json!({ "skills": [skill] }))))
        .mount(&server)
        .await;
    for (uri, content) in reads {
        Mock::given(method("POST"))
            .and(body_partial_json(
                json!({ "method": "resources/read", "params": { "uri": uri } }),
            ))
            .respond_with(rpc(merged(cache(), json!({ "contents": [content] }))))
            .mount(&server)
            .await;
    }
    server
}

fn text(uri: &str, t: &str) -> Value {
    json!({ "uri": uri, "mimeType": "text/markdown", "text": t })
}

async fn client(server: &MockServer) -> McpClient {
    let c = McpClient::new(
        Box::new(HttpTransport::new(&format!("{}/mcp", server.uri())).unwrap()),
        Default::default(),
    );
    c.connect().await.expect("discover");
    c
}

async fn methods_sent(server: &MockServer) -> Vec<String> {
    server
        .received_requests()
        .await
        .unwrap()
        .iter()
        .filter_map(|r| serde_json::from_slice::<Value>(&r.body).ok())
        .filter_map(|v| v["method"].as_str().map(str::to_owned))
        .collect()
}

fn verification(err: McpClientError) -> String {
    match err {
        McpClientError::SkillVerification { reason, .. } => reason,
        other => panic!("expected SkillVerification, got {other:?}"),
    }
}

/// C-1 (:67): no `skills/*` request goes out unless the server declared the extension.
#[tokio::test]
async fn c1_methods_refuse_without_declaration() {
    let server = peer(false, entry(), &[]).await;
    let c = client(&server).await;
    assert!(matches!(
        c.skills_list(None).await,
        Err(McpClientError::ExtensionNotDeclared(_))
    ));
    assert!(matches!(
        c.skills_get(SKILL_URI).await,
        Err(McpClientError::ExtensionNotDeclared(_))
    ));
    assert!(
        !methods_sent(&server)
            .await
            .iter()
            .any(|m| m.starts_with("skills/"))
    );
}

/// C-2 (:231): an entry whose `resources` is neither an array nor "dynamic" is not loaded.
#[tokio::test]
async fn c2_invalid_resources_entry_rejected() {
    let mut bad = entry();
    bad["resources"] = json!("static");
    let server = peer(true, bad, &[]).await;
    let c = client(&server).await;
    assert!(c.skills_get(SKILL_URI).await.is_err());
    assert!(c.skills_list(None).await.is_err());
}

/// C-3 (:229, :482): verified bytes come back for text and blob content.
#[tokio::test]
async fn c3_verified_bytes_returned() {
    let bin = vec![0u8, 1, 2, 255];
    let e = SkillEntry::from_files(
        SkillPath::new([], SkillName::parse("demo").unwrap()).unwrap(),
        vec![
            SkillFile::new("SKILL.md", SKILL_MD),
            SkillFile::new("logo.bin", bin.clone()),
        ],
    )
    .unwrap();
    let wire = serde_json::to_value(e.to_wire()).unwrap();
    let server = peer(
        true,
        wire,
        &[
            (SKILL_URI, text(SKILL_URI, SKILL_MD)),
            (
                "skill://demo/logo.bin",
                json!({ "uri": "skill://demo/logo.bin", "mimeType": "application/octet-stream", "blob": "AAEC/w==" }),
            ),
        ],
    )
    .await;
    let c = client(&server).await;
    let skill = c.skills_get(SKILL_URI).await.unwrap();
    assert_eq!(
        c.skill_fetch(&skill, SKILL_URI).await.unwrap(),
        SKILL_MD.as_bytes()
    );
    assert_eq!(
        c.skill_fetch(&skill, "skill://demo/logo.bin")
            .await
            .unwrap(),
        bin
    );
}

/// C-3 (:229): a read whose length differs from `size` is a verification failure.
#[tokio::test]
async fn c3_size_mismatch_refused() {
    let server = peer(
        true,
        entry(),
        &[(GUIDE_URI, text(GUIDE_URI, "# Guide\nextra\n"))],
    )
    .await;
    let c = client(&server).await;
    let skill = c.skills_get(SKILL_URI).await.unwrap();
    let reason = verification(c.skill_fetch(&skill, GUIDE_URI).await.unwrap_err());
    assert!(reason.contains("size"), "{reason}");
}

/// C-3 (:482-483): same length, different bytes — the digest catches it.
#[tokio::test]
async fn c3_digest_mismatch_refused() {
    let server = peer(true, entry(), &[(GUIDE_URI, text(GUIDE_URI, "# Gxide\n"))]).await;
    let c = client(&server).await;
    let skill = c.skills_get(SKILL_URI).await.unwrap();
    let reason = verification(c.skill_fetch(&skill, GUIDE_URI).await.unwrap_err());
    assert!(reason.contains("digest"), "{reason}");
}

/// C-4 (:491-492): a URI not in the held entry is refused before any request.
#[tokio::test]
async fn c4_unlisted_uri_refused_without_request() {
    let server = peer(true, entry(), &[]).await;
    let c = client(&server).await;
    let skill = c.skills_get(SKILL_URI).await.unwrap();
    verification(
        c.skill_fetch(&skill, "skill://demo/secret.md")
            .await
            .unwrap_err(),
    );
    assert!(
        !methods_sent(&server)
            .await
            .contains(&"resources/read".to_string())
    );
}

/// C-5 (:496-497): an entry whose frontmatter disagrees with the fetched
/// SKILL.md is not loaded, even though the bytes match their digest.
#[tokio::test]
async fn c5_frontmatter_discrepancy_refused() {
    let mut lying = entry();
    lying["frontmatter"]["description"] = json!("Something else.");
    let server = peer(true, lying, &[(SKILL_URI, text(SKILL_URI, SKILL_MD))]).await;
    let c = client(&server).await;
    let skill = c.skills_get(SKILL_URI).await.unwrap();
    let reason = verification(c.skill_fetch(&skill, SKILL_URI).await.unwrap_err());
    assert!(reason.contains("frontmatter"), "{reason}");
}

/// C-5 / ADR-033 §6: frontmatter our parser cannot render faithfully (a typed
/// scalar) cannot be verified, so it fails closed.
#[tokio::test]
async fn c5_typed_frontmatter_fails_closed() {
    let md = "---\nname: demo\ndescription: Does demo.\nversion: 1.0\n---\n";
    let digest = SkillResource::of(SKILL_URI.into(), md.as_bytes());
    let typed = json!({
        "uri": SKILL_URI,
        "frontmatter": { "name": "demo", "description": "Does demo.", "version": 1.0 },
        "resources": [digest],
    });
    let server = peer(true, typed, &[(SKILL_URI, text(SKILL_URI, md))]).await;
    let c = client(&server).await;
    let skill = c.skills_get(SKILL_URI).await.unwrap();
    let reason = verification(c.skill_fetch(&skill, SKILL_URI).await.unwrap_err());
    assert!(reason.contains("frontmatter"), "{reason}");
}

/// C-6 (:503): listing and getting retrieve no file.
#[tokio::test]
async fn c6_list_and_get_never_read_resources() {
    let server = peer(true, entry(), &[]).await;
    let c = client(&server).await;
    let page = c.skills_list(None).await.unwrap();
    assert_eq!(page.skills.len(), 1);
    c.skills_get(SKILL_URI).await.unwrap();
    let sent = methods_sent(&server).await;
    assert_eq!(
        sent,
        ["server/discover", "skills/list", "skills/get"],
        "{sent:?}"
    );
}

/// C-7 (:231): a "dynamic" entry offers no integrity and is declined.
#[tokio::test]
async fn c7_dynamic_entry_declined() {
    let mut dynamic = entry();
    dynamic["resources"] = json!("dynamic");
    let server = peer(true, dynamic, &[(SKILL_URI, text(SKILL_URI, SKILL_MD))]).await;
    let c = client(&server).await;
    let skill = c.skills_get(SKILL_URI).await.unwrap();
    assert_eq!(skill.resources, SkillResources::Dynamic);
    verification(c.skill_fetch(&skill, SKILL_URI).await.unwrap_err());
    assert!(
        !methods_sent(&server)
            .await
            .contains(&"resources/read".to_string())
    );
}

/// End to end against the real server: list every skill, get one, and fetch
/// every manifest file with full verification.
#[tokio::test]
async fn e2e_real_server_list_get_fetch() {
    use turul_mcp_server::ext_skills::SkillCatalog;
    use turul_mcp_server::prelude::*;

    let mut catalog = SkillCatalog::new();
    for name in ["alpha", "beta"] {
        catalog
            .add(
                SkillEntry::from_files(
                    SkillPath::new(["acme"], SkillName::parse(name).unwrap()).unwrap(),
                    vec![
                        SkillFile::new(
                            "SKILL.md",
                            format!("---\nname: {name}\ndescription: Does {name}.\n---\nBody\n"),
                        ),
                        SkillFile::new("data/blob.bin", vec![9u8, 0, 255]),
                        SkillFile::new("notes/a b.md", "spaced\n"),
                    ],
                )
                .unwrap(),
            )
            .unwrap();
    }
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = McpServer::builder()
        .name("skills-e2e")
        .version("0.0.0")
        .with_ext_skills(catalog)
        .listener(listener)
        .build()
        .unwrap();
    tokio::spawn(async move { server.run().await.ok() });

    let url = format!("http://127.0.0.1:{port}/mcp");
    let c = McpClient::new(
        Box::new(HttpTransport::new(&url).unwrap()),
        Default::default(),
    );
    let mut connected = false;
    for _ in 0..50 {
        if c.connect().await.is_ok() {
            connected = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    assert!(connected, "server never answered");

    let page = c.skills_list(None).await.unwrap();
    assert_eq!(page.skills.len(), 2);
    let got = c.skills_get("skill://acme/beta/SKILL.md").await.unwrap();
    assert_eq!(got, page.skills[1]);
    let mut fetched = 0;
    for skill in &page.skills {
        let SkillResources::Manifest(files) = &skill.resources else {
            panic!("manifest expected")
        };
        for f in files {
            let bytes = c.skill_fetch(skill, f.uri()).await.unwrap();
            assert_eq!(bytes.len() as u64, f.size());
            fetched += 1;
        }
    }
    assert_eq!(fetched, 6);
}
