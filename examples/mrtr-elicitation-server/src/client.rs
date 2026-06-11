//! # MRTR Elicitation Client (2026-07-28)
//!
//! The client leg of the MRTR round trip against `mrtr-elicitation-server`:
//!
//! 1. Declare the `elicitation` capability (servers MUST NOT demand inputs a
//!    client didn't declare — without it the call fails with `-32003`).
//! 2. Call `deploy_service` and catch `McpClientError::InputRequired`.
//! 3. "Ask the user" (hardcoded yes here), then retry the ORIGINAL request
//!    via `call_tool_with_input_responses`, echoing `requestState` verbatim.

use turul_mcp_client::error::McpClientError;
use turul_mcp_client::transport::HttpTransport;
use turul_mcp_client::{ClientConfig, McpClient, McpVersion};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let url = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "http://127.0.0.1:8642/mcp".to_string());
    println!("Connecting to {url} (start it with: cargo run -p mrtr-elicitation-server)");

    // Declare elicitation: this rides every request's _meta clientCapabilities.
    let mut config = ClientConfig::default();
    config.declared_capabilities.elicitation = true;

    let transport = Box::new(HttpTransport::new(&url)?);
    let client = McpClient::new(transport, config);
    client.connect().await?;

    if client.negotiated_version().await != Some(McpVersion::V2026_07_28) {
        println!("Peer is not a 2026-07-28 server — MRTR retries need the 2026 wire.");
        return Ok(());
    }

    let args = serde_json::json!({ "service": "billing-api" });

    // First leg: the server demands a confirmation instead of completing.
    println!("\n→ tools/call deploy_service (first leg)");
    let (input_requests, request_state) =
        match client.call_tool("deploy_service", args.clone()).await {
            Err(McpClientError::InputRequired {
                input_requests,
                request_state,
            }) => {
                println!("← input_required:");
                println!(
                    "  inputRequests: {}",
                    serde_json::to_string_pretty(&input_requests)?
                );
                println!("  requestState: {request_state:?}");
                (input_requests, request_state)
            }
            Ok(result) => {
                println!("← unexpected completion (no MRTR leg): {result:?}");
                return Ok(());
            }
            Err(e) => return Err(e.into()),
        };

    // "Ask the user". A real client renders the elicitation form from
    // inputRequests["confirm"].params; we answer yes.
    let message = input_requests
        .as_ref()
        .and_then(|r| r.get("confirm"))
        .and_then(|r| r.pointer("/params/message"))
        .and_then(|m| m.as_str())
        .unwrap_or("(no message)");
    println!("\nServer asks: {message}");
    println!("Answering: proceed = true");

    // Retry leg: the ORIGINAL request + inputResponses + verbatim requestState.
    let input_responses = serde_json::json!({
        "confirm": { "action": "accept", "content": { "proceed": true } }
    });
    println!("\n→ tools/call deploy_service (retry leg, with inputResponses)");
    let result = client
        .call_tool_with_input_responses("deploy_service", args, input_responses, request_state)
        .await?;
    println!("← {}", serde_json::to_string(&result)?);

    println!(
        "\nDone — the round trip used two POSTs to the same method; no session, no server push."
    );
    Ok(())
}
