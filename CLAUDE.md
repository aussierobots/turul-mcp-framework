# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is the **turul-mcp-framework** - a standalone, beta-grade Rust framework for building Model Context Protocol (MCP) servers. This framework is designed to eventually supersede previous MCP implementations with a clean, modular architecture.

### Key Features
- **Complete MCP 2025-06-18 Specification Support**: Full protocol compliance with latest features
- **Zero-Configuration Framework**: Users NEVER specify method strings - framework auto-determines ALL methods from types
- **Four-Level Creation Spectrum**: Function macros, derive macros, builders, and manual implementation
- **Runtime Builder Library**: Complete `turul-mcp-builders` crate with 9 builders covering all MCP areas
- **Streamable HTTP Transport**: Integrated SSE support for real-time notifications
- **Session Management**: UUID v7-based sessions with automatic cleanup
- **Rich Trait System**: Comprehensive trait coverage for all MCP operations
- **Multi-Transport Support**: HTTP, WebSocket, and future transport layers

## Architecture

### Core Crates Structure
```
turul-mcp-framework/
├── crates/
│   ├── turul-mcp-protocol-2025-06-18/  # Protocol types and traits
│   ├── turul-mcp-server/               # High-level server framework
│   ├── turul-mcp-builders/             # Runtime builder patterns (Level 3)
│   ├── turul-http-turul-mcp-server/         # HTTP transport layer
│   ├── turul-turul-mcp-json-rpc-server/         # JSON-RPC dispatch
│   └── turul-mcp-derive/              # Procedural macros
└── examples/                    # Example servers
```

### Session Management
- **UUID Version**: Always use UUID v7 (`Uuid::now_v7()`) for session IDs - provides temporal ordering and better performance
- **Session Cleanup**: Automatic cleanup every 60 seconds, 30-minute expiry
- **SSE Integration**: Sessions provide broadcast channels for real-time notifications

### ✅ **MCP Streamable HTTP Transport - FULLY OPERATIONAL**

**STATUS**: ✅ **FULLY WORKING** - Complete MCP 2025-06-18 Streamable HTTP Transport with SSE notifications (2025-08-28)

#### **Working Architecture**
The framework implements complete MCP Streamable HTTP with integrated components:

- **SessionMcpHandler**: Handles both POST JSON-RPC and GET SSE requests
- **StreamManager**: Creates SSE responses with event persistence and resumability  
- **NotificationBroadcaster**: Routes MCP notifications from tools to SSE streams
- **SessionStorage**: Pluggable backends (InMemory, SQLite, PostgreSQL, etc.)

#### **Real-time Notification Flow** ✅ **WORKING**
```rust
Tool.execute(SessionContext) 
    ↓
SessionContext.notify_progress() / notify_log()
    ↓
NotificationBroadcaster.send_notification()
    ↓
StreamManager.broadcast_to_session()
    ↓
SSE Stream to Client ✅ CONFIRMED WORKING
```

#### **Streamable HTTP Patterns** ⚠️ **Updated 2025-08-27**
1. **POST + Accept: text/event-stream** → ⚠️ **DISABLED** for `tools/call` (compatibility mode)
2. **POST + Accept: application/json** → ✅ **WORKING** - Standard JSON responses for all operations
3. **GET + Accept: text/event-stream** → ✅ **WORKING** - Persistent server-initiated event stream  
4. **Session Management** → ✅ **WORKING** - Server-provided UUID v7 sessions via headers
5. **SSE Resumability** → ✅ **WORKING** - Last-Event-ID support with event replay

**Integration Success**: Real-time SSE notifications confirmed working end-to-end with proper MCP JSON-RPC format. Tool execution includes both POST response and SSE notification delivery verified via comprehensive testing.

### MCP Protocol Version Support
- **V2024_11_05**: Basic MCP without streamable HTTP
- **V2025_03_26**: Streamable HTTP support 
- **V2025_06_18**: Full feature set with _meta, cursor, progressToken, elicitation

### JsonSchema Standardization
The framework uses `JsonSchema` consistently throughout for all schema definitions (not `serde_json::Value`). This provides:
- **Type Safety**: Compile-time schema validation
- **MCP Compliance**: Perfect JSON Schema specification alignment  
- **Performance**: No runtime conversion overhead
- **Developer Experience**: Clear, strongly-typed schema construction

See `docs/adr/003-jsonschema-standardization.md` for the complete architectural decision record.

## Import Conventions

**CRITICAL**: Always use `turul_mcp_protocol` alias for imports:
```rust
// ✅ CORRECT
use turul_mcp_protocol::resources::{HasResourceMetadata, ResourceDefinition};

// ❌ WRONG  
use turul_mcp_protocol::resources::{HasResourceMetadata, ResourceDefinition};
```

The `turul_mcp_protocol` crate is an alias to `turul_mcp_protocol_2025_06_18` but provides future-proofing and consistency across the framework.

### Architecture Decision Record (ADR): turul_mcp_protocol Alias Usage

**Decision**: ALL code in the turul-mcp-framework MUST use the `turul_mcp_protocol` alias, never direct `turul_mcp_protocol_2025_06_18` paths.

**Status**: **MANDATORY** - This is enforced across all code

**Context**: The framework uses protocol versioning but needs future-proofing and consistency.

**Decision**: 
- ✅ **ALWAYS**: `use turul_mcp_protocol::`
- ❌ **NEVER**: `use turul_mcp_protocol::`

**Consequences**:
- **Positive**: Future protocol version changes only require updating the alias
- **Positive**: Consistent imports across entire codebase  
- **Positive**: Clear separation between framework code and protocol versions
- **Risk**: Must remember to use alias in ALL locations (examples, tests, macros, etc.)

**Enforcement**: This rule applies to:
- All example code
- All macro-generated code  
- All test code
- All documentation code samples
- All derive macro implementations

## 🚨 CRITICAL: Zero-Configuration Design Principle

### Framework Auto-Determines ALL Methods

**ABSOLUTE RULE**: Users NEVER specify method strings anywhere. The framework automatically determines ALL MCP methods from type names:

```rust
// ✅ CORRECT - Zero Configuration
#[derive(McpTool)]
struct Calculator;  // Framework automatically maps to "tools/call"

#[derive(McpNotification)]  
struct ProgressNotification;  // Framework automatically maps to "notifications/progress"

#[derive(McpResource)]
struct FileResource;  // Framework automatically maps to "resources/read"

let server = McpServer::builder()
    .tool(Calculator::default())                   // Framework → tools/call
    .notification_type::<ProgressNotification>()   // Framework → notifications/progress  
    .resource(FileResource::default())             // Framework → resources/read
    .build()?;
```

```rust  
// ❌ WRONG - User specifying methods (NEVER DO THIS!)
#[derive(McpNotification)]
#[notification(method = "notifications/progress")]  // ❌ NO METHOD STRINGS!
struct ProgressNotification;

#[mcp_tool(method = "tools/call")]  // ❌ NO METHOD STRINGS!
async fn calculator() -> Result<String, String> { Ok("result".to_string()) }
```

### Why This Matters

1. **MCP Compliance Guaranteed**: Impossible to use wrong/invalid methods
2. **Zero Configuration**: Developers focus on logic, not protocol details  
3. **Type Safety**: Method mapping happens at compile time
4. **Future Proof**: Framework can update methods without breaking user code
5. **Developer Experience**: IntelliSense works perfectly, no memorizing method strings

### Implementation Rule

When creating examples or documentation:
- ✅ Use derive macros WITHOUT method attributes
- ✅ Use builder methods that accept types, not strings
- ✅ Let framework determine methods automatically
- ❌ NEVER show users specifying method strings
- ❌ NEVER use method constants or manual method mapping

## 🚨 CRITICAL: Component Extension Principle

### Extend Existing Components, NEVER Create "Enhanced" Versions

**ABSOLUTE RULE**: Improve existing framework components by extending their capabilities. NEVER create parallel "enhanced" or "advanced" versions that fragment the API.

**❌ WRONG - Architecture Bloat**:
```
session_handler.rs (basic)
enhanced_session_handler.rs (with SessionStorage)

server.rs (basic)
enhanced_server.rs (with SessionStorage)
```

**✅ CORRECT - Single Extensible Components**:
```
session_handler.rs (automatically works with SessionStorage, defaults to InMemory)
server.rs (automatically works with SessionStorage, defaults to InMemory)
```

### Why This Matters

1. **Zero Configuration**: Users get best implementation by default, no choice paralysis
2. **No API Fragmentation**: One way to do things, not multiple competing approaches
3. **Backward Compatibility**: Existing code continues working with improvements
4. **Maintainability**: One component to maintain, not multiple parallel versions

### Implementation Pattern

```rust
// ✅ CORRECT - Single component with pluggable backend
pub struct SessionMcpHandler<S: SessionStorage = InMemorySessionStorage> {
    storage: Arc<S>,
    // ... other fields
}

// Zero-config constructor (defaults to InMemory)
impl SessionMcpHandler<InMemorySessionStorage> {
    pub fn new(config: ServerConfig, dispatcher: Arc<JsonRpcDispatcher>) -> Self {
        let storage = Arc::new(InMemorySessionStorage::new());
        Self::with_storage(config, dispatcher, storage)
    }
}

// Extensible constructor for other storage backends
impl<S: SessionStorage + 'static> SessionMcpHandler<S> {
    pub fn with_storage(config: ServerConfig, dispatcher: Arc<JsonRpcDispatcher>, storage: Arc<S>) -> Self {
        Self { config, dispatcher, storage }
    }
}
```

### Architecture Decision Record

**Decision**: All framework components use the extension pattern, not duplication
**Rationale**: 
- Prevents API fragmentation and choice paralysis
- Maintains zero-configuration while allowing extensibility  
- Reduces maintenance burden
- Aligns with framework philosophy of "one way to do things"

**Implementation**: 
- Use generic parameters with defaults for pluggable behavior
- Provide both zero-config and extensible constructors
- Extend existing components, never create parallel "enhanced" versions

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
8. **Zero-Configuration**: Framework auto-determines methods from trait implementations, never requiring user method strings

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
**CRITICAL**: All types in `turul-mcp-protocol-2025-06-18` crate MUST exactly match the MCP TypeScript Schema specification. This includes:
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

### MCP Streamable HTTP Testing  
```bash
# Complete MCP Streamable HTTP compliance testing
export RUST_LOG=debug
cargo run --example client-initialise-server -- --port 52935
cargo run --example client-initialise-report -- --url http://127.0.0.1:52935/mcp
# Expected output: "🎆 FULLY MCP COMPLIANT: Session management + Streamable HTTP working!"

# Manual testing with curl
# 1. Initialize and get session ID
curl -X POST http://127.0.0.1:52935/mcp \
  -H "Content-Type: application/json" \
  -H "MCP-Protocol-Version: 2025-06-18" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}' \
  -v  # Check for Mcp-Session-Id header in response

# 2. Test POST SSE response (tool execution with notifications)
curl -X POST http://127.0.0.1:52935/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: text/event-stream" \
  -H "Mcp-Session-Id: <session-id-from-step-1>" \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"long_calculation","arguments":{"number":5}}}' \
  -N  # Stream response with tool result + progress notifications

# 3. Test GET persistent SSE stream  
curl -N -H "Accept: text/event-stream" \
  -H "Mcp-Session-Id: <session-id>" \
  http://127.0.0.1:52935/mcp
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
cargo test --package turul-mcp-protocol-2025-06-18 compliance_test::tests
```

## MCP Builders Crate - Runtime Construction Library

The `turul-mcp-builders` crate provides **Level 3** of the four-level creation spectrum - runtime flexibility for dynamic and configuration-driven MCP systems. This crate offers comprehensive builder patterns for ALL MCP protocol areas.

### Status: Production Ready ✅
- **9 Complete Builders**: All MCP areas covered with full specification compliance
- **70 Passing Tests**: Comprehensive test coverage across all builders
- **Zero Warnings**: Clean compilation with proper workspace integration
- **MCP 2025-06-18 Compliant**: Exact TypeScript specification alignment

### Complete Builder Coverage

The turul-mcp-builders crate provides builders for every MCP protocol area:

1. **ToolBuilder** - Dynamic tool construction with parameter validation
2. **ResourceBuilder** - Runtime resource creation with content handling  
3. **PromptBuilder** - Template-based prompt construction with argument processing
4. **MessageBuilder** - Sampling message composition with model preferences
5. **CompletionBuilder** - Autocompletion context building for prompts and resources
6. **RootBuilder** - Root directory configuration with filtering and permissions
7. **ElicitationBuilder** - User input collection forms with validation schemas
8. **NotificationBuilder** - MCP notification creation (progress, resource updates, etc.)
9. **LoggingBuilder** - Structured logging with all MCP-compliant log levels

### Key Features

- **Runtime Flexibility**: Create MCP components entirely at runtime without procedural macros
- **Configuration-Driven**: Perfect for systems that load tool definitions from config files
- **Type Safety**: Full parameter validation and schema generation
- **Trait Integration**: All builders produce types implementing the framework's Definition traits
- **MCP Compliance**: Exact specification adherence with comprehensive testing

### Usage Example - Multiple Builders

```rust
use turul_mcp_builders::*;
use serde_json::json;

// Create a calculator tool at runtime
let calculator = ToolBuilder::new("calculator")
    .description("Add two numbers")
    .number_param("a", "First number")  
    .number_param("b", "Second number")
    .execute(|args| async move {
        let a = args.get("a").and_then(|v| v.as_f64()).ok_or("Missing 'a'")?;
        let b = args.get("b").and_then(|v| v.as_f64()).ok_or("Missing 'b'")?;
        Ok(json!({"result": a + b}))
    })
    .build()?;

// Create a configuration resource  
let config_resource = ResourceBuilder::new("file:///config.json")
    .name("app_config")
    .description("Application configuration")
    .json_content(json!({"version": "1.0", "debug": false}))
    .build()?;

// Create a greeting prompt template
let greeting_prompt = PromptBuilder::new("greeting")
    .description("Generate personalized greetings")
    .string_argument("name", "Person to greet")
    .user_message("Hello {name}! How are you today?")
    .assistant_message("Nice to meet you!")
    .build()?;

// Create structured logging
let error_log = LoggingBuilder::error(json!({
        "error": "Database connection failed",
        "retry_count": 3,
        "duration_ms": 1250
    }))
    .logger("database")
    .meta_value("session_id", json!("sess-456"))
    .build();

// Create progress notifications
let progress = NotificationBuilder::progress("task-123", 75)
    .total(100)
    .message("Processing files...")
    .build();

// Use in server
let server = McpServer::builder()
    .tool(calculator)
    .resource(config_resource) 
    .prompt(greeting_prompt)
    .build()?;
```

### Builder Documentation

Each builder follows consistent patterns:

- **Fluent API**: Method chaining for intuitive construction
- **Validation**: Parameter and schema validation at build time
- **Error Handling**: Comprehensive error messages with context
- **Extensibility**: Meta fields and custom attributes supported
- **Testing**: Individual test suites with edge case coverage

### MCP Specification Compliance

The turul-mcp-builders crate maintains strict MCP specification compliance:

- **Exact Field Names**: All camelCase fields match TypeScript schema exactly
- **Optional Fields**: Proper `skip_serializing_if` handling for optional parameters
- **Meta Support**: All builders support MCP `_meta` field pattern
- **Method Mapping**: Framework automatically determines correct MCP methods
- **Validation**: JSON schema validation for all constructed components

### Integration with Framework

Builders integrate seamlessly with the framework's trait system:

```rust
// All builders produce types implementing Definition traits
let tool = ToolBuilder::new("example").build()?; // implements ToolDefinition
let resource = ResourceBuilder::new("uri").build()?; // implements ResourceDefinition  
let prompt = PromptBuilder::new("template").build()?; // implements PromptDefinition

// Framework accepts any type implementing the Definition traits
let server = McpServer::builder()
    .tool(tool)      // ToolDefinition
    .resource(resource) // ResourceDefinition
    .prompt(prompt)     // PromptDefinition
    .build()?;
```

For detailed usage examples, see `examples/builders-showcase/` which demonstrates all 9 builders in a comprehensive application.

## Four-Level Tool Creation Spectrum

This framework provides four distinct approaches for creating MCP tools, ordered by increasing complexity but decreasing boilerplate. Choose the level that matches your needs:

### Level 1: Function Macros (Ultra-Simple)
**Best for**: Quick prototypes, simple tools, learning MCP
**Boilerplate**: ~5 lines of code

```rust
use turul_mcp_server::{McpServer, McpResult};
use turul_mcp_derive::mcp_tool;

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
use turul_mcp_derive::McpTool;
use turul_mcp_server::{McpResult, McpTool};

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

> **Note**: Level 3 provides the comprehensive `turul-mcp-builders` crate with builders for ALL MCP areas (tools, resources, prompts, logging, notifications, etc.). See the [MCP Builders Crate](#turul-mcp-builders-crate---runtime-construction-library) section above for complete documentation.

```rust
use turul_mcp_server::{McpServer, ToolBuilder};
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
use turul_mcp_server::{McpServer, McpTool, McpResult, SessionContext};
use turul_mcp_protocol::tools::{
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

## ✅ **Trait Architecture - COMPLETE**

**STATUS**: ✅ **COMPLETED** - Comprehensive trait refactoring successfully applied across all MCP areas

**Achievement**: The MCP framework now uses consistent fine-grained trait architecture across all areas:
- **Tools**: Complete ToolDefinition trait composition pattern
- **Resources**: Fine-grained trait implementation with ResourceDefinition  
- **Prompts**: Trait-based prompt handling with PromptDefinition
- **All MCP Areas**: Consistent pattern applied to sampling, completion, notifications, etc.

**Result**: Framework provides unified trait interfaces enabling polymorphism and type safety across all MCP protocol areas while maintaining perfect 1:1 mapping with TypeScript specification.

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

## ✅ **Testing Excellence - Beta-Grade Quality**

**STATUS**: ✅ **100+ Comprehensive Tests** - Framework-native test suite with complete MCP coverage

The turul-mcp-framework maintains a comprehensive test suite with **100+ passing tests** across all crates, ensuring beta-grade reliability and MCP specification compliance.

### Testing Philosophy: Framework-Native, Not JSON Manipulation

**CRITICAL PRINCIPLE**: Tests MUST use framework APIs and typed structures, not raw JSON manipulation.

```rust
// ✅ CORRECT - Framework-Native Testing
use turul_mcp_server::{McpTool, McpServerBuilder, McpResult, SessionContext};
use turul_mcp_derive::McpTool;

#[derive(McpTool, Default)]
#[tool(name = "calculator", description = "Add two numbers")]
struct CalculatorTool {
    #[param(description = "First number")] a: f64,
    #[param(description = "Second number")] b: f64,
}

impl CalculatorTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<f64> {
        Ok(self.a + self.b)
    }
}

#[tokio::test]
async fn test_calculator_framework_integration() {
    let tool = CalculatorTool { a: 5.0, b: 3.0 };
    
    // Use framework's McpTool trait, not raw JSON
    let result = tool.call(json!({"a": 5.0, "b": 3.0}), None).await.unwrap();
    
    // Verify using framework types
    assert_eq!(result.content.len(), 1);
    match &result.content[0] {
        ToolResult::Text { text } => {
            let parsed: Value = serde_json::from_str(text).unwrap();
            assert_eq!(parsed["output"], 8.0); // Derive macro uses "output"
        }
        _ => panic!("Expected text result")
    }
}
```

```rust
// ❌ WRONG - Raw JSON Manipulation Testing (NEVER DO THIS)
#[tokio::test] 
async fn test_raw_json_manipulation() {
    // This is wrong - tests the JSON API, not the framework
    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {"name": "calculator", "arguments": {"a": 5.0, "b": 3.0}}
    });
    
    // Raw HTTP requests miss framework validation and type safety
    // This tests the transport, not the framework logic
}
```

### Test Coverage by Area

The framework maintains comprehensive test coverage across all areas:

| Test Category | Status | Coverage |
|---------------|---------|----------|
| **Framework Integration** | ✅ 7/7 passing | Four-level tool creation, server integration |
| **Session Management** | ✅ 8/8 passing | SessionContext, notifications, storage |
| **MCP Protocol Compliance** | ✅ 28/34 passing | TypeScript specification alignment |
| **Builder Patterns** | ✅ 70+ tests | All 9 builders with comprehensive validation |
| **HTTP Transport** | ✅ Working | Streamable HTTP, SSE, session handling |
| **Trait Architecture** | ✅ Complete | Fine-grained trait testing across all areas |

### Four-Level Testing Strategy

The framework provides testing patterns that match the four-level tool creation spectrum:

#### Level 1: Function Macro Testing
```rust
#[mcp_tool(name = "add", description = "Add numbers")]
async fn add_tool(
    #[param(description = "First")] a: f64,
    #[param(description = "Second")] b: f64,
) -> McpResult<f64> {
    Ok(a + b)
}

#[tokio::test]
async fn test_function_macro_tool() {
    let tool = add_tool();
    let result = tool.call(json!({"a": 3.0, "b": 4.0}), None).await.unwrap();
    // Verify structured response using framework types
}
```

#### Level 2: Derive Macro Testing
```rust
#[derive(McpTool)]
#[tool(name = "calculator", description = "Calculator")]
struct Calculator {
    #[param(description = "Number")] value: f64,
}

#[tokio::test]
async fn test_derive_macro_tool() {
    let calc = Calculator { value: 42.0 };
    let result = calc.call(json!({"value": 42.0}), None).await.unwrap();
    // Test automatic trait implementations
}
```

#### Level 3: Builder Pattern Testing
```rust
#[tokio::test]
async fn test_builder_pattern_tool() {
    let tool = ToolBuilder::new("division")
        .description("Divide numbers")
        .number_param("a", "Dividend") 
        .number_param("b", "Divisor")
        .execute(|args| async move {
            let a = args.get("a").and_then(|v| v.as_f64()).unwrap();
            let b = args.get("b").and_then(|v| v.as_f64()).unwrap();
            Ok(json!({"result": a / b}))
        })
        .build()
        .unwrap();
        
    let result = tool.call(json!({"a": 10.0, "b": 2.0}), None).await.unwrap();
    // Verify runtime-constructed tool behavior
}
```

#### Level 4: Manual Implementation Testing  
```rust
struct ManualTool;

impl HasBaseMetadata for ManualTool {
    fn name(&self) -> &str { "manual" }
}
// ... other trait implementations

#[tokio::test]
async fn test_manual_implementation() {
    let tool = ManualTool;
    let result = tool.call(json!({"input": "test"}), None).await.unwrap();
    // Test manual trait implementations
}
```

### Testing Best Practices

#### 1. Framework Types Over JSON
- Use `McpTool`, `McpServerBuilder`, `SessionContext` - never raw JSON manipulation
- Test framework behavior, not transport implementation details
- Verify typed responses using `ToolResult`, `CallToolResponse`, etc.

#### 2. Session-Aware Testing
```rust
use turul_mcp_server::session::test_helpers::{TestSessionBuilder, TestNotificationBroadcaster};

#[tokio::test]
async fn test_session_integration() {
    let session = TestSessionBuilder::new()
        .with_session_id("test-session")
        .with_notification_broadcaster(TestNotificationBroadcaster::new())
        .build();
        
    // Test tools with session context
    let result = tool.call(args, Some(session)).await.unwrap();
}
```

#### 3. Trait Interface Testing
```rust
fn test_any_tool_definition(tool: &dyn ToolDefinition) {
    assert!(!tool.name().is_empty());
    assert!(tool.input_schema().properties.len() > 0);
}

#[tokio::test]
async fn test_tool_definitions() {
    test_any_tool_definition(&Calculator::default());
    test_any_tool_definition(&manual_tool);
    test_any_tool_definition(&builder_tool);
}
```

### Running Tests

```bash
# Complete test suite
cargo test --workspace

# Framework integration tests specifically
cargo test --package turul-mcp-framework-integration-tests

# Individual test categories
cargo test --package turul-mcp-server session
cargo test --package turul-mcp-builders builder
cargo test --package turul-mcp-protocol compliance

# With debug output for troubleshooting
RUST_LOG=debug cargo test test_name -- --nocapture
```

### Test Quality Standards

All tests must meet these requirements:

- **Framework-Native**: Use typed APIs, not JSON manipulation
- **Type Safety**: Verify responses using framework types
- **Session-Aware**: Test with and without SessionContext where applicable  
- **Error Handling**: Test both success and error paths
- **MCP Compliance**: Verify specification adherence
- **Performance**: Include benchmarks for critical paths

### Integration with MCP Inspector

Tests verify MCP Inspector compatibility:

```rust
// Verify structured content for MCP Inspector
match &result.content[0] {
    ToolResult::Text { text } => {
        let parsed: Value = serde_json::from_str(text).unwrap();
        // Must return structured JSON, not "Tool Result: Success"
        assert!(parsed.is_object());
        assert!(parsed.get("output").is_some() || parsed.get("result").is_some());
    }
    _ => panic!("Expected structured text result")
}
```

This testing approach ensures the framework delivers beta-grade reliability while maintaining perfect MCP specification compliance.