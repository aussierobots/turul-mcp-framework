//! # Dynamic Tools Test Client
//!
//! Uses `turul-mcp-client` to test tool change detection end-to-end.
//!
//! ## Usage
//!
//! 1. Start the dynamic-tools-server:
//!    ```
//!    cargo run -p dynamic-tools-server
//!    ```
//!
//! 2. Run this test client:
//!    ```
//!    cargo run -p dynamic-tools-test-client
//!    ```
//!    Or with a custom URL:
//!    ```
//!    cargo run -p dynamic-tools-test-client -- http://127.0.0.1:9000/mcp
//!    ```

use tracing::info;
use turul_mcp_client::transport::TransportFactory;
use turul_mcp_client::{ClientConfig, McpClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let url = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "http://127.0.0.1:8484/mcp".to_string());

    info!("=== Dynamic Tools Test Client ===");
    info!("Connecting to: {}", url);

    let transport = TransportFactory::from_url(&url)?;
    let config = ClientConfig::default();
    let client = McpClient::new(transport, config);

    // Step 1: Connect and initialize
    info!("\n--- Step 1: Connect and initialize ---");
    client.connect().await?;
    info!("Connected successfully");

    // Step 2: List initial tools
    info!("\n--- Step 2: List initial tools ---");
    let tools = client.list_tools().await?;
    let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
    info!("Initial tools: {:?}", tool_names);

    assert!(
        tool_names.contains(&"multiply"),
        "FAIL: multiply should be in initial tool list"
    );
    assert!(
        tool_names.contains(&"add"),
        "FAIL: add should be in initial tool list"
    );
    info!("PASS: Initial tools correct");

    // Step 3: Deactivate multiply
    info!("\n--- Step 3: Call deactivate_multiply ---");
    let result = client
        .call_tool("deactivate_multiply", serde_json::json!({}))
        .await?;
    info!("Deactivate result: {:?}", result);
    info!("PASS: deactivate_multiply called successfully");

    // Step 4: Refresh and verify multiply is gone
    info!("\n--- Step 4: Verify multiply is gone ---");
    let tools = client.refresh_tools().await?;
    let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
    info!("Tools after deactivation: {:?}", tool_names);

    assert!(
        !tool_names.contains(&"multiply"),
        "FAIL: multiply should NOT be in tool list"
    );
    assert!(
        tool_names.contains(&"activate_multiply"),
        "FAIL: activate_multiply should be available"
    );
    assert!(
        tool_names.contains(&"add"),
        "FAIL: add should still be present"
    );
    info!("PASS: multiply removed, activate_multiply available");

    // Step 5: Reactivate multiply
    info!("\n--- Step 5: Call activate_multiply ---");
    let result = client
        .call_tool("activate_multiply", serde_json::json!({}))
        .await?;
    info!("Activate result: {:?}", result);
    info!("PASS: activate_multiply called successfully");

    // Step 6: Verify multiply is back
    info!("\n--- Step 6: Verify multiply is back ---");
    let tools = client.refresh_tools().await?;
    let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
    info!("Tools after reactivation: {:?}", tool_names);

    assert!(
        tool_names.contains(&"multiply"),
        "FAIL: multiply should be back"
    );
    assert!(
        tool_names.contains(&"deactivate_multiply"),
        "FAIL: deactivate_multiply should be available"
    );
    info!("PASS: multiply restored");

    info!("\n=== ALL TESTS PASSED ===");
    Ok(())
}
