// turul-mcp-client v0.3
// Error handling: match on McpClientError, retry with backoff, graceful degradation.

use serde_json::{json, Value};
use turul_mcp_client::config::RetryConfig;
use turul_mcp_client::error::{McpClientError, ProtocolError, SessionError, TransportError};
use turul_mcp_client::{McpClient, McpClientBuilder, McpClientResult};

#[tokio::main]
async fn main() -> McpClientResult<()> {
    let client = McpClientBuilder::new()
        .with_url("http://localhost:8080/mcp")?
        .build();

    // Connect with error classification
    if let Err(e) = client.connect().await {
        match &e {
            McpClientError::Transport(TransportError::ConnectionFailed(msg)) => {
                eprintln!("Cannot reach server: {msg}");
                return Err(e);
            }
            McpClientError::Protocol(ProtocolError::NegotiationFailed(msg)) => {
                eprintln!("Protocol mismatch: {msg}");
                return Err(e);
            }
            McpClientError::Timeout => {
                eprintln!("Connection timed out");
                return Err(e);
            }
            other => {
                eprintln!("Unexpected connect error: {other}");
                return Err(e);
            }
        }
    }

    // Call a tool with retry logic
    let result = call_tool_with_retry(&client, "add", json!({"a": 1, "b": 2})).await;
    match result {
        Ok(value) => println!("Success: {value:?}"),
        Err(e) => eprintln!("Failed after retries: {e}"),
    }

    // Handle specific server errors
    match client.call_tool("nonexistent", json!({})).await {
        Ok(result) => println!("{result:?}"),
        Err(McpClientError::ServerError { code, message, .. }) => {
            eprintln!("Server error {code}: {message}");
        }
        Err(McpClientError::Session(SessionError::Expired)) => {
            eprintln!("Session expired — reconnect needed");
            // In production: reconnect and retry
        }
        Err(e) => eprintln!("Error: {e}"),
    }

    client.disconnect().await?;
    Ok(())
}

/// Retry a tool call using built-in RetryConfig for backoff calculation.
async fn call_tool_with_retry(
    client: &McpClient,
    name: &str,
    args: Value,
) -> McpClientResult<Vec<turul_mcp_protocol::ToolResult>> {
    let retry = RetryConfig::default(); // max_attempts: 3, backoff: 2.0x

    for attempt in 0..retry.max_attempts {
        match client.call_tool(name, args.clone()).await {
            Ok(result) => return Ok(result),
            Err(e) if e.is_retryable() && retry.should_retry(attempt) => {
                let delay = retry.delay_for_attempt(attempt);
                eprintln!(
                    "Attempt {}/{} failed (retryable): {e} — retrying in {delay:?}",
                    attempt + 1,
                    retry.max_attempts
                );
                tokio::time::sleep(delay).await;
            }
            Err(e) => {
                // Non-retryable or max attempts reached
                eprintln!("Non-retryable error: {e}");
                if e.is_protocol_error() {
                    eprintln!("  → Protocol error (check server compatibility)");
                }
                if e.is_session_error() {
                    eprintln!("  → Session error (may need reconnect)");
                }
                if let Some(code) = e.error_code() {
                    eprintln!("  → Error code: {code}");
                }
                return Err(e);
            }
        }
    }

    Err(McpClientError::generic("max retries exceeded"))
}
