# MCP Framework Examples Summary

This document provides a comprehensive overview of all 26 examples in the MCP Framework, showcasing the complete range of capabilities from basic tools to advanced features like real-time notifications, AI integration, and secure file system access.

## 📋 Examples Overview Table

| # | Example Name | Port | Primary Purpose | MCP Features | Development Patterns | Key Tools/Handlers |
|---|--------------|------|-----------------|--------------|---------------------|-------------------|
| 1 | **minimal-server** | 8000 | Absolute minimum MCP server setup | Basic tool implementation | Manual trait implementation | `echo` |
| 2 | **calculator-server** | 8764 | Mathematical operations with manual trait implementation | Tools, parameter validation, progress notifications | Manual `McpTool` trait implementation | `add`, `subtract`, `multiply`, `divide` |
| 3 | **macro-calculator** | 8765 | Streamlined calculator using derive macros | Minimal boilerplate with derive macros | Derive macro powered tools | `add`, `subtract` |
| 4 | **derive-macro-server** | 8765 | Simplified tool creation using derive macros | Automatic schema generation, parameter attributes | `#[derive(McpTool)]` macro | `text_transform`, `math_operations`, `validate_data`, `counter`, `geometry` |
| 5 | **function-macro-server** | 8003 | Function-style tool definitions | Function-based tool syntax | `#[mcp_tool]` attribute macro | `add`, `string_repeat`, `boolean_logic`, `greet` |
| 6 | **enhanced-tool-macro-test** | 8010 | Advanced derive macro patterns with validation | Enhanced parameter validation, optional parameters | Advanced `#[derive(McpTool)]` | `enhanced_calculator`, `math_functions` |
| 7 | **tool-macro-example** | 8046 | Declarative tool creation with tool! macro | `tool!` declarative macro, inline definitions | Declarative macro syntax | `divide`, `add`, `greet`, `text_process` |
| 8 | **manual-tools-server** | 8040 | Advanced manual implementation patterns | Complex schemas, session state, progress notifications | Manual trait implementation | `file_operations`, `calculator`, `session_counter`, `progress_task` |
| 9 | **comprehensive-server** | 8002 | Complete MCP server with all supported handlers | All MCP handlers (tools, prompts, resources, templates, completion, logging, notifications, roots, sampling) | Multi-handler architecture | `add`, `multiply` + prompts, resources, templates |
| 10 | **stateful-server** | 8006 | Session-based state management | Session state persistence, real-time notifications | Type-safe state storage | `shopping_cart`, `user_preferences`, `session_info` |
| 11 | **notification-server** | 8005 | Real-time notifications via SSE | Server-sent events, progress tracking, broadcast notifications | SSE integration | `simulate_progress`, `send_notification`, `connection_status` |
| 12 | **version-negotiation-server** | 8049 | Protocol version negotiation demonstration | Version negotiation, backward compatibility | Version negotiation logic | `version_info`, `test_version_negotiation` |
| 13 | **spec-compliant-server** | 8043 | Full MCP 2025-06-18 specification compliance | `_meta` fields, progress tokens, cursor pagination | Specification compliance patterns | `process_data`, `metadata_demo` |
| 14 | **resources-server** | 8041 | Various resource types demonstration | Multiple resource patterns, structured data | Custom resource implementation | `fs://`, `docs://`, `config://`, `schema://`, `status://` resources |
| 15 | **resource-server** | 8045 | Resource handling with derive macros | `#[derive(McpResource)]` macro, multiple content types | Derive macro resources | `config`, `system`, `user` resources |
| 16 | **resource-macro-example** | 8047 | Declarative resource creation with inline closures | `resource!` declarative macro, dynamic content | Inline content generation | `config`, `status`, `data` resources |
| 17 | **dynamic-resource-server** | 8048 | Dynamic resources with parameterized URIs | Dynamic URI patterns, multiple content types | Parameterized resource access | `users://`, `products://`, `documents://`, `orders://` resources |
| 18 | **prompts-server** | 8040 | Dynamic prompt generation for AI interactions | Prompts protocol, dynamic generation | Custom prompt implementation | `code-generation`, `documentation`, `code-review`, `debugging`, `architecture-design` |
| 19 | **completion-server** | 8042 | AI-assisted text completion with context-aware suggestions | Completion endpoint, intelligent filtering | Custom completion handler | Language, extension, command, framework completions |
| 20 | **logging-server** | 8043 | Dynamic log level management | Logging protocol, level management | Enhanced logging handler | `generate_logs`, `view_logs`, `log_config` |
| 21 | **pagination-server** | 8044 | Cursor-based pagination for large datasets | Cursor-based pagination, MCP 2025-06-18 `_meta` fields | Advanced pagination patterns | `list_users`, `search_users`, `batch_process` |
| 22 | **roots-server** | 8050 | Root directory management and file system security | Roots protocol, security boundaries, access control | Root directory configuration | `list_roots`, `inspect_root`, `simulate_file_operation`, `demonstrate_root_security` |
| 23 | **sampling-server** | 8051 | AI model sampling through MCP | Sampling protocol, AI model integration | AI sampling requests | `basic_sampling`, `conversational_sampling`, `code_generation_sampling`, `creative_writing_sampling` |
| 24 | **elicitation-server** | 8053 | Structured user input collection via JSON Schema | Elicitation protocol, interactive forms, progress tracking | `ElicitationBuilder` utility | `simple_text_input`, `number_input_validation`, `choice_selection`, `confirmation_dialog`, `complex_form`, `progress_tracking`, `error_handling` |
| 25 | **performance-testing** | 8080 | Comprehensive performance testing suite | Load testing, stress testing, benchmarking | Performance analysis tools | Multiple performance testing tools and benchmarks |
| 26 | **lambda-mcp-server** | N/A (Lambda) | Serverless MCP server with dual event sources | Streaming responses, session persistence, SQS integration | AWS Lambda serverless architecture | `aws_real_time_monitor`, `publish_test_event`, `lambda_diagnostics` |

## 🏗️ Development Approaches Showcased

### 1. **Manual Implementation** (Most Control)
- **Examples**: minimal-server, calculator-server, manual-tools-server
- **Use Case**: Full control over tool behavior, complex schemas, custom validation
- **Pattern**: Direct `McpTool` trait implementation

```rust
#[async_trait]
impl McpTool for MyTool {
    fn name(&self) -> &str { "my_tool" }
    fn description(&self) -> &str { "Description" }
    fn input_schema(&self) -> ToolSchema { /* custom schema */ }
    async fn call(&self, args: Value, session: Option<SessionContext>) -> McpResult<Vec<ToolResult>> {
        // Custom implementation
    }
}
```

### 2. **Derive Macros** (Balanced)
- **Examples**: derive-macro-server, macro-calculator, enhanced-tool-macro-test
- **Use Case**: Automatic schema generation with struct-based parameters
- **Pattern**: `#[derive(McpTool)]` with struct fields

```rust
#[derive(McpTool, Clone)]
#[tool(name = "add", description = "Add two numbers")]
struct AddTool {
    #[param(description = "First number")]
    a: f64,
    #[param(description = "Second number")]
    b: f64,
}
```

### 3. **Function Macros** (Natural Syntax)
- **Examples**: function-macro-server
- **Use Case**: Function-based tool definitions with parameter attributes
- **Pattern**: `#[mcp_tool]` attribute on async functions

```rust
#[mcp_tool(name = "add", description = "Add two numbers")]
async fn add(
    #[param(description = "First number")] a: f64,
    #[param(description = "Second number")] b: f64,
) -> McpResult<String> {
    Ok(format!("{} + {} = {}", a, b, a + b))
}
```

### 4. **Declarative Macros** (Ultra-Concise)
- **Examples**: tool-macro-example, resource-macro-example
- **Use Case**: Inline tool/resource creation with minimal boilerplate
- **Pattern**: `tool!` and `resource!` macros

```rust
let add_tool = tool! {
    name: "add",
    description: "Add two numbers",
    params: {
        a: f64 => "First number",
        b: f64 => "Second number",
    },
    execute: |a, b| async move {
        Ok(format!("{} + {} = {}", a, b, a + b))
    }
};
```

## 🚀 MCP Protocol Features Demonstrated

### Core Features
- **✅ Tools** - All examples demonstrate tool implementation
- **✅ Resources** - Static and dynamic content serving (examples 14-17)
- **✅ Prompts** - AI interaction templates (example 18)
- **✅ Completion** - Text completion and suggestions (example 19)
- **✅ Logging** - Dynamic log level management (example 20)
- **✅ Notifications** - Real-time SSE updates (example 11)
- **✅ Roots** - File system security boundaries (example 22)
- **✅ Sampling** - AI model integration (example 23)
- **✅ Elicitation** - Structured user input collection (example 24)

### Advanced Features
- **✅ Session Management** - State persistence across requests (examples 10, 11)
- **✅ Progress Tracking** - Real-time operation monitoring (examples 11, 24)
- **✅ Pagination** - Cursor-based navigation for large datasets (example 21)
- **✅ Version Negotiation** - Protocol compatibility handling (example 12)
- **✅ _meta Fields** - MCP 2025-06-18 specification compliance (example 13)
- **✅ Performance Testing** - Load and stress testing capabilities (example 25)

## 🔧 Quick Start Commands

### Running Individual Examples

```bash
# Basic examples
cargo run -p minimal-server          # Port 8000
cargo run -p calculator-server       # Port 8764
cargo run -p derive-macro-server      # Port 8765

# Advanced features
cargo run -p stateful-server         # Port 8006 - Session management
cargo run -p notification-server     # Port 8005 - Real-time notifications
cargo run -p elicitation-server      # Port 8053 - Interactive forms

# MCP protocol features
cargo run -p roots-server            # Port 8050 - File system security
cargo run -p sampling-server         # Port 8051 - AI integration
cargo run -p completion-server       # Port 8042 - Text completion

# All-in-one demonstration
cargo run -p comprehensive-server    # Port 8002 - All features combined
```

### Testing Examples

```bash
# Basic functionality test
curl -X POST http://127.0.0.1:8000/mcp \
  -H "Content-Type: application/json" \
  -d '{"method": "tools/call", "params": {"name": "echo", "arguments": {"message": "Hello MCP!"}}}'

# Performance testing
cargo run -p performance-testing --bin performance_client -- throughput --requests-per-second 100
cargo run -p performance-testing --bin stress_test -- memory --max-concurrent 50

# Lambda serverless testing
cd examples/lambda-mcp-server
cargo lambda watch  # Local development
cargo lambda invoke --data-file events/api_gateway_event.json  # Test HTTP events
```

## 🎯 Use Case Recommendations

### For Learning MCP Development
1. **Start**: `minimal-server` → `calculator-server` → `derive-macro-server`
2. **Advance**: `stateful-server` → `notification-server` → `comprehensive-server`
3. **Specialize**: Choose examples based on specific needs (AI, file systems, etc.)

### For Production Development
- **Simple Tools**: Use `derive-macro-server` patterns
- **Complex Logic**: Use `manual-tools-server` patterns  
- **AI Integration**: Use `sampling-server` + `completion-server` patterns
- **File Operations**: Use `roots-server` patterns
- **User Interaction**: Use `elicitation-server` patterns
- **Real-time Updates**: Use `notification-server` patterns

### For Testing and Validation
- **Load Testing**: `performance-testing` suite
- **Protocol Compliance**: `spec-compliant-server`
- **Version Compatibility**: `version-negotiation-server`

## 📚 Framework Capabilities Summary

The MCP Framework provides:

### **🔨 Developer Experience**
- **4 Development Approaches** - From manual to fully automated
- **Comprehensive Macros** - Derive, function, and declarative macros
- **Type Safety** - Full Rust type system integration
- **Error Handling** - Structured error types with descriptive messages

### **🌐 Protocol Support**
- **MCP 2025-06-18 Compliant** - Latest specification support
- **All Standard Endpoints** - Complete protocol implementation
- **Extension Points** - Custom handlers and middleware
- **Version Negotiation** - Backward compatibility support

### **⚡ Production Features**
- **Session Management** - Stateful operations with automatic cleanup
- **Real-time Notifications** - SSE-based event streaming
- **Performance Monitoring** - Built-in metrics and benchmarking
- **Security** - File system access controls and validation

### **🧪 Testing & Validation**
- **Comprehensive Test Suite** - Unit, integration, and performance tests
- **26 Working Examples** - Production-ready reference implementations
- **Benchmarking Tools** - Load testing and stress testing capabilities  
- **Protocol Validation** - Specification compliance verification
- **Serverless Testing** - AWS Lambda development and deployment tools

This comprehensive example suite demonstrates that the MCP Framework is ready for production use, supporting everything from simple calculators to complex, real-time AI-integrated applications with advanced features like secure file access, session management, and performance monitoring.