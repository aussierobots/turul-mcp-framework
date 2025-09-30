# ADR-010: Architectural Guidelines and Design Principles

**Status**: Active
**Date**: 2025-01-25
**Supersedes**: Various sections from CLAUDE.md (consolidated)

## Context

The framework had accumulated extensive architectural guidance scattered across CLAUDE.md, mixing essential rules with historical context. This ADR consolidates core architectural decisions that guide framework development.

## Decisions

### 1. JSON-RPC 2.0 Compliance

**Decision**: Use separate response types for success and error cases per JSON-RPC 2.0 specification.

**Implementation**:
```rust
// SUCCESS RESPONSE
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": { /* success data */ }
}

// ERROR RESPONSE
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32603,
    "message": "Error description"
  }
}
```

**Architecture**: `JsonRpcMessage` enum with `JsonRpcResponse` and `JsonRpcError` variants ensures spec compliance.

**Rationale**: Strict JSON-RPC 2.0 compliance prevents client compatibility issues.

### 2. URI Security Standardization

**Decision**: Use standard `file://` URIs exclusively, avoid custom URI schemes.

**Pattern**:
```rust
// ✅ SECURE
"file:///memory/data.json"
"file:///session/info.json"
"file:///tmp/test.txt"

// ❌ BLOCKED
"memory://data"
"session://info"
"custom://resource"
```

**Rationale**: Custom URI schemes may be blocked by security middleware. Standard `file://` paths ensure maximum client compatibility.

**Reference**: See ADR-007 for security analysis.

### 3. Client Pagination API Design

**Decision**: Pagination helper methods accept cursor parameters only, not full request customization.

**API**:
```rust
// Simple cursor-based navigation
client.list_tools_paginated(cursor).await?;
client.list_resources_paginated(cursor).await?;
client.list_prompts_paginated(cursor).await?;

// Advanced control via underlying call()
client.call("tools/list", Some(RequestParams::List(ListToolsParams {
    cursor: Some(cursor),
    limit: Some(50),
    // ... other params
}))).await?
```

**Rationale**: Helper methods provide simple cursor-based navigation while preserving full control through the underlying `call()` method for advanced use cases.

### 4. MCP Tool Output Schema Compliance

**Decision**: Tools with `outputSchema` MUST provide `structuredContent`. Framework handles automatically.

**Implementation**:
- `mcp_tool` macro automatically uses `CallToolResult::from_result_with_schema()`
- Smart response builder adds `structuredContent` when `outputSchema` exists
- Custom output field names supported via `output_field` attribute

**Pattern**:
```rust
#[mcp_tool(
    name = "word_count",
    description = "Count words in text",
    output_field = "countResult"  // Optional, default "result"
)]
async fn count_words(text: String) -> McpResult<WordCount> {
    Ok(WordCount { count: text.split_whitespace().count() })
}

// Automatically generates:
// - content: ["{"count": 42}"]
// - structuredContent: {"countResult": {"count": 42}}
```

**Rule**: Tests validate MCP spec compliance - code must match spec, never change tests to match code.

**Rationale**: MCP 2025-06-18 specification requires `structuredContent` when `outputSchema` is defined. Framework automation ensures compliance without developer burden.

### 5. Session-Aware Resources (MCP 2025-06-18)

**Decision**: All resources are session-aware per MCP 2025-06-18 specification. No backwards compatibility layer.

**API**:
```rust
#[async_trait]
impl McpResource for MyResource {
    async fn read(&self, params: Option<Value>, session: Option<&SessionContext>)
        -> McpResult<Vec<ResourceContent>> {
        if let Some(ctx) = session {
            // Access session state for personalized content
        }
        // Graceful fallback when no session available
    }
}
```

**Migration**:
```rust
// OLD (Pre-0.2.0)
async fn read(&self, params: Option<Value>) -> McpResult<Vec<ResourceContent>>

// NEW (0.2.0+)
async fn read(&self, params: Option<Value>, session: Option<&SessionContext>) -> McpResult<Vec<ResourceContent>>
```

**Rationale**:
- MCP 2025-06-18 Compliance: Session context is fundamental to the specification
- Zero Configuration: One resource trait, one pattern, no confusing choices
- Future-Proof Architecture: No technical debt from legacy compatibility layers

**Status**: Complete - Breaking change shipped in 0.2.0

### 6. MCP 2025-06-18 Specification Compliance

**Decision**: Strict compliance with MCP 2025-06-18 specification. No custom extensions.

**Features**:
- Standard `ToolAnnotations` with spec-defined hint fields only
- Automatic inclusion in `tools/list` responses
- Wire output strictly follows official JSON schema
- No custom extensions that could cause client compatibility issues

**Example**:
```rust
impl HasAnnotations for LegacyTool {
    fn annotations(&self) -> Option<&ToolAnnotations> {
        Some(&ToolAnnotations::new()
            .with_title("Legacy Calculator (Add Only)")
            .with_read_only_hint(true)
            .with_destructive_hint(false)
            .with_idempotent_hint(true)
        )
    }
}
```

**Rationale**: Full specification compliance ensures maximum client compatibility.

**Status**: Complete - All phases implemented

## Consequences

### Positive
- **Client Compatibility**: Standards compliance ensures broad client support
- **Maintainability**: Clear architectural patterns reduce confusion
- **Specification Alignment**: Framework behavior matches MCP 2025-06-18 exactly
- **Developer Experience**: Automatic compliance features reduce boilerplate

### Negative
- **Breaking Changes**: Session-aware resources required migration in 0.2.0
- **API Constraints**: Pagination helpers limited to cursor-only parameters
- **URI Restrictions**: Custom URI schemes not supported

### Migration Support
- Clear migration guides for breaking changes
- Comprehensive examples demonstrating correct patterns
- Test-driven validation of specification compliance

## References

- **MCP 2025-06-18 Specification**: Official protocol specification
- **ADR-007**: Auto-Detection Resource Security
- **ADR-009**: Protocol-Based Handler Routing
- **WORKING_MEMORY.md**: Historical context and completed phases