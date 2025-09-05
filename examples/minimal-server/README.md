# Minimal MCP Server Example

This example demonstrates the absolute minimum setup for an MCP server using the turul-mcp-server.

## What This Example Shows

- **Minimal Setup**: Just 50 lines of code for a working MCP server
- **Basic Tool Implementation**: Simple echo tool using manual trait implementation
- **Default Configuration**: HTTP on 127.0.0.1:8641
- **Essential MCP Functionality**: Initialize, list tools, call tools

## Running the Example

```bash
cargo run --bin minimal-server
```

The server will start on `http://127.0.0.1:8641/mcp` and provide:
- One tool: `echo` - echoes back text input
- Standard MCP endpoints: initialize, tools/list, tools/call

## Testing the Server

### 1. Initialize the Connection
```bash
curl -X POST http://127.0.0.1:8641/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "initialize",
    "params": {
      "protocolVersion": "2025-06-18",
      "capabilities": {},
      "clientInfo": {"name": "test-client", "version": "1.0.0"}
    }
  }'
```

### 2. List Available Tools
```bash
curl -X POST http://127.0.0.1:8641/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 2,
    "method": "tools/list",
    "params": {}
  }'
```

### 3. Call the Echo Tool
```bash
curl -X POST http://127.0.0.1:8641/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 3,
    "method": "tools/call",
    "params": {
      "name": "echo",
      "arguments": {"text": "Hello, MCP!"}
    }
  }'
```

## Key Concepts Demonstrated

1. **McpTool Trait**: Manual implementation of the core trait
2. **Schema Definition**: JSON Schema for tool input parameters
3. **Server Builder**: Fluent API for server configuration
4. **Error Handling**: Basic error handling for tool execution

## Next Steps

- See [manual-tools-server](../manual-tools-server) for more complex manual implementations
- See [macro-calculator](../macro-calculator) for derive macro examples  
- See [comprehensive-server](../comprehensive-server) for all MCP features