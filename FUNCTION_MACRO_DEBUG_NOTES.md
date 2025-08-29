# Function Macro Debug Notes: `#[mcp_tool]` Issue

**Issue**: `#[mcp_tool]` function attribute macro fails with JsonSchema vs Value type mismatch
**Status**: ✅ **RESOLVED** - Function macro now works correctly
**Date**: 2025-08-28 (Initial), 2025-08-28 (Resolved)

## Problem Description

### Error
```
error[E0308]: mismatched types
   --> examples/minimal-server/src/main.rs:10:1
    |
 10 | #[mcp_tool(name = "echo", description = "Echo back the input text")]
    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    | |
    | expected `HashMap<String, Value>`, found `HashMap<String, JsonSchema>`
```

### Affected Examples
- `examples/minimal-server/` - Simple echo function
- `examples/function-macro-server/` - Multiple function tools

## Investigation Results

### What Works ✅
- **Derive macros**: `#[derive(McpTool)]` compiles perfectly
- **Core framework**: All 5 framework crates compile with zero errors/warnings
- **Declarative macros**: `tool!{}` macro works correctly

### What Fails ❌
- **Function attribute macro**: `#[mcp_tool]` has persistent type mismatch

### Technical Analysis

#### Same Code Pattern
Both derive macro (`tool_derive.rs`) and function macro (`tool_attr.rs`) use identical HashMap construction:

```rust
// Both use this pattern:
.with_properties(HashMap::from([
    #(#schema_properties),*
]))

// Both generate schema_properties like:
schema_properties.push(quote! {
    (#param_name_str.to_string(), serde_json::to_value(&#schema).unwrap_or_else(|_| serde_json::json!({"type": "string"})))
});
```

#### Different Results
- **derive-macro-server**: ✅ Compiles successfully
- **minimal-server**: ❌ HashMap type mismatch error

#### Debugging Attempts
1. **Runtime conversion**: `serde_json::to_value(&#schema)` - Failed
2. **Compile-time generation**: `type_to_json_value()` function - Failed
3. **Clean rebuilds**: Multiple attempts - Failed
4. **Token stream analysis**: Same pattern in both macros

### Root Cause Hypothesis
The function attribute macro token expansion context differs from derive macro context in how `#schema` tokens are resolved, despite identical code patterns.

## Workaround 

### Use Derive Macros Instead
```rust
// ❌ BROKEN: Function attribute approach
#[mcp_tool(name = "echo", description = "Echo back the input text")]
async fn echo(text: String) -> Result<String, String> {
    Ok(format!("Echo: {}", text))
}

// ✅ WORKING: Derive macro approach  
#[derive(McpTool)]
#[tool(name = "echo", description = "Echo back the input text")]
struct EchoTool {
    #[param(description = "Text to echo")]
    text: String,
}

impl EchoTool {
    async fn execute(&self) -> McpResult<String> {
        Ok(format!("Echo: {}", self.text))
    }
}
```

## Impact Assessment

### Production Impact: **MINIMAL**
- Core framework: ✅ Fully functional
- Alternative approach: ✅ Available and working
- Examples affected: 2 out of 25 examples
- User impact: Minimal - derive macros provide same functionality

### Future Debugging Strategy
1. **Macro expansion analysis**: Use `cargo expand` to compare token streams
2. **AST debugging**: Analyze syntax tree differences between contexts  
3. **Rustc version testing**: Check if issue is compiler-version specific
4. **Incremental approach**: Isolate the specific token causing type mismatch

## Documentation Updates
- NEW_OUTSTANDING_ITEMS.md: Added function macro issue to medium priority
- Examples: Both approaches documented (function and derive)
- Workaround: Clearly documented in framework guides

## ✅ Resolution (2025-08-28)

The function macro issue has been **completely resolved** through JsonSchema standardization:

### Root Cause Identified
The issue was caused by `ToolSchema::with_properties()` expecting `HashMap<String, serde_json::Value>` while the macros were trying to pass `HashMap<String, JsonSchema>`. The `serde_json::to_value()` conversion wasn't working correctly in the macro expansion context.

### Solution Implemented
1. **Standardized ToolSchema**: Changed `ToolSchema.properties` to use `HashMap<String, JsonSchema>` directly
2. **Simplified Macros**: Removed `serde_json::to_value()` conversion - pass JsonSchema objects directly
3. **Updated Builders**: Fixed both `mcp-protocol-2025-06-18` and `mcp-builders` tool construction
4. **Architecture Improvement**: Unified schema handling across the entire framework

### Current Status
- ✅ **minimal-server compiles**: `#[mcp_tool]` function macro works perfectly
- ✅ **derive-macro-server compiles**: No regression in derive macro functionality  
- ✅ **function-macro-server compiles**: Additional function examples work
- ✅ **Clean compilation**: Zero warnings across core framework

### Benefits of Resolution
- **Simpler Architecture**: No conversion layer needed between JsonSchema and Value
- **Type Safety**: Stronger typing with JsonSchema enum vs generic Value
- **MCP Compliance**: JsonSchema serializes to identical JSON format as before
- **Performance**: Eliminates runtime conversion overhead

## Legacy Recommendation (No Longer Needed)
~~**Proceed with derive macros** for now. Function attribute macros are a convenience feature, not core functionality. The framework is production-ready with derive macro support.~~

**New Status**: Both function macros (`#[mcp_tool]`) and derive macros (`#[derive(McpTool)]`) work correctly and can be used interchangeably based on developer preference.