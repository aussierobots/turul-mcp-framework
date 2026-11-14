//! ADR-034: `McpServerBuilder::listener` serves on a socket the caller has
//! already bound, so the port is never released between choosing it and the
//! server accepting on it.

#[tokio::test]
async fn mcp_server_listener_serves_on_the_given_socket() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind ephemeral");
    let addr = listener.local_addr().expect("local_addr");

    let server = turul_mcp_server::McpServer::builder()
        .name("prebound-listener-test")
        .listener(listener)
        .build()
        .expect("build server");
    tokio::spawn(async move {
        server.run().await.ok();
    });

    // The socket was listening before `run` started, so the connection queues
    // in the backlog and is answered once the accept loop runs: no retry loop.
    let resp = reqwest::Client::new()
        .get(format!("http://{addr}/mcp"))
        .send()
        .await
        .expect("the pre-bound socket accepts without a readiness wait");
    assert!(
        resp.status().is_client_error() || resp.status().is_success(),
        "an HTTP answer from the MCP endpoint, got {}",
        resp.status()
    );
}
