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

let tool = ToolBuilder::new("calculator")
    .description("Add numbers")
    .number_param("a", "First number")
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