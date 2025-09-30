# Zero-Configuration MCP Server

**The simplest possible MCP server using the completed framework architecture.**

## Key Features

✅ **Zero Configuration** - Users NEVER specify method strings  
✅ **Auto-Determination** - Framework maps types to methods automatically  
✅ **Simple Patterns** - Derive macros replace complex trait implementations  
✅ **Pluggable Storage** - InMemory (default) → SQLite → PostgreSQL → AWS  

## How It Works

### Tool Definition (30 seconds)
```rust
// ✅ Framework auto-determines name: "calculator"
#[derive(McpTool)]
struct Calculator {
    #[param(description = "First number")]
    a: f64,
    #[param(description = "Second number")]  
    b: f64,
}

impl Calculator {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<f64> {
        Ok(self.a + self.b)
    }
}
```

### Built-in Progress Notifications (30 seconds)
```rust
// ✅ Use session's built-in progress notification
if let Some(session) = session {
session.notify_progress("Processing", 75).await;
}
```

### Server Creation (30 seconds)
```rust
let server = McpServer::builder()
    .name("my-server")
    .tool(Calculator { a: 0.0, b: 0.0 })           // → tools/call
    .build()?;

server.run().await
```

**Total Development Time: 90 seconds** ⚡

## Architecture Benefits

### For Developers
- **No Method Strings**: Framework auto-determines everything
- **No Boilerplate**: Derive macros handle trait implementations
- **Type Safety**: Impossible to use invalid MCP methods
- **IntelliSense**: Perfect IDE integration with zero memorization

### For Production
- **Pluggable Storage**: Start with InMemory, scale to PostgreSQL/AWS
- **MCP 2025-06-18 Compliance**: Latest specification support
- **SSE Resumability**: Last-Event-ID header for reconnection
- **Session Management**: Automatic cleanup and lifecycle handling

## Running

```bash
cargo run --package zero-config-getting-started
```

## Testing

```bash
# Initialize connection
curl -X POST http://127.0.0.1:8000/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}'

# Call calculator tool (framework auto-determined method)  
curl -X POST http://127.0.0.1:8000/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"calculator","arguments":{"a":5,"b":3}}}'
```

## Framework Comparison

| Framework | Method Definition | Lines of Code | Configuration |
|-----------|-------------------|---------------|---------------|
| **This Framework** | **Auto-determined** | **~30 lines** | **Zero** |
| Other MCP Libs | Manual strings | ~100+ lines | Complex |

## Next Steps

1. **Add More Tools**: Use `#[derive(McpTool)]` on any struct.
2. **Change Storage**: Swap the default in-memory storage for a persistent backend, e.g., `PostgresSessionStorage::with_config(config).await?`.
3. **Add Resources**: Use the `#[mcp_resource]` macro to serve data and files.
4. **Scale Up**: The framework's architecture is ready for enterprise deployment.
