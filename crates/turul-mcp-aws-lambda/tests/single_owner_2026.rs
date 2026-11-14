//! ADR-035 executable contract: Lambda serves exactly what `McpServerBuilder`
//! builds — one owner of handlers, capabilities, extensions and the
//! resource-security policy. Each test names the ADR-035 clause it proves.
#![cfg(feature = "protocol-2026-07-28")]

use std::sync::Arc;

use bytes::Bytes;
use http_body_util::{BodyExt, combinators::UnsyncBoxBody};
use lambda_http::Body as LambdaBody;
use serde_json::{Value, json};

use turul_mcp_aws_lambda::{LambdaMcpHandler, LambdaMcpServerBuilder};
use turul_mcp_builders::ResourceBuilder;
use turul_mcp_protocol::MCP_VERSION;
use turul_mcp_server::McpServer;
use turul_mcp_session_storage::InMemorySessionStorage;

const VENDOR_EXT: &str = "com.example/ext";

fn meta() -> Value {
    json!({
        "io.modelcontextprotocol/protocolVersion": MCP_VERSION,
        "io.modelcontextprotocol/clientInfo": { "name": "t", "version": "1" },
        "io.modelcontextprotocol/clientCapabilities": {}
    })
}

async fn call(
    handler: &LambdaMcpHandler,
    method: &str,
    name: Option<&str>,
    params: Value,
) -> Value {
    let mut params = params;
    params["_meta"] = meta();
    let body = json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params });
    let mut req = http::Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", MCP_VERSION)
        .header("Mcp-Method", method);
    if let Some(n) = name {
        req = req.header("Mcp-Name", n);
    }
    let req = req
        .body(LambdaBody::from(serde_json::to_string(&body).unwrap()))
        .unwrap();
    let resp: lambda_http::Response<UnsyncBoxBody<Bytes, hyper::Error>> = handler
        .handle_streaming(req)
        .await
        .expect("handle_streaming");
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).expect("JSON body")
}

fn resource(uri: &str) -> turul_mcp_builders::resource::DynamicResource {
    ResourceBuilder::new(uri)
        .name("r")
        .text_content("hello")
        .build()
        .unwrap()
}

/// ADR-035 §1/§3: for one configuration, Lambda's `server/discover`
/// capabilities equal the owner's, except `resources.subscribe`, which Lambda
/// cannot serve and therefore does not advertise.
#[tokio::test]
async fn lambda_discover_matches_server() {
    let core = McpServer::builder()
        .name("parity")
        .version("1.0.0")
        .resource(resource("file:///data/a.txt"))
        .extension_capability(VENDOR_EXT, json!({}))
        .build()
        .unwrap();
    let mut expected = serde_json::to_value(&core.capabilities).unwrap();
    expected["resources"]["subscribe"] = json!(false);

    let lambda = LambdaMcpServerBuilder::new()
        .name("parity")
        .version("1.0.0")
        .resource(resource("file:///data/a.txt"))
        .extension_capability(VENDOR_EXT, json!({}))
        .storage(Arc::new(InMemorySessionStorage::new()))
        .sse(false)
        .build()
        .await
        .unwrap()
        .handler()
        .await
        .unwrap();
    let got = call(&lambda, "server/discover", None, json!({})).await;
    assert_eq!(got["result"]["capabilities"], expected, "{got}");
}

/// ADR-035 §3: Lambda `resources/read` enforces the owner's read policy — a
/// URI the auto policy blocks is refused, as on the HTTP server.
#[tokio::test]
async fn lambda_read_enforces_server_policy() {
    let lambda = LambdaMcpServerBuilder::new()
        .name("policy")
        .version("1.0.0")
        .resource(resource("file:///etc/app.txt"))
        .storage(Arc::new(InMemorySessionStorage::new()))
        .sse(false)
        .build()
        .await
        .unwrap()
        .handler()
        .await
        .unwrap();
    let r = call(
        &lambda,
        "resources/read",
        Some("file:///etc/app.txt"),
        json!({ "uri": "file:///etc/app.txt" }),
    )
    .await;
    assert_eq!(r["error"]["code"], -32602, "{r}");
    assert!(
        r["error"]["message"]
            .as_str()
            .unwrap_or("")
            .contains("blocked patterns"),
        "refused by something other than the read policy: {r}"
    );
}

/// ADR-035 §3: an invalid resource URI fails `build()` on Lambda, as on the server.
#[tokio::test]
async fn lambda_invalid_resource_uri_fails_build() {
    let built = LambdaMcpServerBuilder::new()
        .name("invalid")
        .version("1.0.0")
        .resource(resource("no-scheme"))
        .storage(Arc::new(InMemorySessionStorage::new()))
        .sse(false)
        .build()
        .await;
    assert!(built.is_err(), "an invalid resource URI must fail build()");
}

/// ADR-035 §5: Tasks on Lambda is deferred, so Lambda never advertises it —
/// the builder offers no Tasks wiring, and discover carries no Tasks entry.
#[tokio::test]
async fn lambda_does_not_advertise_tasks() {
    let builder_src = include_str!("../src/builder.rs");
    let manifest = include_str!("../Cargo.toml");
    for needle in [
        "with_ext_tasks",
        "ext_task_tool",
        "turul-mcp-server/ext-tasks",
    ] {
        assert!(
            !builder_src.contains(needle) && !manifest.contains(needle),
            "Lambda exposes Tasks wiring (`{needle}`) before ADR-035 §5 is decided"
        );
    }

    let lambda = LambdaMcpServerBuilder::new()
        .name("no-tasks")
        .version("1.0.0")
        .storage(Arc::new(InMemorySessionStorage::new()))
        .sse(false)
        .build()
        .await
        .unwrap()
        .handler()
        .await
        .unwrap();
    let got = call(&lambda, "server/discover", None, json!({})).await;
    let ext = &got["result"]["capabilities"]["extensions"];
    assert!(ext.get("io.modelcontextprotocol/tasks").is_none(), "{got}");
}

/// ADR-035 §2: no parallel registration remains in the Lambda crate — handlers,
/// capabilities and the read policy come only from the owner.
#[test]
fn no_parallel_registration() {
    fn rust_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                rust_files(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }
    let mut files = Vec::new();
    rust_files(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut files,
    );
    assert!(files.len() > 5, "scanned too few files: {files:?}");
    for path in files {
        let text = std::fs::read_to_string(&path).unwrap();
        let code = text.split("#[cfg(test)]\nmod tests {").next().unwrap();
        for needle in [
            "ResourcesReadHandler::new",
            "ResourcesListHandler::new",
            "ListToolsHandler::new",
            "DiscoverHandler::new",
            "SessionAwareToolHandler::new",
            "PromptsListHandler::new",
            "SessionAwareInitializeHandler::new",
            "SessionAwareMcpHandlerBridge::new",
            "register_method",
            "without_security",
        ] {
            assert!(
                !code.contains(needle),
                "{} still builds `{needle}` itself",
                path.display()
            );
        }
    }
}

/// ADR-035 §4 / ADR-033: Skills reach Lambda through the owner with no Lambda
/// skills code — discover, list, get (unknown = -32602) and digest-exact reads.
#[cfg(feature = "ext-skills")]
#[tokio::test]
async fn lambda_serves_skills_through_the_owner() {
    use turul_mcp_ext_skills::{SkillEntry, SkillFile, SkillName, SkillPath, SkillResource};
    use turul_mcp_server::ext_skills::SkillCatalog;

    let mut catalog = SkillCatalog::new();
    catalog
        .add(
            SkillEntry::from_files(
                SkillPath::new([], SkillName::parse("demo").unwrap()).unwrap(),
                vec![
                    SkillFile::new(
                        "SKILL.md",
                        "---\nname: demo\ndescription: Does demo.\n---\nBody\n",
                    ),
                    SkillFile::new("data/x.bin", vec![0u8, 255]),
                ],
            )
            .unwrap(),
        )
        .unwrap();
    let lambda = LambdaMcpServerBuilder::new()
        .name("skills")
        .version("1.0.0")
        .with_ext_skills(catalog)
        .storage(Arc::new(InMemorySessionStorage::new()))
        .sse(false)
        .build()
        .await
        .unwrap()
        .handler()
        .await
        .unwrap();

    let d = call(&lambda, "server/discover", None, json!({})).await;
    assert_eq!(
        d["result"]["capabilities"]["extensions"]["io.modelcontextprotocol/skills"],
        json!({}),
        "{d}"
    );
    assert!(d["result"]["capabilities"]["resources"].is_object(), "{d}");

    let list = call(&lambda, "skills/list", None, json!({})).await;
    let skill = &list["result"]["skills"][0];
    assert_eq!(skill["uri"], "skill://demo/SKILL.md", "{list}");

    let unknown = call(
        &lambda,
        "skills/get",
        None,
        json!({ "uri": "skill://nope/SKILL.md" }),
    )
    .await;
    assert_eq!(unknown["error"]["code"], -32602, "{unknown}");

    for res in skill["resources"].as_array().unwrap() {
        let uri = res["uri"].as_str().unwrap();
        let r = call(&lambda, "resources/read", Some(uri), json!({ "uri": uri })).await;
        let c = &r["result"]["contents"][0];
        let bytes = match c["text"].as_str() {
            Some(t) => t.as_bytes().to_vec(),
            None => {
                use base64::Engine;
                base64::engine::general_purpose::STANDARD
                    .decode(c["blob"].as_str().unwrap())
                    .unwrap()
            }
        };
        let expected: SkillResource = serde_json::from_value(res.clone()).unwrap();
        assert_eq!(
            SkillResource::of(uri.to_string(), &bytes),
            expected,
            "{uri}"
        );
    }
}

/// ADR-033 §5 (Q10) on Lambda: a blocklisted skill file fails build, naming it.
#[cfg(feature = "ext-skills")]
#[tokio::test]
async fn lambda_q10_blocklisted_skill_file_refused_at_build() {
    use turul_mcp_ext_skills::{SkillEntry, SkillFile, SkillName, SkillPath};
    use turul_mcp_server::ext_skills::SkillCatalog;

    let mut catalog = SkillCatalog::new();
    catalog
        .add(
            SkillEntry::from_files(
                SkillPath::new([], SkillName::parse("tool").unwrap()).unwrap(),
                vec![
                    SkillFile::new("SKILL.md", "---\nname: tool\ndescription: T.\n---\n"),
                    SkillFile::new("etc/config.md", "x"),
                ],
            )
            .unwrap(),
        )
        .unwrap();
    let built = LambdaMcpServerBuilder::new()
        .name("q10")
        .version("1.0.0")
        .with_ext_skills(catalog)
        .storage(Arc::new(InMemorySessionStorage::new()))
        .sse(false)
        .build()
        .await;
    let err = built.err().expect("blocklisted skill file accepted");
    assert!(
        err.to_string().contains("skill://tool/etc/config.md"),
        "{err}"
    );
}
