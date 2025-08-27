# MCP Framework - Example Fix Guide

**Purpose**: Step-by-step guide for fixing examples broken by trait refactoring  
**Status**: 5 documented examples need updates to use new ToolDefinition trait pattern

## 🔧 **Root Cause: Trait Refactoring**

The framework evolved from manual trait methods to fine-grained trait composition:

### Old Pattern (Broken)
```rust
// ❌ THIS PATTERN NO LONGER WORKS
impl McpTool for MyTool {
    fn name(&self) -> &str { "my_tool" }           // ← Method removed
    fn description(&self) -> &str { "Description" } // ← Method removed  
    fn input_schema(&self) -> ToolSchema { ... }    // ← Method removed
    
    async fn call(&self, args: Value, session: Option<SessionContext>) -> McpResult<CallToolResult> {
        // Implementation
    }
}
```

### New Pattern (Working)
```rust
// ✅ THIS IS THE NEW REQUIRED PATTERN
use mcp_protocol::tools::{HasBaseMetadata, HasDescription, HasInputSchema, ToolDefinition};

impl HasBaseMetadata for MyTool {
    fn name(&self) -> &str { "my_tool" }
    fn title(&self) -> Option<&str> { Some("Display Name") }
}

impl HasDescription for MyTool {
    fn description(&self) -> Option<&str> { Some("Description") }
}

impl HasInputSchema for MyTool {
    fn input_schema(&self) -> &ToolSchema { &self.schema }
}

// MyTool automatically implements ToolDefinition via trait composition

#[async_trait]
impl McpTool for MyTool {
    async fn call(&self, args: Value, session: Option<SessionContext>) -> McpResult<CallToolResult> {
        // Implementation (unchanged)
    }
}
```

## 📋 **Broken Examples Fix List**

### 1. completion-server ⚠️

**Issue**: Import errors
```rust
// ❌ BROKEN IMPORTS:
use mcp_protocol::completion::{CompleteRequest, CompletionResponse, CompletionSuggestion};
```

**Fix**: Update imports to correct types
```rust
// ✅ FIXED IMPORTS:
use mcp_protocol::completion::{CompleteRequest, CompleteResult, CompletionReference};
```

### 2. pagination-server ⚠️

**Issue**: Manual trait methods not in McpTool
```rust
// ❌ BROKEN - These methods no longer exist in McpTool:
impl McpTool for ListUsersTool {
    fn name(&self) -> &str { "list_users" }
    fn description(&self) -> &str { "List users..." }
    fn input_schema(&self) -> ToolSchema { ... }
}
```

**Fix**: Implement ToolDefinition trait
```rust
// ✅ FIXED - Use fine-grained traits:
use mcp_protocol::tools::{HasBaseMetadata, HasDescription, HasInputSchema};

struct ListUsersTool {
    schema: ToolSchema,  // Store schema as field
}

impl HasBaseMetadata for ListUsersTool {
    fn name(&self) -> &str { "list_users" }
    fn title(&self) -> Option<&str> { Some("List Users") }
}

impl HasDescription for ListUsersTool {
    fn description(&self) -> Option<&str> { 
        Some("List users with SQLite-based cursor pagination")
    }
}

impl HasInputSchema for ListUsersTool {
    fn input_schema(&self) -> &ToolSchema { &self.schema }
}

// Automatically implements ToolDefinition

#[async_trait]
impl McpTool for ListUsersTool {
    async fn call(&self, args: Value, session: Option<SessionContext>) -> McpResult<CallToolResult> {
        // Existing implementation unchanged
    }
}
```

### 3. elicitation-server ⚠️

**Issue**: ElicitationBuilder API changed
```rust
// ❌ BROKEN API CALL:
let elicitation = ElicitationBuilder::form("user_survey")
```

**Fix**: Use correct ElicitationBuilder API
```rust
// ✅ FIXED - Check current API:
let elicitation = ElicitationBuilder::new("user_survey")
    .with_title("User Survey")
    .with_description("Collect user information")
    .build()?;
```

### 4. dynamic-resource-server ⚠️

**Issue**: Manual trait methods + missing ToolDefinition bounds
```rust
// ❌ BROKEN - Same manual trait method issue:
impl McpTool for ApiHealthCheckTool {
    fn name(&self) -> &str { "call_enterprise_api" }
    // ... other manual methods
}
```

**Fix**: Same pattern as pagination-server
```rust
// ✅ FIXED - Use ToolDefinition trait pattern:
impl HasBaseMetadata for ApiHealthCheckTool {
    fn name(&self) -> &str { "call_enterprise_api" }
}

impl HasDescription for ApiHealthCheckTool {
    fn description(&self) -> Option<&str> { 
        Some("Health check for enterprise API endpoints")
    }
}

impl HasInputSchema for ApiHealthCheckTool {
    fn input_schema(&self) -> &ToolSchema { &self.schema }
}
```

### 5. logging-server ⚠️

**Issue**: Manual trait methods + unused imports
```rust
// ❌ BROKEN - Manual methods + unused imports:
use mcp_protocol::tools::{HasBaseMetadata, HasDescription}; // Unused!

impl McpTool for LogGeneratorTool {
    fn name(&self) -> &str { "generate_logs" }
    // ... manual methods
}
```

**Fix**: Remove unused imports, use ToolDefinition pattern
```rust
// ✅ FIXED - Use imports correctly:
use mcp_protocol::tools::{HasBaseMetadata, HasDescription, HasInputSchema};

impl HasBaseMetadata for LogGeneratorTool {
    fn name(&self) -> &str { "generate_logs" }
}

impl HasDescription for LogGeneratorTool {
    fn description(&self) -> Option<&str> { 
        Some("Generate log entries at different levels")
    }
}
```

## 🛠️ **Step-by-Step Fix Process**

### Step 1: Add Required Imports
```rust
use mcp_protocol::tools::{
    HasBaseMetadata, HasDescription, HasInputSchema, 
    HasOutputSchema, HasAnnotations, HasToolMeta,
    ToolDefinition
};
use async_trait::async_trait;
```

### Step 2: Convert Manual Methods to Trait Implementations

#### For each tool struct, replace:
```rust
// ❌ OLD (remove this):
impl McpTool for MyTool {
    fn name(&self) -> &str { "my_tool" }
    fn description(&self) -> &str { "Description" } 
    fn input_schema(&self) -> ToolSchema { /* schema */ }
    // ... other manual methods
    
    async fn call(&self, args: Value, session: Option<SessionContext>) -> McpResult<CallToolResult> {
        // Keep this part
    }
}
```

#### With:
```rust
// ✅ NEW (add these separate implementations):
impl HasBaseMetadata for MyTool {
    fn name(&self) -> &str { "my_tool" }
    fn title(&self) -> Option<&str> { None }  // Optional
}

impl HasDescription for MyTool {
    fn description(&self) -> Option<&str> { Some("Description") }
}

impl HasInputSchema for MyTool {
    fn input_schema(&self) -> &ToolSchema { 
        // Store schema as struct field or static
        &self.schema 
    }
}

// Implement other traits as needed:
impl HasOutputSchema for MyTool {
    fn output_schema(&self) -> Option<&ToolSchema> { None }
}

impl HasAnnotations for MyTool {
    fn annotations(&self) -> Option<&ToolAnnotations> { None }
}

impl HasToolMeta for MyTool {
    fn tool_meta(&self) -> Option<&HashMap<String, Value>> { None }
}

// MyTool now automatically implements ToolDefinition

#[async_trait]
impl McpTool for MyTool {
    async fn call(&self, args: Value, session: Option<SessionContext>) -> McpResult<CallToolResult> {
        // Keep existing implementation unchanged
    }
}
```

### Step 3: Store Schemas as Struct Fields

```rust
// ✅ PATTERN: Store schema as field for efficient access
struct MyTool {
    schema: ToolSchema,
}

impl MyTool {
    fn new() -> Self {
        Self {
            schema: ToolSchema::object()
                .with_properties(HashMap::from([
                    ("param1".to_string(), JsonSchema::string()),
                    ("param2".to_string(), JsonSchema::number()),
                ]))
                .with_required(vec!["param1".to_string()])
        }
    }
}

impl HasInputSchema for MyTool {
    fn input_schema(&self) -> &ToolSchema { &self.schema }
}
```

### Step 4: Test Compilation

```bash
# Test specific example
cargo check -p completion-server
cargo check -p pagination-server
cargo check -p elicitation-server
cargo check -p dynamic-resource-server  
cargo check -p logging-server

# Test all examples
cargo check
```

## 🎯 **Priority Order for Fixes**

### High Priority (Documented Examples)
1. **completion-server** - AI integration showcase
2. **pagination-server** - Data handling showcase  
3. **elicitation-server** - User interaction showcase
4. **logging-server** - Observability showcase
5. **dynamic-resource-server** - Resource handling showcase

### Medium Priority (Other Examples)
- Any other examples that fail compilation
- Examples not mentioned in EXAMPLES_SUMMARY.md

### Low Priority  
- Examples that compile with warnings only
- Deprecated examples that could be removed

## 📚 **Reference Implementations**

### Working Examples to Reference
- **client-initialise-report** - Uses current patterns, compiles cleanly
- **notification-server** - Fixed and working  
- **minimal-server** - Simple working example

### Trait Documentation  
- **mcp-protocol::tools module** - All available traits
- **MCP_SESSION_ARCHITECTURE.md** - Current architecture reference
- **STREAMABLE_HTTP_GUIDE.md** - Implementation examples

## 🧪 **Testing Fixed Examples**

### Compilation Test
```bash
cargo check -p example-name --quiet
# Should show 0 warnings, 0 errors
```

### Runtime Test  
```bash
cargo run -p example-name
# Should start without errors
```

### Integration Test
```bash
# Test with MCP compliance checker
cargo run --example client-initialise-report -- --url http://127.0.0.1:PORT/mcp
```

## ⚡ **Quick Fix Template**

For any broken example:

1. **Identify broken patterns**: Look for manual trait method implementations
2. **Add required imports**: HasBaseMetadata, HasDescription, HasInputSchema, etc.
3. **Split into trait implementations**: Each trait gets its own impl block
4. **Store schemas as fields**: Don't generate schemas in methods
5. **Keep call method unchanged**: Only the metadata methods change
6. **Test compilation**: Ensure 0 warnings/errors
7. **Update documentation**: If example is documented, verify it works

---

**BOTTOM LINE**: The trait refactoring improved the framework architecture but broke examples using old patterns. The fix is systematic: replace manual trait methods with fine-grained trait implementations while keeping tool execution logic unchanged.