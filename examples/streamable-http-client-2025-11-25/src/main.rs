//! # MCP 2025-11-25 Streamable HTTP — raw wire client
//!
//! **Deliberately pinned to the 2025-11-25 lane.** Everything it shows —
//! the `initialize` handshake, the `Mcp-Session-Id` header carried on every
//! subsequent request, and `DELETE` session termination — was REMOVED by the
//! 2026-07-28 stateless core. For the 2026 equivalent see
//! `streamable-http-client`, which uses the high-level `McpClient`.
//!
//! This client speaks the wire directly with `reqwest` rather than through
//! `turul-mcp-client`, so every byte the spec requires is visible in one
//! file. That is the point: it is the reference for anyone debugging their
//! own 2025-11-25 Streamable HTTP implementation.
//!
//! What it demonstrates:
//!
//! 1. **Session lifecycle** — `initialize` → read `Mcp-Session-Id` from the
//!    RESPONSE HEADER → `notifications/initialized` (202) → header on every
//!    later request → `DELETE` to terminate.
//! 2. **Accept negotiation** — `application/json, text/event-stream` lets the
//!    server choose; a tool that emits notifications answers with
//!    `Content-Type: text/event-stream`.
//! 3. **Progress opt-in** — `params._meta.progressToken`. Without it the
//!    server has no token to echo and emits no progress.
//! 4. **Concurrent SSE processing** — one task parses the event stream while
//!    a second drains progress notifications, so updates are observed as
//!    they arrive instead of after the final result.
//!
//! ## Usage
//!
//! ```bash
//! # Terminal 1 — a 2025-11-25 server exposing the `echo_sse` tool:
//! cargo run -p client-initialise-server -- --port 52950
//!
//! # Terminal 2:
//! cargo run -p streamable-http-client-2025-11-25 -- --url http://127.0.0.1:52950/mcp
//! ```

use anyhow::{Context, Result, bail};
use clap::Parser;
use futures::StreamExt;
use reqwest::Client;
use serde_json::{Value, json};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

const PROTOCOL_VERSION: &str = "2025-11-25";
/// Opaque token the client picks; the server MUST echo it on every
/// `notifications/progress` for this request.
const PROGRESS_TOKEN: &str = "streamable-demo-1";

#[derive(Parser)]
#[command(
    name = "streamable-http-client-2025-11-25",
    about = "MCP 2025-11-25 Streamable HTTP raw-wire client"
)]
struct Args {
    /// MCP server URL
    #[arg(short, long, default_value = "http://127.0.0.1:52950/mcp")]
    url: String,

    /// Tool to call for the streaming demonstration
    #[arg(short, long, default_value = "echo_sse")]
    tool: String,

    /// Tool arguments (JSON object)
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

/// Progress notification observed on the SSE stream
#[derive(Debug, Clone)]
struct ProgressUpdate {
    progress: Option<f64>,
    message: Option<String>,
    token: Option<String>,
}

/// Outcome of one streamed `tools/call`
#[derive(Debug)]
struct StreamingToolResult {
    final_result: Value,
    progress_updates: Vec<ProgressUpdate>,
    total_events: usize,
    /// True when the server answered `Content-Type: text/event-stream`
    streamed: bool,
    duration: Duration,
}

/// Raw 2025-11-25 Streamable HTTP client: owns its session id and stamps it
/// on every request after `initialize`.
struct StreamableHttpMcpClient {
    http: Client,
    url: String,
    session_id: Option<String>,
    next_id: u64,
}

impl StreamableHttpMcpClient {
    fn new(url: &str) -> Result<Self> {
        Ok(Self {
            http: Client::builder().timeout(Duration::from_secs(30)).build()?,
            url: url.to_string(),
            session_id: None,
            next_id: 1,
        })
    }

    fn take_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// Apply the headers the spec requires on every request. `Mcp-Session-Id`
    /// is absent only on `initialize`, which is what mints it.
    fn request(&self, body: &Value) -> reqwest::RequestBuilder {
        let mut req = self
            .http
            .post(&self.url)
            .header("Content-Type", "application/json")
            // Let the server pick: JSON for plain results, SSE when the call
            // also carries notifications.
            .header("Accept", "application/json, text/event-stream")
            .header("MCP-Protocol-Version", PROTOCOL_VERSION);
        if let Some(sid) = &self.session_id {
            req = req.header("Mcp-Session-Id", sid);
        }
        req.json(body)
    }

    /// `initialize` → capture `Mcp-Session-Id` from the response HEADER, then
    /// `notifications/initialized` to enable the session.
    async fn connect(&mut self) -> Result<Value> {
        info!("📡 initialize (protocolVersion {PROTOCOL_VERSION})");

        let id = self.take_id();
        let response = self
            .request(&json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": "initialize",
                "params": {
                    "protocolVersion": PROTOCOL_VERSION,
                    "capabilities": {},
                    "clientInfo": { "name": "streamable-http-client-2025-11-25", "version": "0.4.0" }
                }
            }))
            .send()
            .await
            .context("initialize request failed")?;

        if !response.status().is_success() {
            bail!("initialize returned HTTP {}", response.status());
        }

        // The session id lives in the header, not the body.
        let session_id = response
            .headers()
            .get("mcp-session-id")
            .and_then(|v| v.to_str().ok())
            .map(str::to_string)
            .context("server did not return an Mcp-Session-Id header")?;
        info!("🔑 Mcp-Session-Id: {session_id}");
        self.session_id = Some(session_id);

        let body: Value = response.json().await.context("initialize body")?;
        if let Some(err) = body.get("error") {
            bail!("initialize failed: {err}");
        }
        let result = body.get("result").cloned().unwrap_or(Value::Null);

        // Until this lands the server rejects everything else on the session.
        let status = self
            .request(&json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }))
            .send()
            .await
            .context("notifications/initialized failed")?
            .status();
        if status != reqwest::StatusCode::ACCEPTED {
            warn!("⚠️  notifications/initialized returned {status}, expected 202 Accepted");
        } else {
            info!("✅ notifications/initialized accepted (202) — session enabled");
        }

        Ok(result)
    }

    async fn list_tools(&mut self) -> Result<Vec<String>> {
        let id = self.take_id();
        let body: Value = self
            .request(&json!({"jsonrpc":"2.0","id":id,"method":"tools/list","params":{}}))
            .send()
            .await
            .context("tools/list failed")?
            .json()
            .await?;
        if let Some(err) = body.get("error") {
            bail!("tools/list failed: {err}");
        }
        Ok(body
            .pointer("/result/tools")
            .and_then(Value::as_array)
            .map(|tools| {
                tools
                    .iter()
                    .filter_map(|t| t.get("name").and_then(Value::as_str))
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default())
    }

    /// Call a tool, opting into progress, and process the SSE response with
    /// two concurrent tasks.
    async fn call_tool_streaming(
        &mut self,
        tool_name: &str,
        args: Value,
    ) -> Result<StreamingToolResult> {
        info!("🔧 tools/call '{tool_name}' (progressToken {PROGRESS_TOKEN})");
        let start_time = std::time::Instant::now();

        let (progress_tx, mut progress_rx) = mpsc::unbounded_channel();
        let (result_tx, mut result_rx) = mpsc::unbounded_channel();

        let request_id = self.take_id();
        let response = self
            .request(&json!({
                "jsonrpc": "2.0",
                "id": request_id,
                "method": "tools/call",
                "params": {
                    "name": tool_name,
                    "arguments": args,
                    // Progress is opt-in: no token, no notifications/progress.
                    "_meta": { "progressToken": PROGRESS_TOKEN }
                }
            }))
            .send()
            .await
            .context("tools/call request failed")?;

        let status = response.status();
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown")
            .to_string();
        info!("📥 HTTP {status} • Content-Type: {content_type}");

        if !status.is_success() {
            let body: Value = response.json().await.unwrap_or(Value::Null);
            bail!("tools/call returned HTTP {status}: {body}");
        }

        if !content_type.starts_with("text/event-stream") {
            // Legitimate: the spec lets the server answer plain JSON when the
            // call carries no notifications.
            info!("📄 Server answered JSON (no notifications for this call)");
            let result: Value = response.json().await?;
            return Ok(StreamingToolResult {
                final_result: result,
                progress_updates: Vec::new(),
                total_events: 0,
                streamed: false,
                duration: start_time.elapsed(),
            });
        }

        info!("📡 SSE stream — starting concurrent processing");

        // Task 1: parse SSE frames, routing the final response one way and
        // notifications the other.
        let stream_processor = tokio::spawn(async move {
            let mut stream = response.bytes_stream();
            let mut buffer = String::new();
            let mut event_count = 0usize;

            while let Some(chunk) = stream.next().await {
                match chunk {
                    Ok(bytes) => {
                        buffer.push_str(&String::from_utf8_lossy(&bytes));
                        while let Some(pos) = buffer.find("\n\n") {
                            let event_text = buffer[..pos].to_string();
                            buffer = buffer[pos + 2..].to_string();
                            event_count += 1;
                            debug!(
                                "📡 SSE event #{event_count}: {}",
                                event_text.replace('\n', "\\n")
                            );

                            if let Some(event_data) = parse_sse_event(&event_text)
                                && let Ok(json_data) = serde_json::from_str::<Value>(&event_data)
                            {
                                if json_data.get("id").is_some()
                                    && (json_data.get("result").is_some()
                                        || json_data.get("error").is_some())
                                {
                                    info!("📦 Final JSON-RPC response received on the stream");
                                    let _ = result_tx.send(json_data);
                                } else if let Some(method) =
                                    json_data.get("method").and_then(Value::as_str)
                                    && method == "notifications/progress"
                                {
                                    let _ =
                                        progress_tx.send(parse_progress_notification(&json_data));
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("❌ SSE stream error: {e}");
                        break;
                    }
                }
            }
            info!("📡 SSE stream ended after {event_count} events");
            event_count
        });

        // Task 2: drain progress updates as they arrive.
        let progress_collector = tokio::spawn(async move {
            let mut updates = Vec::new();
            while let Some(progress) = progress_rx.recv().await {
                info!("📈 progress: {progress:?}");
                updates.push(progress);
                if updates.len() > 50 {
                    warn!("⚠️  progress update limit reached");
                    break;
                }
            }
            updates
        });

        let final_result = timeout(Duration::from_secs(15), result_rx.recv())
            .await
            .map_err(|_| anyhow::anyhow!("timed out waiting for the tool result"))?
            .ok_or_else(|| anyhow::anyhow!("stream closed before the tool result arrived"))?;

        let total_events = stream_processor.await?;
        let progress_updates = progress_collector.await?;

        Ok(StreamingToolResult {
            final_result,
            progress_updates,
            total_events,
            streamed: true,
            duration: start_time.elapsed(),
        })
    }

    /// `DELETE` with the session header terminates the session server-side.
    async fn disconnect(&mut self) -> Result<()> {
        let Some(session_id) = self.session_id.take() else {
            return Ok(());
        };
        let status = self
            .http
            .delete(&self.url)
            .header("MCP-Protocol-Version", PROTOCOL_VERSION)
            .header("Mcp-Session-Id", &session_id)
            .send()
            .await
            .context("session DELETE failed")?
            .status();
        info!("👋 DELETE session {session_id} → HTTP {status}");
        Ok(())
    }
}

fn parse_sse_event(event_text: &str) -> Option<String> {
    event_text
        .lines()
        .find_map(|line| line.strip_prefix("data: ").map(str::to_string))
}

fn parse_progress_notification(json: &Value) -> ProgressUpdate {
    let params = json.get("params").cloned().unwrap_or(Value::Null);
    ProgressUpdate {
        progress: params.get("progress").and_then(Value::as_f64),
        message: params
            .get("message")
            .and_then(Value::as_str)
            .map(str::to_string),
        token: params
            .get("progressToken")
            .and_then(Value::as_str)
            .map(str::to_string),
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

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

    info!("🚀 MCP {PROTOCOL_VERSION} Streamable HTTP — raw wire client");
    info!("📡 Target: {}", args.url);

    let tool_args: Value =
        serde_json::from_str(&args.args).context("--args must be a JSON object")?;

    let mut client = StreamableHttpMcpClient::new(&args.url)?;
    let init_result = client.connect().await?;
    if let Some(version) = init_result.get("protocolVersion").and_then(Value::as_str) {
        info!("🤝 Server negotiated protocolVersion: {version}");
        if version != PROTOCOL_VERSION {
            warn!("⚠️  Expected {PROTOCOL_VERSION} — this client only speaks that lane");
        }
    }

    info!("");
    info!("🔍 Step 1: tools/list");
    let available_tools = client.list_tools().await?;
    info!("🛠️  Tools: {available_tools:?}");

    let selected_tool = if available_tools.contains(&args.tool) {
        args.tool.clone()
    } else {
        warn!(
            "⚠️  '{}' not found; using the first tool instead",
            args.tool
        );
        available_tools
            .first()
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("server advertises no tools"))?
    };

    info!("");
    info!("🌊 Step 2: streamed tools/call — {selected_tool}");
    let result = client
        .call_tool_streaming(&selected_tool, tool_args)
        .await?;

    info!("");
    info!("📊 Step 3: results");
    info!("⏱️  Duration: {:?}", result.duration);
    info!("📡 SSE events: {}", result.total_events);
    info!("📈 Progress updates: {}", result.progress_updates.len());
    info!(
        "📋 Final result: {}",
        serde_json::to_string(&result.final_result)?
    );

    for (i, update) in result.progress_updates.iter().enumerate() {
        info!(
            "  {}. progress={:?} token={:?} message={:?}",
            i + 1,
            update.progress,
            update.token,
            update.message
        );
    }

    // One verdict for the run, evaluated after everything is collected.
    info!("");
    if !result.streamed {
        warn!("⚠️  Server answered JSON, not SSE — no stream to process");
    } else if result.progress_updates.is_empty() {
        info!("✅ Streamable HTTP verified (SSE stream + final result)");
        warn!("⚠️  No progress notifications — expected unless the tool emits them");
    } else {
        info!("🎉 Streamable HTTP verified end to end");
        info!("✅ Concurrent SSE processing");
        info!(
            "✅ {} progress notification(s)",
            result.progress_updates.len()
        );
        let echoed = result
            .progress_updates
            .iter()
            .all(|u| u.token.as_deref() == Some(PROGRESS_TOKEN));
        if echoed {
            info!("✅ Server echoed our progressToken '{PROGRESS_TOKEN}'");
        } else {
            // `ProgressNotificationParams.progressToken` is "the progress token
            // which was given in the initial request", and the spec's progress
            // pattern makes referencing any other token a MUST NOT. A probe that
            // detected that and still exited 0 would report a compliance failure
            // as a pass, so this is an error rather than a warning.
            let seen: Vec<String> = result
                .progress_updates
                .iter()
                .filter_map(|u| u.token.clone())
                .collect();
            error!("❌ Server did NOT echo progressToken '{PROGRESS_TOKEN}' — saw {seen:?}");
            client.disconnect().await.ok();
            anyhow::bail!(
                "progress notifications referenced {seen:?} instead of the request's token \
                 '{PROGRESS_TOKEN}'"
            );
        }
    }

    client.disconnect().await?;
    info!("🏁 Done");
    Ok(())
}
