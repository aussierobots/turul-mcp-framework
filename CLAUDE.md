# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is the **mcp-framework** - a standalone, production-ready Rust framework for building Model Context Protocol (MCP) servers. This framework is designed to eventually supersede previous MCP implementations with a clean, modular architecture.

### Key Features
- **Complete MCP 2025-06-18 Specification Support**: Full protocol compliance with latest features
- **Streamable HTTP Transport**: Integrated SSE support for real-time notifications
- **Session Management**: UUID v7-based sessions with automatic cleanup
- **Rich Trait System**: Comprehensive trait coverage for all MCP operations
- **Derive Macros**: Automatic tool generation with schema validation
- **Multi-Transport Support**: HTTP, WebSocket, and future transport layers

## Architecture

### Core Crates Structure
```
mcp-framework/
├── crates/
│   ├── mcp-protocol-2025-06-18/  # Protocol types and traits
│   ├── mcp-server/               # High-level server framework
│   ├── http-mcp-server/         # HTTP transport layer
│   ├── json-rpc-server/         # JSON-RPC dispatch
│   └── mcp-derive/              # Procedural macros
└── examples/                    # Example servers
```

### Session Management
- **UUID Version**: Always use UUID v7 (`Uuid::now_v7()`) for session IDs - provides temporal ordering and better performance
- **Session Cleanup**: Automatic cleanup every 60 seconds, 30-minute expiry
- **SSE Integration**: Sessions provide broadcast channels for real-time notifications

### MCP Protocol Version Support
- **V2024_11_05**: Basic MCP without streamable HTTP
- **V2025_03_26**: Streamable HTTP support 
- **V2025_06_18**: Full feature set with _meta, cursor, progressToken, elicitation

## Import Conventions

**CRITICAL**: Always use `mcp_protocol` alias for imports:
```rust
// ✅ CORRECT
use mcp_protocol::resources::{HasResourceMetadata, ResourceDefinition};

// ❌ WRONG  
use mcp_protocol_2025_06_18::resources::{HasResourceMetadata, ResourceDefinition};
```

The `mcp_protocol` crate is an alias to `mcp_protocol_2025_06_18` but provides future-proofing and consistency across the framework.

## MCP Trait-Based Architecture Pattern

### Core Design Pattern

This framework follows a consistent trait-based architecture pattern across all MCP specification implementations:

#### Pattern: Concrete Struct + Trait Interface

1. **Fine-Grained Traits**: Break down TypeScript interfaces into focused, composable Rust traits
2. **Composed Definition Trait**: Combine fine-grained traits into complete definition interfaces  
3. **Concrete Struct Implementation**: Protocol structs implement the definition traits
4. **Dynamic Implementation**: Runtime implementations also implement the same definition traits
5. **Framework Interface Usage**: All framework code uses trait interfaces, not concrete types

#### Example: Tool Implementation

```rust
// 1. Fine-grained traits matching TypeScript fields
pub trait HasBaseMetadata {
    fn name(&self) -> &str;
    fn title(&self) -> Option<&str> { None }
}

pub trait HasInputSchema {
    fn input_schema(&self) -> &ToolSchema;
}

// 2. Composed definition trait
pub trait ToolDefinition: 
    HasBaseMetadata + HasInputSchema + /* ... */ + Send + Sync 
{
    // Convenience methods using composed traits
    fn display_name(&self) -> &str {
        // TypeScript spec: Tool.title > annotations.title > Tool.name
        if let Some(title) = self.title() {
            title
        } else if let Some(annotations) = self.annotations() {
            if let Some(title) = &annotations.title {
                title
            } else {
                self.name()
            }
        } else {
            self.name()
        }
    }
}

// 3. Concrete struct implements all traits
impl HasBaseMetadata for Tool {
    fn name(&self) -> &str { &self.name }
    fn title(&self) -> Option<&str> { self.title.as_deref() }
}
// Tool automatically implements ToolDefinition via trait composition

// 4. Dynamic implementations also implement the same traits
impl HasBaseMetadata for CalculatorTool {
    fn name(&self) -> &str { "calculator" }
    fn title(&self) -> Option<&str> { Some("Calculator") }
}
// CalculatorTool automatically implements ToolDefinition via trait composition

// 5. Framework uses trait interfaces everywhere
fn process_tool(tool: &dyn ToolDefinition) {
    println!("Tool: {}", tool.display_name());
}
```

### Apply This Pattern To All MCP Areas

This pattern must be applied consistently across:

- **Tools**: `Tool` struct + `ToolDefinition` trait + `McpTool` trait
- **Resources**: `Resource` struct + `ResourceDefinition` trait + `McpResource` trait  
- **Prompts**: `Prompt` struct + `PromptDefinition` trait + `McpPrompt` trait
- **Sampling**: Message types + definition traits + handler traits
- **Completion**: Completion types + definition traits + provider traits

### Benefits of This Pattern

1. **TypeScript Alignment**: Perfect 1:1 mapping with TypeScript interfaces
2. **Unified Interface**: Same trait methods for concrete and dynamic implementations
3. **Framework Consistency**: All code uses trait interfaces, enabling polymorphism
4. **Composability**: Fine-grained traits can be mixed and matched
5. **Testability**: Easy to mock trait interfaces for testing
6. **Performance**: Trait dispatch vs runtime type checking
7. **Extensibility**: Add new fine-grained traits and compose automatically

### Implementation Checklist

When implementing any MCP specification area:

- [ ] Analyze TypeScript interface structure
- [ ] Create fine-grained `HasXxx` traits for each field/group
- [ ] Create composed `XxxDefinition` trait combining fine-grained traits
- [ ] Implement all traits for concrete protocol struct
- [ ] Ensure dynamic implementations use same trait interface
- [ ] Update framework code to use trait interfaces only
- [ ] Add trait-based helper methods and builders
- [ ] Test both concrete and dynamic implementations through same interface

### Code Review Guidelines

- All protocol structs MUST implement their corresponding definition traits
- Framework code MUST use `&dyn XxxDefinition` interfaces, not concrete types
- Derive macros MUST generate trait implementations, not struct methods
- Helper functions MUST accept trait parameters: `fn helper(item: &dyn ItemDefinition)`
- Tests MUST verify both concrete and dynamic implementations work identically

### MCP TypeScript Specification Compliance
**CRITICAL**: All types in `mcp-protocol-2025-06-18` crate MUST exactly match the MCP TypeScript Schema specification. This includes:
- **Request Pattern**: Every MCP request type must follow `XxxRequest { method, params: XxxParams }` pattern
- **Params Pattern**: Every params type includes method-specific fields PLUS optional `_meta` field  
- **Response Pattern**: Every response type includes method-specific fields PLUS optional top-level `_meta` field
- **Field Naming**: All fields must use exact camelCase names from TypeScript schema
- **Optional Fields**: Use `Option<T>` with proper `skip_serializing_if` attributes for optional TypeScript fields
- **Inheritance**: Rust structs must replicate TypeScript interface inheritance via composition
- **Trait Implementation**: ALL request/response types MUST implement corresponding traits from `traits.rs` for compile-time specification compliance

**Example Pattern**:
```rust
// Matches: export interface CallToolRequest extends Request
pub struct CallToolRequest {
    pub method: String,  // "tools/call" from Request.method
    pub params: CallToolParams,  // from Request.params
}

// Matches: params: { name: string; arguments?: {...}; _meta?: {...} }
pub struct CallToolParams {
    pub name: String,
    pub arguments: Option<HashMap<String, Value>>,
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<HashMap<String, Value>>,
}

// MUST implement traits for compile-time compliance
impl Params for CallToolParams {}
impl HasCallToolParams for CallToolParams { /* trait methods */ }
impl CallToolRequest for CallToolRequest { /* trait methods */ }
```

## Build Commands

### Standard Development
```bash
# Build entire workspace
cargo build

# Check compilation
cargo check

# Run tests
cargo test

# Format and lint
cargo fmt
cargo clippy

# Run specific example
cargo run --example minimal-server
```

### HTTP Transport Testing
```bash
# Test with curl (initialize)
curl -X POST http://127.0.0.1:8000/mcp \
  -H "Content-Type: application/json" \
  -H "MCP-Protocol-Version: 2025-06-18" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}'

# Test SSE connection (requires session ID from initialize)  
curl -N -H "Accept: text/event-stream" \
  -H "Mcp-Session-Id: <session-id>" \
  http://127.0.0.1:8000/mcp
```

## MCP TypeScript Specification Compliance

The MCP framework now fully implements the MCP TypeScript specification (2025-06-18) with comprehensive trait-based validation and compile-time compliance checking.

### Request/Params Pattern
All MCP requests follow the TypeScript schema pattern:
```rust
// TypeScript: { method: string, params: { ...fields, _meta?: {...} } }
pub struct CallToolRequest {
    pub method: String,  // "tools/call"
    pub params: CallToolParams,
}

pub struct CallToolParams {
    pub name: String,
    pub arguments: Option<Value>,
    #[serde(rename = "_meta")]
    pub meta: Option<HashMap<String, Value>>,
}
```

### Notification Pattern
Notifications follow the TypeScript pattern:
```rust
// TypeScript: { method: string, params?: { _meta?: {...}, [key: string]: unknown } }
pub struct ResourcesListChangedNotification {
    pub method: String,  // "notifications/resources/listChanged"
    pub params: Option<NotificationParams>,
}
```

### Trait-Based Validation
All types implement corresponding traits for compile-time specification compliance:
- `HasMethod`, `HasParams`, `HasMetaParam` for requests
- `HasData`, `HasMeta` for responses  
- `JsonRpcRequestTrait`, `JsonRpcNotificationTrait`, `JsonRpcResponseTrait` for JSON-RPC

### Testing Compliance
Run the MCP TypeScript specification compliance tests:
```bash
cargo test --package mcp-protocol-2025-06-18 compliance_test::tests
```

## Four-Level Tool Creation Spectrum

This framework provides four distinct approaches for creating MCP tools, ordered by increasing complexity but decreasing boilerplate. Choose the level that matches your needs:

### Level 1: Function Macros (Ultra-Simple)
**Best for**: Quick prototypes, simple tools, learning MCP
**Boilerplate**: ~5 lines of code

```rust
use mcp_server::{McpServer, McpResult};
use mcp_derive::mcp_tool;

#[mcp_tool(name = "calculator_add", description = "Add two numbers")]
async fn calculator_add(
    #[param(description = "First number")] a: f64,
    #[param(description = "Second number")] b: f64,
) -> McpResult<f64> {
    Ok(a + b)
}

// With custom output field name
#[mcp_tool(name = "calculator_sum", description = "Add numbers", output_field = "sum")]
async fn calculator_sum(
    #[param(description = "First number")] a: f64,
    #[param(description = "Second number")] b: f64,
) -> McpResult<f64> {
    Ok(a + b)  // Returns {"sum": 7.0} instead of {"result": 7.0}
}

// Usage: Simply pass the function name to the server
let server = McpServer::builder()
    .tool_fn(calculator_add)  // Original function name!
    .tool_fn(calculator_sum)
    .build()?;
```

**Key Features**:
- Automatic output schema generation from `McpResult<T>` return types
- Intuitive usage with original function names
- Parameter validation and extraction handled automatically
- Structured content wrapping for MCP Inspector compliance
- **Custom output field names** with `output_field` parameter

### Level 2: Derive Macros (Struct-Based) 
**Best for**: Complex tools with multiple related functions, organized codebases
**Boilerplate**: ~15 lines of code

```rust
use mcp_derive::McpTool;
use mcp_server::{McpResult, McpTool};

#[derive(McpTool)]
#[tool(name = "calculator_add_derive", description = "Add two numbers using derive")]
struct CalculatorAddTool {
    #[param(description = "First number")]
    a: f64,
    #[param(description = "Second number")]  
    b: f64,
}

impl CalculatorAddTool {
    async fn execute(&self) -> McpResult<f64> {
        Ok(self.a + self.b)
    }
}

// Usage: Create instance and add to server
let server = McpServer::builder()
    .tool(CalculatorAddTool::default())
    .build()?;
```

**Key Features**:
- Struct-based organization enables complex tools with helper methods
- Automatic trait implementations for all MCP tool interfaces
- Parameter validation and schema generation from struct fields
- Clean separation between data and logic

### Level 3: Builder Pattern (Runtime Flexibility)
**Best for**: Dynamic tools, configuration-driven systems, runtime tool creation
**Boilerplate**: ~20 lines of code

```rust
use mcp_server::{McpServer, ToolBuilder};
use serde_json::json;

let add_tool = ToolBuilder::new("calculator_add_builder")
    .description("Add two numbers using builder pattern")
    .number_param("a", "First number")
    .number_param("b", "Second number") 
    .number_output() // Generates {"result": number} schema
    .execute(|args| async move {
        let a = args.get("a").and_then(|v| v.as_f64())
            .ok_or("Missing parameter 'a'")?;
        let b = args.get("b").and_then(|v| v.as_f64())
            .ok_or("Missing parameter 'b'")?;
        
        let sum = a + b;
        Ok(json!({"result": sum}))
    })
    .build()?;

// Usage: Add built tool to server
let server = McpServer::builder()
    .tool(add_tool)
    .build()?;
```

**Key Features**:
- Construct tools entirely at runtime without procedural macros
- Perfect for configuration-driven or plugin-based systems
- Fluent API for schema definition and parameter validation
- Type-safe execution with automatic error handling

### Level 4: Manual Implementation (Maximum Control)
**Best for**: Performance optimization, custom behavior, learning framework internals
**Boilerplate**: ~25 lines of code

```rust
use mcp_server::{McpServer, McpTool, McpResult, SessionContext};
use mcp_protocol::tools::{
    ToolResult, CallToolResponse, ToolSchema,
    HasBaseMetadata, HasDescription, HasInputSchema, 
    HasOutputSchema, HasAnnotations, HasToolMeta
};
use async_trait::async_trait;

#[derive(Clone)]
struct CalculatorAddTool;

// Minimal manual trait implementations - no helpers, no stored metadata
impl HasBaseMetadata for CalculatorAddTool {
    fn name(&self) -> &str { "calculator_add_manual" }
    fn title(&self) -> Option<&str> { Some("Manual Calculator") }
}

impl HasDescription for CalculatorAddTool {
    fn description(&self) -> Option<&str> { 
        Some("Add two numbers (Level 4 - Manual Implementation)")
    }
}

impl HasInputSchema for CalculatorAddTool {
    fn input_schema(&self) -> &ToolSchema {
        // Static schema - no dynamic construction
        static INPUT_SCHEMA: std::sync::OnceLock<ToolSchema> = std::sync::OnceLock::new();
        INPUT_SCHEMA.get_or_init(|| {
            ToolSchema::object()
                .with_properties(HashMap::from([
                    ("a".to_string(), JsonSchema::number()),
                    ("b".to_string(), JsonSchema::number()),
                ]))
                .with_required(vec!["a".to_string(), "b".to_string()])
        })
    }
}

impl HasOutputSchema for CalculatorAddTool {
    fn output_schema(&self) -> Option<&ToolSchema> { None } // No validation
}

impl HasAnnotations for CalculatorAddTool {
    fn annotations(&self) -> Option<&ToolAnnotations> { None }
}

impl HasToolMeta for CalculatorAddTool {
    fn tool_meta(&self) -> Option<&HashMap<String, Value>> { None }
}

#[async_trait]
impl McpTool for CalculatorAddTool {
    async fn call(&self, args: Value, _session: Option<SessionContext>) -> McpResult<CallToolResponse> {
        // Manual parameter extraction - no helper methods
        let a = args.get("a").and_then(|v| v.as_f64())
            .ok_or_else(|| McpError::missing_param("a"))?;
        let b = args.get("b").and_then(|v| v.as_f64())
            .ok_or_else(|| McpError::missing_param("b"))?;
        
        // Simple text response - no structured content
        Ok(CallToolResponse::success(vec![
            ToolResult::text(format!("Sum: {}", a + b))
        ]))
    }
}

// Usage: Zero-sized type, no constructor needed
let server = McpServer::builder()
    .tool(CalculatorAddTool)
    .build()?;
```

**Key Features**:
- Hardcoded trait implementations with no helpers or builders
- Static schema definitions for optimal performance
- Raw parameter extraction and error handling
- Direct response construction without validation
- Zero-sized struct with minimal memory footprint

### Choosing the Right Level

| Level | Use Case | Development Time | Runtime Performance | Flexibility |
|-------|----------|------------------|-------------------|-------------|
| 1 - Function | Prototypes, simple tools | Fastest (~5 lines) | Good | Limited |
| 2 - Derive | Organized codebases | Fast (~15 lines) | Good | Moderate |
| 3 - Builder | Dynamic/config systems | Moderate (~20 lines) | Good | High |
| 4 - Manual | Performance critical, learning | Moderate (~25 lines) | Best | Maximum |

**Key Differences**:
- **Level 1**: Framework does everything automatically
- **Level 2**: Framework + your struct definition  
- **Level 3**: Runtime flexibility without procedural macros
- **Level 4**: Manual trait implementations, static schemas, direct control

### Architecture Consistency

All four levels implement the same trait-based architecture:
- **ToolDefinition trait**: Automatic composition from fine-grained traits
- **McpTool trait**: Uniform execution interface
- **Smart response builders**: Automatic structured content handling
- **MCP Inspector compliance**: Proper JSON schema validation

This ensures that tools created at any level work seamlessly together in the same server and provide identical behavior to MCP clients.

## 🚧 **Trait Architecture Refactoring Status**

**⚠️ IMPORTANT:** The MCP framework is currently undergoing comprehensive trait refactoring to apply the successful fine-grained pattern from tools to ALL MCP areas (resources, prompts, sampling, etc.).

**Current Status:** See detailed progress and tasks in → **[TODO_traits_refactor.md](TODO_traits_refactor.md)**

**Note:** This file (`TODO_traits_refactor.md`) serves as working memory during the refactoring process and should be **removed** when all trait refactoring is completed and all areas consistently follow the fine-grained trait architecture pattern.

## Key Implementation Guidelines

### UUID Usage
- **ALWAYS use UUID v7**: `Uuid::now_v7()` for all session IDs and temporal identifiers
- **Never use UUID v4**: Provides no temporal ordering benefits

### Error Handling  
- Use `JsonRpcError` framework types instead of custom errors
- Proper MCP error codes per specification
- Structured error responses with detailed context

### Session Architecture
- Single HTTP endpoint `/mcp` with method-based routing
- GET + `Accept: text/event-stream` + `Mcp-Session-Id` = SSE connection
- POST = JSON-RPC requests
- DELETE = Session cleanup
- OPTIONS = CORS preflight

### Protocol Version Detection
- Extract `MCP-Protocol-Version` header
- Feature flags based on protocol capabilities
- Graceful fallback to latest supported version

## Testing with MCP Inspector
The framework must return structured JSON data, not generic "Tool Result: Success" messages:

```bash
# Good: Returns actual JSON data structure
{"result": {"value": 42, "message": "calculated"}}

# Bad: Generic success message  
"Tool Result: Success"
```

## Recent Architecture Changes
- **Removed separate SSE manager**: Integrated directly into session management
- **Protocol version detection**: Automatic feature flag selection
- **Streamable HTTP focus**: SSE as integral part, not separate transport
- **Trait system overhaul**: Rich functional traits vs empty marker traits
- **UUID v7 adoption**: All session IDs use temporal UUIDs for better ordering

## Development Standards
- **No GPS project references**: This is a standalone framework
- **Clean separation**: Each crate has focused responsibilities  
- **Future-proof design**: Architecture supports additional transports
- **Production ready**: Performance, security, and reliability first

## Testing Strategy
- Unit tests for all core functionality
- Integration tests with MCP Inspector
- Protocol compliance testing across all supported versions
- Performance benchmarks for session management and SSE streaming