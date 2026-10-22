//! End-to-end: a connected bilingual `McpClient` keeps working when the server
//! behind its URL changes spec revision (a rolling upgrade or a rollback).
//!
//! Real servers on both sides, no stubs. This package is pinned to the
//! 2025-11-25 lane, so that server runs in-process; the 2026-07-28 server is
//! the `minimal-server` example built for the default lane into its own target
//! directory and run as a child process (the two lanes cannot share a build).
//! A small TCP proxy stands in for the load balancer: the client only ever
//! sees the proxy's address, and `switch` repoints it and drops open
//! connections, as a deploy does.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;

use serde_json::json;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;
use turul_mcp_client::config::ClientConfig;
use turul_mcp_client::transport::http::HttpTransport;
use turul_mcp_client::{McpClient, McpVersion};
use turul_mcp_derive::McpTool;
use turul_mcp_server::prelude::*;

#[derive(McpTool, Clone, Default)]
#[tool(name = "echo", description = "Echo back the input text", output = String)]
struct EchoTool {
    #[param(description = "Text to echo back")]
    text: String,
}

impl EchoTool {
    async fn execute(
        &self,
        _session: Option<turul_mcp_server::SessionContext>,
    ) -> McpResult<String> {
        Ok(format!("Echo: {}", self.text))
    }
}

fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

async fn wait_listening(addr: SocketAddr) {
    for _ in 0..200 {
        if TcpStream::connect(addr).await.is_ok() {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    panic!("nothing listening on {addr}");
}

async fn start_2025_server() -> SocketAddr {
    let addr: SocketAddr = format!("127.0.0.1:{}", free_port()).parse().unwrap();
    let server = McpServer::builder()
        .name("upgrade-e2e-2025")
        .version("0.4.0")
        .tool(EchoTool::default())
        .bind_address(addr)
        .build()
        .expect("build 2025-11-25 server");
    tokio::spawn(async move {
        server.run().await.ok();
    });
    wait_listening(addr).await;
    addr
}

/// The `minimal-server` example, built for the default (2026-07-28) lane into
/// a dedicated target directory so it never clobbers or reuses this package's
/// 2025 artifacts.
struct Server2026 {
    child: Child,
    addr: SocketAddr,
}

impl Drop for Server2026 {
    fn drop(&mut self) {
        self.child.kill().ok();
        self.child.wait().ok();
    }
}

async fn start_2026_server() -> Server2026 {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let target = root.join("target-upgrade-e2e");
    let built = Command::new("cargo")
        .args(["build", "-p", "minimal-server", "--bin", "minimal-server"])
        .env("CARGO_TARGET_DIR", &target)
        .current_dir(&root)
        .status()
        .expect("run cargo build for the 2026 server");
    assert!(built.success(), "building the 2026 minimal-server failed");

    let port = free_port();
    let child = Command::new(target.join("debug").join("minimal-server"))
        .args(["--port", &port.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn the 2026 minimal-server");
    let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
    wait_listening(addr).await;
    Server2026 { child, addr }
}

/// A TCP forwarder with a switchable backend. `switch` repoints it and cuts
/// every open connection so the client's next request dials the new backend.
struct Proxy {
    addr: SocketAddr,
    backend: Arc<tokio::sync::Mutex<SocketAddr>>,
    cut: watch::Sender<u64>,
}

impl Proxy {
    async fn start(initial: SocketAddr) -> Proxy {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let backend = Arc::new(tokio::sync::Mutex::new(initial));
        let (cut, _) = watch::channel(0u64);

        let accept_backend = backend.clone();
        let accept_cut = cut.clone();
        tokio::spawn(async move {
            loop {
                let Ok((mut inbound, _)) = listener.accept().await else {
                    return;
                };
                let target = *accept_backend.lock().await;
                let mut cut_rx = accept_cut.subscribe();
                tokio::spawn(async move {
                    let Ok(mut outbound) = TcpStream::connect(target).await else {
                        return;
                    };
                    tokio::select! {
                        _ = tokio::io::copy_bidirectional(&mut inbound, &mut outbound) => {}
                        _ = cut_rx.changed() => {}
                    }
                });
            }
        });
        Proxy { addr, backend, cut }
    }

    async fn switch(&self, to: SocketAddr) {
        *self.backend.lock().await = to;
        self.cut.send_modify(|n| *n += 1);
    }

    fn url(&self) -> String {
        format!("http://{}/mcp", self.addr)
    }
}

async fn connect(url: &str) -> McpClient {
    let client = McpClient::new(
        Box::new(HttpTransport::new(url).unwrap()),
        ClientConfig::default(),
    );
    client.connect().await.expect("connect");
    client
}

async fn echo(client: &McpClient, text: &str) -> turul_mcp_client::McpClientResult<String> {
    client
        .call_tool("echo", json!({ "text": text }))
        .await
        .map(|r| serde_json::to_string(&r).unwrap_or_default())
}

/// Call across the flip. The in-flight request is allowed to fail while the
/// flip settles: with `-32022` (built for the old spec; the client
/// re-negotiates), or with a transport error when the proxy has just cut the
/// pooled connection. The connection must then be usable without a manual
/// reconnect.
async fn echo_after_flip(client: &McpClient, text: &str) -> String {
    for _ in 0..3 {
        match echo(client, text).await {
            Ok(out) => return out,
            Err(e)
                if e.is_unsupported_protocol_version()
                    || e.is_missing_session_id()
                    || matches!(e, turul_mcp_client::McpClientError::Transport(_)) => {}
            Err(e) => panic!("unexpected failure across the flip: {e:?}"),
        }
    }
    panic!("the client did not recover across the flip within 3 calls");
}

#[tokio::test]
async fn live_client_survives_a_real_server_upgrade_2025_to_2026() {
    let old = start_2025_server().await;
    let new = start_2026_server().await;
    let proxy = Proxy::start(old).await;

    let client = connect(&proxy.url()).await;
    assert_eq!(
        client.negotiated_version().await,
        Some(McpVersion::V2025_11_25)
    );
    assert!(
        echo(&client, "before")
            .await
            .unwrap()
            .contains("Echo: before")
    );

    proxy.switch(new.addr).await;

    let out = echo_after_flip(&client, "after").await;
    assert!(out.contains("Echo: after"), "{out}");
    assert_eq!(
        client.negotiated_version().await,
        Some(McpVersion::V2026_07_28),
        "the client must have re-negotiated onto the upgraded server"
    );
}

#[tokio::test]
async fn live_client_survives_a_real_server_rollback_2026_to_2025() {
    let old = start_2025_server().await;
    let new = start_2026_server().await;
    let proxy = Proxy::start(new.addr).await;

    let client = connect(&proxy.url()).await;
    assert_eq!(
        client.negotiated_version().await,
        Some(McpVersion::V2026_07_28)
    );
    assert!(
        echo(&client, "before")
            .await
            .unwrap()
            .contains("Echo: before")
    );

    proxy.switch(old).await;

    let out = echo_after_flip(&client, "after").await;
    assert!(out.contains("Echo: after"), "{out}");
    assert_eq!(
        client.negotiated_version().await,
        Some(McpVersion::V2025_11_25),
        "the client must have re-negotiated onto the rolled-back server"
    );
}
