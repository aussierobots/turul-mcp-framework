# Migration Guide: v0.2.0 → v0.2.1

## Overview

Version 0.2.1 introduces **protocol crate purity** - all framework traits have been moved from `turul-mcp-protocol` to `turul-mcp-builders`. This ensures the protocol crate remains a clean mirror of the official MCP specification.

## Breaking Changes

### 1. Framework Trait Location Change

**All framework traits moved:**
- FROM: `turul-mcp-protocol::{ToolDefinition, ResourceDefinition, PromptDefinition, ...}`
- TO: `turul-mcp-builders::traits::{ToolDefinition, ResourceDefinition, PromptDefinition, ...}`

**Affected traits:**
- Tool traits: `HasBaseMetadata`, `HasDescription`, `HasInputSchema`, `HasOutputSchema`, `HasAnnotations`, `HasToolMeta`, `ToolDefinition`
- Resource traits: `HasResourceMetadata`, `HasResourceDescription`, `HasResourceUri`, `HasResourceMimeType`, `HasResourceSize`, `HasResourceAnnotations`, `HasResourceMeta`, `ResourceDefinition`
- Prompt traits: `HasPromptMetadata`, `HasPromptDescription`, `HasPromptArguments`, `HasPromptAnnotations`, `HasPromptMeta`, `PromptDefinition`
- Root traits: `HasRootMetadata`, `HasRootPermissions`, `HasRootFiltering`, `HasRootAnnotations`, `RootDefinition`
- Sampling traits: `HasSamplingConfig`, `HasSamplingContext`, `HasModelPreferences`, `SamplingDefinition`, `HasSamplingMessageMetadata`
- Logging traits: `HasLoggingMetadata`, `HasLogLevel`, `HasLogFormat`, `HasLogTransport`, `LoggerDefinition`
- Completion traits: `HasCompletionMetadata`, `HasCompletionContext`, `HasCompletionHandling`, `CompletionDefinition`
- Elicitation traits: `HasElicitationMetadata`, `HasElicitationSchema`, `HasElicitationHandling`, `ElicitationDefinition`
- Notification traits: `HasNotificationMetadata`, `HasNotificationPayload`, `HasNotificationRules`, `NotificationDefinition`

### 2. Notification Payload API Change

**Trait signature changed:**
```rust
// Before (v0.2.0)
trait HasNotificationPayload {
    fn payload(&self) -> Option<&Value>;
}

// After (v0.2.1)
trait HasNotificationPayload {
    fn payload(&self) -> Option<Value>;  // Returns owned Value
}
```

**Why**: Enables on-demand serialization of typed params. Protocol types store typed structs (e.g., `ProgressNotificationParams`), not serialized `Value` objects. Cannot return reference to computed value.

**Impact**: Custom notification implementations must be updated.

## Migration Steps

### Step 1: Update Imports

**Use preludes for clean imports:**

```rust
// ✅ RECOMMENDED - Use preludes
use turul_mcp_server::prelude::*;   // Server development
use turul_mcp_builders::prelude::*; // Library development

// ❌ OLD - Don't import traits from protocol
use turul_mcp_protocol::{ToolDefinition, ResourceDefinition};
```

**Prelude contents:**
- `turul_mcp_server::prelude::*` - Re-exports builders prelude + server types
- `turul_mcp_builders::prelude::*` - All framework traits + common types

### Step 2: Update Custom Notification Implementations

If you implement `HasNotificationPayload` for custom notifications:

```rust
// ❌ BEFORE (v0.2.0)
impl HasNotificationPayload for MyNotification {
    fn payload(&self) -> Option<&Value> {
        self.data.as_ref()  // Returns reference
    }
}

// ✅ AFTER (v0.2.1)
impl HasNotificationPayload for MyNotification {
    fn payload(&self) -> Option<Value> {
        self.data.clone()  // Returns owned value
    }
}

// OR serialize on-demand:
impl HasNotificationPayload for MyNotification {
    fn payload(&self) -> Option<Value> {
        serde_json::to_value(&self.params).ok()
    }
}
```

### Step 3: Update Derive Macro Usage

**No changes needed** - derive macros automatically generate correct trait implementations:

```rust
// Works in both versions
#[derive(McpTool)]
struct Calculator {
    a: f64,
    b: f64,
}

#[mcp_tool(name = "add")]
async fn add(a: f64, b: f64) -> McpResult<f64> {
    Ok(a + b)
}
```

### Step 4: Verify Protocol Type Implementations

**Protocol types now implement traits automatically:**

All protocol types (Tool, Resource, Prompt, etc.) have trait implementations in `turul-mcp-builders/src/protocol_impls.rs`. You don't need to implement these yourself.

```rust
use turul_mcp_protocol::Tool;
use turul_mcp_builders::prelude::*;

// Tool automatically implements ToolDefinition
let tool = Tool {
    name: "my_tool".to_string(),
    description: Some("My tool".to_string()),
    input_schema: ToolSchema::default(),
    // ...
};

// Can use trait methods
assert_eq!(tool.name(), "my_tool");
assert_eq!(tool.description(), Some("My tool"));
```

## What Stays the Same

### 1. Protocol Types
All protocol types remain in `turul-mcp-protocol`:
```rust
use turul_mcp_protocol::{
    Tool, Resource, Prompt, Root,
    ToolSchema, ContentBlock, Annotations,
    // ... etc
};
```

### 2. Derive Macros
All derive macros remain in `turul-mcp-derive`:
```rust
use turul_mcp_derive::{McpTool, McpResource, McpPrompt, mcp_tool};
```

### 3. Server API
Server builder API unchanged:
```rust
let server = McpServer::builder()
    .name("my-server")
    .tool(my_tool)
    .resource(my_resource)
    .build()?;
```

## Benefits of This Change

1. **Protocol Crate Purity**: `turul-mcp-protocol` is now 100% MCP spec-compliant with zero framework code
2. **Clear Separation**: Protocol types (spec) vs. framework traits (impl) are architecturally distinct
3. **Better Maintainability**: Spec updates don't touch framework code
4. **Improved Notifications**: All notifications now properly serialize payloads (data loss bug fixed)

## Testing Your Migration

```bash
# Verify builds
cargo build --workspace

# Run tests
cargo test --workspace

# Check for warnings
cargo check --workspace

# Verify compliance
cargo test --test mcp_compliance_tests
cargo test --test notification_payload_correctness
```

## Need Help?

- Check examples in `examples/` directory - all updated to v0.2.1
- See `WORKING_MEMORY.md` for detailed technical context
- Review `CHANGELOG.md` for complete change list
- File issues at: https://github.com/anthropics/turul-mcp-framework/issues

## Quick Reference

| Old Import (v0.2.0) | New Import (v0.2.1) |
|---------------------|---------------------|
| `use turul_mcp_protocol::ToolDefinition;` | `use turul_mcp_builders::prelude::*;` |
| `use turul_mcp_protocol::{ResourceDefinition, PromptDefinition};` | `use turul_mcp_server::prelude::*;` |
| `fn payload(&self) -> Option<&Value>` | `fn payload(&self) -> Option<Value>` |

## Summary

✅ Use `turul_mcp_server::prelude::*` or `turul_mcp_builders::prelude::*` for imports
✅ Update custom `HasNotificationPayload` implementations to return owned `Value`
✅ Protocol types and derive macros work exactly as before
✅ All examples and tests updated and passing
