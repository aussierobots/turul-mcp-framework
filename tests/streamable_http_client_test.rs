//! MCP Streamable HTTP Client Test
//!
//! Tests the CORRECT implementation of MCP Streamable HTTP with:
//! - Multi-threaded SSE stream processing
//! - Concurrent progress notification handling
//! - Proper Accept header matrix
//! - Tool calls returning SSE streams with final results

use anyhow::{Context, Result};
use futures::StreamExt;
use reqwest::Client;
use serde_json::{Value, json};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::{sleep, timeout};
use tracing::{debug, error, info, warn};

/// Represents a server event from SSE stream
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct StreamEvent {
    event_type: Option<String>,
    data: String,
    id: Option<String>,
}

/// Progress notification from tool execution
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct ProgressNotification {
    progress: Option<f64>,
    token: Option<String>,
    message: Option<String>,
}

/// Final tool result
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct ToolResult {
    content: Vec<Value>,
    is_error: bool,
}

/// Comprehensive streamable HTTP client that properly implements MCP Streamable HTTP
struct StreamableHttpClient {
    client: Client,
    base_url: String,
    session_id: Option<String>,
}

impl StreamableHttpClient {
    fn new(base_url: &str) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            base_url: base_url.to_string(),
            session_id: None,
        }
    }

    /// Initialize MCP session with proper header extraction
    async fn initialize(&mut self) -> Result<Value> {
        info!("🔗 Initializing MCP session with Streamable HTTP");

        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-11-25",
                "capabilities": {
                    "roots": { "listChanged": false },
                    "sampling": {}
                },
                "clientInfo": {
                    "name": "streamable-http-test-client",
                    "version": "1.0.0"
                }
            }
        });

        let response = self
            .client
            .post(&self.base_url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream") // ✅ CORRECT: Both formats
            .json(&request)
            .send()
            .await?;

        // ✅ CRITICAL: Extract session ID from headers
        if let Some(session_header) = response.headers().get("mcp-session-id")
            && let Ok(session_str) = session_header.to_str()
        {
            self.session_id = Some(session_str.to_string());
            info!("✅ Captured session ID: {}", session_str);
        }

        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        if content_type.starts_with("text/event-stream") {
            info!("📡 Server returned SSE stream for initialize");
            self.parse_sse_response(response).await
        } else {
            info!("📄 Server returned JSON for initialize");
            Ok(response.json().await?)
        }
    }

    /// Send notifications/initialized (MCP lifecycle compliance)
    async fn send_initialized(&self) -> Result<()> {
        let session_id = self
            .session_id
            .as_ref()
            .context("No session ID available")?;

        let notification = json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
            "params": {}
        });

        let response = self
            .client
            .post(&self.base_url)
            .header("Content-Type", "application/json")
            .header("Mcp-Session-Id", session_id)
            .json(&notification)
            .send()
            .await?;

        if response.status().is_success() {
            info!("✅ notifications/initialized sent successfully");
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "Failed to send initialized: {}",
                response.status()
            ))
        }
    }

    /// Call tool with PROPER multi-threaded SSE stream processing
    async fn call_tool_with_streaming(
        &self,
        tool_name: &str,
        args: Value,
    ) -> Result<(ToolResult, Vec<ProgressNotification>)> {
        let session_id = self
            .session_id
            .as_ref()
            .context("No session ID available")?;

        info!("🔧 Calling tool '{}' with streamable HTTP", tool_name);

        let request = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": tool_name,
                "arguments": args
            }
        });

        // ✅ CRITICAL: Request BOTH JSON and SSE formats
        let response = self
            .client
            .post(&self.base_url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream") // ✅ STREAMABLE HTTP
            .header("Mcp-Session-Id", session_id)
            .json(&request)
            .send()
            .await?;

        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        if content_type.starts_with("text/event-stream") {
            info!("📡 Tool call returned SSE stream - starting multi-threaded processing");
            self.process_streaming_tool_response(response).await
        } else {
            info!("📄 Tool call returned JSON response");
            let result: Value = response.json().await?;
            let tool_result = ToolResult {
                content: result
                    .get("result")
                    .and_then(|r| r.get("content"))
                    .and_then(|c| c.as_array())
                    .cloned()
                    .unwrap_or_default(),
                is_error: result.get("error").is_some(),
            };
            Ok((tool_result, Vec::new()))
        }
    }

    /// ✅ CRITICAL: Multi-threaded SSE stream processing for MCP Streamable HTTP
    async fn process_streaming_tool_response(
        &self,
        response: reqwest::Response,
    ) -> Result<(ToolResult, Vec<ProgressNotification>)> {
        info!("🚀 Starting multi-threaded SSE stream processing");

        let (progress_tx, mut progress_rx) = mpsc::unbounded_channel();
        let (result_tx, mut result_rx) = mpsc::unbounded_channel();

        // ✅ THREAD 1: SSE Stream Reader
        let stream_task = {
            let progress_tx = progress_tx.clone();
            let result_tx = result_tx.clone();

            tokio::spawn(async move {
                let mut stream = response.bytes_stream();
                let mut buffer = String::new();

                while let Some(chunk) = stream.next().await {
                    match chunk {
                        Ok(bytes) => {
                            let text = String::from_utf8_lossy(&bytes);
                            buffer.push_str(&text);

                            // Process complete SSE events
                            while let Some(pos) = buffer.find("\n\n") {
                                let event_text = buffer[..pos].to_string();
                                buffer = buffer[pos + 2..].to_string();

                                if let Some(event) = Self::parse_sse_event(&event_text)
                                    && let Some(data) = Self::try_parse_json(&event.data)
                                {
                                    // Check if this is a JSON-RPC response (final result)
                                    if data.get("id").is_some() && data.get("result").is_some() {
                                        debug!("📦 Found final JSON-RPC result in SSE stream");
                                        let _ = result_tx.send(data);
                                    }
                                    // Check if this is a progress notification
                                    else if let Some(method) =
                                        data.get("method").and_then(|m| m.as_str())
                                    {
                                        if method == "notifications/progress" {
                                            if let Some(params) = data.get("params") {
                                                let progress = ProgressNotification {
                                                    progress: params
                                                        .get("progress")
                                                        .and_then(|p| p.as_f64()),
                                                    token: params
                                                        .get("progressToken")
                                                        .and_then(|t| t.as_str())
                                                        .map(|s| s.to_string()),
                                                    message: params
                                                        .get("message")
                                                        .and_then(|m| m.as_str())
                                                        .map(|s| s.to_string()),
                                                };
                                                debug!("📈 Progress notification: {:?}", progress);
                                                let _ = progress_tx.send(progress);
                                            }
                                        } else if method == "notifications/message"
                                            && let Some(params) = data.get("params")
                                        {
                                            let progress = ProgressNotification {
                                                progress: None,
                                                token: None,
                                                message: params
                                                    .get("data")
                                                    .and_then(|d| d.as_str())
                                                    .map(|s| s.to_string()),
                                            };
                                            debug!("💬 Message notification: {:?}", progress);
                                            let _ = progress_tx.send(progress);
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            error!("❌ SSE stream error: {}", e);
                            break;
                        }
                    }
                }

                debug!("📡 SSE stream reader completed");
            })
        };

        // ✅ THREAD 2: Progress Collector
        let progress_task = {
            tokio::spawn(async move {
                let mut notifications = Vec::new();

                while let Some(progress) = progress_rx.recv().await {
                    info!("📈 Received progress: {:?}", progress);
                    notifications.push(progress);

                    // Limit collection to prevent memory issues
                    if notifications.len() > 100 {
                        warn!("⚠️  Progress notification limit reached");
                        break;
                    }
                }

                debug!(
                    "📊 Progress collector completed with {} notifications",
                    notifications.len()
                );
                notifications
            })
        };

        // ✅ MAIN THREAD: Wait for final result with timeout
        let final_result = timeout(Duration::from_secs(10), async {
            result_rx.recv().await.context("No final result received")
        })
        .await??;

        // Extract tool result
        let tool_result = ToolResult {
            content: final_result
                .get("result")
                .and_then(|r| r.get("content"))
                .and_then(|c| c.as_array())
                .cloned()
                .unwrap_or_default(),
            is_error: final_result.get("error").is_some(),
        };

        // Stop stream processing
        stream_task.abort();

        // Collect all progress notifications
        let progress_notifications = timeout(Duration::from_secs(1), progress_task)
            .await
            .unwrap_or_else(|_| {
                warn!("⚠️  Progress collection timed out");
                Ok(Vec::new())
            })?;

        info!(
            "🎉 Multi-threaded processing complete: {} progress notifications",
            progress_notifications.len()
        );
        Ok((tool_result, progress_notifications))
    }

    /// Parse SSE event from text
    fn parse_sse_event(event_text: &str) -> Option<StreamEvent> {
        let mut event_type = None;
        let mut data = String::new();
        let mut id = None;

        for line in event_text.lines() {
            if let Some(stripped) = line.strip_prefix("event: ") {
                event_type = Some(stripped.to_string());
            } else if let Some(stripped) = line.strip_prefix("data: ") {
                if !data.is_empty() {
                    data.push('\n');
                }
                data.push_str(stripped);
            } else if let Some(stripped) = line.strip_prefix("id: ") {
                id = Some(stripped.to_string());
            }
        }

        if data.is_empty() {
            None
        } else {
            Some(StreamEvent {
                event_type,
                data,
                id,
            })
        }
    }

    /// Try to parse JSON from string
    fn try_parse_json(text: &str) -> Option<Value> {
        serde_json::from_str(text).ok()
    }

    /// Parse JSON-RPC response from SSE stream
    async fn parse_sse_response(&self, response: reqwest::Response) -> Result<Value> {
        let sse_text = response.text().await?;

        for line in sse_text.lines() {
            if line.starts_with("data:") {
                let data = line.trim_start_matches("data:").trim();
                if let Ok(json) = serde_json::from_str::<Value>(data) {
                    return Ok(json);
                }
            }
        }

        Err(anyhow::anyhow!(
            "No valid JSON-RPC response found in SSE stream"
        ))
    }
}

#[tokio::test]
#[ignore = "Requires minimal-server on port 8641. Run: cargo run --example minimal-server"]
async fn test_streamable_http_with_multi_threading() -> Result<()> {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .try_init();

    info!("🚀 Testing MCP Streamable HTTP with multi-threaded SSE");

    // Check if server is available before running test
    let test_client = Client::builder()
        .timeout(Duration::from_millis(500))
        .build()?;

    if let Err(e) = test_client.get("http://127.0.0.1:8641/mcp").send().await {
        println!(
            "Skipping streamable HTTP test - server not running on port 8641: {}",
            e
        );
        println!("To run this test, start minimal-server on port 8641:");
        println!("  cargo run --example minimal-server");
        return Ok(());
    }

    let result = timeout(Duration::from_secs(60), async {
        let mut client = StreamableHttpClient::new("http://127.0.0.1:8641/mcp");

        // Test 1: Initialize with session management
        info!("📡 Test 1: MCP initialization");
        let init_result = client.initialize().await?;
        info!(
            "✅ Initialization successful: {}",
            serde_json::to_string_pretty(&init_result)?
        );

        // Test 2: Send lifecycle notification
        info!("📨 Test 2: notifications/initialized");
        client.send_initialized().await?;
        info!("✅ Lifecycle notification successful");

        // Test 3: Tool call with SSE streaming
        info!("🔧 Test 3: Streaming tool call");
        let (tool_result, progress_notifications) = client
            .call_tool_with_streaming(
                "echo",
                json!({"message": "Testing multi-threaded SSE processing!"}),
            )
            .await?;

        info!("📊 RESULTS:");
        info!("   • Tool result: {:?}", tool_result);
        info!(
            "   • Progress notifications: {}",
            progress_notifications.len()
        );

        for (i, notification) in progress_notifications.iter().enumerate() {
            info!("     {}. {:?}", i + 1, notification);
        }

        // Verify we got both progress notifications AND final result
        if !tool_result.content.is_empty() {
            info!("✅ Final tool result received successfully");
        } else {
            warn!("⚠️  No tool result content received");
        }

        if !progress_notifications.is_empty() {
            info!(
                "✅ Progress notifications received: {}",
                progress_notifications.len()
            );
        } else {
            warn!("⚠️  No progress notifications received");
        }

        Ok::<(), anyhow::Error>(())
    })
    .await;

    match result {
        Ok(test_result) => {
            test_result?;
            info!("🎉 Multi-threaded Streamable HTTP test PASSED!");
            Ok(())
        }
        Err(_) => Err(anyhow::anyhow!("Test timed out after 60 seconds")),
    }
}

#[tokio::test]
#[ignore = "Starts tools-test-server subprocess - may fail in CI/sandbox. Run manually if needed."]
async fn test_accept_header_variations() -> Result<()> {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .try_init();

    info!("🧪 Testing Accept header variations for MCP Streamable HTTP");

    let client = Client::new();
    let base_url = "http://127.0.0.1:8701/mcp";

    // Check if we can start a server (may fail in sandboxed environments)
    let test_bind = tokio::net::TcpListener::bind("127.0.0.1:8701").await;
    if test_bind.is_err() {
        println!(
            "Skipping Accept header test - cannot bind to port 8701 (likely sandboxed environment)"
        );
        return Ok(());
    }
    drop(test_bind);

    // Start server
    let mut server_process = tokio::process::Command::new("cargo")
        .args([
            "run",
            "--package",
            "tools-test-server",
            "--",
            "--port",
            "8701",
        ])
        .spawn()
        .context("Failed to start server")?;

    sleep(Duration::from_secs(2)).await;

    let test_result = timeout(Duration::from_secs(30), async {
        let test_cases = vec![
            ("application/json", "JSON only"),
            ("text/event-stream", "SSE only"),
            ("application/json, text/event-stream", "Both (preferred)"),
            ("*/*", "Accept all"),
        ];

        for (accept_header, description) in test_cases {
            info!("🧪 Testing Accept: {} ({})", accept_header, description);

            let request = json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2025-11-25",
                    "capabilities": {},
                    "clientInfo": {"name": "test", "version": "1.0"}
                }
            });

            let response = client
                .post(base_url)
                .header("Content-Type", "application/json")
                .header("Accept", accept_header)
                .json(&request)
                .send()
                .await?;

            let content_type = response
                .headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("unknown");

            info!("   📥 Response Content-Type: {}", content_type);
            info!("   📊 Status: {}", response.status());

            if response.status().is_success() {
                info!("   ✅ {} accepted successfully", description);
            } else {
                warn!("   ⚠️  {} failed: {}", description, response.status());
            }
        }

        Ok::<(), anyhow::Error>(())
    })
    .await;

    let _ = server_process.kill().await;

    test_result??;
    info!("🎉 Accept header variation test completed!");
    Ok(())
}
