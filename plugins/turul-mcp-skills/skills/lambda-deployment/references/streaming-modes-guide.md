# Lambda Streaming Modes Guide

Deep dive into snapshot vs real-time SSE streaming in Lambda MCP servers.

## The 4 Combinations

All 4 combinations of `.sse()` and handler method work — none hang or crash. The difference is capability:

| # | `.sse()` | Handler | Runtime | GET /mcp | POST /mcp SSE | Real-time? |
|---|---|---|---|---|---|---|
| 1 | `false` | `handle()` | `run()` | 405 | No SSE | No |
| 2 | `true` | `handle()` | `run()` | Snapshot events | Snapshot SSE | **No** (snapshots) |
| 3 | `false` | `handle_streaming()` | `run_streaming()` / `run_streaming_with()` | 405 | No SSE | No |
| 4 | `true` | `handle_streaming()` | `run_streaming()` / `run_streaming_with()` | **Real-time stream** | **Real-time SSE** | **Yes** |

**Combination 1**: Simplest. No SSE at all. Best for tools that return immediately.

**Combination 2**: SSE is enabled but `handle()` collects the response body into `LambdaBody` (buffered). You get a snapshot of accumulated events, not a real-time stream.

**Combination 3**: Streaming runtime available but SSE endpoints are disabled. `handle_streaming()` processes POST requests normally; GET /mcp returns 405.

**Combination 4**: Full real-time SSE. Requires the `streaming` Cargo feature + `run_streaming()` (or `run_streaming_with()`) + `handle_streaming()`.

## Protocol Version Routing

`handle_streaming()` routes requests based on protocol version:

```
Lambda Request
    │
    ▼
lambda_to_hyper_request()          ← Convert Lambda → hyper format
    │
    ▼
Check MCP-Protocol-Version header
    │
    ├─ >= 2025-03-26 ─→ StreamableHttpHandler  (chunked SSE, MCP 2025-11-25)
    └─ <= 2024-11-05 ─→ SessionMcpHandler      (buffered JSON, legacy)
    │
    ▼
hyper_to_lambda_streaming()        ← Preserve streaming body
    │
    ▼
Lambda Streaming Response
```

`handle()` always uses `SessionMcpHandler` (legacy path) and collects the body into `LambdaBody`. This means `handle()` does not produce MCP 2025-11-25 streamable HTTP responses even when clients send the new protocol version header. Use `handle_streaming()` if you need proper protocol version routing.

## Body Types

| Handler | Response Body Type | Behavior |
|---|---|---|
| `handle()` | `LambdaBody` (`Text`/`Binary`/`Empty`) | Fully buffered. Entire response collected before returning. |
| `handle_streaming()` | `UnsyncBoxBody<Bytes, hyper::Error>` | Streaming. Bytes flow as produced. |

The body type determines whether Lambda can stream the response to API Gateway. `LambdaBody` is a complete payload; `UnsyncBoxBody` is a byte stream that Lambda's streaming runtime sends incrementally.

## StreamConfig

Fine-tune SSE behavior:

```rust
use turul_http_mcp_server::StreamConfig;

let config = StreamConfig {
    channel_buffer_size: 1000,        // SSE channel buffer (default: 1000)
    max_replay_events: 100,           // Events replayed on reconnect (default: 100)
    keepalive_interval_seconds: 30,   // SSE keepalive interval (default: 30)
    cors_origin: "https://example.com".to_string(),  // CORS origin for SSE
};

let server = LambdaMcpServerBuilder::new()
    .stream_config(config)
    .build()
    .await?;
```

## Integration Flow

```
┌─────────────┐     ┌──────────────────┐     ┌──────────────────────┐
│ API Gateway  │────▶│  Lambda Runtime   │────▶│  lambda_handler()    │
│ (HTTP API)   │     │  (streaming or    │     │                      │
│              │     │   standard)       │     │  HANDLER.get_or_     │
│              │     │                   │     │  try_init(create)    │
└─────────────┘     └──────────────────┘     └──────────┬───────────┘
                                                         │
                                              ┌──────────▼───────────┐
                                              │  LambdaMcpHandler    │
                                              │                      │
                                              │  handle() or         │
                                              │  handle_streaming()  │
                                              └──────────┬───────────┘
                                                         │
                                              ┌──────────▼───────────┐
                                              │  adapter.rs          │
                                              │  lambda_to_hyper()   │
                                              │  hyper_to_lambda()   │
                                              └──────────────────────┘
```

## When to Use Streaming

Use real-time streaming (combination 4) when:
- Tools send progress notifications during execution
- Task status updates need real-time delivery
- Clients subscribe to resource change notifications
- You need SSE reconnection support (`Last-Event-ID`)

Use snapshot mode (combination 1 or 2) when:
- Tools return quickly (< 1 second)
- No need for real-time notifications
- Cost optimization is important (streaming incurs duration-based charges)
- You want the simplest deployment model

## Cargo Features

```toml
# Snapshot mode (default features are sufficient)
[dependencies]
turul-mcp-aws-lambda = { version = "0.3" }
# default = ["cors", "sse"] — SSE enabled but only snapshots without streaming feature

# Real-time streaming mode
[dependencies]
turul-mcp-aws-lambda = { version = "0.3", features = ["streaming"] }
# streaming implies sse

# Snapshot mode, explicitly no SSE
[dependencies]
turul-mcp-aws-lambda = { version = "0.3", default-features = false, features = ["cors"] }
```

## CORS and Streaming

CORS headers are applied to both snapshot and streaming responses. The `LambdaMcpHandler` handles CORS preflight (OPTIONS) requests before delegating to the protocol handler, ensuring browser clients can connect regardless of streaming mode.
