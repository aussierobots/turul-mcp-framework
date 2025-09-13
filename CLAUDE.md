# CLAUDE.md

This file provides guidance to Claude Code when working with the **turul-mcp-framework**.

## Project Overview

A standalone, beta-grade Rust framework for building Model Context Protocol (MCP) servers with zero-configuration design and complete MCP 2025-06-18 specification support.

### Key Features
- **Zero-Configuration**: Framework auto-determines ALL methods from types (see [ADR-003](docs/adr/003-zero-configuration-principle.md))
- **Four-Level Creation Spectrum**: Function macros → derive macros → builders → manual implementation
- **Complete MCP 2025-06-18 Support**: All features including streamable HTTP transport with SSE
- **Trait-Based Architecture**: Composable, type-safe components (see [ADR-005](docs/adr/005-trait-based-architecture.md))

## Critical Rules

### 🚨 Import Conventions
**RECOMMENDED**: Use prelude modules for concise imports:
```rust
// ✅ BEST - Use preludes for main crates
use turul_mcp_server::prelude::*;        // Server + protocol + common types
use turul_mcp_builders::prelude::*;      // All builders + protocol
use turul_mcp_protocol::prelude::*;      // Protocol traits + common types

// ✅ GOOD - Individual derive macros (proc-macro crates can't export preludes)
use turul_mcp_derive::{McpTool, McpResource, McpPrompt, mcp_tool};

// ✅ GOOD - Direct protocol imports
use turul_mcp_protocol::resources::{HasResourceMetadata, ResourceDefinition};

// ❌ WRONG - Versioned imports
use turul_mcp_protocol_2025_06_18::resources::{HasResourceMetadata, ResourceDefinition};
```

**Available Preludes**:
- `turul_mcp_protocol::prelude::*` - All protocol traits, types, and common utilities
- `turul_mcp_server::prelude::*` - Server types + protocol prelude + async_trait + serde
- `turul_mcp_builders::prelude::*` - All builders + protocol prelude + HashMap + json

See [ADR-001](docs/adr/001-protocol-alias-usage.md) for details.

### 🚨 Zero-Configuration Design
Users NEVER specify method strings. Framework auto-determines ALL methods:
```rust
// ✅ CORRECT - Zero Configuration
#[derive(McpTool)]
struct Calculator;  // Framework → tools/call

// ❌ WRONG - Manual methods
#[mcp_tool(method = "tools/call")]  // NO METHOD STRINGS!
```

### 🚨 Resource Implementation
Use framework's trait-based pattern (see [ADR-002](docs/adr/002-resources-integration-pattern.md)):
```rust
use turul_mcp_derive::McpResource;
use turul_mcp_protocol::prelude::*;  // Protocol traits + common types

#[derive(McpResource, Clone, Serialize, Deserialize)]
#[resource(name = "my_resource", uri = "file:///data/{id}.json", description = "My resource")]
pub struct MyResource {
    pub id: String,
}
```

## Architecture

### Core Crates
```
turul-mcp-framework/
├── crates/
│   ├── turul-mcp-protocol-2025-06-18/  # Protocol types and traits
│   ├── turul-mcp-server/               # High-level server framework
│   ├── turul-mcp-builders/             # Runtime builder patterns
│   ├── turul-http-mcp-server/          # HTTP transport layer
│   └── turul-mcp-derive/               # Procedural macros
└── examples/                           # Example servers
```

### Four-Level Tool Creation Spectrum

| Level | Use Case | Code | Performance |
|-------|----------|------|-------------|
| 1 - Function | Prototypes | `#[mcp_tool] fn calc() {}` | Good |
| 2 - Derive | Production | `#[derive(McpTool)] struct Calc {}` | Good |
| 3 - Builder | Runtime/Config | `ToolBuilder::new("calc").build()` | Good |
| 4 - Manual | Performance | Manual trait impls | Best |

### Session Management
- **UUID v7** (`Uuid::now_v7()`) for session IDs
- Automatic cleanup every 60 seconds, 30-minute expiry
- Streamable HTTP with SSE notifications ✅ FULLY OPERATIONAL

## Quick Start

### 1. Basic Tool (Level 1 - Function)
```rust
use turul_mcp_derive::mcp_tool;
use turul_mcp_server::prelude::*;

#[mcp_tool(name = "add", description = "Add two numbers")]
async fn add(a: f64, b: f64) -> McpResult<f64> {
    Ok(a + b)
}
```

### 2. Basic Resource (Level 2 - Derive)
```rust
use turul_mcp_derive::McpResource;
use turul_mcp_protocol::prelude::*;

#[derive(McpResource)]
#[resource(name = "user", uri = "file:///users/{id}.json")]
struct UserResource { id: String }
```

### 3. Basic Prompt (Level 2 - Derive)
```rust
use turul_mcp_derive::McpPrompt;
use turul_mcp_protocol::prelude::*;

#[derive(McpPrompt)]
#[prompt(name = "review", description = "Code review prompt")]
struct ReviewPrompt {
    #[argument(name = "code", required = true)]
    code: String,
}
```

### 4. Runtime Tool (Level 3 - Builder)
```rust
use turul_mcp_builders::prelude::*;

<<<<<<< HEAD
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

## Rust Edition Standard

**CRITICAL**: Always use Rust edition 2024 in all Cargo.toml files:

```toml
[package]
edition = "2024"
```

This applies to:
- All new crate creation
- All example projects  
- All test projects
- All documentation examples
- All workspace configurations

The framework targets the latest Rust features and edition 2024 provides the best developer experience with improved diagnostics, syntax, and async support.

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
=======
let tool = ToolBuilder::new("calculator")
    .description("Add numbers")
    .number_param("a", "First number")
>>>>>>> cebf93d8ec27b383dd751b6b1dde698217dca626
    .number_param("b", "Second number")
    .execute(|args| async move {
        let a = args.get("a").and_then(|v| v.as_f64()).ok_or("Missing 'a'")?;
        let b = args.get("b").and_then(|v| v.as_f64()).ok_or("Missing 'b'")?;
        Ok(json!({"result": a + b}))
    })
    .build()?;
```

## Build Commands

```bash
# Development
cargo build
cargo test
cargo run --example minimal-server

# MCP Streamable HTTP Testing
cargo run --example client-initialise-server -- --port 52935
cargo run --example client-initialise-report -- --url http://127.0.0.1:52935/mcp
```

## Testing Philosophy

Use framework APIs, not JSON manipulation:
```rust
use turul_mcp_derive::McpTool;
use turul_mcp_server::prelude::*;

// ✅ Framework-native testing
let tool = CalculatorTool { a: 5.0, b: 3.0 };
let result = tool.call(json!({"a": 5.0, "b": 3.0}), None).await.unwrap();

// ❌ Raw JSON manipulation
let json_request = r#"{"method":"tools/call","params":{"name":"calc"}}"#;
```

## Key Implementation Guidelines

- **UUID Usage**: Always use UUID v7 for session IDs
- **Error Handling**: Use framework `McpError` types
- **Session Architecture**: Single `/mcp` endpoint with method-based routing
- **Testing**: Return structured JSON data, not generic "Success" messages
- **Component Extension**: Extend existing components, never create "enhanced" versions (see [ADR-004](docs/adr/004-component-extension-principle.md))

## Architecture Decision Records

For detailed technical decisions, see:
- [ADR-001: Protocol Alias Usage](docs/adr/001-protocol-alias-usage.md)
- [ADR-002: Resources Integration Pattern](docs/adr/002-resources-integration-pattern.md)  
- [ADR-003: Zero-Configuration Principle](docs/adr/003-zero-configuration-principle.md)
- [ADR-004: Component Extension Principle](docs/adr/004-component-extension-principle.md)
- [ADR-005: Trait-Based Architecture](docs/adr/005-trait-based-architecture.md)

## Development Standards
- Clean separation of responsibilities per crate
- Future-proof design supporting additional transports
- Production-ready: performance, security, and reliability first
- Complete MCP 2025-06-18 specification compliance

## Core Crate Modification Rules

### 🚨 Critical Guidelines
- **NO PANICS in production code** - Core crates must never use `panic!` in user-facing APIs
- **Builder pattern stability** - Changes to builder methods require breaking change analysis
- **Error handling consistency** - Use `Result` types for fallible operations
- **Zero-configuration principle** - Framework should gracefully handle invalid inputs when possible

### Before Modifying Core Crates:
1. **Ultra Think Analysis Required**
   - Analyze full impact on all examples, tests, and user code
   - Consider backwards compatibility implications
   - Evaluate alternative approaches
   - Document breaking changes clearly

2. **Error Handling Patterns**
   - Use `Result<T, McpError>` for fallible operations
   - Log errors with appropriate levels (warn/error)
   - Prefer graceful degradation over crashes
   - Early validation when possible without breaking APIs

3. **Production Safety**
   - Never crash the process due to user input
   - Validate inputs but handle failures gracefully
   - Provide clear error messages for debugging
   - Consider fail-fast vs. fail-safe trade-offs