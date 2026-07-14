//! Wire-level acceptance: a bilingual client, locked to 2026-07-28, routes every
//! supported operation through the 2026 path (each request carries the required
//! per-request `_meta`) and parses the 2026-shaped result. Removed-from-core
//! methods (`ping`, `tasks/*`) are rejected on a 2026 connection.
//!
//! The 2026 server is a wiremock peer returning 2026 wire shapes; every op mock
//! matches on `params._meta.io.modelcontextprotocol/protocolVersion = "2026-07-28"`,
//! so a stub only responds if the client actually sent a 2026 request.

use turul_mcp_client::config::ClientConfig;
use turul_mcp_client::transport::http::HttpTransport;
use turul_mcp_client::{McpClient, McpVersion};
use wiremock::matchers::{body_partial_json, header, method};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn meta_match(rpc_method: &str) -> serde_json::Value {
    serde_json::json!({
        "method": rpc_method,
        "params": { "_meta": { "io.modelcontextprotocol/protocolVersion": "2026-07-28" } }
    })
}

async fn mount_2026_result(server: &MockServer, rpc_method: &str, result: serde_json::Value) {
    // SEP-2243: every 2026 request must mirror its method into Mcp-Method and
    // advertise the negotiated version — the stub only answers compliant requests.
    Mock::given(method("POST"))
        .and(header("Mcp-Method", rpc_method))
        .and(header("MCP-Protocol-Version", "2026-07-28"))
        .and(body_partial_json(meta_match(rpc_method)))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json")
                .set_body_json(
                    serde_json::json!({ "jsonrpc": "2.0", "id": "x", "result": result }),
                ),
        )
        .mount(server)
        .await;
}

async fn start_2026_server() -> MockServer {
    let server = MockServer::start().await;
    // SSE GET listener -> 404 terminal.
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;
    // server/discover => locks the connection to 2026-07-28. The probe itself
    // must carry the 2026 request-metadata headers (its _meta says 2026-07-28
    // and the header MUST match the body).
    Mock::given(method("POST"))
        .and(header("Mcp-Method", "server/discover"))
        .and(header("MCP-Protocol-Version", "2026-07-28"))
        .and(body_partial_json(
            serde_json::json!({"method": "server/discover"}),
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json")
                .set_body_json(serde_json::json!({
                    "jsonrpc": "2.0", "id": "d",
                    "result": {
                        "resultType": "complete", "ttlMs": 0, "cacheScope": "public",
                        "supportedVersions": ["2026-07-28"], "capabilities": {},
                        "serverInfo": { "name": "mock-2026", "version": "1.0.0" }
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

#[tokio::test]
async fn all_supported_ops_route_through_2026_path() {
    let server = start_2026_server().await;

    mount_2026_result(
        &server, "tools/call",
        serde_json::json!({"resultType":"complete","content":[{"type":"text","text":"ok"}],"isError":false}),
    ).await;
    mount_2026_result(
        &server, "resources/list",
        serde_json::json!({"resultType":"complete","ttlMs":0,"cacheScope":"public","resources":[{"uri":"file:///a","name":"a"}]}),
    ).await;
    mount_2026_result(
        &server, "resources/read",
        serde_json::json!({"resultType":"complete","ttlMs":0,"cacheScope":"public","contents":[{"uri":"file:///a","text":"hello","mimeType":"text/plain"}]}),
    ).await;
    mount_2026_result(
        &server, "resources/templates/list",
        serde_json::json!({"resultType":"complete","ttlMs":0,"cacheScope":"public","resourceTemplates":[{"uriTemplate":"file:///{id}","name":"t"}]}),
    ).await;
    mount_2026_result(
        &server, "prompts/list",
        serde_json::json!({"resultType":"complete","ttlMs":0,"cacheScope":"public","prompts":[{"name":"p"}]}),
    ).await;
    mount_2026_result(
        &server, "prompts/get",
        serde_json::json!({"resultType":"complete","messages":[{"role":"user","content":{"type":"text","text":"hi"}}]}),
    ).await;

    let client = connect_2026(&server).await;

    let call = client
        .call_tool("echo", serde_json::json!({}))
        .await
        .expect("call_tool");
    assert_eq!(call.is_error, Some(false));

    let resources = client.list_resources().await.expect("list_resources");
    assert_eq!(resources.len(), 1);
    assert_eq!(resources[0].uri, "file:///a");

    let contents = client
        .read_resource("file:///a")
        .await
        .expect("read_resource");
    assert_eq!(contents.len(), 1);

    let templates = client
        .list_resource_templates()
        .await
        .expect("list_resource_templates");
    assert_eq!(templates.len(), 1);

    let prompts = client.list_prompts().await.expect("list_prompts");
    assert_eq!(prompts.len(), 1);
    assert_eq!(prompts[0].name, "p");

    let prompt = client.get_prompt("p", None).await.expect("get_prompt");
    assert_eq!(prompt.messages.len(), 1);
}

#[tokio::test]
async fn removed_methods_are_rejected_on_2026_connection() {
    // No ping/tasks stubs mounted: the client must reject BEFORE sending, so the
    // request never reaches the wire. (On a 2025 connection these would be sent.)
    let server = start_2026_server().await;
    let client = connect_2026(&server).await;

    assert!(
        client.ping().await.is_err(),
        "`ping` was removed from MCP 2026-07-28 core — must be rejected on a 2026 connection"
    );
    assert!(
        client.get_task("t1").await.is_err(),
        "`tasks/*` moved to an extension in 2026-07-28 — must be rejected on a 2026 connection"
    );
    assert!(
        client.list_tasks().await.is_err(),
        "`tasks/list` is not in 2026-07-28 core — must be rejected on a 2026 connection"
    );
}

#[tokio::test]
async fn paginated_list_routes_through_2026_with_meta_and_cursor() {
    let server = start_2026_server().await;
    // The matcher requires BOTH the 2026 `_meta` and the cursor, so it only
    // responds if the paginated call routed through the 2026 path with `_meta`.
    Mock::given(method("POST"))
        .and(body_partial_json(serde_json::json!({
            "method": "resources/list",
            "params": {
                "_meta": { "io.modelcontextprotocol/protocolVersion": "2026-07-28" },
                "cursor": "page-2"
            }
        })))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json")
                .set_body_json(serde_json::json!({
                    "jsonrpc": "2.0", "id": "x",
                    "result": {
                        "resultType": "complete", "ttlMs": 0, "cacheScope": "public",
                        "resources": [{ "uri": "file:///b", "name": "b" }],
                        "nextCursor": "page-3"
                    }
                })),
        )
        .mount(&server)
        .await;

    let client = connect_2026(&server).await;
    let page = client
        .list_resources_paginated(Some(turul_mcp_client::MetaCursor::new("page-2")))
        .await
        .expect("paginated resources/list must round-trip through the 2026 path");
    assert_eq!(page.resources.len(), 1);
    assert_eq!(
        page.next_cursor.as_ref().map(|c| c.as_str()),
        Some("page-3")
    );
}

#[tokio::test]
async fn client_advertises_2026_protocol_version_on_the_wire() {
    use wiremock::matchers::header;
    let server = start_2026_server().await;
    // The mock only responds when the request carries the full 2026 header set:
    // `MCP-Protocol-Version: 2026-07-28` plus the SEP-2243 `Mcp-Method` and
    // (for tools/call) `Mcp-Name` mirrors. A green call_tool proves the client
    // emits all three on the wire.
    Mock::given(method("POST"))
        .and(header("MCP-Protocol-Version", "2026-07-28"))
        .and(header("Mcp-Method", "tools/call"))
        .and(header("Mcp-Name", "echo"))
        .and(body_partial_json(meta_match("tools/call")))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json")
                .set_body_json(serde_json::json!({
                    "jsonrpc": "2.0", "id": "x",
                    "result": {"resultType":"complete","content":[{"type":"text","text":"ok"}],"isError":false}
                })),
        )
        .mount(&server)
        .await;

    let client = connect_2026(&server).await;
    let call = client
        .call_tool("echo", serde_json::json!({}))
        .await
        .expect("tools/call must reach the MCP-Protocol-Version: 2026-07-28-gated mock");
    assert_eq!(call.is_error, Some(false));
}

/// Tools §x-mcp-header: "Clients using the Streamable HTTP transport MUST
/// reject tool definitions where any x-mcp-header value violates these
/// constraints. Rejection means the client MUST exclude the invalid tool
/// from the result of tools/list" — and valid tools must survive.
#[tokio::test]
async fn invalid_x_mcp_header_tools_are_excluded_from_tools_list() {
    let server = start_2026_server().await;
    mount_2026_result(
        &server,
        "tools/list",
        serde_json::json!({
            "resultType": "complete", "ttlMs": 0, "cacheScope": "public",
            "tools": [
                {
                    "name": "good_tool",
                    "inputSchema": { "type": "object", "properties": {
                        "region": { "type": "string", "x-mcp-header": "Region" }
                    }}
                },
                {
                    "name": "bad_tool",
                    "inputSchema": { "type": "object", "properties": {
                        // space + '!' violate the tchar constraint on header names
                        "region": { "type": "string", "x-mcp-header": "Bad Header!" }
                    }}
                }
            ]
        }),
    )
    .await;

    let client = connect_2026(&server).await;
    let tools = client.list_tools().await.expect("tools/list");
    let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
    assert!(
        names.contains(&"good_tool"),
        "valid tools must survive: {names:?}"
    );
    assert!(
        !names.contains(&"bad_tool"),
        "a tool with an invalid x-mcp-header value MUST be excluded: {names:?}"
    );
}

/// SEP-2243: an `x-mcp-header` annotation reachable only through `items`
/// (not a plain `properties` chain) is a misplaced annotation that
/// `scan_x_mcp_headers` cannot see on its own — `find_misplaced_x_mcp_header`
/// is the detector that catches it, and the whole tool must be excluded.
#[tokio::test]
async fn misplaced_x_mcp_header_under_items_excludes_the_tool_from_tools_list() {
    let server = start_2026_server().await;
    mount_2026_result(
        &server,
        "tools/list",
        serde_json::json!({
            "resultType": "complete", "ttlMs": 0, "cacheScope": "public",
            "tools": [
                {
                    "name": "good_tool",
                    "inputSchema": { "type": "object", "properties": {
                        "region": { "type": "string", "x-mcp-header": "Region" }
                    }}
                },
                {
                    "name": "misplaced_header_tool",
                    "inputSchema": { "type": "object", "properties": {
                        "tags": { "type": "array", "items": {
                            "type": "string", "x-mcp-header": "Tag"
                        }}
                    }}
                }
            ]
        }),
    )
    .await;

    let client = connect_2026(&server).await;
    let tools = client.list_tools().await.expect("tools/list");
    let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
    assert!(
        names.contains(&"good_tool"),
        "a valid top-level x-mcp-header must survive: {names:?}"
    );
    assert!(
        !names.contains(&"misplaced_header_tool"),
        "x-mcp-header nested under `items` MUST exclude the tool: {names:?}"
    );
}

/// A tool advertising an `inputSchema` that fails JSON Schema 2020-12
/// meta-validation must be excluded from `tools/list`.
#[tokio::test]
async fn invalid_input_schema_tool_is_excluded_from_tools_list() {
    let server = start_2026_server().await;
    mount_2026_result(
        &server,
        "tools/list",
        serde_json::json!({
            "resultType": "complete", "ttlMs": 0, "cacheScope": "public",
            "tools": [
                {
                    "name": "good_tool",
                    "inputSchema": { "type": "object", "properties": {
                        "name": { "type": "string" }
                    }}
                },
                {
                    "name": "bad_schema_tool",
                    // "type": 123 fails 2020-12 meta-validation.
                    "inputSchema": { "type": "object", "properties": {
                        "bad": { "type": 123 }
                    }}
                }
            ]
        }),
    )
    .await;

    let client = connect_2026(&server).await;
    let tools = client.list_tools().await.expect("tools/list");
    let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
    assert!(
        names.contains(&"good_tool"),
        "a well-formed inputSchema must survive: {names:?}"
    );
    assert!(
        !names.contains(&"bad_schema_tool"),
        "an invalid inputSchema MUST exclude the tool: {names:?}"
    );
}
