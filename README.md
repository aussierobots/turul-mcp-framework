# Turul MCP Framework - this is Work In Progress - Rust Implementation

A comprehensive Rust framework for building Model Context Protocol (MCP) servers and clients with modern patterns, extensive tooling, and enterprise-grade features. Fully compliant with **MCP 2025-06-18 specification**.

## There are still significant architecture changes occurring
Use at own risk. Suggest forking

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

### Fine-Grained Trait Architecture
**Modern composable design pattern for all MCP areas:**

```rust
// Fine-grained trait composition for maximum flexibility
impl HasBaseMetadata for MyTool {
    fn name(&self) -> &str { "my_tool" }
}

impl HasDescription for MyTool {
    fn description(&self) -> Option<&str> { Some("Tool description") }
}

// ... implement other trait aspects as needed

// ToolDefinition automatically implemented via blanket impl
// McpTool trait provides execution interface
```

**Supported Areas:**
- **Tools** (`ToolDefinition`) - Dynamic tool execution with validation
- **Resources** (`ResourceDefinition`) - Static and dynamic content serving  
- **Prompts** (`PromptDefinition`) - AI interaction template generation
- **Sampling** (`SamplingDefinition`) - AI model integration patterns
- **Completion** (`CompletionDefinition`) - Context-aware text completion
- **Logging** (`LoggerDefinition`) - Dynamic log level management  
- **Roots** (`RootDefinition`) - Secure file system access boundaries
- **Elicitation** (`ElicitationDefinition`) - Structured user input collection
- **Notifications** (`NotificationDefinition`) - Real-time event broadcasting

### Comprehensive Server Builder
**All MCP areas supported with consistent builder pattern:**

```rust
let server = McpServer::builder()
    .name("comprehensive-server")
    .version("1.0.0")
    .instructions("Full-featured MCP server with all areas")
    // Tools
    .tool(WeatherTool::new())
    .tools(vec![CalculatorTool::new(), ValidationTool::new()])
    // Resources  
    .resource(AppConfigResource::new())
    .resources(vec![LogsResource::new(), MetricsResource::new()])
    // Prompts
    .prompt_provider(CodeReviewPrompt::new()) 
    .prompt_providers(vec![DocumentationPrompt::new(), TestPrompt::new()])
    // Sampling
    .sampling_provider(CreativeSampling::new())
    .sampling_providers(vec![CodeSampling::new(), TechnicalSampling::new()])
    // Completion
    .completion_provider(IdeCompletion::new())
    .completion_providers(vec![SqlCompletion::new(), JsonCompletion::new()])
    // Logging
    .logger(AuditLogger::new())
    .loggers(vec![SecurityLogger::new(), PerformanceLogger::new()])
    // Roots
    .root_provider(WorkspaceRoot::new())
    .root_providers(vec![ConfigRoot::new(), TempRoot::new()])
    // Elicitation  
    .elicitation_provider(OnboardingElicitation::new())
    .elicitation_providers(vec![SurveyElicitation::new(), FeedbackElicitation::new()])
    // Notifications
    .notification_provider(ProgressNotification::new())
    .notification_providers(vec![AlertNotification::new(), StatusNotification::new()])
    // Server configuration
    .bind_address("127.0.0.1:8080".parse()?)
    .build()?;
```

### Complete MCP Implementation
**All areas implemented with fine-grained trait architecture:**

- ✅ **Tools** (`ToolDefinition`) - Dynamic tool execution with validation, schema generation, and metadata
- ✅ **Resources** (`ResourceDefinition`) - Static and dynamic content serving with access control
- ✅ **Prompts** (`PromptDefinition`) - AI interaction template generation with parameter validation
- ✅ **Completion** (`CompletionDefinition`) - Context-aware text completion with model preferences  
- ✅ **Logging** (`LoggerDefinition`) - Dynamic log level management with structured output
- ✅ **Notifications** (`NotificationDefinition`) - Real-time SSE event broadcasting with filtering
- ✅ **Roots** (`RootDefinition`) - Secure file system access boundaries with permissions
- ✅ **Sampling** (`SamplingDefinition`) - AI model integration patterns with constraints
- ✅ **Elicitation** (`ElicitationDefinition`) - Structured user input collection with validation
- ✅ **Session Management** - Stateful operations with UUID v7 correlation IDs

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

**Available derive macros for all MCP areas:**
```rust
// Tools
#[derive(McpTool, Clone)]
#[tool(name = "weather", description = "Get weather information")]
struct WeatherTool {
    #[param(description = "City name")]
    city: String,
    #[param(description = "Temperature unit", optional)]
    unit: Option<String>,
}

// Resources  
#[derive(McpResource, Clone)]
#[resource(uri = "config://app.json", description = "Application configuration")]
struct AppConfigResource;

// Prompts
#[derive(McpPrompt, Clone)]  
#[prompt(name = "code_review", description = "Generate code review prompts")]
struct CodeReviewPrompt {
    #[param(description = "Programming language")]
    language: String,
}

// Sampling
#[derive(McpSampling, Clone)]
#[sampling(description = "Creative writing with style controls")]  
struct CreativeSampling;

// Completion
#[derive(McpCompletion, Clone)]
#[completion(description = "Context-aware IDE completions")]
struct IdeCompletion;

// Logging
#[derive(McpLogger, Clone)]
#[logger(name = "audit", description = "Compliance audit logging")]
struct AuditLogger;

// Roots  
#[derive(McpRoot, Clone)]
#[root(uri = "file:///workspace", description = "Project workspace")]
struct WorkspaceRoot;

// Elicitation
#[derive(McpElicitation, Clone)]
#[elicit(description = "Collect customer onboarding information")]
struct OnboardingElicitation;

// Notifications
#[derive(McpNotification, Clone)]
#[notification(method = "progress/update", description = "Progress updates")]
struct ProgressNotification;
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

### 4. Manual Implementation (Fine-Grained Traits)
**Best for:** Maximum control, complex business logic
```rust
use mcp_protocol::tools::*;

struct CustomTool {
    input_schema: ToolSchema,
}

// Implement fine-grained trait composition
impl HasBaseMetadata for CustomTool {
    fn name(&self) -> &str { "custom_business_logic" }
}

impl HasDescription for CustomTool {
    fn description(&self) -> Option<&str> { Some("Custom business logic tool") }
}

impl HasInputSchema for CustomTool {
    fn input_schema(&self) -> &ToolSchema { &self.input_schema }
}

impl HasOutputSchema for CustomTool {
    fn output_schema(&self) -> Option<&ToolSchema> { None }
}

impl HasAnnotations for CustomTool {
    fn annotations(&self) -> Option<&ToolAnnotations> { None }
}

impl HasToolMeta for CustomTool {
    fn tool_meta(&self) -> Option<&HashMap<String, Value>> { None }
}

// ToolDefinition is automatically implemented via blanket impl!

#[async_trait]
impl McpTool for CustomTool {
    async fn call(&self, args: Value, session: Option<SessionContext>)
        -> McpResult<CallToolResult> {
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
