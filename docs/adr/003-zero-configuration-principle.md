# ADR-003: Zero-Configuration Design Principle

**Status**: MANDATORY  
**Date**: 2024-01-01  
**Decision Makers**: Framework Team

## Context

Users should never need to specify MCP method strings manually. The framework should auto-determine ALL methods from type information to ensure correctness and eliminate configuration errors.

## Problem

- Manual method specification is error-prone
- Users might use incorrect/invalid MCP method names
- Method strings scattered throughout codebase
- Hard to maintain MCP compliance

## Decision

**ABSOLUTE RULE**: Users NEVER specify method strings anywhere. The framework automatically determines ALL MCP methods from type names.

## Implementation

### ✅ CORRECT - Zero Configuration
```rust
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

### ❌ WRONG - User specifying methods (NEVER DO THIS!)
```rust
#[derive(McpNotification)]
#[notification(method = "notifications/progress")]  // ❌ NO METHOD STRINGS!
struct ProgressNotification;

#[mcp_tool(method = "tools/call")]  // ❌ NO METHOD STRINGS!
async fn calculator() -> Result<String, String> { Ok("result".to_string()) }
```

## Benefits

1. **MCP Compliance Guaranteed**: Impossible to use wrong/invalid methods
2. **Zero Configuration**: Developers focus on logic, not protocol details  
3. **Type Safety**: Method mapping happens at compile time
4. **Future Proof**: Framework can update methods without breaking user code
5. **Developer Experience**: IntelliSense works perfectly, no memorizing method strings

## Consequences

- **Positive**: Eliminates entire class of configuration errors
- **Positive**: Perfect MCP compliance by design
- **Positive**: Superior developer experience
- **Risk**: Framework must correctly implement all method mappings
- **Risk**: Less flexibility for edge cases (acceptable trade-off)