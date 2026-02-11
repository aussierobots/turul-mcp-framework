# ADR-015: MCP 2025-11-25 Protocol Crate Strategy

**Status**: Accepted

**Date**: 2026-02-07

## Context

The MCP specification released version 2025-11-25 with several new features:
- **Icons** on tools, resources, prompts, resource templates, and implementations
- **URL Elicitation** via `StringFormat::Uri` and builder conveniences
- **Sampling Tools** allowing tools in `CreateMessageParams`
- **Tasks** (experimental) for long-running operation tracking

We needed to decide how to support the new spec version alongside the existing
2025-06-18 implementation. Three options were considered:

1. **Feature flags** on the existing `turul-mcp-protocol-2025-06-18` crate
2. **In-place upgrade** of the existing crate to 2025-11-25
3. **Separate crate** `turul-mcp-protocol-2025-11-25`

### Why not feature flags?

Feature flags would make the existing crate more complex. The 2025-11-25 spec adds
new fields to existing types (e.g., `icon` on `Tool`, `tools` on `CreateMessageParams`),
which means every struct definition would need `#[cfg(feature = "...")]` annotations.
This creates a maintenance burden, makes the code harder to read, and risks subtle
serialization bugs when the wrong feature combination is used.

### Why not in-place upgrade?

Servers that negotiate 2025-06-18 with clients must not send 2025-11-25 fields.
An in-place upgrade would either break backward compatibility or require runtime
field stripping, which is error-prone.

## Decision

Create a separate `turul-mcp-protocol-2025-11-25` crate that mirrors the structure
of `turul-mcp-protocol-2025-06-18` but includes all 2025-11-25 additions.

### Key design choices:

1. **Same module structure** - Both crates have identical module layout (tools.rs,
   resources.rs, prompts.rs, etc.) so developers familiar with one can navigate the other.

2. **Same trait hierarchy** - Both crates implement the same protocol traits (HasMethod,
   HasParams, HasData, HasMeta, RpcResult, Params) ensuring framework compatibility.

3. **Icon in tools.rs** - The `Icon` type (with `src`, `mime_type`, `sizes`, `theme` fields)
   lives in `tools.rs` (not a separate module) and is referenced by other modules via
   `crate::tools::Icon`.

4. **Tasks as a new module** - `tasks.rs` follows the same Request/Params/Result pattern
   as tools, resources, and prompts.

5. **Version negotiation via McpVersion** - The `version.rs` module defines
   `McpVersion::V2025_11_25` with capability detection methods like `supports_tasks()`,
   `supports_icons()`, etc.

6. **Runtime protocol selection** - The HTTP transport layer (turul-http-mcp-server)
   will route to the correct protocol crate based on the negotiated version,
   extending the existing protocol-based handler routing (ADR 009).

## Consequences

### Positive

- **Clean separation** - Each crate is a self-contained, correct implementation of its
  spec version. No conditional compilation complexity.
- **Parallel development** - Changes to 2025-11-25 types cannot accidentally break
  2025-06-18 compliance.
- **Independent testing** - Each crate has its own test suite (121+ tests for 2025-11-25)
  that validates spec compliance in isolation.
- **Clear versioning** - The crate name itself communicates which spec version it implements.
- **Gradual migration** - Framework integration (Phase 6) can proceed incrementally
  without touching the stable 2025-06-18 code path.

### Negative

- **Code duplication** - Common types (JsonRpcRequest, ContentBlock, etc.) exist in both
  crates. Changes to shared patterns must be applied in both places.
- **Workspace size** - An additional crate adds to build times and workspace complexity.
- **Version alias complexity** - `turul-mcp-protocol` currently aliases to 2025-06-18.
  When 2025-11-25 becomes the default, this alias must be updated carefully.

### Risks

- **Divergence** - The two crates could diverge in non-spec ways (different error messages,
  different builder patterns). Mitigation: maintain consistent patterns and review changes
  across both crates.
- **Framework integration complexity** - The server and builders crates must support both
  protocol crate versions. Mitigation: use trait-based dispatch at the handler level,
  keeping protocol-specific logic contained.

## Implementation

### Crate Structure

```
crates/turul-mcp-protocol-2025-11-25/
  Cargo.toml
  README.md
  src/
    lib.rs          # Re-exports, McpError, McpResult
    tools.rs        # Tool, Icon, ToolSchema, CallTool*, ListTools*
    resources.rs    # Resource, ResourceTemplate (with icon field)
    prompts.rs      # Prompt (with icon field)
    tasks.rs        # TaskStatus, Task, TaskMetadata, query/result types
    sampling.rs     # CreateMessageParams (with tools field)
    elicitation.rs  # StringFormat::Uri, ElicitationBuilder::url_input()
    initialize.rs   # Implementation (with icon field)
    version.rs      # McpVersion::V2025_11_25 with capability methods
    traits.rs       # Protocol trait hierarchy
    ...             # (remaining modules mirror 2025-06-18)
```

### New Types (2025-11-25 only)

| Module | New Types | Purpose |
|--------|-----------|---------|
| tools.rs | `Icon` | Icon with src, mime_type, sizes, theme |
| tasks.rs | `TaskStatus`, `Task`, `TaskMetadata` | Task lifecycle |
| tasks.rs | `GetTask*`, `CancelTask*`, `ListTasks*`, `GetTaskPayload*`, `CreateTaskResult` | Task queries and result reporting |
| elicitation.rs | `StringFormat::Uri` | URL format constraint |
| sampling.rs | `tools` field on `CreateMessageParams` | Sampling tools |

### Version Negotiation Flow

```
Client sends initialize request with protocolVersion
  -> Server checks McpVersion::from_str(version)
  -> If V2025_11_25: use turul-mcp-protocol-2025-11-25 types
  -> If V2025_06_18: use turul-mcp-protocol-2025-06-18 types
  -> Handler routing via StreamableHttpHandler (see ADR 009)
```

## See Also

- [ADR 009: Protocol-Based Handler Routing](./009-protocol-based-handler-routing.md)
- [ADR 014: Schemars Schema Generation](./014-schemars-schema-generation.md)
- [WORKING_MEMORY.md](../../WORKING_MEMORY.md) - Current migration status
