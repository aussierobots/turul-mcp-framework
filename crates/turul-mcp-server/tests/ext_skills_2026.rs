//! Skills extension (`io.modelcontextprotocol/skills`) over Streamable HTTP —
//! the executable contract of ADR-033 §5. Test names cite the clause proved.
#![cfg(feature = "ext-skills")]

mod common;

use base64::Engine;
use serde_json::{Value, json};
use turul_mcp_ext_skills::SkillName;
use turul_mcp_ext_skills::v2026_07_28::{
    EXTENSION_IDENTIFIER, SkillEntry, SkillFile, SkillPath, SkillResource,
};
use turul_mcp_server::ext_skills::SkillCatalog;
use turul_mcp_server::prelude::*;

fn skill_md(name: &str) -> SkillFile {
    SkillFile::new(
        "SKILL.md",
        format!("---\nname: {name}\ndescription: Does {name}.\nlicense: MIT\n---\nSteps.\n"),
    )
}

fn entry(prefix: &[&str], name: &str, extra: Vec<SkillFile>) -> SkillEntry {
    let path = SkillPath::new(prefix.iter().copied(), SkillName::parse(name).unwrap()).unwrap();
    let mut files = vec![skill_md(name)];
    files.extend(extra);
    SkillEntry::from_files(path, files).unwrap()
}

/// The small nested fixture: `outer` contains `inner`, both published.
fn nested_fixture() -> Vec<SkillEntry> {
    let inner_md = skill_md("inner").bytes;
    vec![
        entry(
            &["acme"],
            "outer",
            vec![
                SkillFile::new("references/GUIDE.md", "# Guide\n"),
                SkillFile::new("notes/a b.md", "spaced\n"),
                SkillFile::new("assets/logo.bin", vec![0u8, 1, 2, 255]),
                SkillFile::new("inner/SKILL.md", inner_md.clone()),
            ],
        ),
        entry(&["acme", "outer"], "inner", vec![]),
    ]
}

fn catalog(entries: Vec<SkillEntry>) -> SkillCatalog {
    let mut c = SkillCatalog::new();
    for e in entries {
        c.add(e).unwrap();
    }
    c
}

async fn start(builder: McpServerBuilder) -> String {
    let (listener, port) = common::bind_ephemeral();
    let server = builder
        .name("ext-skills-2026")
        .version("0.0.0")
        .listener(listener)
        .build()
        .expect("build");
    tokio::spawn(async move {
        server.run().await.ok();
    });
    let url = format!("http://127.0.0.1:{port}/mcp");
    let client = reqwest::Client::new();
    for _ in 0..50 {
        if client.get(&url).send().await.is_ok() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    url
}

fn meta() -> Value {
    json!({
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientInfo": { "name": "t", "version": "1" },
        "io.modelcontextprotocol/clientCapabilities": {}
    })
}

async fn post(url: &str, method: &str, name: Option<&str>, params: Value) -> Value {
    let mut req = reqwest::Client::new()
        .post(url)
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", method);
    if let Some(n) = name {
        req = req.header("Mcp-Name", n);
    }
    req.json(&json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params }))
        .send()
        .await
        .expect("POST")
        .json()
        .await
        .expect("json")
}

async fn list_all(url: &str) -> Vec<Value> {
    let mut out = Vec::new();
    let mut cursor: Option<String> = None;
    loop {
        let mut p = json!({ "_meta": meta() });
        if let Some(c) = &cursor {
            p["cursor"] = json!(c);
        }
        let r = post(url, "skills/list", None, p).await;
        let result = &r["result"];
        out.extend(result["skills"].as_array().expect("skills").iter().cloned());
        match result["nextCursor"].as_str() {
            Some(c) => cursor = Some(c.to_string()),
            None => return out,
        }
    }
}

/// S-G1: discover declares the extension (`{}`, directoryRead false) and `resources`.
#[tokio::test]
async fn s_g1_discover_advertises_skills_and_resources() {
    let url = start(McpServer::builder().with_ext_skills(catalog(nested_fixture()))).await;
    let r = post(&url, "server/discover", None, json!({ "_meta": meta() })).await;
    let caps = &r["result"]["capabilities"];
    assert_eq!(caps["extensions"][EXTENSION_IDENTIFIER], json!({}), "{r}");
    assert!(caps["resources"].is_object(), "{r}");
}

/// S-G1: the declaration forces `resources` even when no skill is served yet.
#[tokio::test]
async fn s_g1_empty_catalog_still_declares_resources() {
    let url = start(McpServer::builder().with_ext_skills(SkillCatalog::new())).await;
    let r = post(&url, "server/discover", None, json!({ "_meta": meta() })).await;
    let caps = &r["result"]["capabilities"];
    assert_eq!(caps["extensions"][EXTENSION_IDENTIFIER], json!({}), "{r}");
    assert!(caps["resources"].is_object(), "{r}");
    let l = post(&url, "skills/list", None, json!({ "_meta": meta() })).await;
    assert_eq!(l["result"]["skills"], json!([]), "{l}");
}

/// SEP-2133 default-off: a build that never calls `with_ext_skills` advertises
/// nothing and does not answer `skills/*`.
#[tokio::test]
async fn default_build_advertises_nothing() {
    let url = start(McpServer::builder()).await;
    let r = post(&url, "server/discover", None, json!({ "_meta": meta() })).await;
    assert!(
        r["result"]["capabilities"]["extensions"]
            .get(EXTENSION_IDENTIFIER)
            .is_none(),
        "{r}"
    );
    let l = post(&url, "skills/list", None, json!({ "_meta": meta() })).await;
    assert_eq!(l["error"]["code"], -32601, "{l}");
}

/// S-G2: pagination never splits an entry; every page is `complete` with cache
/// fields; an unknown cursor is `-32602`.
#[tokio::test]
async fn s_g2_list_paginates_atomic_entries() {
    let names: Vec<String> = (0..60).map(|i| format!("s{i:02}")).collect();
    let entries = names
        .iter()
        .map(|n| {
            entry(
                &[],
                n,
                vec![SkillFile::new("a.md", "a"), SkillFile::new("b/c.md", "c")],
            )
        })
        .collect();
    let url = start(McpServer::builder().with_ext_skills(catalog(entries))).await;

    let first = post(&url, "skills/list", None, json!({ "_meta": meta() })).await;
    let result = &first["result"];
    assert_eq!(result["resultType"], "complete");
    assert!(result["ttlMs"].is_number() && result["cacheScope"].is_string());
    assert!(result["nextCursor"].is_string(), "{first}");

    let all = list_all(&url).await;
    assert_eq!(all.len(), 60);
    for s in &all {
        assert_eq!(s["resources"].as_array().unwrap().len(), 3, "{s}");
    }

    let bad = post(
        &url,
        "skills/list",
        None,
        json!({ "_meta": meta(), "cursor": "nope" }),
    )
    .await;
    assert_eq!(bad["error"]["code"], -32602, "{bad}");
}

/// S-G2: `skills/get` answers for a served skill (here one beyond the first
/// page), is `complete` with cache fields and no cursor; an unknown URI, a
/// supporting-file URI and missing `_meta` are `-32602`.
#[tokio::test]
async fn s_g2_get_by_uri_and_unknown_is_invalid_params() {
    let entries = (0..60)
        .map(|i| entry(&[], &format!("s{i:02}"), vec![]))
        .collect();
    let url = start(McpServer::builder().with_ext_skills(catalog(entries))).await;

    let r = post(
        &url,
        "skills/get",
        None,
        json!({ "_meta": meta(), "uri": "skill://s59/SKILL.md" }),
    )
    .await;
    let result = &r["result"];
    assert_eq!(result["resultType"], "complete", "{r}");
    assert_eq!(result["skill"]["frontmatter"]["name"], "s59");
    assert!(result["ttlMs"].is_number() && result["cacheScope"].is_string());
    assert!(result.get("nextCursor").is_none());

    for uri in ["skill://nope/SKILL.md", "skill://s01/a.md"] {
        let e = post(
            &url,
            "skills/get",
            None,
            json!({ "_meta": meta(), "uri": uri }),
        )
        .await;
        assert_eq!(e["error"]["code"], -32602, "{e}");
    }
    let no_meta = post(
        &url,
        "skills/get",
        None,
        json!({ "uri": "skill://s01/SKILL.md" }),
    )
    .await;
    assert_eq!(no_meta["error"]["code"], -32602, "{no_meta}");
}

/// S-G4 / S-G6: every manifest URI is readable, and the bytes read (text or
/// blob) have exactly the manifest's digest and size.
#[tokio::test]
async fn s_g6_read_bytes_match_manifest() {
    let url = start(McpServer::builder().with_ext_skills(catalog(nested_fixture()))).await;
    let skills = list_all(&url).await;
    let mut checked = 0;
    for s in &skills {
        for res in s["resources"].as_array().unwrap() {
            let uri = res["uri"].as_str().unwrap();
            let r = post(
                &url,
                "resources/read",
                Some(uri),
                json!({ "_meta": meta(), "uri": uri }),
            )
            .await;
            let c = &r["result"]["contents"][0];
            let bytes = match (c["text"].as_str(), c["blob"].as_str()) {
                (Some(t), _) => t.as_bytes().to_vec(),
                (_, Some(b)) => base64::engine::general_purpose::STANDARD.decode(b).unwrap(),
                _ => panic!("no content for {uri}: {r}"),
            };
            let expected: SkillResource = serde_json::from_value(res.clone()).unwrap();
            assert_eq!(SkillResource::of(uri.to_string(), &bytes), expected);
            checked += 1;
        }
    }
    // outer: SKILL.md, GUIDE.md, "a b.md" (percent-encoded), logo.bin, inner/SKILL.md;
    // inner: SKILL.md.
    assert_eq!(checked, 6);
}

/// Resource Metadata: SKILL.md is `text/markdown` with the frontmatter name and
/// description.
#[tokio::test]
async fn skill_md_resource_metadata() {
    let url = start(McpServer::builder().with_ext_skills(catalog(nested_fixture()))).await;
    let r = post(&url, "resources/list", None, json!({ "_meta": meta() })).await;
    let md = r["result"]["resources"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["uri"] == "skill://acme/outer/SKILL.md")
        .expect("listed");
    assert_eq!(md["mimeType"], "text/markdown");
    assert_eq!(md["name"], "outer");
    assert_eq!(md["description"], "Does outer.");
    // A nested SKILL.md is also a supporting file of its parent; its own entry's
    // metadata wins (ADR-033 §5).
    let inner = r["result"]["resources"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["uri"] == "skill://acme/outer/inner/SKILL.md")
        .expect("listed");
    assert_eq!(inner["name"], "inner");
    assert_eq!(inner["description"], "Does inner.");
}

/// Nested Skills: publication is flat; the enclosing manifest lists the nested
/// SKILL.md as a supporting file.
#[tokio::test]
async fn nested_skill_publishes_flat() {
    let url = start(McpServer::builder().with_ext_skills(catalog(nested_fixture()))).await;
    let skills = list_all(&url).await;
    let uris: Vec<&str> = skills.iter().map(|s| s["uri"].as_str().unwrap()).collect();
    assert_eq!(
        uris,
        [
            "skill://acme/outer/SKILL.md",
            "skill://acme/outer/inner/SKILL.md"
        ]
    );
    let outer_files: Vec<&str> = skills[0]["resources"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["uri"].as_str().unwrap())
        .collect();
    assert!(outer_files.contains(&"skill://acme/outer/inner/SKILL.md"));
}

/// S-G7: `directoryRead` is not declared, so the method is unknown.
#[tokio::test]
async fn s_g7_directory_read_is_method_not_found() {
    let url = start(McpServer::builder().with_ext_skills(catalog(nested_fixture()))).await;
    let r = post(
        &url,
        "resources/directory/read",
        None,
        json!({ "_meta": meta(), "uri": "skill://acme/outer" }),
    )
    .await;
    assert_eq!(r["error"]["code"], -32601, "{r}");
}

/// ADR-033 §5 (Q10): a manifest URI the read policy would reject is refused at
/// build, and the error names the file.
#[tokio::test]
async fn q10_blocklisted_manifest_uri_refused_at_build() {
    for (rel, uri) in [
        ("etc/config.md", "skill://tool/etc/config.md"),
        ("proc/steps.md", "skill://tool/proc/steps.md"),
        ("notes..md", "skill://tool/notes..md"),
    ] {
        let e = entry(&[], "tool", vec![SkillFile::new(rel, "x")]);
        let err = McpServer::builder()
            .name("q10")
            .version("0.0.0")
            .with_ext_skills(catalog(vec![e]))
            .build()
            .err()
            .unwrap_or_else(|| panic!("{rel} was accepted"));
        assert!(err.to_string().contains(uri), "{err}");
    }
}

/// ADR-033 §5: a URI two entries list with different bytes fails the build.
#[tokio::test]
async fn one_uri_one_content_conflicting_digest_refused() {
    let stale = entry(
        &["acme"],
        "outer",
        vec![SkillFile::new(
            "inner/SKILL.md",
            "---\nname: inner\ndescription: Old.\n---\n",
        )],
    );
    let err = McpServer::builder()
        .name("dup")
        .version("0.0.0")
        .with_ext_skills(catalog(vec![
            stale,
            entry(&["acme", "outer"], "inner", vec![]),
        ]))
        .build()
        .err()
        .unwrap_or_else(|| panic!("conflicting content accepted"));
    assert!(
        err.to_string()
            .contains("skill://acme/outer/inner/SKILL.md"),
        "{err}"
    );
}

struct Plain(&'static str);
impl HasResourceMetadata for Plain {
    fn name(&self) -> &str {
        "plain"
    }
}
impl HasResourceUri for Plain {
    fn uri(&self) -> &str {
        self.0
    }
}
impl HasResourceDescription for Plain {}
impl HasResourceMimeType for Plain {}
impl HasResourceSize for Plain {}
impl HasResourceAnnotations for Plain {}
impl HasResourceMeta for Plain {}
impl HasIcons for Plain {}
#[async_trait::async_trait]
impl McpResource for Plain {
    async fn read(
        &self,
        _p: Option<Value>,
        _s: Option<&SessionContext>,
    ) -> McpResult<Vec<turul_mcp_protocol::resources::ResourceContent>> {
        Ok(vec![turul_mcp_protocol::resources::ResourceContent::text(
            self.0, "other",
        )])
    }
}

/// ADR-033 §5: another resource at a skill file's URI — registered before or
/// after `with_ext_skills` — fails the build.
#[tokio::test]
async fn one_uri_one_content_foreign_resource_refused() {
    const URI: &str = "skill://acme/outer/references/GUIDE.md";
    let before = McpServer::builder()
        .name("dup")
        .version("0.0.0")
        .resource(Plain(URI))
        .with_ext_skills(catalog(nested_fixture()))
        .build()
        .err()
        .unwrap_or_else(|| panic!("resource registered before was accepted"));
    assert!(before.to_string().contains(URI), "{before}");
    let after = McpServer::builder()
        .name("dup")
        .version("0.0.0")
        .with_ext_skills(catalog(nested_fixture()))
        .resource(Plain(URI))
        .build()
        .err()
        .unwrap_or_else(|| panic!("resource registered after was accepted"));
    assert!(after.to_string().contains(URI), "{after}");
}
