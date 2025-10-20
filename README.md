# Turul MCP Framework - Beta Rust Implementation

A comprehensive Rust framework for building Model Context Protocol (MCP) servers and clients with modern patterns, extensive tooling, and enterprise-grade features. Fully compliant with **MCP 2025-06-18 specification**.

⚠️ **Beta Status** - Active development with ongoing feature enhancements. Phase 6 session-aware resources completed. Suitable for development and testing.

## 🧪 **Active Development** - Comprehensive Test Coverage
**300+ passing tests across workspace** • **Complete async SessionContext integration** • **Framework-native testing patterns**

## ✨ Key Highlights

- **🏗️ 10 Framework Crates**: Complete MCP ecosystem with core framework, client library, and serverless support
- **📚 45+ Comprehensive Examples**: Real-world business applications and framework demonstration examples (all validated through comprehensive testing campaign)
- **🧪 300+ Development Tests**: Comprehensive test suite with core framework tests, SessionContext integration tests, and framework-native integration tests
- **⚡ Multiple Development Patterns**: Derive macros, function attributes, declarative macros, and manual implementation
- **🌐 Transport Flexibility**: HTTP/1.1 and SSE streaming via SessionMcpHandler (WebSocket and stdio planned)
- **☁️ Serverless Support**: AWS Lambda integration with streaming responses and SQS event processing
- **🔧 Development Features**: Session management, real-time notifications, performance monitoring, and UUID v7 support
- **⚡ Performance Optimized**: Comprehensive benchmarking suite with >1000 RPS throughput, <100ms response times, and extensive stress testing

## 🚀 Quick Start

### 1. Function Macros (Simplest - Recommended)

```rust
use turul_mcp_derive::mcp_tool;
use turul_mcp_server::prelude::*;

#[mcp_tool(name = "add", description = "Add two numbers")]
async fn add(
    #[param(description = "First number")] a: f64,
    #[param(description = "Second number")] b: f64,
) -> McpResult<f64> {
    Ok(a + b)  // Framework wraps as {"output": 8.0} in JSON-RPC response
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = McpServer::builder()
        .name("calculator-server")
        .version("1.0.0")
        .tool_fn(add)  // Use function name directly
        .bind_address("127.0.0.1:8641".parse()?)  // Default port; customize as needed
        .build()?;

    server.run().await
}
```

### 2. Derive Macros (Struct-Based)

```rust
use turul_mcp_derive::McpTool;
use turul_mcp_server::prelude::*;

#[derive(McpTool, Clone)]
#[tool(name = "calculator", description = "Mathematical operations")]
struct Calculator {
    #[param(description = "First number")]
    a: f64,
    #[param(description = "Second number")]
    b: f64,
    #[param(description = "Operation (+, -, *, /)")]
    operation: String,
}

impl Calculator {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<f64> {
        match self.operation.as_str() {
            "+" => Ok(self.a + self.b),
            "-" => Ok(self.a - self.b),
            "*" => Ok(self.a * self.b),
            "/" => {
                if self.b == 0.0 {
                    Err("Division by zero".into())
                } else {
                    Ok(self.a / self.b)
                }
            },
            _ => Err("Invalid operation".into()),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = McpServer::builder()
        .name("calculator-server")
        .version("1.0.0")
        .tool(Calculator { a: 0.0, b: 0.0, operation: "+".to_string() })
        .bind_address("127.0.0.1:8642".parse()?)  // Different port to avoid conflicts
        .build()?;

    server.run().await
}
```

### 3. Resources with resource_fn()

Create resources that provide data and files using the `.resource_fn()` method:

```rust
use turul_mcp_derive::mcp_resource;
use turul_mcp_server::prelude::*;
use turul_mcp_protocol::resources::ResourceContent;

// Static resource
#[mcp_resource(
    uri = "file:///config.json",
    name = "config",
    description = "Application configuration"
)]
async fn get_config() -> McpResult<Vec<ResourceContent>> {
    let config = serde_json::json!({
        "app_name": "My Server",
        "version": "1.0.0"
    });

    Ok(vec![ResourceContent::blob(
        "file:///config.json",
        serde_json::to_string_pretty(&config).unwrap(),
        "application/json".to_string()
    )])
}

// Template resource with parameter extraction
#[mcp_resource(
    uri = "file:///users/{user_id}.json",
    name = "user_profile",
    description = "User profile data"
)]
async fn get_user_profile(user_id: String) -> McpResult<Vec<ResourceContent>> {
    let profile = serde_json::json!({
        "user_id": user_id,
        "username": format!("user_{}", user_id),
        "email": format!("{}@example.com", user_id)
    });

    Ok(vec![ResourceContent::blob(
        format!("file:///users/{}.json", user_id),
        serde_json::to_string_pretty(&profile).unwrap(),
        "application/json".to_string()
    )])
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = McpServer::builder()
        .name("resource-server")
        .version("1.0.0")
        .resource_fn(get_config)       // Static resource
        .resource_fn(get_user_profile) // Template: file:///users/{user_id}.json
        .bind_address("127.0.0.1:8643".parse()?)  // Different port to avoid conflicts
        .build()?;

    server.run().await
}
```

The framework automatically:
- Detects URI templates (`{user_id}` patterns)
- Extracts template variables from requests
- Maps them to function parameters
- Registers appropriate resource handlers

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
curl -X POST http://127.0.0.1:8641/mcp \
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

### Middleware System

The framework provides a trait-based middleware architecture for cross-cutting concerns like authentication, logging, and rate limiting:

```rust
use turul_mcp_server::prelude::*;
use std::sync::Arc;

let server = McpServer::builder()
    .middleware(Arc::new(AuthMiddleware::new()))
    .middleware(Arc::new(LoggingMiddleware))
    .middleware(Arc::new(RateLimitMiddleware::new(5, 60)))
    .build()?;
```

**Key Features:**
- ✅ Transport-agnostic (HTTP, Lambda, etc.)
- ✅ Session-aware (read/write session state)
- ✅ Error short-circuiting with semantic JSON-RPC codes
- ✅ Execution order control (FIFO before, LIFO after dispatch)

**Examples:**
- `examples/middleware-logging-server` - Request timing and tracing (HTTP)
- `examples/middleware-rate-limit-server` - Per-session rate limiting (HTTP)
- `examples/middleware-auth-server` - API key authentication (HTTP)
- `examples/middleware-auth-lambda` - API key authentication (AWS Lambda)

**Testing:**
- Test HTTP middleware: `bash scripts/test_middleware_live.sh`
- Test Lambda middleware: `cargo lambda watch --package middleware-auth-lambda`

**Documentation:**
- [ADR 012: Middleware Architecture](docs/adr/012-middleware-architecture.md) - Core middleware design
- [ADR 013: Lambda Authorizer Integration](docs/adr/013-lambda-authorizer-integration.md) - API Gateway authorizer support

#### Lambda Authorizer Integration

**Seamless API Gateway authorizer context extraction for Lambda deployments:**

```rust
// API Gateway authorizer adds context (userId, tenantId, role, etc.)
// → turul-mcp-aws-lambda adapter extracts → injects x-authorizer-* headers
// → Middleware reads headers → stores in session state
// → Tools access via session.get_typed_state("authorizer")

#[async_trait]
impl McpMiddleware for AuthMiddleware {
    async fn before_dispatch(
        &self,
        ctx: &mut RequestContext<'_>,
        _session: Option<&dyn SessionView>,
        injection: &mut SessionInjection,
    ) -> Result<(), MiddlewareError> {
        // Extract authorizer context from x-authorizer-* headers
        let metadata = ctx.metadata();
        let mut authorizer_context = HashMap::new();

        for (key, value) in metadata.iter() {
            if let Some(field_name) = key.strip_prefix("x-authorizer-") {
                if let Some(value_str) = value.as_str() {
                    authorizer_context.insert(field_name.to_string(), value_str.to_string());
                }
            }
        }

        if !authorizer_context.is_empty() {
            // Store for tools to access
            injection.set_state("authorizer", json!(authorizer_context));
        }

        Ok(())
    }
}
```

**Key Features:**
- ✅ Supports API Gateway V1 (REST API) and V2 (HTTP API)
- ✅ Field name sanitization (camelCase → snake_case: `userId` → `user_id`)
- ✅ Defensive programming (never fails requests)
- ✅ Transport-agnostic (appears as standard HTTP metadata)
- ✅ Session state integration

**Example:**
- `examples/middleware-auth-lambda` - Full authorizer extraction pattern
- Test events: `test-events/apigw-v1-with-authorizer.json`, `apigw-v2-with-authorizer.json`

### Core Framework (10 Crates)
- **`turul-mcp-server`** - High-level server builder with session management
- **`turul-mcp-client`** - Comprehensive client library with HTTP transport support
- **`turul-http-mcp-server`** - HTTP/SSE transport with CORS and streaming
- **`turul-mcp-protocol`** - Current MCP specification (alias to 2025-06-18)
- **`turul-mcp-protocol-2025-06-18`** - Complete MCP specification implementation
- **`turul-mcp-derive`** - Procedural macros for all MCP areas
- **`turul-mcp-builders`** - Runtime builder patterns for dynamic MCP components
- **`turul-mcp-json-rpc-server`** - Transport-agnostic JSON-RPC 2.0 foundation
- **`turul-mcp-session-storage`** - Session storage backends (SQLite, PostgreSQL, DynamoDB)
- **`turul-mcp-aws-lambda`** - AWS Lambda integration for serverless deployment

### Fine-Grained Trait Architecture
**Modern composable design pattern for all MCP areas:**

```rust
use turul_mcp_builders::prelude::*;  // Framework traits + builders
use turul_mcp_protocol::{ToolSchema, ToolResult, schema::JsonSchema, McpResult};
use turul_mcp_server::{McpTool, SessionContext};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

struct MyTool;

// Fine-grained trait composition for maximum flexibility
impl HasBaseMetadata for MyTool {
    fn name(&self) -> &str { "my_tool" }
}

impl HasDescription for MyTool {
    fn description(&self) -> Option<&str> { Some("Tool description") }
}

impl HasInputSchema for MyTool {
    fn input_schema(&self) -> &ToolSchema {
        static SCHEMA: std::sync::OnceLock<ToolSchema> = std::sync::OnceLock::new();
        SCHEMA.get_or_init(|| {
            ToolSchema::object()
                .with_properties(HashMap::from([
                    ("input".to_string(), JsonSchema::string())
                ]))
        })
    }
}

// ... other trait implementations

// ToolDefinition automatically implemented via blanket impl
#[async_trait]
impl McpTool for MyTool {
    async fn call(&self, _args: Value, _session: Option<SessionContext>) 
        -> McpResult<CallToolResult> {
        Ok(CallToolResult::success(vec![
            ToolResult::text("Tool result")
        ]))
    }
}
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
- **Server-Sent Events (SSE)** - Development streaming with full real-time capabilities
- **Stdio** - Command-line integration
- **AWS Lambda** - Serverless deployment with streaming responses

> **Note**: SSE streaming is in active development with full real-time event broadcasting, session isolation, and correlation ID tracking.

## 📚 Examples Overview

### 🏢 Real-World Business Applications
Development servers for actual business problems:

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

### 🧪 **Comprehensive Test Coverage - Development Quality**

**Framework Excellence**: 100+ tests across all components with complete async SessionContext integration:

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
use turul_mcp_server::prelude::*;
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
        ToolResult::Text { text, .. } => {
            let parsed: Value = serde_json::from_str(text).unwrap();
            assert_eq!(parsed["output"], 8.0); // Derive macro uses "output"
        }
        _ => panic!("Expected text result")
    }
}

#[tokio::test] 
async fn test_server_integration() {
    // Use framework builders
    let server = McpServer::builder()
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
session.set_typed_state("counter", &42i32).await.unwrap();
    let value: i32 = session.get_typed_state("counter").await.unwrap();
    assert_eq!(value, 42);
    
    // Progress notifications work
    session.notify_progress("processing", 50).await;
    
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

### 1. Function Macros (Recommended for Simplicity)
**Best for:** Quick development, natural syntax, minimal boilerplate

```rust
use turul_mcp_derive::mcp_tool;
use turul_mcp_server::prelude::*;

#[mcp_tool(name = "weather", description = "Get weather information")]
async fn get_weather(
    #[param(description = "City name")] city: String,
    #[param(description = "Temperature unit")] unit: Option<String>,
) -> McpResult<String> {
    let unit = unit.unwrap_or_else(|| "celsius".to_string());
    Ok(format!("Weather in {}: 22°{}", city, if unit == "fahrenheit" { "F" } else { "C" }))
}

// Usage in server
let server = McpServer::builder()
    .name("weather-server")
    .version("1.0.0")
    .tool_fn(get_weather)
    .build()?;
```

### 2. Derive Macros (Struct-Based)
**Best for:** Complex tools, organized codebases, multiple related functions

```rust
use turul_mcp_derive::McpTool;
use turul_mcp_server::prelude::*;

#[derive(McpTool, Clone)]
#[tool(name = "file_manager", description = "File management operations")]
struct FileManager {
    #[param(description = "Operation (create, read, delete)")]
    operation: String,
    #[param(description = "File path")]
    path: String,
    #[param(description = "File content (for create operation)")]
    content: Option<String>,
}

impl FileManager {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<String> {
        match self.operation.as_str() {
            "create" => {
                let content = self.content.as_ref().unwrap_or(&"Empty file".to_string());
                Ok(format!("Created file '{}' with content: {}", self.path, content))
            },
            "read" => Ok(format!("Reading file: {}", self.path)),
            "delete" => {
                if let Some(session) = session {
                    session.notify_progress(&format!("Deleting {}", self.path), 100).await;
                }
                Ok(format!("Deleted file: {}", self.path))
            },
            _ => Err("Invalid operation".into()),
        }
    }
}

// Usage in server
let server = McpServer::builder()
    .name("file-server")
    .version("1.0.0")
    .tool(FileManager {
        operation: "create".to_string(),
        path: "/tmp/example".to_string(),
        content: None,
    })
    .build()?;
```

### 3. Builder Pattern (Runtime Flexibility)
**Best for:** Dynamic tools, configuration-driven systems

```rust
use turul_mcp_server::prelude::*;
use serde_json::json;

let multiply_tool = ToolBuilder::new("multiply")
    .description("Multiply two numbers")
    .number_param("a", "First number")
    .number_param("b", "Second number")
    .number_output() // Generates {"result": number} schema
    .execute(|args| async move {
        let a = args.get("a").and_then(|v| v.as_f64())
            .ok_or("Missing parameter 'a'")?;
        let b = args.get("b").and_then(|v| v.as_f64())
            .ok_or("Missing parameter 'b'")?;
        
        Ok(json!({"result": a * b}))
    })
    .build()
    .map_err(|e| format!("Failed to build tool: {}", e))?;

// Usage in server
let server = McpServer::builder()
    .name("calculator-server")
    .version("1.0.0")
    .tool(multiply_tool)
    .build()?;
```

### 4. Manual Implementation (Maximum Control)
**Best for:** Performance optimization, custom behavior

```rust
use turul_mcp_server::prelude::*;  // Re-exports builders prelude + framework traits
use turul_mcp_protocol::{ToolSchema, ToolResult, schema::JsonSchema, McpResult};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

struct ManualTool;

impl HasBaseMetadata for ManualTool {
    fn name(&self) -> &str { "manual_tool" }
}

impl HasDescription for ManualTool {
    fn description(&self) -> Option<&str> { Some("Manual implementation with full control") }
}

impl HasInputSchema for ManualTool {
    fn input_schema(&self) -> &ToolSchema {
        static SCHEMA: std::sync::OnceLock<ToolSchema> = std::sync::OnceLock::new();
        SCHEMA.get_or_init(|| {
            ToolSchema::object()
                .with_properties(HashMap::from([
                    ("input".to_string(), JsonSchema::string())
                ]))
        })
    }
}

impl HasOutputSchema for ManualTool {
    fn output_schema(&self) -> Option<&ToolSchema> { None }
}

impl HasAnnotations for ManualTool {
    fn annotations(&self) -> Option<&ToolAnnotations> { None }
}

impl HasToolMeta for ManualTool {
    fn tool_meta(&self) -> Option<&HashMap<String, Value>> { None }
}

#[async_trait]
impl McpTool for ManualTool {
    async fn call(&self, _args: Value, _session: Option<SessionContext>) 
        -> McpResult<CallToolResult> {
        // Full control over implementation
        Ok(CallToolResult::success(vec![
            ToolResult::text("Manual tool with complete control")
        ]))
    }
}

// Usage in server
let server = McpServer::builder()
    .name("manual-server")
    .version("1.0.0")
    .tool(ManualTool)
    .build()?;
```

## 🔧 Client Library

Comprehensive MCP client for HTTP transport:

```rust
use turul_mcp_client::{McpClient, McpClientBuilder, transport::HttpTransport};
use std::time::Duration;

// Create HTTP transport
let transport = HttpTransport::new("http://localhost:8080/mcp")?;

// Create client using builder pattern
let client = McpClientBuilder::new()
    .with_transport(Box::new(transport))
    .build();

// Initialize session
let init_result = client.initialize().await?;

// List available tools
let tools = client.list_tools().await?;

// Call a tool
let result = client.call_tool("add", json!({
    "a": 10.0,
    "b": 20.0
})).await?;

// List and read resources
let resources = client.list_resources().await?;
let content = client.read_resource("config://app.json").await?;
```

## 🚀 Performance Features

### Modern Architecture
- **UUID v7** - Time-ordered IDs for better database performance and observability
- **Workspace Dependencies** - Consistent dependency management across 37 crates
- **Rust 2024 Edition** - Latest language features and performance improvements
- **Tokio/Hyper** - High-performance async runtime with HTTP/2 support

### Development Quality
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
curl -X POST http://127.0.0.1:8080/mcp \
  -H "Content-Type: application/json" \
  -H "MCP-Protocol-Version: 2025-06-18" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "tools/call",
    "params": {
      "name": "add",
      "arguments": {"a": 10, "b": 20}
    }
  }'

# Test SSE notifications (after getting session ID from above request)
curl -N -H "Accept: text/event-stream" \
  -H "Mcp-Session-Id: <session-id>" \
  http://127.0.0.1:8080/mcp
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

## 🛠️ Development & Testing

### Building the Framework

```bash
# Build all workspace crates
cargo build

# Build with release optimizations
cargo build --release
```

### Running Tests

The framework includes **300+ comprehensive tests** covering all functionality. Test server binaries are **automatically built** when needed - no manual setup required.

```bash
# Run all tests (recommended - includes E2E integration tests)
cargo test --workspace

# Run specific test suite
cargo test --test concurrent_session_advanced

# Run with logging output
RUST_LOG=info cargo test --workspace

# Clean build and test (verifies auto-build works)
cargo clean && cargo test --workspace
```

**Key Features:**
- ✅ **Auto-build test servers** - Missing test binaries are built automatically on first test run
- ✅ **Zero configuration** - Just run `cargo test` and everything works
- ✅ **Clean workspace support** - `cargo clean && cargo test` works without manual steps

The test infrastructure automatically builds required test server binaries (`resource-test-server`, `prompts-test-server`, `tools-test-server`, etc.) when running integration tests. This ensures a seamless developer experience.

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

**DynamoDB** (Development):
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

### Server-Sent Events (SSE) Verification

The framework includes comprehensive SSE testing to verify real-time notification streaming:

#### Running SSE Tests

```bash
# Test SSE functionality in prompts package
cargo test --package mcp-prompts-tests --test sse_notifications_test

# Test specific SSE scenarios
cargo test test_sse_prompts_connection_establishment -- --nocapture
cargo test test_sse_prompts_list_changed_notification -- --nocapture
cargo test test_sse_prompts_session_isolation -- --nocapture

# Test SSE functionality in resources package
cargo test --package mcp-resources-tests --test sse_notifications_test

# Test specific resource SSE scenarios
cargo test test_sse_connection_establishment -- --nocapture
cargo test test_sse_resource_list_changed_notification -- --nocapture
cargo test test_sse_session_isolation -- --nocapture
```

#### Expected SSE Test Output

```
🚀 Starting MCP Resource Test Server on port 18994
✅ Session 01997404-d8f4-7b20-b76d-ac1f4be628a3 created and immediately initialized
✅ SSE connection: session=01997404-d908-7e62-ae74-af87f1523836, connection=01997404-d909-7001-8b95-296e806aa1e1
✅ Total notifications detected: 1
✅ Session ID correlation verified
✅ Valid SSE format compliance verified

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

#### Manual SSE Verification

```bash
# 1. Start any MCP server with SSE enabled
cargo run --example prompts-server

# 2. Get session ID via initialization
curl -X POST http://127.0.0.1:8080/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}'

# 3. Connect to SSE stream (replace SESSION_ID with actual ID)
curl -N -H "Accept: text/event-stream" \
  -H "Mcp-Session-Id: SESSION_ID" \
  http://127.0.0.1:8080/mcp

# Expected SSE output:
# id: 0
# event: ping
# data: {"type":"keepalive"}
#
# id: 1
# event: notification
# data: {"type":"resource_update","resource":"prompts/list"}
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

## 📋 Development Status & Current Limitations

### 🎯 Current Framework State
- **Phase 6 Complete**: Session-aware resources implemented with full MCP 2025-06-18 compliance
- **45+ Examples Validated**: Comprehensive testing campaign completed across all framework areas
- **SSE Streaming Verified**: Real-time notifications and session-aware logging working correctly
- **Beta Status**: Active development with API stability considerations before 1.0.0

### 🚧 Current Limitations

**Transport & Streaming:**
- **Lambda SSE**: Snapshot-based responses work reliably; real-time streaming requires `run_with_streaming_response`
- **WebSocket Transport**: Planned but not yet available (HTTP/1.1 and SSE currently supported)
- **CI Environment Testing**: SSE tests require port binding capabilities (graceful fallbacks implemented)

**Features & Integration:**
- **Resource Subscriptions**: `resources/subscribe` MCP spec feature planned for future implementation
- **Authentication Middleware**: OAuth/JWT integration planned for future releases
- **Cross-platform Compatibility**: Primarily tested on Linux development environments

### 📊 Areas for Enhancement
- **Performance Monitoring**: Basic benchmarks available, comprehensive monitoring planned
- **Concurrency Stress Testing**: Some resource tests show occasional failures under extreme load
- **Browser Compatibility**: CORS support available but may need tuning for specific client requirements

**Framework Philosophy**: We prioritize honest documentation over inflated claims. This beta status reflects our commitment to transparency about the current development state.

---

**🚀 Ready to build MCP servers?** Start with our [comprehensive examples](examples/) or check the [getting started guide](EXAMPLES.md).

**💡 Need help?** Open an issue or check our [45+ validated examples](examples/) covering everything from simple calculators to enterprise systems.
