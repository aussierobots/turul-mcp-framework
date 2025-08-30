# MCP Framework Examples

This document provides a comprehensive overview of all **27 examples** in the MCP Framework, organized by learning progression from basic concepts to advanced implementations.

**Legend**:
- 🎓 **Educational** - Teaches manual trait implementation patterns
- 🚀 **Optimized** - Uses macros for minimal code (derive/function macros)
- 🔧 **Builder** - Runtime construction patterns

## 🟢 **GETTING STARTED** (4 examples) - Start Here

| Example | Port | Purpose | Development Pattern |
|---------|------|---------|-------------------|
| **minimal-server** 🚀 | 8000 | Simplest possible MCP server | `#[mcp_tool]` function macro |
| **calculator-add-function-server** 🚀 | 8001 | Function macro pattern | `#[mcp_tool]` attribute |
| **calculator-add-simple-server-derive** 🚀 | 8002 | Derive macro pattern | `#[derive(McpTool)]` |
| **calculator-add-builder-server** 🔧 | 8003 | Builder pattern | Runtime construction |

## 🟡 **CORE MCP FEATURES** (8 examples)

| Example | Port | Purpose | Key Features |
|---------|------|---------|--------------|
| **calculator-add-manual-server** 🎓 | 8004 | Manual implementation (educational) | Complete trait control |
| **function-macro-server** 🚀 | 8005 | Multiple function tools | `#[mcp_tool]` patterns |
| **derive-macro-server** 🚀 | 8006 | Multiple derive tools | `#[derive(McpTool)]` patterns |
| **resources-server** | 8007 | Resource handling | Multiple resource types |
| **resource-server** | 8008 | Derive macro resources | `#[derive(McpResource)]` |
| **stateful-server** 🚀 | 8006 | Session management | Persistent state |
| **spec-compliant-server** | 8010 | MCP 2025-06-18 features | `_meta`, progress tokens |
| **version-negotiation-server** | 8011 | Protocol versions | Version compatibility |

## 🔵 **INTERACTIVE FEATURES** (6 examples)

| Example | Port | Purpose | Advanced Features |
|---------|------|---------|-------------------|
| **notification-server** | 8012 | Real-time notifications | SSE streaming |
| **elicitation-server** | 8013 | User input collection | Form handling |
| **prompts-server** | 8014 | AI prompt generation | Dynamic prompts |
| **sampling-server** | 8015 | AI model sampling | Model integration |
| **completion-server** 🚀 | 8042 | Text completion | Context-aware suggestions |
| **roots-server** | 8017 | File system security | Access control |

## 🟠 **PRODUCTION EXAMPLES** (4 examples)

| Example | Port | Purpose | Production Features |
|---------|------|---------|-------------------|
| **comprehensive-server** | 8018 | All MCP features | Complete implementation |
| **performance-testing** | 8019 | Load testing | Benchmarking |
| **pagination-server** | 8020 | Large datasets | Cursor pagination |
| **dynamic-resource-server** | 8021 | Parameterized resources | Dynamic URIs |

## 🔴 **SPECIALIZED EXAMPLES** (5 examples)

| Example | Port | Purpose | Use Case |
|---------|------|---------|---------|
| **alert-system-server** 🎓 | 8010 | Alert management | Monitoring systems |
| **audit-trail-server** | 8023 | Audit logging | Compliance tracking |
| **lambda-mcp-client** | 8024 | Serverless client | AWS Lambda |
| **simple-logging-server** 🚀 | 8025 | Structured logging | Log management |
| **zero-config-getting-started** 🚀 | 8026 | Zero configuration | Minimal setup |

## 🚀 **FRAMEWORK SHOWCASE** (2 examples)

| Example | Port | Purpose | Demonstration |
|---------|------|---------|---------------|
| **builders-showcase** 🔧 | 8027 | Runtime builder patterns | Level 3 construction |
| **manual-tools-server** 🎓 | 8028 | Advanced manual patterns (educational) | Complex trait implementations |

**Note**: The `archived/` folder contains historical examples that were consolidated during framework development.

## 📚 **LEARNING PROGRESSION**

### **Level 1: Basics** (Start Here)
1. Run `minimal-server` - See simplest MCP server
2. Try `calculator-add-function-server` - Learn function macros
3. Explore `calculator-add-simple-server-derive` - Understand derive macros
4. Build `calculator-add-builder-server` - Experience runtime construction

### **Level 2: Core Features** 
5. `stateful-server` - Learn session management
6. `resources-server` - Handle different resource types
7. `notification-server` - Real-time SSE streaming
8. `spec-compliant-server` - MCP 2025-06-18 compliance

### **Level 3: Advanced Integration**
9. `elicitation-server` - Complex user interactions
10. `sampling-server` - AI model integration
11. `comprehensive-server` - All features together
12. `performance-testing` - Production considerations

## 🔧 **DEVELOPMENT PATTERNS**

The framework supports **4 tool creation approaches**:

1. **Function Macros** - `#[mcp_tool]` attribute on functions
2. **Derive Macros** - `#[derive(McpTool)]` on structs
3. **Builder Pattern** - Runtime construction with `turul-mcp-builders`
4. **Manual Implementation** - Direct trait implementation

Each approach offers different trade-offs between simplicity and control.

## 🎯 **TESTING MCP COMPLIANCE**

Use the **client-initialise examples** for comprehensive testing:
- `client-initialise-server` - Start test server
- `client-initialise-report` - Run compliance tests

These verify complete MCP 2025-06-18 specification compliance including SSE notifications.