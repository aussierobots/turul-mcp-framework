//! Wire-level acceptance for OAuth 2.1 resource-server behavior on the 2026
//! default transport (Builder → `server.run()` → real HTTP).
//!
//! Authorization §Token Handling: "Invalid or expired tokens MUST receive a
//! HTTP 401 response." §Overview: "MCP servers MUST implement OAuth 2.0
//! Protected Resource Metadata (RFC 9728)" — the `WWW-Authenticate`
//! challenge points clients at the metadata URL.
//!
//! Built only under the 2026 feature; compiles to nothing under 2025-11-25.
#![cfg(feature = "protocol-2026-07-28")]

mod common;

use turul_mcp_derive::McpTool;
use turul_mcp_oauth::ProtectedResourceMetadata;
use turul_mcp_server::prelude::*;

#[derive(McpTool, Clone, Default)]
#[tool(name = "echo", description = "Echo", output = String)]
struct EchoTool {}

impl EchoTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<String> {
        Ok("ok".to_string())
    }
}

/// OAuth-protected 2026 server. The JWKS URI is unreachable on purpose —
/// missing/garbage bearers must be rejected before any JWKS fetch.
async fn start_oauth_server() -> String {
    let reserved = common::reserve_port().await;
    let port = reserved.port;

    let metadata = ProtectedResourceMetadata::new(
        format!("http://127.0.0.1:{port}/mcp"),
        vec!["https://auth.example.test".to_string()],
    )
    .expect("metadata")
    .with_scopes(vec!["mcp:read".to_string()]);

    let (auth_middleware, routes) =
        turul_mcp_oauth::oauth_resource_server(metadata, "http://127.0.0.1:1/jwks")
            .expect("oauth setup");

    let mut builder = McpServer::builder()
        .name("oauth-2026-test")
        .version("0.4.0")
        .tool(EchoTool::default())
        .middleware(auth_middleware)
        .bind_address(format!("127.0.0.1:{port}").parse().unwrap());
    for (path, handler) in routes {
        builder = builder.route(&path, handler);
    }
    let server = builder.build().expect("build 2026 server");

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

fn meta() -> serde_json::Value {
    serde_json::json!({
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientInfo": { "name": "test-client", "version": "1.0.0" },
        "io.modelcontextprotocol/clientCapabilities": {}
    })
}

async fn discover(url: &str, bearer: Option<&str>) -> reqwest::Response {
    let client = reqwest::Client::new();
    let mut req = client
        .post(url)
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "server/discover");
    if let Some(token) = bearer {
        req = req.header("Authorization", format!("Bearer {token}"));
    }
    req.json(&serde_json::json!({
        "jsonrpc": "2.0", "id": 1, "method": "server/discover",
        "params": { "_meta": meta() }
    }))
    .send()
    .await
    .expect("POST")
}

#[tokio::test]
async fn missing_bearer_gets_401_with_www_authenticate_challenge() {
    let url = start_oauth_server().await;
    let resp = discover(&url, None).await;
    assert_eq!(resp.status(), 401, "no token → 401");
    let challenge = resp
        .headers()
        .get("www-authenticate")
        .and_then(|v| v.to_str().ok())
        .expect("WWW-Authenticate header must be present")
        .to_string();
    assert!(challenge.starts_with("Bearer"), "{challenge}");
    assert!(
        challenge.contains("resource_metadata="),
        "challenge must point at the RFC 9728 metadata URL: {challenge}"
    );
}

#[tokio::test]
async fn garbage_bearer_gets_401_invalid_token() {
    let url = start_oauth_server().await;
    let resp = discover(&url, Some("not-a-jwt")).await;
    assert_eq!(resp.status(), 401, "malformed token → 401");
    let challenge = resp
        .headers()
        .get("www-authenticate")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_string();
    assert!(
        challenge.contains("invalid_token"),
        "challenge must carry error=\"invalid_token\": {challenge}"
    );
}

/// Auth outranks request validation: a request that ALSO lacks `_meta` gets
/// the 401 challenge, not the -32602 — token validation runs first.
#[tokio::test]
async fn auth_401_outranks_meta_validation_400() {
    let url = start_oauth_server().await;
    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "tools/list")
        .json(&serde_json::json!({
            "jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}
        }))
        .send()
        .await
        .expect("POST");
    assert_eq!(
        resp.status(),
        401,
        "401 must outrank the missing-_meta 400 (auth runs before validation)"
    );
}

#[tokio::test]
async fn protected_resource_metadata_is_served_on_well_known_routes() {
    let url = start_oauth_server().await;
    let base = url.strip_suffix("/mcp").unwrap();
    let client = reqwest::Client::new();

    // RFC 9728 §3: root form + path form, both unauthenticated.
    for path in [
        "/.well-known/oauth-protected-resource",
        "/.well-known/oauth-protected-resource/mcp",
    ] {
        let resp = client
            .get(format!("{base}{path}"))
            .send()
            .await
            .expect("GET well-known");
        assert_eq!(resp.status(), 200, "{path} must be public");
        let body: serde_json::Value = resp.json().await.expect("json");
        assert_eq!(
            body["authorization_servers"][0], "https://auth.example.test",
            "{path}: {body}"
        );
        assert!(body["resource"].as_str().unwrap_or("").ends_with("/mcp"));
    }
}

/// RFC 6750 §3: a challenge carries credentials-related material, so it "MUST
/// NOT be cached". Asserted on every status the challenge builder emits, since
/// they share one response path — a cached 401 would keep a client locked out
/// after it obtains a token.
#[tokio::test]
async fn challenges_are_not_cacheable() {
    let url = start_oauth_server().await;

    let no_store = |resp: &reqwest::Response| -> bool {
        resp.headers()
            .get("cache-control")
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
            .to_ascii_lowercase()
            .contains("no-store")
    };

    let resp = discover(&url, None).await;
    assert_eq!(resp.status(), 401);
    assert!(
        no_store(&resp),
        "401 challenge must be Cache-Control: no-store"
    );

    let resp = discover(&url, Some("not-a-jwt")).await;
    assert_eq!(resp.status(), 401);
    assert!(
        no_store(&resp),
        "invalid_token challenge must be Cache-Control: no-store"
    );

    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "server/discover")
        .header("Authorization", "Basic Zm9vOmJhcg==")
        .json(&serde_json::json!({
            "jsonrpc": "2.0", "id": 4, "method": "server/discover",
            "params": { "_meta": meta() }
        }))
        .send()
        .await
        .expect("POST");
    assert_eq!(resp.status(), 400);
    assert!(
        no_store(&resp),
        "invalid_request challenge must be Cache-Control: no-store"
    );
}

/// RFC 9728 §3.1 has clients fetch protected-resource metadata from a resource
/// server they are not same-origin with, so the metadata routes are outside the
/// DNS-rebinding gate that answers 403 on the MCP endpoint. A hostile `Origin`
/// must not turn discovery into a 403 — the document is public and carries no
/// credentials.
#[tokio::test]
async fn hostile_origin_does_not_block_the_well_known_metadata() {
    let url = start_oauth_server().await;
    let base = url.strip_suffix("/mcp").unwrap();
    let client = reqwest::Client::new();

    for path in [
        "/.well-known/oauth-protected-resource",
        "/.well-known/oauth-protected-resource/mcp",
    ] {
        let resp = client
            .get(format!("{base}{path}"))
            .header("Origin", "http://attacker.example")
            .send()
            .await
            .expect("GET well-known with hostile Origin");
        assert_eq!(
            resp.status(),
            200,
            "{path} must stay reachable cross-origin, not 403"
        );
        let body: serde_json::Value = resp.json().await.expect("json");
        assert_eq!(
            body["authorization_servers"][0], "https://auth.example.test",
            "{path}: {body}"
        );
    }

    // The MCP endpoint itself is still gated by the same hostile Origin.
    let resp = client
        .post(&url)
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "server/discover")
        .header("Origin", "http://attacker.example")
        .json(&serde_json::json!({
            "jsonrpc": "2.0", "id": 5, "method": "server/discover",
            "params": { "_meta": meta() }
        }))
        .send()
        .await
        .expect("POST");
    assert_eq!(
        resp.status(),
        403,
        "the MCP endpoint must still reject the hostile origin"
    );
}

/// Authorization §Error Handling: "400 Bad Request: Malformed authorization
/// request" (RFC 6750 §3.1 invalid_request) — a PRESENT but malformed
/// Authorization header is 400, distinguishable from the missing-header 401.
#[tokio::test]
async fn malformed_authorization_header_gets_400_invalid_request() {
    let url = start_oauth_server().await;
    let client = reqwest::Client::new();
    for bad in ["Basic Zm9vOmJhcg==", "Bearer", "Bearer two tokens"] {
        let resp = client
            .post(&url)
            .header("Accept", "application/json")
            .header("MCP-Protocol-Version", "2026-07-28")
            .header("Mcp-Method", "server/discover")
            .header("Authorization", bad)
            .json(&serde_json::json!({
                "jsonrpc": "2.0", "id": 3, "method": "server/discover",
                "params": { "_meta": meta() }
            }))
            .send()
            .await
            .expect("POST");
        assert_eq!(
            resp.status(),
            400,
            "malformed Authorization {bad:?} must be 400, not 401-as-missing"
        );
        let challenge = resp
            .headers()
            .get("www-authenticate")
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default();
        assert!(
            challenge.contains("invalid_request"),
            "challenge must carry error=\"invalid_request\": {challenge}"
        );
    }
}
