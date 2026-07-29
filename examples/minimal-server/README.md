# Minimal MCP Server Example

This example demonstrates the absolute minimum setup for an MCP server using the turul-mcp-server.

## What This Example Shows

- **Minimal Setup**: Just 50 lines of code for a working MCP server
- **Basic Tool Implementation**: Simple echo tool using function macro
- **Default Configuration**: HTTP on 127.0.0.1:8641
- **Essential MCP Functionality**: Discover the server, list tools, call tools

## Running the Example

```bash
cargo run --bin minimal-server
```

The server will start on `http://127.0.0.1:8641/mcp` and provide:
- One tool: `echo` - echoes back text input
- Standard MCP methods: server/discover, tools/list, tools/call

## Testing the Server

The 2026-07-28 core is stateless: there is no `initialize`/`notifications/initialized`
handshake and no `Mcp-Session-Id`. Every request carries its own per-request `_meta`
(`io.modelcontextprotocol/protocolVersion`, `clientInfo`, `clientCapabilities`) and the
`MCP-Protocol-Version: 2026-07-28` header.

### 1. Discover the Server
```bash
curl -X POST http://127.0.0.1:8641/mcp \
  -H "Content-Type: application/json" \
  -H "MCP-Protocol-Version: 2026-07-28" \
  -H "Mcp-Method: server/discover" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "server/discover",
    "params": {
      "_meta": {
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientInfo": {"name": "test-client", "version": "1.0.0"},
        "io.modelcontextprotocol/clientCapabilities": {}
      }
    }
  }'
```

### 2. List Available Tools
```bash
curl -X POST http://127.0.0.1:8641/mcp \
  -H "Content-Type: application/json" \
  -H "MCP-Protocol-Version: 2026-07-28" \
  -H "Mcp-Method: tools/list" \
  -d '{
    "jsonrpc": "2.0",
    "id": 2,
    "method": "tools/list",
    "params": {
      "_meta": {
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientInfo": {"name": "test-client", "version": "1.0.0"},
        "io.modelcontextprotocol/clientCapabilities": {}
      }
    }
  }'
```

### 3. Call the Echo Tool
```bash
curl -X POST http://127.0.0.1:8641/mcp \
  -H "Content-Type: application/json" \
  -H "MCP-Protocol-Version: 2026-07-28" \
  -H "Mcp-Method: tools/call" \
  -H "Mcp-Name: echo" \
  -d '{
    "jsonrpc": "2.0",
    "id": 3,
    "method": "tools/call",
    "params": {
      "_meta": {
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientInfo": {"name": "test-client", "version": "1.0.0"},
        "io.modelcontextprotocol/clientCapabilities": {}
      },
      "name": "echo",
      "arguments": {"text": "Hello, MCP!"}
    }
  }'
```

## Key Concepts Demonstrated

1. **Function Macro**: Using #[mcp_tool] attribute for simplicity
2. **Schema Definition**: JSON Schema for tool input parameters
3. **Server Builder**: Fluent API for server configuration
4. **Error Handling**: Basic error handling for tool execution

## Next Steps

- See [calculator-add-manual-server](../calculator-add-manual-server) for the manual trait-implementation reference
- See [streamable-http-client](../streamable-http-client) for the paired 2026 client walkthrough