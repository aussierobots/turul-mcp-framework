# ADR: SessionContext Accessibility in Derive and Function Macros

**Status**: 🔴 **CRITICAL** - Framework Breaking Issue  
**Date**: 2025-08-28  
**Type**: Critical Bug Fix  

## Context

The MCP framework has a **fundamental architectural flaw** where SessionContext is not accessible to tool implementations when using derive macros (`#[derive(McpTool)]`) or function macros (`#[mcp_tool]`). This breaks the entire session-based architecture of MCP.

## The Problem: call() vs execute() Disconnect

### Current Architecture (BROKEN)
```
1. Framework calls: McpTool.call(args, session: Option<SessionContext>)
2. Derive macro generates: call() method that ignores `_session` parameter  
3. User implements: execute() method with NO session access
4. Result: 💥 SessionContext is completely lost
```

### What Should Happen
```
1. Framework calls: McpTool.call(args, session: Option<SessionContext>)
2. Macro passes session to user's execute method
3. User code: Can access session for state, notifications, tracking
4. Result: ✅ Full MCP functionality available
```

## Impact Analysis

### Critical Features Broken by This Bug:

1. **State Management**: `session.get_typed_state().await` / `set_typed_state().await` - UNAVAILABLE
2. **Real-time Notifications**: `session.notify_progress()` - UNAVAILABLE  
3. **Session Tracking**: `session.session_id` - UNAVAILABLE
4. **Future Session Features**: Any session-based capability - UNAVAILABLE

### Affected Code:

- **All derive macro tools**: Cannot access session
- **All function macro tools**: Cannot access session  
- **90% of framework examples**: Limited to stateless operations
- **Production deployments**: Cannot use advanced MCP features

## Current Evidence

### Derive Macro (tool_derive.rs:192)
```rust
async fn call(&self, args: Value, _session: Option<SessionContext>) -> ... {
    // _session is IGNORED! (underscore prefix = unused)
    instance.execute().await  // No session passed to execute()
}
```

### Function Macro (tool_attr.rs:207)  
```rust
async fn call(&self, args: Value, _session: Option<SessionContext>) -> ... {
    // _session is IGNORED!
    original_fn(extracted_params).await  // No session passed to function
}
```

### User Experience Impact
- Users must choose between 90% code reduction (macros) OR advanced features (manual implementation)
- This defeats the purpose of the macro system
- Forces anti-patterns like global state with Mutex

## Decision

**We MUST fix this immediately** because:

1. **Framework Integrity**: This breaks the core promise of session-based MCP
2. **User Experience**: Macros should provide ALL framework features, not a subset
3. **Production Viability**: Stateless tools severely limit real-world applications
4. **Simple Fix**: The solution is backward compatible and straightforward

## Solution

### Phase 1: Fix Derive Macro

**File**: `crates/turul-mcp-derive/src/tool_derive.rs`

**Changes**:
1. Line 192: Change `_session` → `session`
2. Add detection for `execute_with_session` method
3. Pass session to execute method when available
4. Maintain backward compatibility

**Implementation Options**:
```rust
// Option A: Always pass session (breaking change)
async fn execute(&self, session: Option<SessionContext>) -> McpResult<T>

// Option B: Detect method signature (backward compatible) ✅ PREFERRED
async fn execute_with_session(&self, session: Option<SessionContext>) -> McpResult<T>
async fn execute(&self) -> McpResult<T>  // Falls back to this
```

### Phase 2: Fix Function Macro

**File**: `crates/turul-mcp-derive/src/tool_attr.rs`

**Changes**:
1. Line 207: Change `_session` → `session`
2. Add support for `#[session]` parameter attribute
3. Pass session to function when `#[session]` parameter detected

**New Syntax**:
```rust
#[mcp_tool(name = "my_tool")]
async fn my_tool(
    #[param(description = "User input")] input: String,
    #[session] session: Option<SessionContext>,  // NEW!
) -> McpResult<String> {
    // Can now use session.notify_progress(), session.get_typed_state(), etc.
}
```

### Phase 3: Update Examples

Convert stateful examples to demonstrate session usage:
- `simple-logging-server` - Use macros with session state
- `stateful-server` - Show session patterns
- Performance examples - Add session-based notifications

## Implementation Plan

### Step 1: Derive Macro Fix
```rust
// In tool_derive.rs, replace:
async fn call(&self, args: Value, _session: Option<SessionContext>) -> ... {

// With:
async fn call(&self, args: Value, session: Option<SessionContext>) -> ... {
    // Check if struct implements execute_with_session
    // If yes, call it with session
    // If no, fall back to execute() for backward compatibility
}
```

### Step 2: Function Macro Fix
```rust
// In tool_attr.rs, detect #[session] parameter:
for input_arg in &input.sig.inputs {
    if has_session_attribute(input_arg) {
        // Generate code that passes session to function
    }
}
```

### Step 3: Testing & Validation
- Update existing tests to verify session passing
- Add new tests for session functionality
- Verify backward compatibility
- Test with examples

## SessionContext Availability Analysis

### Critical Question: Can SessionContext be stored in &self?

**Answer: NO** - SessionContext cannot and should not be stored in tool struct fields.

#### Technical Reasons:

1. **Lifecycle Mismatch**:
   ```rust
   // Tools are stored once and shared across all sessions
   tools: HashMap<String, Arc<dyn McpTool>>,  // Shared state
   
   // SessionContext is created per-request
   async fn handle_request(session: SessionContext) { /* ... */ }
   ```

2. **SessionContext Structure Contains Non-Storable Elements**:
```rust
use std::future::Future;
use std::pin::Pin;

type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send + 'static>>;

pub struct SessionContext {
    pub session_id: String,
    pub get_state: Arc<dyn Fn(&str) -> BoxFuture<Option<Value>> + Send + Sync>, // Async closures!
    pub set_state: Arc<dyn Fn(&str, Value) -> BoxFuture<()> + Send + Sync>,     // Async closures!
    pub broadcaster: Option<Arc<dyn std::any::Any + Send + Sync>>,              // Session-specific
}
```

3. **Function Macros Generate Zero-Sized Structs**:
   ```rust
   #[mcp_tool(name = "calculator")]
   async fn calculator(a: f64, b: f64) -> McpResult<f64> { Ok(a + b) }
   
   // Generates:
   #[derive(Clone)]
   struct CalculatorToolImpl;  // No fields to store SessionContext!
   ```

4. **Trait Requirements**:
   ```rust
   // Tools must be Send + Sync for Arc<dyn McpTool>
   // SessionContext contains session-specific closures that can't be shared
   ```

#### Architectural Implications:

1. **Parameter Passing is the Only Solution**:
   - Derive macros: `execute(session: Option<SessionContext>)`
   - Function macros: `tool(params..., session: Option<SessionContext>)`

2. **Session-Specific vs Tool-Shared**:
   - **Tool Logic**: Shared across all sessions (in &self)
   - **Session State**: Passed per-request (as parameter)
   - **Session Actions**: Available through parameter only

3. **No Workarounds Possible**:
   - Cannot store SessionContext in static variables (session-specific)
   - Cannot store in Arc<Mutex<>> (breaks async + performance)
   - Cannot store in &self fields (lifecycle + sharing violations)

### Conclusion: Parameter-Based Architecture is Correct

The current approach of passing SessionContext as a parameter is the **only architecturally sound solution**. Any attempt to store SessionContext in tool structs would violate fundamental Rust ownership principles and MCP session semantics.

## Expected Benefits

1. **Full Feature Access**: All MCP features available with macros
2. **Maintained Efficiency**: Still 90% code reduction vs manual implementation
3. **Type Safety**: Compile-time checking for session usage
4. **Backward Compatible**: Existing derive macro tools continue working
5. **Framework Consistency**: All tool creation methods support sessions
6. **Correct Architecture**: SessionContext properly scoped to individual requests

## Migration Path

### For Existing Derive Macro Tools:
```rust
// Before (no session access):
impl MyTool {
    async fn execute(&self) -> McpResult<String> {
        // No session access
    }
}

// After (with session access):
impl MyTool {
    async fn execute_with_session(&self, session: Option<SessionContext>) -> McpResult<String> {
        if let Some(session) = session {
        session.notify_progress("working", 50).await;
        }
        // Full functionality now available
    }
}
```

### For Function Macro Tools:
```rust
// Before (SessionContext not available):
#[mcp_tool(name = "tool")]
async fn my_tool(input: String) -> McpResult<String> { ... }

// After (SessionContext available as parameter):
#[mcp_tool(name = "tool")]
async fn my_tool(
    input: String,
    session: Option<SessionContext>  // No #[session] attribute needed!
) -> McpResult<String> { 
    if let Some(session) = session {
        session.notify_progress("working", 50).await;
    session.set_typed_state("last_input", &input).await.unwrap();
    }
    // Full functionality now available
    Ok(format!("Processed: {}", input))
}
```

**Note**: The function macro automatically detects `SessionContext` parameters by type - no attribute annotation required.

## Success Criteria

1. ✅ Derive macros can access SessionContext
2. ✅ Function macros can access SessionContext  
3. ✅ Existing code continues to work (backward compatible)
4. ✅ simple-logging-server converted to use macros with full functionality
5. ✅ All tests pass
6. ✅ Documentation updated with session usage patterns

## Consequences

### Positive
- **Unified Developer Experience**: All tool creation methods equally powerful
- **Production Ready**: Macros can be used for real applications
- **Framework Integrity**: MCP session architecture fully functional

### Risks
- **Implementation Complexity**: Need to handle both old and new method signatures
- **Testing Required**: Ensure backward compatibility works correctly
- **Documentation Updates**: Need to update all macro documentation

## Conclusion

This is a **critical architectural bug** that must be fixed immediately. The solution maintains backward compatibility while enabling full MCP functionality through macros, fulfilling the framework's promise of providing powerful tools with minimal boilerplate.

**Priority**: 🔴 **CRITICAL** - Blocks all other feature work until resolved
