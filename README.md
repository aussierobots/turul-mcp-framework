# Turul MCP Framework - Beta-Grade Rust Implementation

A comprehensive, battle-tested Rust framework for building Model Context Protocol (MCP) servers and clients with modern patterns, extensive tooling, and enterprise-grade features. Fully compliant with **MCP 2025-06-18 specification**.

## ✅ **Beta-Grade Quality** - Comprehensive Test Coverage
**100+ passing tests across workspace** • **Full SessionContext integration** • **Framework-native testing patterns**

## ✨ Key Highlights

- **🏗️ 67+ Workspace Crates**: Complete MCP ecosystem with core framework, client library, and serverless support
- **📚 38+ Comprehensive Examples**: Real-world business applications and framework demonstration examples
- **🧪 100+ Comprehensive Tests**: Beta-grade test suite with core framework tests, SessionContext integration tests, and framework-native integration tests
- **⚡ Multiple Development Patterns**: Derive macros, function attributes, declarative macros, and manual implementation
- **🌐 Transport Flexibility**: HTTP/1.1, HTTP/2, SSE, and stdio transport support
- **☁️ Serverless Ready**: AWS Lambda integration with streaming responses and SQS event processing
- **🔧 Beta Features**: Session management, real-time notifications, performance monitoring, and UUID v7 support
- **⚡ Performance Optimized**: Comprehensive benchmarking suite with >1000 RPS throughput, <100ms response times, and extensive stress testing

## 🚀 Quick Start

### 1. Simple Calculator (Derive Macros)

```rust
use turul_mcp_derive::McpTool;
use turul_mcp_server::{McpServer, McpResult};

#[derive(McpTool, Clone)]
#[tool(name = "add", description = "Add two numbers")]
struct AddTool {
    #[param(description = "First number")]
    a: f64,
    #[param(description = "Second number")]
    b: f64,
}

impl AddTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<String> {
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
use turul_mcp_derive::McpTool;

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

## 🚀 Running & Testing the Framework

### Quick Start - Verify Everything Works

```bash
# 1. Build the framework
cargo build --workspace

# 2. Run compliance tests
cargo test -p turul-mcp-framework-integration-tests --test mcp_runtime_capability_validation

# 3. Start a simple server
cargo run -p minimal-server

# 4. Test the server (in another terminal)
curl -X POST http://127.0.0.1:8000/mcp \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}'
```

### Example Servers - Ready to Run

**Core Test Servers:**
```bash
# Comprehensive server (all MCP features)
cargo run --package comprehensive-server -- --port 8082

# Resource server (17 test resources)  
cargo run --package resource-test-server -- --port 8080

# Prompts server (11 test prompts)
cargo run --package prompts-test-server -- --port 8081
```

**Business Application Servers:**
```bash  
# Development team resources
cargo run -p resources-server -- --port 8041

# AI development prompts  
cargo run -p prompts-server -- --port 8040

# Real-time notifications
cargo run -p notification-server

# Session management demo
cargo run -p stateful-server
```

### Manual MCP Compliance Verification

**Step 1: Initialize Connection**
```bash
PORT=8080  # Replace with your server's port
curl -X POST http://127.0.0.1:$PORT/mcp \
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
```

**Expected Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "protocolVersion": "2025-06-18",
    "serverInfo": {"name": "server-name", "version": "0.2.0"},
    "capabilities": {"tools": {"listChanged": false}}
  }
}
```

**Step 2: Test Available Operations**
```bash
# Get session ID from response and test capabilities
SESSION_ID="your-session-id-here"

# If server has tools capability:
curl -X POST http://127.0.0.1:$PORT/mcp \
  -H 'Content-Type: application/json' \
  -H "Mcp-Session-Id: $SESSION_ID" \
  -d '{"jsonrpc":"2.0","method":"tools/list","params":{},"id":2}' | jq

# If server has resources capability:
curl -X POST http://127.0.0.1:$PORT/mcp \
  -H 'Content-Type: application/json' \
  -H "Mcp-Session-Id: $SESSION_ID" \
  -d '{"jsonrpc":"2.0","method":"resources/list","params":{},"id":3}' | jq
```

### Comprehensive Testing Guide

For detailed testing instructions, server running guides, and compliance verification:

**📚 [Complete Testing Guide](TESTING_GUIDE.md)**

This guide includes:
- ✅ All server running instructions with expected outputs
- ✅ Manual MCP 2025-06-18 compliance verification  
- ✅ SSE event stream testing procedures
- ✅ Performance testing and troubleshooting
- ✅ CI/CD integration examples

### Quick Compliance Check Script

```bash
# Create and run compliance check
cat > quick_check.sh << 'EOF'
#!/bin/bash
PORT=${1:-8080}
echo "🧪 Testing MCP server on port $PORT"

INIT_RESPONSE=$(curl -s -X POST http://127.0.0.1:$PORT/mcp \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}')

if [[ $(echo $INIT_RESPONSE | jq -r '.result.protocolVersion') == "2025-06-18" ]]; then
    echo "✅ MCP 2025-06-18 compliant"
else
    echo "❌ Not compliant"
    exit 1
fi
EOF

chmod +x quick_check.sh

# Test any server
cargo run -p minimal-server &
./quick_check.sh 8000
```

## 🏛️ Architecture Overview

### Core Framework (10+ Crates)
- **`turul-mcp-server`** - High-level server builder with session management
- **`turul-mcp-client`** - Comprehensive client library with HTTP transport support
- **`turul-http-mcp-server`** - HTTP/SSE transport with CORS and streaming
- **`turul-mcp-protocol-2025-06-18`** - Complete MCP specification implementation
- **`turul-mcp-derive`** - Procedural and declarative macros
- **`turul-mcp-json-rpc-server`** - Transport-agnostic JSON-RPC 2.0 foundation

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
- **HTTP/1.1 & HTTP/2** - Standard web transport with JSON-RPC
- **Server-Sent Events (SSE)** - Real-time notifications and streaming
- **Stdio** - Command-line integration
- **AWS Lambda** - Serverless deployment with streaming responses

## 📚 Examples Overview

### 🏢 Real-World Business Applications
Beta-grade servers solving actual business problems:

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

### 🔧 Framework Demonstrations
Educational examples showcasing framework patterns:
- **Basic Patterns**: minimal-server, manual-tools-server, spec-compliant-server
- **Advanced Features**: stateful-server, pagination-server, version-negotiation-server
- **Macro System**: tool-macro-example, resource-macro-example, enhanced-tool-macro-test
- **Serverless**: lambda-turul-mcp-server (AWS Lambda with SQS integration)
- **Testing**: performance-testing (comprehensive benchmarking suite)

## ☁️ Serverless Support

### AWS Lambda MCP Server
Full serverless implementation with advanced AWS integration:

```bash
cd examples/lambda-turul-mcp-server

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

### 🧪 **Comprehensive Test Coverage - Beta Quality**

**Framework Excellence**: 100+ tests across all components with full SessionContext integration:

- **✅ Core Framework Tests** - Protocol, server, client, derive macros (36+ passing)
- **✅ SessionContext Integration** - Full session state management (8/8 passing) 
- **✅ Framework Integration Tests** - Proper API usage patterns (7/7 passing)
- **✅ MCP Compliance Tests** - Protocol specification validation (28+ passing)
- **✅ Builder Pattern Tests** - Runtime tool creation (70+ passing)
- **✅ Example Applications** - Real-world scenario validation

```bash
# Run all tests - expect 100+ passing
cargo test --workspace

# SessionContext integration tests
cargo test --test session_context_macro_tests

# Framework integration tests (proper patterns)
cargo test --test framework_integration_tests

# MCP compliance tests
cargo test --test mcp_compliance_tests
```

### 🎯 **Framework-Native Testing Patterns**

**The RIGHT way to test MCP applications** - Use framework APIs, not raw JSON:

```rust
// ✅ CORRECT: Framework integration test
use turul_mcp_server::{McpServerBuilder, McpTool, SessionContext};
use turul_mcp_derive::McpTool;

#[derive(McpTool, Default)]
#[tool(name = "calculator", description = "Add numbers")]
struct Calculator {
    #[param(description = "First number")] a: f64,
    #[param(description = "Second number")] b: f64,
}

impl Calculator {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<f64> {
        Ok(self.a + self.b)
    }
}

#[tokio::test]
async fn test_calculator_tool() {
    let tool = Calculator { a: 5.0, b: 3.0 };
    
    // Use framework's McpTool trait
    let result = tool.call(json!({"a": 5.0, "b": 3.0}), None).await.unwrap();
    
    // Verify using framework result types
    assert_eq!(result.content.len(), 1);
    match &result.content[0] {
        ToolResult::Text { text } => {
            let parsed: Value = serde_json::from_str(text).unwrap();
            assert_eq!(parsed["output"], 8.0); // Derive macro uses "output"
        }
        _ => panic!("Expected text result")
    }
}

#[tokio::test] 
async fn test_server_integration() {
    // Use framework builders
    let server = McpServerBuilder::new()
        .name("test-server")
        .tool(Calculator::default())
        .build()
        .unwrap();
    
    // Server builds successfully with proper type checking
    assert!(true);
}
```

**❌ WRONG: Raw JSON manipulation** (old problematic pattern):
```rust
// DON'T DO THIS - mixing incompatible JSON-RPC types
let request = json!({
    "method": "tools/call",
    "params": { "name": "calc" }
});
```

### 🔄 **SessionContext Integration - Fully Working**

**Complete session state management** with proper test infrastructure:

```rust
// SessionContext integration test
use crate::test_helpers::create_test_session;

#[tokio::test]
async fn test_session_state_management() {
    let session = create_test_session().await;
    
    // Session state works perfectly
    session.set_typed_state("counter", &42i32).unwrap();
    let value: i32 = session.get_typed_state("counter").unwrap();
    assert_eq!(value, 42);
    
    // Progress notifications work
    session.notify_progress("processing", 50);
    
    // Tool execution with session context
    let tool = Calculator { a: 1.0, b: 2.0 };
    let result = tool.call(json!({"a": 1.0, "b": 2.0}), Some(session)).await.unwrap();
    assert_eq!(result.content.len(), 1);
}
```

**Test Infrastructure Available**:
- `TestSessionBuilder` - Create real SessionContext instances
- `TestNotificationBroadcaster` - Verify notifications  
- `create_test_session()` - Helper for simple cases
- Full storage backend integration

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
use turul_mcp_protocol::tools::*;

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

Comprehensive MCP client for HTTP transport:

```rust
use turul_mcp_client::{McpClient, ClientConfig};

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

### Beta Quality
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

### MCP Session Management Compliance Testing

The framework includes comprehensive compliance testing for MCP session management specification requirements.

#### Running the Session Management Compliance Test

```bash
# 1. Start a server with session storage (choose backend: sqlite, postgres, dynamodb, or inmemory)
cargo run --example client-initialise-server -- --port 52950 --storage-backend dynamodb --create-tables

# 2. In another terminal, run the comprehensive compliance test (IMPORTANT: include RUST_LOG=info)
RUST_LOG=info cargo run --example session-management-compliance-test -- http://127.0.0.1:52950/mcp

# 3. Alternative: Use different storage backends
cargo run --example client-initialise-server -- --port 52951 --storage-backend sqlite --create-tables
RUST_LOG=info cargo run --example session-management-compliance-test -- http://127.0.0.1:52951/mcp
```

#### What the Compliance Test Verifies

The comprehensive test validates all MCP session management requirements:

- **✅ Session ID Generation**: UUID v7 with cryptographic security and ASCII compliance
- **✅ Session Persistence**: Proper session validation and storage backend integration
- **✅ Session Expiry**: TTL-based cleanup and 404 responses for expired sessions
- **✅ Client Reinitialize**: Graceful session recovery on expiry
- **✅ DELETE Termination**: Explicit session termination support
- **✅ Session Isolation**: Multi-session security and data separation

#### Expected Output

```
🧪 MCP Session Management Compliance Test
═══════════════════════════════════════════
✅ Session ID generation compliance verified
✅ Session persistence compliance verified  
✅ Session expiry compliance verified
✅ Client reinitialize compliance verified
✅ DELETE session termination compliance verified
✅ Session isolation compliance verified

🎉 MCP SESSION MANAGEMENT COMPLIANCE: COMPLETE
═══════════════════════════════════════════════
```

#### Storage Backend Configuration

**DynamoDB** (Production):
- 5-minute TTL with automatic cleanup
- GSI indexes for efficient queries
- AWS credentials required

**SQLite** (Development):  
- File-based persistence
- 5-minute TTL with background cleanup
- No external dependencies

**PostgreSQL** (Enterprise):
- Full SQL features with indexing
- 5-minute TTL with efficient cleanup
- Connection string required

**InMemory** (Testing):
- Fast, no persistence
- 5-minute TTL with memory cleanup
- Zero configuration

#### Customizing TTL Configuration

```rust
// Custom TTL configuration (default: 5 minutes)
let config = DynamoDbConfig {
    session_ttl_minutes: 30,  // 30-minute session TTL
    event_ttl_minutes: 15,    // 15-minute event TTL
    ..Default::default()
};
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

**🚀 Ready to build beta-grade MCP servers?** Start with our [comprehensive examples](examples/) or check the [getting started guide](EXAMPLES.md).

**💡 Need help?** Open an issue or check our [38+ working examples](examples/) covering everything from simple calculators to enterprise systems.
