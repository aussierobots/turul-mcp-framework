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

/// S-G1 / §7: discover declares the extension (`directoryRead: true`) and `resources`.
#[tokio::test]
async fn s_g1_discover_advertises_skills_and_resources() {
    let url = start(McpServer::builder().with_ext_skills(catalog(nested_fixture()))).await;
    let r = post(&url, "server/discover", None, json!({ "_meta": meta() })).await;
    let caps = &r["result"]["capabilities"];
    assert_eq!(
        caps["extensions"][EXTENSION_IDENTIFIER],
        json!({ "directoryRead": true }),
        "{r}"
    );
    assert!(caps["resources"].is_object(), "{r}");
}

/// S-G1: the declaration forces `resources` even when no skill is served yet.
#[tokio::test]
async fn s_g1_empty_catalog_still_declares_resources() {
    let url = start(McpServer::builder().with_ext_skills(SkillCatalog::new())).await;
    let r = post(&url, "server/discover", None, json!({ "_meta": meta() })).await;
    let caps = &r["result"]["capabilities"];
    assert_eq!(
        caps["extensions"][EXTENSION_IDENTIFIER],
        json!({ "directoryRead": true }),
        "{r}"
    );
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

async fn dir_read(url: &str, uri: &str, cursor: Option<&str>) -> Value {
    let mut p = json!({ "_meta": meta(), "uri": uri });
    if let Some(c) = cursor {
        p["cursor"] = json!(c);
    }
    post(url, "resources/directory/read", None, p).await
}

fn child_uris(r: &Value) -> Vec<String> {
    r["result"]["resources"]
        .as_array()
        .unwrap_or_else(|| panic!("no resources: {r}"))
        .iter()
        .map(|c| c["uri"].as_str().unwrap().to_string())
        .collect()
}

/// §7 (:63): declaring `directoryRead` is what commits the server to the method.
#[tokio::test]
async fn s_g7_capability_declares_directory_read() {
    let url = start(McpServer::builder().with_ext_skills(catalog(nested_fixture()))).await;
    let r = post(&url, "server/discover", None, json!({ "_meta": meta() })).await;
    assert_eq!(
        r["result"]["capabilities"]["extensions"][EXTENSION_IDENTIFIER]["directoryRead"], true,
        "{r}"
    );
}

/// §7 (:556-560): direct children only; files carry their `resources/list`
/// metadata, subdirectories are `inode/directory`; not recursive.
#[tokio::test]
async fn s_g7_directory_read_lists_direct_children() {
    let url = start(McpServer::builder().with_ext_skills(catalog(nested_fixture()))).await;
    let r = dir_read(&url, "skill://acme/outer", None).await;
    assert_eq!(r["result"]["resultType"], "complete", "{r}");
    assert!(r["result"]["ttlMs"].is_number() && r["result"]["cacheScope"].is_string());
    assert_eq!(
        child_uris(&r),
        [
            "skill://acme/outer/SKILL.md",
            "skill://acme/outer/assets",
            "skill://acme/outer/inner",
            "skill://acme/outer/notes",
            "skill://acme/outer/references",
        ]
    );
    let children = r["result"]["resources"].as_array().unwrap();
    let listed = post(&url, "resources/list", None, json!({ "_meta": meta() })).await;
    let in_list = |uri: &str| {
        listed["result"]["resources"]
            .as_array()
            .unwrap()
            .iter()
            .find(|x| x["uri"] == uri)
            .cloned()
    };
    for c in children {
        let uri = c["uri"].as_str().unwrap();
        match in_list(uri) {
            Some(file) => assert_eq!(c, &file, "file child must equal its resources/list entry"),
            None => {
                assert_eq!(c["mimeType"], "inode/directory", "{c}");
                assert_eq!(c["name"], uri.rsplit('/').next().unwrap(), "{c}");
            }
        }
    }
    // Not recursive: the nested file is not a child of the root.
    assert!(!child_uris(&r).contains(&"skill://acme/outer/references/GUIDE.md".to_string()));
}

/// §7 (:562): every directory in the namespace answers — roots and every
/// subdirectory, nested skills included; descending reaches every file.
#[tokio::test]
async fn s_g7_every_directory_is_readable() {
    let url = start(McpServer::builder().with_ext_skills(catalog(nested_fixture()))).await;
    let mut seen_files = Vec::new();
    let mut queue = vec![
        "skill://acme/outer".to_string(),
        "skill://acme/outer/inner".to_string(),
    ];
    while let Some(dir) = queue.pop() {
        let r = dir_read(&url, &dir, None).await;
        for c in r["result"]["resources"]
            .as_array()
            .unwrap_or_else(|| panic!("{dir}: {r}"))
        {
            let uri = c["uri"].as_str().unwrap().to_string();
            if c["mimeType"] == "inode/directory" {
                queue.push(uri);
            } else {
                seen_files.push(uri);
            }
        }
    }
    seen_files.sort();
    seen_files.dedup();
    let mut manifest: Vec<String> = list_all(&url)
        .await
        .iter()
        .flat_map(|s| s["resources"].as_array().unwrap().clone())
        .map(|r| r["uri"].as_str().unwrap().to_string())
        .collect();
    manifest.sort();
    manifest.dedup();
    assert_eq!(seen_files, manifest);
}

/// §7 (:556): an unknown URI, a file URI and a non-skill URI are -32602.
#[tokio::test]
async fn s_g7_non_directory_is_invalid_params() {
    let url = start(McpServer::builder().with_ext_skills(catalog(nested_fixture()))).await;
    for uri in [
        "skill://acme/outer/SKILL.md",
        "skill://acme/outer/nope",
        "skill://acme",
        "skill://acme/outer/",
    ] {
        let r = dir_read(&url, uri, None).await;
        assert_eq!(r["error"]["code"], -32602, "{uri}: {r}");
    }
}

/// §7 (:560): pagination mirrors resources/list; an unknown cursor is -32602.
#[tokio::test]
async fn s_g7_paginates() {
    let files = (0..120)
        .map(|i| SkillFile::new(format!("t/f{i:03}.md"), "x"))
        .collect();
    let url =
        start(McpServer::builder().with_ext_skills(catalog(vec![entry(&[], "big", files)]))).await;
    let mut got = Vec::new();
    let mut cursor: Option<String> = None;
    loop {
        let r = dir_read(&url, "skill://big/t", cursor.as_deref()).await;
        got.extend(child_uris(&r));
        match r["result"]["nextCursor"].as_str() {
            Some(c) => cursor = Some(c.to_string()),
            None => break,
        }
    }
    assert_eq!(got.len(), 120);
    let bad = dir_read(&url, "skill://big/t", Some("nope")).await;
    assert_eq!(bad["error"]["code"], -32602, "{bad}");
}

/// §7 (:520): directory resources are not added to resources/list.
#[tokio::test]
async fn s_g7_directories_not_in_resources_list() {
    let url = start(McpServer::builder().with_ext_skills(catalog(nested_fixture()))).await;
    let listed = post(&url, "resources/list", None, json!({ "_meta": meta() })).await;
    for r in listed["result"]["resources"].as_array().unwrap() {
        assert_ne!(r["mimeType"], "inode/directory", "{r}");
    }
}

/// A generated skill (§8): files depend on a counter, so each call regenerates.
struct Generated {
    calls: std::sync::atomic::AtomicUsize,
}

#[async_trait::async_trait]
impl turul_mcp_server::ext_skills::DynamicSkill for Generated {
    fn path(&self) -> SkillPath {
        SkillPath::new(["reports"], SkillName::parse("daily").unwrap()).unwrap()
    }
    async fn files(&self) -> McpResult<Vec<SkillFile>> {
        let n = self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(vec![
            SkillFile::new(
                "SKILL.md",
                "---\nname: daily\ndescription: Today's report.\n---\nRead data/today.md\n",
            ),
            SkillFile::new("data/sub/today.md", format!("generation {n}\n")),
        ])
    }
}

fn dynamic_catalog() -> SkillCatalog {
    let mut c = catalog(nested_fixture());
    c.add_dynamic(std::sync::Arc::new(Generated {
        calls: Default::default(),
    }))
    .unwrap();
    c
}

/// §8 (:231): a generated skill's entry publishes `resources: "dynamic"`.
#[tokio::test]
async fn dynamic_entry_publishes_dynamic_resources() {
    let url = start(McpServer::builder().with_ext_skills(dynamic_catalog())).await;
    let g = post(
        &url,
        "skills/get",
        None,
        json!({ "_meta": meta(), "uri": "skill://reports/daily/SKILL.md" }),
    )
    .await;
    assert_eq!(g["result"]["skill"]["resources"], "dynamic", "{g}");
    let listed = list_all(&url).await;
    assert!(
        listed
            .iter()
            .any(|s| s["uri"] == "skill://reports/daily/SKILL.md" && s["resources"] == "dynamic"),
        "{listed:?}"
    );
}

/// §8 / S-G3 (:210): a generated entry's frontmatter equals the generated SKILL.md's.
#[tokio::test]
async fn dynamic_frontmatter_matches_generated_skill_md() {
    let url = start(McpServer::builder().with_ext_skills(dynamic_catalog())).await;
    let g = post(
        &url,
        "skills/get",
        None,
        json!({ "_meta": meta(), "uri": "skill://reports/daily/SKILL.md" }),
    )
    .await;
    assert_eq!(
        g["result"]["skill"]["frontmatter"],
        json!({ "name": "daily", "description": "Today's report." }),
        "{g}"
    );
}

/// §8: reads regenerate and serve the bytes at that URI, multi-segment paths included.
#[tokio::test]
async fn dynamic_read_serves_generated_bytes() {
    let url = start(McpServer::builder().with_ext_skills(dynamic_catalog())).await;
    let uri = "skill://reports/daily/data/sub/today.md";
    let a = post(
        &url,
        "resources/read",
        Some(uri),
        json!({ "_meta": meta(), "uri": uri }),
    )
    .await;
    let b = post(
        &url,
        "resources/read",
        Some(uri),
        json!({ "_meta": meta(), "uri": uri }),
    )
    .await;
    let ta = a["result"]["contents"][0]["text"]
        .as_str()
        .unwrap_or_else(|| panic!("{a}"));
    let tb = b["result"]["contents"][0]["text"]
        .as_str()
        .unwrap_or_else(|| panic!("{b}"));
    assert!(ta.starts_with("generation "), "{a}");
    assert_ne!(ta, tb, "each read regenerates");
    let md = "skill://reports/daily/SKILL.md";
    let r = post(
        &url,
        "resources/read",
        Some(md),
        json!({ "_meta": meta(), "uri": md }),
    )
    .await;
    assert!(
        r["result"]["contents"][0]["text"]
            .as_str()
            .unwrap_or("")
            .contains("name: daily"),
        "{r}"
    );
}

/// §8: a file the generator does not produce is -32602.
#[tokio::test]
async fn dynamic_unknown_file_is_invalid_params() {
    let url = start(McpServer::builder().with_ext_skills(dynamic_catalog())).await;
    let uri = "skill://reports/daily/data/nope.md";
    let r = post(
        &url,
        "resources/read",
        Some(uri),
        json!({ "_meta": meta(), "uri": uri }),
    )
    .await;
    assert_eq!(r["error"]["code"], -32602, "{r}");
}

/// §8 (:584): for a dynamic skill, directory reads are how its files are discovered.
#[tokio::test]
async fn dynamic_directory_read_discovers_files() {
    let url = start(McpServer::builder().with_ext_skills(dynamic_catalog())).await;
    let root = dir_read(&url, "skill://reports/daily", None).await;
    assert_eq!(
        child_uris(&root),
        [
            "skill://reports/daily/SKILL.md",
            "skill://reports/daily/data"
        ],
        "{root}"
    );
    let sub = dir_read(&url, "skill://reports/daily/data/sub", None).await;
    assert_eq!(
        child_uris(&sub),
        ["skill://reports/daily/data/sub/today.md"],
        "{sub}"
    );
}

/// §8: a dynamic skill may not overlap another skill's directory, either way.
#[tokio::test]
async fn dynamic_overlapping_paths_refused_at_build() {
    struct At(&'static [&'static str], &'static str);
    #[async_trait::async_trait]
    impl turul_mcp_server::ext_skills::DynamicSkill for At {
        fn path(&self) -> SkillPath {
            SkillPath::new(self.0.iter().copied(), SkillName::parse(self.1).unwrap()).unwrap()
        }
        async fn files(&self) -> McpResult<Vec<SkillFile>> {
            Ok(vec![])
        }
    }
    for (prefix, name) in [
        (&["acme"][..], "outer"),
        (&["acme", "outer"][..], "deep"),
        (&[][..], "acme"),
    ] {
        let mut c = catalog(nested_fixture());
        let added = c.add_dynamic(std::sync::Arc::new(At(prefix, name)));
        let built = added.and_then(|_| {
            McpServer::builder()
                .name("overlap")
                .version("0.0.0")
                .with_ext_skills(c)
                .build()
                .map(|_| ())
        });
        assert!(
            built.is_err(),
            "{prefix:?}/{name} overlapped and was accepted"
        );
    }
}
