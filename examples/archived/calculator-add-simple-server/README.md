# Calculator Add Simple Server

## Code Reduction Demonstration

This example demonstrates the dramatic code reduction possible when using the turul-mcp-framework's function macro approach compared to manual implementation.

### Comparison

| Implementation | Lines of Code | Boilerplate | Maintainability |
|----------------|---------------|-------------|-----------------|
| **Manual** (calculator-add-manual-server) | **100+ lines** | High | Complex |
| **Simple** (this example) | **10 lines** | Zero | Trivial |

### Manual Implementation (Level 4)
- 100+ lines of boilerplate code
- Manual trait implementations for 6 different traits
- Static schema construction with OnceLock
- Manual parameter extraction and validation
- Complex error handling

### Function Macro Implementation (Level 1)
- 10 lines of actual code
- Zero boilerplate - everything automatic
- Framework handles all trait implementations
- Automatic parameter extraction from function signature
- Built-in error handling and validation

### Key Benefits of Function Macros

1. **Zero Configuration**: Framework auto-determines method names from function names
2. **Type Safety**: Parameter types and validation generated from function signature
3. **Schema Generation**: Input/output schemas automatically generated from types
4. **Error Handling**: McpResult provides standardized error responses
5. **MCP Compliance**: Guaranteed spec compliance without manual implementation

### Running

```bash
# Start the server
cargo run --example calculator-add-simple-server

# Test with curl
curl -X POST http://127.0.0.1:8647/mcp \
  -H "Content-Type: application/json" \
  -H "MCP-Protocol-Version: 2025-11-25" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "tools/call",
    "params": {
      "name": "calculator_add_simple",
      "arguments": {"a": 5, "b": 3}
    }
  }'
```

Expected response:
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "content": [{"type": "text", "text": "8"}],
    "isError": false
  }
}
```