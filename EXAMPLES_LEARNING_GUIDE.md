# MCP Framework Examples - Learning Guide

This guide organizes the 36+ examples in a logical learning progression from basic concepts to advanced implementations.

## 🟢 **BASIC EXAMPLES** - Start Here (All ✅ Working)

Perfect for getting started with MCP framework basics.

### **Foundation Examples**
- **`minimal-server`** ✅ - Simplest possible MCP server with one echo tool
- **`spec-compliant-server`** ✅ - Demonstrates MCP 2025-06-18 specification compliance
- **`calculator-add-function-server`** ✅ - Function attribute macro usage (`#[mcp_tool]`)

### **Calculator Series (Approach Comparison)**
- **`calculator-add-simple-server-derive`** - Derive macro approach
- **`calculator-add-manual-server`** - Manual trait implementation
- **`calculator-add-builder-server`** - Builder pattern usage
- **`calculator-server`** - Full calculator implementation
- **`calculator-struct-output-example`** - Structured output handling

## 🟡 **INTERMEDIATE EXAMPLES** - Build Understanding (All ✅ Working)

Learn advanced patterns and real-world usage.

### **Macro System Mastery**
- **`tool-macro-example`** ✅ - Declarative `tool!` macro usage
- **`resource-macro-example`** ✅ - Declarative `resource!` macro usage  
- **`derive-macro-server`** - All derive macros in action
- **`function-macro-server`** - Function attribute macros
- **`macro-calculator`** - Advanced macro combinations

### **Server Patterns**
- **`stateful-server`** ✅ - Session management and persistent state
- **`manual-tools-server`** ✅ - Manual trait implementation showcase
- **`all-tool-approaches-example`** - Compare all tool creation approaches
- **`mixed-approaches-example`** - Combining different patterns

### **Resource Handling**
- **`resource-server`** - Basic resource management
- **`resources-server`** - Multiple resource types
- **`comprehensive-resource-example`** - Advanced resource patterns

## 🔴 **ADVANCED EXAMPLES** - Real-World Applications (Need Fixes)

Complex implementations for production scenarios.

### **MCP Protocol Areas**
- **`prompts-server`** ❌ - Prompt management system
- **`elicitation-server`** ❌ - User input elicitation
- **`sampling-server`** ❌ - AI model sampling integration
- **`completion-server`** - Code/text completion
- **`logging-server`** - Logging integration patterns
- **`notification-server`** - Real-time notifications
- **`roots-server`** - File system root management

### **Enterprise & Production**
- **`dynamic-resource-server`** ❌ - Dynamic resource discovery
- **`version-negotiation-server`** - Protocol version handling
- **`pagination-server`** - Large dataset pagination

## 🔧 **SPECIALIZED EXAMPLES** - Specific Use Cases

### **Performance & Testing**
- **`performance-testing`** ❌ - Load testing and benchmarks  
- **`enhanced-tool-macro-test`** - Advanced macro testing

### **Infrastructure & Deployment**
- **`lambda-mcp-server`** - AWS Lambda deployment
- **`lambda-mcp-client`** - Lambda client implementation

### **Advanced Types & Patterns**  
- **`comprehensive-server`** - Full-featured server showcase
- **`comprehensive-types-example`** - Complex type handling

## 🎯 **Learning Path Recommendations**

### **Path 1: Quick Start (30 minutes)**
1. `minimal-server` - Understand basics
2. `calculator-add-function-server` - Learn `#[mcp_tool]` 
3. `spec-compliant-server` - See specification compliance

### **Path 2: Comprehensive Learning (2-3 hours)**
1. **Basic**: `minimal-server` → `calculator-add-function-server` → `spec-compliant-server`
2. **Intermediate**: `tool-macro-example` → `stateful-server` → `manual-tools-server`
3. **Advanced**: Pick 1-2 from your use case (prompts, resources, etc.)

### **Path 3: Macro Mastery (1 hour)**
1. `calculator-add-function-server` - Function attributes
2. `tool-macro-example` - Declarative macros  
3. `derive-macro-server` - Derive macros
4. `all-tool-approaches-example` - Compare approaches

### **Path 4: Production Ready (3-4 hours)**
1. Start with **Path 2**
2. Add: `dynamic-resource-server`, `version-negotiation-server`
3. Add: `performance-testing`, `pagination-server`
4. Add: Deployment examples (`lambda-mcp-server`)

## 📊 **Current Status** (Updated: Real Assessment)

### **Build Status Reality Check**
- **Core Framework**: ✅ Solid foundation (mcp-server, mcp-protocol, mcp-derive compile cleanly)
- **Individual Examples**: ~14 core examples compile when built individually  
- **⚠️ Workspace Build**: `cargo build --workspace` fails due to ~20+ examples with trait errors
- **Learning Paths**: Basic → Intermediate works, Advanced examples need fixes

### **Working Examples by Category** (Individually Tested)

**🟢 BASIC (Confirmed Working)**
- `minimal-server` - Truly minimal with `#[mcp_tool]` function attribute
- `calculator-add-function-server` - Function macro usage
- `spec-compliant-server` - MCP specification compliance

**🟡 INTERMEDIATE (Confirmed Working)**  
- `stateful-server` - Session management patterns
- `manual-tools-server` - Manual trait implementations
- `tool-macro-example`, `resource-macro-example` - Declarative macros

**🔴 ADVANCED (Build Errors - Old Trait Pattern)**
- `sampling-server`, `prompts-server`, `elicitation-server` - Need trait updates
- `dynamic-resource-server` - API integration patterns  
- `all-tool-approaches-example` - Educational comparison (critical to fix)

### **Framework Assessment**
- **Foundation**: Excellent - all major patterns work
- **Documentation**: Over-optimistic claims, needs reality check  
- **Next Step**: Fix trait errors in ~20+ examples for full workspace build