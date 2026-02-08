# Comprehensive Server Example

This example demonstrates a complete MCP server with all supported handlers and capabilities enabled. It showcases the full range of MCP 2025-11-25 specification features in a single server implementation.

## 🚀 What This Example Shows

- **All MCP Handlers**: Complete implementation of every MCP endpoint
- **Tools**: Mathematical operations with manual trait implementations
- **Prompts**: Code generation prompts for different languages
- **Resources**: Project file access and content delivery
- **Templates**: Code template generation
- **Completion**: AI-assisted text completion
- **Logging**: Log level management and configuration
- **Notifications**: Event handling and broadcasting
- **Roots**: Workspace root management
- **Sampling**: Message generation and sampling

## 🛠️ Available Features

### Tools
- **`add`**: Add two numbers together
- **`multiply`**: Multiply two numbers

### Prompts  
- **`generate_code`**: Generate code based on requirements and language

### Resources
- **`file://project`**: Access to project source files (simulated)

### Templates
- **`function`**: Generate function templates with customizable names and return types

### Enabled Capabilities
- ✅ **Completion**: AI-assisted text completion
- ✅ **Logging**: Log level management
- ✅ **Notifications**: Real-time event broadcasting
- ✅ **Roots**: Workspace root listing
- ✅ **Sampling**: Message generation
- ✅ **Templates**: Template rendering

## 🏃 Running the Example

### Quick Start
```bash
# Run with default settings (port 8002)
cargo run -p comprehensive-server

# Expected output:
# INFO comprehensive_server: 🚀 Starting Comprehensive MCP Server on port 8002
# INFO turul_mcp_server::builder: 🔧 Auto-configured server capabilities:
# INFO turul_mcp_server::builder:    - Tools: true
# INFO turul_mcp_server::builder:    - Resources: true
# INFO turul_mcp_server::builder:    - Prompts: true
# INFO turul_mcp_server::server: ✅ Server started successfully on http://127.0.0.1:8002/mcp
```

### Custom Configuration
```bash
# Specify custom port
cargo run -p comprehensive-server -- --port 9000

# Specify full bind address  
cargo run -p comprehensive-server -- 0.0.0.0:9000
```

### Quick Compliance Verification
```bash
# In another terminal, test the server
curl -X POST http://127.0.0.1:8002/mcp \
  -H 'Content-Type: application/json' \
  -d '{
    "jsonrpc": "2.0",
    "method": "initialize",
    "params": {
      "protocolVersion": "2025-06-18",
      "capabilities": {},
      "clientInfo": {"name": "test", "version": "1.0"}
    },
    "id": 1
  }' | jq

# Should return MCP 2025-11-25 compliant response with all capabilities
```

## 🧪 Testing All Features

### 1. Initialize the Connection
```bash
curl -X POST http://127.0.0.1:8002/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "initialize",
    "params": {
      "protocolVersion": "2025-06-18",
      "capabilities": {},
      "clientInfo": {"name": "test-client", "version": "1.0.0"}
    },
    "id": "1"
  }'
```

### 2. List Available Tools
```bash
curl -X POST http://127.0.0.1:8002/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/list",
    "params": {},
    "id": "2"
  }'
```

### 3. Call a Tool
```bash
curl -X POST http://127.0.0.1:8002/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "add",
      "arguments": {"a": 15.5, "b": 27.3}
    },
    "id": "3"
  }'
```

### 4. List Available Prompts
```bash
curl -X POST http://127.0.0.1:8002/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "prompts/list",
    "params": {},
    "id": "4"
  }'
```

### 5. Get a Prompt
```bash
curl -X POST http://127.0.0.1:8002/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "prompts/get",
    "params": {
      "name": "generate_code",
      "arguments": {
        "language": "python",
        "task": "create a REST API endpoint"
      }
    },
    "id": "5"
  }'
```

### 6. List Available Resources
```bash
curl -X POST http://127.0.0.1:8002/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "resources/list",
    "params": {},
    "id": "6"
  }'
```

### 7. Read a Resource
```bash
curl -X POST http://127.0.0.1:8002/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "resources/read",
    "params": {
      "uri": "file://project"
    },
    "id": "7"
  }'
```

### 8. List Available Templates
```bash
curl -X POST http://127.0.0.1:8002/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "resources/templates/list",
    "params": {},
    "id": "8"
  }'
```

### 9. Set Log Level
```bash
curl -X POST http://127.0.0.1:8002/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "logging/setLevel",
    "params": {
      "level": "debug"
    },
    "id": "9"
  }'
```

### 10. List Workspace Roots
```bash
curl -X POST http://127.0.0.1:8002/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "roots/list",
    "params": {},
    "id": "10"
  }'
```

### 11. Create Sample Message
```bash
curl -X POST http://127.0.0.1:8002/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "sampling/createMessage",
    "params": {
      "messages": [
        {"role": "user", "content": {"type": "text", "text": "Hello!"}}
      ],
      "maxTokens": 100
    },
    "id": "11"
  }'
```

### 12. Request Text Completion
```bash
curl -X POST http://127.0.0.1:8002/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "completion/complete",
    "params": {
      "ref": {
        "type": "ref",
        "uri": "file://project"
      },
      "argument": {
        "name": "query",
        "value": "function to calculate"
      }
    },
    "id": "12"
  }'
```

## 🔧 Architecture Overview

### Handler Implementation Pattern
```rust
// Tool Implementation
#[async_trait]
impl McpTool for AddTool {
    fn name(&self) -> &str { "add" }
    fn description(&self) -> &str { "Add two numbers" }
    fn input_schema(&self) -> ToolSchema { /* schema */ }
    async fn call(&self, args: Value, session: Option<SessionContext>) -> Result<Vec<ToolResult>, String> {
        // Implementation
    }
}

// Prompt Implementation
#[async_trait]
impl McpPrompt for CodePrompt {
    fn name(&self) -> &str { "generate_code" }
    fn description(&self) -> &str { "Generate code based on requirements" }
    async fn generate(&self, args: HashMap<String, Value>) -> McpResult<Vec<PromptMessage>> {
        // Implementation
    }
}

// Resource Implementation
#[async_trait]
impl McpResource for ProjectResource {
    fn uri(&self) -> &str { "file://project" }
    fn name(&self) -> &str { "Project Files" }
    fn description(&self) -> &str { "Access to project source files" }
    async fn read(&self) -> McpResult<Vec<ResourceContent>> {
        // Implementation
    }
}

// Template Implementation
#[async_trait]
impl McpTemplate for FunctionTemplate {
    fn name(&self) -> &str { "function" }
    fn description(&self) -> &str { "Generate a function template" }
    async fn generate(&self, args: HashMap<String, Value>) -> McpResult<String> {
        // Implementation
    }
}
```

### Server Builder Configuration
```rust
let server = McpServer::builder()
    .name("comprehensive-server")
    .version("1.0.0")
    .title("Comprehensive MCP Server Example")
    .instructions("Server description...")
    .bind_address(bind_address)
    
    // Add custom tools
    .tool(AddTool)
    .tool(MultiplyTool)
    
    // Enable all MCP handlers
    .with_completion()      // completion/complete
    .with_prompts()         // prompts/list, prompts/get
    .with_resources()       // resources/list, resources/read
    .with_logging()         // logging/setLevel
    .with_notifications()   // notifications/*
    .with_roots()           // roots/list
    .with_sampling()        // sampling/createMessage
    
    .build()?;
```

## 📊 MCP 2025-11-25 Specification Compliance

This example demonstrates full compliance with the MCP specification:

| Feature Category | Endpoints | Implementation Status |
|------------------|-----------|----------------------|
| **Core** | initialize, ping | ✅ Complete |
| **Tools** | tools/list, tools/call | ✅ Complete |
| **Prompts** | prompts/list, prompts/get | ✅ Complete |
| **Resources** | resources/list, resources/read | ✅ Complete |
| **Completion** | completion/complete | ✅ Complete |
| **Logging** | logging/setLevel | ✅ Complete |
| **Notifications** | notifications/* | ✅ Complete |
| **Roots** | roots/list | ✅ Complete |
| **Sampling** | sampling/createMessage | ✅ Complete |
| **Templates** | resources/templates/list | ✅ Complete |

## 🎯 Key Concepts Demonstrated

### 1. Multi-Handler Architecture
Shows how to build servers that support multiple MCP capabilities simultaneously.

### 2. Handler Registration
Demonstrates the framework's builder pattern for enabling specific MCP features.

### 3. Protocol Compliance
Full implementation of MCP 2025-11-25 specification with proper JSON-RPC structure.

### 4. Error Handling
Proper error handling across all handler types with descriptive error messages.

### 5. Type Safety
Compile-time type safety for all handler implementations and parameter validation.

## 🚨 Production Considerations

This example provides **basic implementations** for demonstration. In production, you should:

### 1. Security
- Add authentication and authorization
- Implement rate limiting
- Validate all inputs thoroughly
- Use secure communication (HTTPS)

### 2. Real Implementations
- Connect prompts to actual AI services
- Implement real file system access for resources
- Add database-backed template storage
- Integrate with actual completion services

### 3. Performance
- Add caching layers
- Implement connection pooling
- Use async I/O for file operations
- Add monitoring and metrics

### 4. Reliability
- Add proper error recovery
- Implement circuit breakers
- Add health checks
- Include graceful shutdown

## 🔄 Customization Examples

### Adding Custom Tools
```rust
struct CustomTool;

#[async_trait]
impl McpTool for CustomTool {
    fn name(&self) -> &str { "custom_operation" }
    fn description(&self) -> &str { "Perform custom operation" }
    
    fn input_schema(&self) -> ToolSchema {
        ToolSchema::object()
            .with_properties(HashMap::from([
                ("input".to_string(), JsonSchema::string()),
            ]))
            .with_required(vec!["input".to_string()])
    }
    
    async fn call(&self, args: Value, _session: Option<SessionContext>) -> Result<Vec<ToolResult>, String> {
        let input = args.get("input").and_then(|v| v.as_str()).unwrap_or("");
        Ok(vec![ToolResult::text(format!("Processed: {}", input))])
    }
}

// Add to server
.tool(CustomTool)
```

### Adding Custom Prompts
```rust
struct CustomPrompt;

#[async_trait]
impl McpPrompt for CustomPrompt {
    fn name(&self) -> &str { "custom_prompt" }
    fn description(&self) -> &str { "Generate custom prompts" }
    
    async fn generate(&self, args: HashMap<String, Value>) -> McpResult<Vec<PromptMessage>> {
        let context = args.get("context").and_then(|v| v.as_str()).unwrap_or("general");
        let content = format!("Custom prompt for context: {}", context);
        Ok(vec![PromptMessage::text(content)])
    }
}
```

### Adding Custom Resources
```rust
struct CustomResource;

#[async_trait]
impl McpResource for CustomResource {
    fn uri(&self) -> &str { "custom://data" }
    fn name(&self) -> &str { "Custom Data" }
    fn description(&self) -> &str { "Access to custom data sources" }
    
    async fn read(&self) -> McpResult<Vec<ResourceContent>> {
        Ok(vec![ResourceContent::text("Custom data content".to_string())])
    }
}
```

## 📚 Related Examples

### Simpler Examples
- **[minimal-server](../minimal-server/)**: Basic MCP server setup
- **[derive-macro-server](../derive-macro-server/)**: Using derive macros
- **[function-macro-server](../function-macro-server/)**: Function-style tools

### Specialized Examples
- **[stateful-server](../stateful-server/)**: Session management focus
- **[notification-server](../notification-server/)**: Real-time notifications
- **[performance-testing](../performance-testing/)**: Load testing tools

## 🤝 Best Practices

1. **Handler Organization**: Group related handlers together
2. **Error Messages**: Provide clear, actionable error messages
3. **Documentation**: Document all tools, prompts, resources, and templates
4. **Testing**: Test each handler type thoroughly
5. **Logging**: Use structured logging for debugging
6. **Configuration**: Make server configurable for different environments

## 🛠️ Extending This Example

This comprehensive example serves as a foundation for building production MCP servers. You can:

1. **Replace Mock Implementations**: Connect to real services and data sources
2. **Add Authentication**: Implement proper security measures
3. **Scale Handlers**: Add more tools, prompts, resources, and templates
4. **Add Middleware**: Implement request/response processing
5. **Monitor Performance**: Add metrics and monitoring

---

This example demonstrates the full power of the MCP framework and serves as a template for building complete, production-ready MCP servers.