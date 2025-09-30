# Release Notes - Version 0.2.0

**Release Date**: September 29, 2025
**Status**: 🔧 **Beta Release** - Comprehensive MCP 2025-06-18 framework with Phase 6 session-aware resources

## 🎉 Major Achievements

### **Complete MCP Framework Implementation**
- ✅ **4 Tool Creation Levels**: Function macros, derive macros, builders, and manual implementation
- ✅ **Comprehensive MCP 2025-06-18 Implementation**: Core protocol areas implemented (tools, resources, prompts, SSE) with production-grade stability
- ✅ **Session Management**: UUID v7 sessions with pluggable storage backends (InMemory, SQLite, PostgreSQL, DynamoDB)
- ✅ **Real-time Notifications**: Server-Sent Events (SSE) streaming with JSON-RPC format
- ✅ **Zero-Configuration Design**: Framework auto-determines all methods from types

### **Enhanced Developer Experience**
- ✅ **Auto-Detection Template Resources**: Eliminated URI template redundancy with automatic detection
- ✅ **Resource Function Macro**: New `#[mcp_resource]` procedural macro for async function resources
- ✅ **Simplified Resource API**: Single `.resource()` method handles both static and template resources
- ✅ **Comprehensive Documentation**: All README files verified for accuracy with working examples

### **Phase 6: Session-Aware Resources**
- ✅ **SessionContext Integration**: All resources now receive session context for personalized content
- ✅ **Breaking Change Implementation**: Updated McpResource trait with session parameter
- ✅ **Comprehensive Example Updates**: All resource examples updated for session-aware functionality
- ✅ **Full MCP 2025-06-18 Compliance**: Session-aware resources meet specification requirements

## 🔧 Critical Production Fixes

### **Lifecycle & Security Enhancements**
- ✅ **Extended Strict Lifecycle Gating**: All MCP endpoints now respect initialized state (not just tools/call)
- ✅ **Async Runtime Safety**: Removed `futures::executor::block_on` usage preventing Tokio runtime panics
- ✅ **Complete Session Context Migration**: All 410+ call sites converted to async with proper `.await` patterns

### **MCP Spec Compliance Improvements**
- ✅ **Pagination Response Structure**: Fixed `nextCursor` emission in all list handlers (prompts, resources, tools)
- ✅ **Meta Propagation**: Proper `_meta` round-trip handling across all endpoints
- ✅ **Tools List Stability**: Added sorting and typed parameter parsing for consistent pagination
- ✅ **Resource Templates Response**: Corrected response shape to match MCP 2025-06-18 specification

### **Framework Reliability**
- ✅ **Deadlock Prevention**: Eliminated blocking calls in async contexts for production safety
- ✅ **Consistent API Patterns**: Standardized pagination handling across all list endpoints
- ✅ **Robust Error Handling**: Improved lifecycle violation error responses

## 📚 Documentation Quality Improvements

### **README Accuracy Audit**
- **API Pattern Standardization**: Fixed `McpServerBuilder::new()` → `McpServer::builder()` across all documentation
- **Import Consistency**: Standardized on `use turul_mcp_server::prelude::*;` pattern
- **Code Example Verification**: All examples now compile and match actual implementations
- **Documentation Alignment**: Examples consistent with working code in `examples/`

### **Files Updated**
- Main `README.md` - All quick start examples updated
- 6 core crate READMEs with corrected API patterns:
  - `turul-mcp-server/README.md` - 9 instances fixed
  - `turul-mcp-derive/README.md` - 2 instances fixed
  - `turul-mcp-builders/README.md` - 2 instances fixed
  - `turul-mcp-protocol/README.md` - 1 instance fixed
  - `turul-mcp-protocol-2025-06-18/README.md` - 1 instance fixed
  - `turul-http-mcp-server/README.md` - 1 instance + import fix

## 🔧 Technical Enhancements

### **Auto-Detection Template Resources** (September 15, 2025)
- **Resource Function Macro**: New `#[mcp_resource]` procedural macro for async function resources
- **Auto-Detection Logic**: Builder automatically detects template URIs based on `{variable}` patterns
- **Unified API**: Single `.resource()` method handles both static and template resources
- **Backward Compatibility**: `.template_resource()` method remains available
- **Comprehensive Testing**: 10 new unit tests covering all auto-detection scenarios

### **Security & URI Management**
- **Auto-Detection Resource Security**: Zero-configuration security that generates patterns from registered resources
- **File:// URI Migration**: Complete migration from custom URI schemes for security middleware compatibility
- **Production Safety**: Eliminated dangerous `.test_mode()` usage in production code

### **MCP Compliance Achievements**
- **Prompt System Redesign**: Fixed render method conflicts enabling both simple and complex prompt patterns
- **Session Management**: Complete MCP 2025-06-18 session lifecycle compliance
- **SSE Notifications**: Full real-time streaming with proper camelCase compliance
- **Schema Generation**: Compile-time schemas with custom output field support

## 🏗️ Architecture Improvements

### **Core Framework Status**
- **Zero Warnings**: `cargo check --workspace` passes cleanly
- **Version Synchronization**: All 69 crates synchronized to version 0.2.0
- **Publishing Ready**: No circular dependencies, clean crate structure
- **MCP Inspector Compatible**: Works perfectly with standard JSON responses

### **Session Storage Backends**
All storage backends fully implemented and production-ready:
- **InMemory**: Fast, zero-config for development
- **SQLite**: File-based persistence for single-instance deployments
- **PostgreSQL**: Multi-instance production deployments
- **DynamoDB**: Serverless deployments with auto-table creation

### **Testing Infrastructure**
- **300+ Tests**: Comprehensive test coverage across workspace
- **Example Validation**: 45+ examples validated through systematic testing campaign
- **Framework-Native Testing**: Proper API usage patterns validated
- **MCP Compliance**: Full MCP 2025-06-18 specification compliance verified

## 📦 Framework Components

### **Core Crates (10 packages)**
- `turul-mcp-server` - High-level framework with session management
- `turul-mcp-client` - Comprehensive client library
- `turul-http-mcp-server` - HTTP/SSE transport with CORS
- `turul-mcp-protocol` - Current MCP specification (alias to 2025-06-18)
- `turul-mcp-derive` - Procedural macros for all MCP areas
- `turul-mcp-builders` - Runtime builder patterns
- `turul-mcp-json-rpc-server` - Transport-agnostic JSON-RPC foundation
- `turul-mcp-session-storage` - Pluggable storage backends
- `turul-mcp-aws-lambda` - AWS Lambda integration
- `turul-mcp-protocol-2025-06-18` - Complete specification implementation

### **Examples (45+ validated examples)**
- **Comprehensive Testing Campaign**: All examples validated through systematic testing
- **Business Applications**: Real-world servers solving actual problems
- **Framework Demonstrations**: Educational examples showcasing patterns
- **Learning Progression**: Simple → Complex (Function → Derive → Builder → Manual)

## 🚀 Getting Started

### Quick Start (Function Macros)
```rust
use turul_mcp_derive::mcp_tool;
use turul_mcp_server::prelude::*;

#[mcp_tool(name = "add", description = "Add two numbers")]
async fn add(
    #[param(description = "First number")] a: f64,
    #[param(description = "Second number")] b: f64,
) -> McpResult<f64> {
    Ok(a + b)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = McpServer::builder()
        .name("calculator-server")
        .version("1.0.0")
        .tool_fn(add)
        .bind_address("127.0.0.1:8080".parse()?)
        .build()?;

    server.run().await
}
```

### Verification Commands
```bash
# Build the framework
cargo build --workspace

# Run compliance tests
cargo test -p turul-mcp-framework-integration-tests

# Start a simple server
cargo run -p minimal-server

# Test with MCP Inspector
# Connect to http://127.0.0.1:8641/mcp
```

## 🔧 Framework Maturity

### **Quality Metrics**
- **Zero-Configuration**: Framework auto-determines ALL methods from types
- **Type Safety**: Compile-time schema generation and validation
- **Memory Safety**: Rust's ownership system prevents vulnerabilities
- **Session Isolation**: Secure multi-tenant operation
- **Real-time Capable**: SSE streaming with proper isolation

### **Developer Experience**
- **Clean Compilation**: Zero warnings across all framework crates
- **Comprehensive Documentation**: All README examples compile and work
- **Multiple Patterns**: Choose the right tool for your complexity level
- **Real-world Examples**: Business applications included for learning

### **Enterprise-Ready Features**
- **Pluggable Storage**: Choose the right backend for your deployment
- **AWS Lambda Integration**: Complete serverless support
- **Session Management**: UUID v7 with automatic cleanup
- **Security**: Built-in protection with configurable middleware

## 📈 What's Next

The turul-mcp-framework 0.2.0 represents a **comprehensive beta implementation** of the MCP 2025-06-18 specification with Phase 6 session-aware resources complete. All core functionality is implemented, tested, and documented.

**Planned enhancements for future releases**:
- Performance optimization and stress testing
- Additional storage backends (Redis, S3)
- Authentication & authorization features
- WebSocket transport support
- Advanced tooling and CLI utilities

## 🙏 Acknowledgments

This release represents the culmination of comprehensive development work focused on creating a production-grade MCP framework that prioritizes developer experience, type safety, and specification compliance.

---

**Ready to build production MCP servers?** Start with our [comprehensive examples](examples/) or check the [main README](README.md) for getting started guides.

**Need help?** All 25+ examples compile and work out of the box, demonstrating everything from simple calculators to enterprise systems.