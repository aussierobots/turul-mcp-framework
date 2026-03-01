// turul-mcp-client v0.3
// Custom configuration: client info, timeouts, retries, explicit transport.

use serde_json::json;
use std::time::Duration;
use turul_mcp_client::transport::http::HttpTransport;
use turul_mcp_client::{McpClientBuilder, McpClientResult};
use turul_mcp_client::config::*;

#[tokio::main]
async fn main() -> McpClientResult<()> {
    // Build a custom configuration
    let config = ClientConfig {
        client_info: ClientInfo {
            name: "my-dashboard".into(),
            version: "2.1.0".into(),
            description: Some("Dashboard MCP integration".into()),
            vendor: Some("Acme Corp".into()),
            metadata: None,
        },
        timeouts: TimeoutConfig {
            connect: Duration::from_secs(5),
            request: Duration::from_secs(15),
            long_operation: Duration::from_secs(120),
            initialization: Duration::from_secs(10),
            heartbeat: Duration::from_secs(20),
        },
        retry: RetryConfig {
            max_attempts: 5,
            initial_delay: Duration::from_millis(200),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 1.5,
            jitter: 0.2,
            exponential_backoff: true,
        },
        connection: ConnectionConfig::default(),
        logging: LoggingConfig {
            level: "debug".into(),
            log_requests: true,
            log_responses: true,
            log_transport: true,
            redact_sensitive: true,
        },
    };

    // Option A: Auto-detect transport with custom config
    let client = McpClientBuilder::new()
        .with_url("http://localhost:8080/mcp")?
        .with_config(config.clone())
        .build();

    client.connect().await?;

    // Check connection details
    let status = client.connection_status().await;
    println!("Transport: {:?}", status.transport_type);
    println!("Session ID: {:?}", status.session_id);
    println!("Protocol: {:?}", status.protocol_version);
    println!("Ready: {}", status.is_ready());

    let result = client.call_tool("add", json!({"a": 1, "b": 2})).await?;
    println!("Result: {result:?}");

    // Check transport statistics
    let stats = client.transport_stats().await;
    println!("Requests sent: {}", stats.requests_sent);
    println!("Avg response time: {:.1}ms", stats.avg_response_time_ms);

    client.disconnect().await?;

    // Option B: Explicit transport construction
    let transport = HttpTransport::new("http://localhost:9090/mcp")?;
    let client = McpClientBuilder::new()
        .with_transport(Box::new(transport))
        .with_config(config)
        .build();

    client.connect().await?;
    // ... use client ...
    client.disconnect().await?;

    Ok(())
}
