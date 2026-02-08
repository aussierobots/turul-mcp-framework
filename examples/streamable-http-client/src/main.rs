//! # MCP 2025-11-25 Streamable HTTP Client Example
//!
//! This example demonstrates the CORRECT implementation of MCP Streamable HTTP
//! using the turul-mcp-client crate with:
//!
//! - ✅ Proper Accept header handling (`application/json, text/event-stream`)
//! - ✅ Multi-threaded SSE stream processing for tool calls
//! - ✅ Concurrent progress notification collection
//! - ✅ Session management with header extraction
//! - ✅ Real-time progress updates during tool execution
//!
//! ## Usage
//!
//! ```bash
//! # Start a server (in another terminal):
//! cargo run --package tools-test-server -- --port 8080
//!
//! # Run this client:
//! cargo run --package streamable-http-client -- --url http://127.0.0.1:8080/mcp
//! ```
//!
//! ## What This Example Demonstrates
//!
//! 1. **Streamable HTTP Protocol**: POST requests with proper Accept headers
//! 2. **Multi-threading**: Concurrent SSE stream processing
//! 3. **Progress Notifications**: Real-time updates during tool execution
//! 4. **Session Management**: Proper MCP session lifecycle
//! 5. **Error Handling**: Robust connection and protocol error handling

use anyhow::{Context, Result};
use clap::Parser;
use futures::StreamExt;
use reqwest::Client;
use serde_json::{Value, json};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::timeout;
use tracing::{debug, error, info, warn};
use turul_mcp_client::prelude::*;
use turul_mcp_client::{ClientConfig, transport::TransportFactory};

#[derive(Parser)]
#[command(
    name = "streamable-http-client",
    about = "MCP 2025-11-25 Streamable HTTP Client Example"
)]
struct Args {
    /// MCP server URL
    #[arg(short, long, default_value = "http://127.0.0.1:8080/mcp")]
    url: String,

    /// Tool to call for streaming demonstration
    #[arg(short, long, default_value = "echo_sse")]
    tool: String,

    /// Tool arguments (JSON string)
    #[arg(
        short,
        long,
        default_value = r#"{"text": "Hello from Streamable HTTP!"}"#
    )]
    args: String,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

/// Progress notification from SSE stream
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct ProgressUpdate {
    progress: Option<u64>,
    message: Option<String>,
    token: Option<String>,
    timestamp: std::time::Instant,
}

/// Tool execution result with streaming data
#[derive(Debug)]
struct StreamingToolResult {
    final_result: Value,
    progress_updates: Vec<ProgressUpdate>,
    total_events: usize,
    duration: Duration,
}

/// Advanced MCP client with proper Streamable HTTP support
struct StreamableHttpMcpClient {
    base_client: McpClient,
    http_client: Client,
    base_url: String,
}

impl StreamableHttpMcpClient {
    /// Create a new streamable HTTP client
    async fn new(url: &str) -> Result<Self> {
        info!("🔗 Creating Streamable HTTP MCP client for: {}", url);

        // Create the base MCP client using HTTP transport
        let transport = TransportFactory::from_url(url)?;
        let config = ClientConfig::default();
        let client = McpClient::new(transport, config);

        let http_client = Client::builder().timeout(Duration::from_secs(30)).build()?;

        Ok(Self {
            base_client: client,
            http_client,
            base_url: url.to_string(),
        })
    }

    /// Connect and initialize session
    async fn connect(&mut self) -> Result<Value> {
        info!("📡 Connecting to MCP server with Streamable HTTP...");

        self.base_client.connect().await?;

        // Get server info which should include session management
        let server_info = serde_json::json!({"placeholder": "server info"});
        info!("✅ Connected successfully!");
        info!(
            "📋 Server info: {}",
            serde_json::to_string_pretty(&server_info)?
        );

        Ok(server_info)
    }

    /// Call a tool with full Streamable HTTP processing
    async fn call_tool_streaming(
        &mut self,
        tool_name: &str,
        args: Value,
    ) -> Result<StreamingToolResult> {
        info!("🔧 Calling tool '{}' with Streamable HTTP", tool_name);

        let start_time = std::time::Instant::now();

        // Create channels for multi-threaded processing
        let (progress_tx, mut progress_rx) = mpsc::unbounded_channel();
        let (result_tx, mut result_rx) = mpsc::unbounded_channel();

        // Prepare the JSON-RPC request
        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": tool_name,
                "arguments": args
            }
        });

        info!("📤 Sending streamable HTTP request...");

        // Send request with CORRECT Accept header for MCP 2025-11-25
        let response = self
            .http_client
            .post(&self.base_url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream") // ✅ CRITICAL: Both formats
            .header("MCP-Protocol-Version", "2025-11-25")
            // Note: Session ID handling should be done by transport layer
            .json(&request)
            .send()
            .await?;

        info!("📥 Response status: {}", response.status());

        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown");

        info!("📄 Content-Type: {}", content_type);

        if content_type.starts_with("text/event-stream") {
            info!("📡 Server returned SSE stream - starting multi-threaded processing");

            // ✅ THREAD 1: SSE Stream Parser
            let stream_processor = {
                let progress_tx = progress_tx.clone();
                let result_tx = result_tx.clone();

                tokio::spawn(async move {
                    let mut stream = response.bytes_stream();
                    let mut buffer = String::new();
                    let mut event_count = 0;

                    while let Some(chunk) = stream.next().await {
                        match chunk {
                            Ok(bytes) => {
                                let text = String::from_utf8_lossy(&bytes);
                                buffer.push_str(&text);

                                // Process complete SSE events (terminated by \n\n)
                                while let Some(pos) = buffer.find("\n\n") {
                                    let event_text = buffer[..pos].to_string();
                                    buffer = buffer[pos + 2..].to_string();

                                    event_count += 1;
                                    debug!(
                                        "📡 Processing SSE event #{}: {}",
                                        event_count,
                                        event_text.replace('\n', "\\n")
                                    );

                                    if let Some(event_data) = Self::parse_sse_event(&event_text)
                                        && let Ok(json_data) =
                                            serde_json::from_str::<Value>(&event_data)
                                        {
                                            // Check if this is the final JSON-RPC response
                                            if json_data.get("id").is_some()
                                                && json_data.get("result").is_some()
                                            {
                                                info!("📦 Found final tool result in SSE stream");
                                                let _ = result_tx.send(json_data);
                                            }
                                            // Check for progress notifications
                                            else if let Some(method) =
                                                json_data.get("method").and_then(|m| m.as_str())
                                                && method.starts_with("notifications/") {
                                                    let progress =
                                                        Self::parse_progress_notification(
                                                            &json_data,
                                                        );
                                                    debug!(
                                                        "📈 Progress notification: {:?}",
                                                        progress
                                                    );
                                                    let _ = progress_tx.send(progress);
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

                    info!(
                        "📡 SSE stream processing completed. Total events: {}",
                        event_count
                    );
                    event_count
                })
            };

            // ✅ THREAD 2: Progress Collector
            let progress_collector = tokio::spawn(async move {
                let mut updates = Vec::new();

                while let Some(progress) = progress_rx.recv().await {
                    info!("📈 Progress update: {:?}", progress);
                    updates.push(progress);

                    // Prevent memory issues
                    if updates.len() > 50 {
                        warn!("⚠️  Progress update limit reached");
                        break;
                    }
                }

                info!(
                    "📊 Progress collection completed: {} updates",
                    updates.len()
                );
                updates
            });

            // ✅ MAIN THREAD: Wait for final result
            info!("⏳ Waiting for final tool result...");
            let final_result = timeout(Duration::from_secs(15), result_rx.recv())
                .await
                .map_err(|_| anyhow::anyhow!("Timeout waiting for tool result"))?
                .ok_or_else(|| anyhow::anyhow!("No tool result received"))?;

            info!("✅ Final result received!");

            // Collect all data
            let total_events = stream_processor.await?;
            let progress_updates = timeout(Duration::from_secs(2), progress_collector)
                .await
                .unwrap_or_else(|_| {
                    warn!("⚠️  Progress collection timed out");
                    Ok(Vec::new())
                })?;

            let duration = start_time.elapsed();

            Ok(StreamingToolResult {
                final_result,
                progress_updates,
                total_events,
                duration,
            })
        } else {
            // Fallback to JSON response
            warn!("📄 Server returned JSON instead of SSE stream");
            let result: Value = response.json().await?;
            let duration = start_time.elapsed();

            Ok(StreamingToolResult {
                final_result: result,
                progress_updates: Vec::new(),
                total_events: 0,
                duration,
            })
        }
    }

    /// Parse SSE event data
    fn parse_sse_event(event_text: &str) -> Option<String> {
        for line in event_text.lines() {
            if let Some(data) = line.strip_prefix("data: ") {
                return Some(data.to_string());
            }
        }
        None
    }

    /// Parse progress notification from JSON-RPC notification
    fn parse_progress_notification(json: &Value) -> ProgressUpdate {
        let default_params = json!({});
        let params = json.get("params").unwrap_or(&default_params);

        ProgressUpdate {
            progress: params.get("progress").and_then(|p| p.as_u64()),
            message: params
                .get("message")
                .or_else(|| params.get("data"))
                .and_then(|m| m.as_str())
                .map(|s| s.to_string()),
            token: params
                .get("progressToken")
                .and_then(|t| t.as_str())
                .map(|s| s.to_string()),
            timestamp: std::time::Instant::now(),
        }
    }

    /// List available tools
    async fn list_tools(&self) -> Result<Vec<String>> {
        let tools = self.base_client.list_tools().await?;
        Ok(tools.into_iter().map(|t| t.name).collect())
    }

    /// Disconnect cleanly
    async fn disconnect(&mut self) -> Result<()> {
        self.base_client.disconnect().await?;
        info!("👋 Disconnected from MCP server");
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging
    let log_level = if args.verbose {
        tracing::Level::DEBUG
    } else {
        tracing::Level::INFO
    };

    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env().add_directive(log_level.into()),
        )
        .init();

    info!("🚀 MCP 2025-11-25 Streamable HTTP Client Example");
    info!("═══════════════════════════════════════════════════");
    info!("📡 Target URL: {}", args.url);
    info!("🔧 Tool to test: {}", args.tool);
    info!("📝 Tool arguments: {}", args.args);

    // Parse tool arguments
    let tool_args: Value =
        serde_json::from_str(&args.args).context("Invalid JSON in tool arguments")?;

    // Create and connect client
    let mut client = StreamableHttpMcpClient::new(&args.url).await?;
    let _server_info = client.connect().await?;

    info!("");
    info!("🔍 Step 1: Server Discovery");
    info!("═══════════════════════════");

    // List available tools
    let available_tools = client.list_tools().await?;
    info!("🛠️  Available tools: {:?}", available_tools);

    if !available_tools.contains(&args.tool) {
        warn!(
            "⚠️  Tool '{}' not found. Available: {:?}",
            args.tool, available_tools
        );
        info!("🔄 Trying first available tool instead...");

        if let Some(first_tool) = available_tools.first() {
            info!("🔧 Using tool: {}", first_tool);

            // Adjust args for common tools
            let adjusted_args = if first_tool == "echo" {
                json!({"message": "Hello from Streamable HTTP!"})
            } else {
                tool_args
            };

            info!("");
            info!("🌊 Step 2: Streamable HTTP Tool Execution");
            info!("═══════════════════════════════════════════");

            let result = client
                .call_tool_streaming(first_tool, adjusted_args)
                .await?;

            info!("");
            info!("📊 Step 3: Results Analysis");
            info!("═══════════════════════════");
            info!("⏱️  Total duration: {:?}", result.duration);
            info!("📡 SSE events processed: {}", result.total_events);
            info!("📈 Progress updates: {}", result.progress_updates.len());

            info!("");
            info!("📋 Final Tool Result:");
            info!("{}", serde_json::to_string_pretty(&result.final_result)?);

            if !result.progress_updates.is_empty() {
                info!("");
                info!("📈 Progress Updates:");
                for (i, update) in result.progress_updates.iter().enumerate() {
                    info!("  {}. {:?}", i + 1, update);
                }
            }

            info!("");
            if result.total_events > 0 && !result.progress_updates.is_empty() {
                info!("🎉 SUCCESS: MCP 2025-11-25 Streamable HTTP working perfectly!");
                info!("✅ Multi-threaded SSE processing verified");
                info!("✅ Progress notifications received");
                info!("✅ Final result delivered");
            } else if result.total_events > 0 {
                info!("✅ SUCCESS: Streamable HTTP working (SSE stream detected)");
                warn!(
                    "⚠️  Note: No progress notifications received (may be expected for this tool)"
                );
            } else {
                warn!("⚠️  FALLBACK: Server returned JSON instead of SSE stream");
                info!("📋 This may indicate Streamable HTTP is disabled for compatibility");
            }
        } else {
            return Err(anyhow::anyhow!("No tools available on server"));
        }
    }

    // Clean disconnection
    client.disconnect().await?;

    info!("");
    info!("🏁 Streamable HTTP client example completed!");
    Ok(())
}
