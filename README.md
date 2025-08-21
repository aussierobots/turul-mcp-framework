# MCP Framework - Production-Ready Rust Implementation

A comprehensive Rust framework for building Model Context Protocol (MCP) servers and clients with modern patterns, extensive tooling, and enterprise-grade features. Fully compliant with **MCP 2025-06-18 specification**.

## ✨ Key Highlights

- **🏗️ 37 Workspace Crates**: Complete MCP ecosystem with core framework, client library, and serverless support
- **📚 26 Comprehensive Examples**: 10 real-world business applications + 16 framework demonstration examples
- **🧪 217+ Test Functions**: Extensive test coverage with 155 core framework tests + 18 integration tests + 44 example tests
- **⚡ Multiple Development Patterns**: Derive macros, function attributes, declarative macros, and manual implementation
- **🌐 Transport Flexibility**: HTTP/1.1, HTTP/2, WebSocket, SSE, and stdio transport support
- **☁️ Serverless Ready**: AWS Lambda integration with streaming responses and SQS event processing
- **🔧 Production Features**: Session management, real-time notifications, performance monitoring, and UUID v7 support

## 🚀 Quick Start

### 1. Simple Calculator (Derive Macros)

```rust
use mcp_derive::McpTool;
use mcp_server::{McpServer, McpResult};

#[derive(McpTool, Clone)]
#[tool(name = "add", description = "Add two numbers")]
struct AddTool {
    #[param(description = "First number")]
    a: f64,
    #[param(description = "Second number")]  
    b: f64,
}

impl AddTool {
    async fn execute(&self) -> McpResult<String> {
        Ok(format!("{} + {} = {}", self.a, self.b, self.a + self.b))
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let server = McpServer::builder()
        .name("calculator-server")
        .version("1.0.0")
        .tool(AddTool { a: 0.0, b: 0.0 })
        .bind_address("127.0.0.1:8080".parse()?)
        .build()?;

    server.run().await
}
```

### 2. Business Application Example

```rust
// From examples/logging-server - Enterprise audit system
use mcp_derive::McpTool;

#[derive(McpTool, Clone)]
#[tool(name = "audit_log", description = "Create compliance audit entry")]
struct AuditTool {
    #[param(description = "Log level (Info, Warn, Error)")]
    level: String,
    #[param(description = "Audit message")]
    message: String,
    #[param(description = "Business category")]
    category: String,
}

impl AuditTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<String> {
        let audit_entry = AuditEntry {
            id: uuid::Uuid::now_v7().to_string(), // Modern UUID v7
            timestamp: Utc::now(),
            level: self.level.parse()?,
            category: self.category.clone(),
            message: self.message.clone(),
            correlation_id: Some(format!("audit_{}", uuid::Uuid::now_v7())),
            compliance_tags: vec!["sox".to_string(), "gdpr".to_string()],
            retention_days: 2555, // 7 years for compliance
        };
        
        // Business logic with external data configuration
        let audit_policies = load_audit_policies("data/audit_policies.yaml")?;
        let formatted_entry = format_audit_entry(&audit_entry, &audit_policies)?;
        
        Ok(formatted_entry)
    }
}
```

## 🏛️ Architecture Overview

### Core Framework (6 Crates)
- **`mcp-server`** - High-level server builder with session management
- **`mcp-client`** - Comprehensive client library with multi-transport support  
- **`http-mcp-server`** - HTTP/SSE transport with CORS and streaming
- **`mcp-protocol-2025-06-18`** - Complete MCP specification implementation
- **`mcp-derive`** - Procedural and declarative macros
- **`json-rpc-server`** - Transport-agnostic JSON-RPC 2.0 foundation

### Complete MCP Implementation
- ✅ **Tools** - Dynamic tool execution with validation
- ✅ **Resources** - Static and dynamic content serving
- ✅ **Prompts** - AI interaction template generation
- ✅ **Completion** - Context-aware text completion
- ✅ **Logging** - Dynamic log level management
- ✅ **Notifications** - Real-time SSE event broadcasting
- ✅ **Session Management** - Stateful operations with UUID v7
- ✅ **Roots** - Secure file system access boundaries
- ✅ **Sampling** - AI model integration patterns
- ✅ **Elicitation** - Structured user input collection

### Transport Support
- **HTTP/1.1 & HTTP/2** - Standard web transport
- **Server-Sent Events (SSE)** - Real-time notifications
- **WebSocket** - Bidirectional communication
- **Stdio** - Command-line integration
- **AWS Lambda Streaming** - Serverless deployment

## 📚 Examples Overview

### 🏢 Real-World Business Applications (10 Examples)
Production-ready servers solving actual business problems:

1. **comprehensive-server** → Development Team Integration Platform
2. **dynamic-resource-server** → Enterprise API Data Gateway  
3. **logging-server** → Application Audit & Compliance System
4. **elicitation-server** → Customer Onboarding Platform
5. **notification-server** → Development Team Alert System
6. **completion-server** → IDE Auto-Completion Server
7. **prompts-server** → AI-Assisted Development Prompts
8. **derive-macro-server** → Code Generation & Template Engine
9. **calculator-server** → Business Financial Calculator
10. **resources-server** → Development Team Resource Hub

### 🔧 Framework Demonstrations (16 Examples)
Educational examples showcasing framework patterns:
- **Basic Patterns**: minimal-server, manual-tools-server, spec-compliant-server
- **Advanced Features**: stateful-server, pagination-server, version-negotiation-server
- **Macro System**: tool-macro-example, resource-macro-example, enhanced-tool-macro-test
- **Serverless**: lambda-mcp-server (AWS Lambda with SQS integration)
- **Testing**: performance-testing (comprehensive benchmarking suite)

## ☁️ Serverless Support

### AWS Lambda MCP Server
Full serverless implementation with advanced AWS integration:

```bash
cd examples/lambda-mcp-server

# Local development
cargo lambda watch

# Deploy to AWS
cargo lambda build --release
sam deploy --guided
```

**Features:**
- 🔄 Dual event sources (HTTP + SQS)
- 📡 200MB streaming responses
- 🗄️ DynamoDB session management
- ⚡ Sub-200ms cold starts
- 📊 CloudWatch + X-Ray integration

## 🧪 Testing & Quality

### Comprehensive Test Coverage
- **217+ Test Functions** across the entire framework
- **155 Core Framework Tests** - Protocol, server, client, macros
- **18 Integration Tests** - MCP 2025-06-18 specification compliance  
- **44 Example Tests** - Real-world scenario validation
- **Performance Benchmarks** - Load testing and stress testing

```bash
# Run all tests
cargo test --workspace

# Integration tests  
cargo test -p mcp-framework-integration-tests

# Performance benchmarks
cd examples/performance-testing
cargo run --bin performance_client -- throughput --concurrent 100
```

## 🎯 Development Patterns

### 1. Derive Macros (Recommended)
**Best for:** Type safety, IDE support, automatic schema generation
```rust
#[derive(McpTool, Clone)]
#[tool(name = "weather", description = "Get weather information")]
struct WeatherTool {
    #[param(description = "City name")]
    city: String,
    #[param(description = "Temperature unit", optional)]
    unit: Option<String>,
}
```

### 2. Function Attributes
**Best for:** Natural function syntax, minimal boilerplate
```rust
#[mcp_tool(name = "multiply", description = "Multiply two numbers")]
async fn multiply(
    #[param(description = "First number")] a: f64,
    #[param(description = "Second number")] b: f64,
) -> McpResult<String> {
    Ok(format!("{} × {} = {}", a, b, a * b))
}
```

### 3. Declarative Macros
**Best for:** Inline definitions, rapid prototyping
```rust
tool! {
    name: "fibonacci",
    description: "Calculate fibonacci number",
    params: {
        n: u64 => "Position in sequence" { min: 0, max: 100 },
        cache: Option<bool> => "Use caching" { default: true },
    },
    execute: |n, cache| async move {
        // Implementation
    }
}
```

### 4. Manual Implementation
**Best for:** Maximum control, complex business logic
```rust
#[async_trait]
impl McpTool for CustomTool {
    fn name(&self) -> &str { "custom_business_logic" }
    
    async fn call(&self, args: Value, session: Option<SessionContext>) 
        -> McpResult<Vec<ToolResult>> {
        // Full control over implementation
    }
}
```

## 🔧 Client Library

Comprehensive MCP client with multi-transport support:

```rust
use mcp_client::{McpClient, ClientConfig};

let client = McpClient::builder()
    .with_url("http://localhost:8080/mcp")?
    .build();

await client.connect()?;

// List available tools
let tools = client.list_tools().await?;

// Call a tool
let result = client.call_tool("add", json!({
    "a": 10.0,
    "b": 20.0
})).await?;

// Read resources
let resources = client.list_resources().await?;
let content = client.read_resource("config://app.json").await?;
```

## 🚀 Performance Features

### Modern Architecture
- **UUID v7** - Time-ordered IDs for better database performance and observability
- **Workspace Dependencies** - Consistent dependency management across 37 crates
- **Rust 2024 Edition** - Latest language features and performance improvements
- **Tokio/Hyper** - High-performance async runtime with HTTP/2 support

### Production Ready
- **Session Management** - Automatic cleanup and state persistence
- **Real-time Notifications** - SSE-based event streaming
- **CORS Support** - Browser client compatibility
- **Comprehensive Logging** - Structured logging with correlation IDs
- **Error Handling** - Detailed error types with recovery strategies

## 🔍 MCP Protocol Compliance

**Full MCP 2025-06-18 specification support:**

- ✅ **JSON-RPC 2.0** - Complete request/response with `_meta` fields
- ✅ **Protocol Negotiation** - Version compatibility and capability exchange
- ✅ **Progress Tracking** - Long-running operation support
- ✅ **Cursor Pagination** - Efficient large dataset navigation
- ✅ **Session Isolation** - Secure multi-client support
- ✅ **Transport Agnostic** - Multiple transport implementations

### Testing Your Server
```bash
# Test tool execution
curl -X POST http://127.0.0.1:8080/mcp \\
  -H "Content-Type: application/json" \\
  -H "MCP-Protocol-Version: 2025-06-18" \\
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "tools/call",
    "params": {
      "name": "add",
      "arguments": {"a": 10, "b": 20}
    }
  }'

# Test SSE notifications
curl -N -H "Accept: text/event-stream" \\
  http://127.0.0.1:8080/mcp/events
```

## 📊 Business Value Examples

### Enterprise Integration
- **dynamic-resource-server**: API orchestration across Customer, Inventory, Financial, and HR systems
- **logging-server**: SOX, PCI DSS, GDPR, and HIPAA compliance reporting
- **comprehensive-server**: Team collaboration with project management and workflow automation

### Developer Productivity  
- **completion-server**: Context-aware IDE completions for multiple languages and frameworks
- **prompts-server**: AI-powered code review and architecture guidance
- **derive-macro-server**: Template-based code generation with validation

### Customer Experience
- **elicitation-server**: GDPR-compliant customer onboarding with regulatory forms
- **notification-server**: Real-time incident management with escalation workflows

## 🛡️ Security & Reliability

- **Memory Safety** - Rust's ownership system prevents common vulnerabilities
- **Type Safety** - Compile-time validation with automatic schema generation
- **Input Validation** - Parameter constraints and sanitization
- **Session Isolation** - Secure multi-tenant operation
- **Audit Logging** - Comprehensive activity tracking with UUID v7 correlation
- **Resource Limits** - Configurable timeouts and memory constraints

## 🤝 Contributing

1. **Fork** the repository
2. **Create** a feature branch (`git checkout -b feature/amazing-feature`)  
3. **Add tests** for your changes
4. **Run** the full test suite (`cargo test --workspace`)
5. **Benchmark** performance impact if applicable
6. **Commit** changes (`git commit -m 'Add amazing feature'`)
7. **Push** to branch (`git push origin feature/amazing-feature`)
8. **Open** a Pull Request

## 📝 License

This project is licensed under the MIT OR Apache-2.0 License - see the LICENSE files for details.

## 🙏 Acknowledgments

- **[Model Context Protocol](https://modelcontextprotocol.io)** - The foundational specification
- **[Tokio](https://tokio.rs)** - Async runtime powering the framework  
- **[Hyper](https://hyper.rs)** - HTTP foundation with HTTP/2 support
- **[Serde](https://serde.rs)** - Serialization framework
- **Rust Community** - For exceptional tooling and ecosystem

---

**🚀 Ready to build production MCP servers?** Start with our [comprehensive examples](examples/) or check the [getting started guide](EXAMPLES_OVERVIEW.md).

**💡 Need help?** Open an issue or check our [26 working examples](examples/) covering everything from simple calculators to enterprise systems.