//! # 2026-07-28 Streamable HTTP Client
//!
//! The client half of the 2026 stateless pair — its corresponding server
//! example is `minimal-server` (run `cargo run -p minimal-server` first,
//! then `cargo run -p streamable-http-client`).
//!
//! What this demonstrates on a 2026-07-28 connection:
//! - `connect()` negotiation: the bilingual client probes `server/discover`
//!   and locks the wire spec per connection (no `initialize` handshake, no
//!   `Mcp-Session-Id` — every request carries its own `_meta` and the
//!   SEP-2243 `Mcp-Method`/`Mcp-Name` headers, added by the transport).
//! - The retained discover body: server capabilities, instructions,
//!   supported versions.
//! - `list_tools` / `call_tool`.
//! - `call_tool_with_progress`: the request-scoped progress API. NOTE:
//!   minimal-server's `echo` emits no progress events, so this shows the
//!   API shape only — a tool opts in via
//!   `SessionContext::notify_request_progress`.
//! - `subscriptions_listen`: the ack-first long-lived POST stream that
//!   replaces 2025's GET SSE. Against minimal-server this shows the
//!   acknowledgement only; point it at `notification-server` (port 8005)
//!   to watch real `*/list_changed` and `resources/updated` deliveries.

use turul_mcp_client::transport::HttpTransport;
use turul_mcp_client::{McpClient, McpVersion};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let url = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "http://127.0.0.1:8641/mcp".to_string());
    println!("Connecting to {url} (start it with: cargo run -p minimal-server)");

    let transport = Box::new(HttpTransport::new(&url)?);
    let client = McpClient::new(transport, Default::default());

    // One call negotiates the wire spec for the connection's lifetime.
    client.connect().await?;
    let version = client.negotiated_version().await;
    println!("Negotiated protocol: {version:?}");
    if version != Some(McpVersion::V2026_07_28) {
        println!("Peer is not a 2026-07-28 server — the bilingual client fell back.");
        println!("This example pairs with a 2026 server such as minimal-server.");
        return Ok(());
    }

    // The server/discover body is retained for the connection.
    if let Some(discovered) = client.discovered_server().await {
        println!("Server: {:?}", discovered.server_info);
        println!("Supported versions: {:?}", discovered.supported_versions);
        if let Some(instructions) = &discovered.instructions {
            println!("Instructions: {instructions}");
        }
        if let Some(caps) = &discovered.capabilities {
            println!("Capabilities: {caps}");
        }
    }

    // Stateless operations — no session, no handshake, just requests.
    let tools = client.list_tools().await?;
    println!("\nTools ({}):", tools.len());
    for tool in &tools {
        println!(
            "  - {} — {}",
            tool.name,
            tool.description.as_deref().unwrap_or("")
        );
    }

    // minimal-server's `echo` tool takes a `text` argument (its inputSchema
    // is in the tools/list result above).
    if tools.iter().any(|t| t.name == "echo") {
        let args = serde_json::json!({ "text": "hello from the 2026 client" });
        let result = client.call_tool("echo", args.clone()).await?;
        println!("\ncall_tool(echo) → {}", serde_json::to_string(&result)?);

        // Request-scoped progress: the token rides _meta.progressToken and
        // notifications/progress arrive on this request's own SSE stream.
        // (echo emits none — tools opt in via notify_request_progress; the
        // feed is shown here so the API shape is visible.)
        let result = client
            .call_tool_with_progress("echo", args, serde_json::json!("demo-token"), |p| {
                println!("progress: {p}");
            })
            .await?;
        println!(
            "call_tool_with_progress(echo) → {}",
            serde_json::to_string(&result)?
        );
    } else {
        println!("(no `echo` tool — point this client at minimal-server for the full demo)");
    }

    // subscriptions/listen: the ack-first long-lived POST stream that replaces
    // 2025's GET SSE. The server's first frame is the acknowledgement (the
    // honored filter subset + subscriptionId); change notifications then
    // arrive on this same stream. Dropping the stream cancels it — clients
    // re-issue after a drop, they don't resume.
    let filter = serde_json::json!({
        "toolsListChanged": true,
        "resourcesListChanged": true,
        "resourceSubscriptions": ["file:///watched.txt"]
    });
    let mut stream = client.subscriptions_listen(filter).await?;
    println!("\nsubscriptions/listen acknowledged:");
    println!("  honored filter: {}", stream.honored);
    println!("  subscriptionId: {:?}", stream.subscription_id);

    if tools.iter().any(|t| t.name == "trigger_changes") {
        // Paired with notification-server: fire its broadcast tool and watch
        // the deliveries land on the listen stream.
        client
            .call_tool("trigger_changes", serde_json::json!({}))
            .await?;
        println!("called trigger_changes — draining notifications:");
        while let Ok(Some(n)) =
            tokio::time::timeout(std::time::Duration::from_secs(2), stream.next()).await
        {
            println!(
                "  ← {}",
                n.get("method").and_then(|m| m.as_str()).unwrap_or("?")
            );
        }
    } else {
        println!("(no broadcast source here — run notification-server on port 8005 and");
        println!(" point this client at it to watch list_changed deliveries live)");
    }
    drop(stream); // closing the stream IS the unsubscribe

    println!("\nDone — no session to clean up: the 2026 core is stateless.");
    Ok(())
}
