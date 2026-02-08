# MCP 2025-11-25 Full Server Stack Compliance Report

**Date**: 2026-02-07
**Reviewer**: spec-compliance agent
**Scope**: Full MCP server stack (protocol, server, handlers, builders, HTTP transport, client, examples)
**Reference**: https://github.com/modelcontextprotocol/specification/blob/main/schema/2025-11-25/schema.ts

---

## Executive Summary

This report extends the protocol-only compliance review to cover the entire Turul MCP Framework stack. The protocol crate issues from the original COMPLIANCE_REPORT.md remain valid and are referenced here. This report adds findings for the server layer, handler dispatch, builder traits, HTTP transport, client crate, error codes, and examples.

**New findings**: 5 CRITICAL, 6 WARNING, 4 INFO

---

## CRITICAL Issues

### C1. No Task Handlers in Server or HTTP Transport

**Files**:
- `crates/turul-mcp-server/src/handlers/mod.rs` - No task handlers defined
- `crates/turul-mcp-server/src/dispatch/mod.rs` - No `tasks/*` routes registered
- `crates/turul-http-mcp-server/` - No `tasks/` handling anywhere

The server defines handlers for: `ping`, `completion/complete`, `prompts/list`, `prompts/get`, `resources/list`, `resources/read`, `resources/templates/list`, `sampling/createMessage`, `elicitation/create`, `logging/setLevel`, `roots/list`, `notifications/*`, and `notifications/initialized`.

**Missing handlers** (required by MCP 2025-11-25):
- `tasks/get` - Retrieve task status
- `tasks/cancel` - Cancel a running task
- `tasks/list` - List all tasks with pagination
- `tasks/result` - Retrieve task payload

**Missing notification** handler:
- `notifications/tasks/status` - Task status change notifications

Even though the protocol crate defines task types, no server-side handler processes these requests. A client sending `tasks/get` would receive a "Method not found" error.

### C2. Client Crate Test Compilation Failure

**File**: `crates/turul-mcp-client/src/session.rs:421`

The test `test_session_lifecycle` fails to compile because the `ServerCapabilities` struct literal is missing the `tasks` field that was added in the protocol-2025-11-25 crate:

```rust
let server_caps = ServerCapabilities {
    experimental: None,
    logging: None,
    prompts: None,
    resources: None,
    tools: None,
    completions: None,
    elicitation: None,
    // MISSING: tasks: None,  <-- causes E0063
};
```

This blocks `cargo test --package turul-mcp-client` and `cargo test` (workspace-wide).

**Note**: `cargo check --package turul-mcp-client` (non-test) passes fine because the missing field is only in test code.

### C3. Builder Traits Do Not Expose Icon Fields

**Files**:
- `crates/turul-mcp-builders/src/traits/tool_traits.rs:88-99` - `to_tool()` hardcodes `icon: None`
- `crates/turul-mcp-builders/src/traits/resource_traits.rs:227-239` - `to_resource()` hardcodes `icon: None`
- `crates/turul-mcp-builders/src/traits/prompt_traits.rs:204-213` - `to_prompt()` hardcodes `icon: None`

All three `ToolDefinition`, `ResourceDefinition`, and `PromptDefinition` traits produce protocol types with `icon: None`. There is no `HasIcon` trait or any way for users to provide icon data through the trait system.

Even if the protocol types are fixed to use `icons: Vec<Icon>`, the builder traits would still produce empty icon arrays because there's no trait method to supply icons.

### C4. Builder Traits Missing `tools` Field in SamplingDefinition

**File**: `crates/turul-mcp-builders/src/traits/sampling_traits.rs:157-170`

The `to_create_params()` method always sets `tools: None`. There is no trait method (e.g., `HasSamplingTools`) to provide tools for sampling requests, despite the protocol crate supporting `tools: Option<Vec<Tool>>` on `CreateMessageParams`.

### C5. ServerCapabilities.tasks Advertised But Not Served

**File**: `crates/turul-mcp-protocol-2025-11-25/src/initialize.rs:111-117`

The `TasksCapabilities` struct exists and can be set on `ServerCapabilities`, but:
1. No handlers exist to serve task requests (see C1)
2. The capability fields are wrong (has `listChanged: Option<bool>` but spec requires `list?`, `cancel?`, `requests?`)
3. A server advertising `tasks` capability but not handling `tasks/*` methods violates the spec

---

## WARNING Issues

### W1. Dispatch Error for Unknown Method Returns Wrong Error Code

**File**: `crates/turul-mcp-server/src/dispatch/mod.rs:148-153`

When no handler is found for a method, the dispatcher returns:
```rust
Err(McpError::InvalidParameters(format!("Method not found: {}", method)))
```

This maps to JSON-RPC error code `-32602` (Invalid params). The correct JSON-RPC error code for "Method not found" is **`-32601`** (Method not found). The `McpError` enum does not have a `MethodNotFound` variant.

### W2. ToolNotFound Error Code is Non-Standard

**File**: `crates/turul-mcp-protocol-2025-11-25/src/lib.rs:313-315`

`McpError::ToolNotFound` maps to error code `-32001`. The MCP spec does not define specific error codes for "tool not found" -- this should probably use the standard `-32602` (Invalid params) since the tool name is a parameter, or `-32601` (Method not found) if the tool is considered a sub-method.

Similarly:
- `ResourceNotFound` -> `-32002` (non-standard)
- `PromptNotFound` -> `-32003` (non-standard)

While not strictly wrong (these are in the server error range -32000 to -32099), they are framework-invented codes, not MCP-specified.

### W3. NotificationsHandler Missing `notifications/tasks/status`

**File**: `crates/turul-mcp-server/src/handlers/mod.rs:1354-1365`

The `NotificationsHandler::supported_methods()` lists:
- `notifications/message`
- `notifications/progress`
- `notifications/resources/listChanged`
- `notifications/resources/updated`
- `notifications/tools/listChanged`
- `notifications/prompts/listChanged`
- `notifications/roots/listChanged`

Missing: `notifications/tasks/status` (required by MCP 2025-11-25 for task lifecycle).

### W4. Client Session Uses Hardcoded Protocol Version "2025-06-18"

**File**: `crates/turul-mcp-client/src/session.rs:327`

The `create_initialize_request()` method hardcodes `protocol_version: "2025-06-18"`. If this client is used with the 2025-11-25 protocol crate, it will still request the old version. This should either use `MCP_VERSION` constant or be configurable.

### W5. No Builder Traits for Tasks

**File**: `crates/turul-mcp-builders/src/traits/mod.rs`

The traits module defines builder traits for tools, resources, prompts, sampling, elicitation, completion, logging, notifications, and roots. There is NO `task_traits.rs` module for composing task types via the builder pattern.

### W6. Elicitation Builder Lacks URL Mode

**File**: `crates/turul-mcp-builders/src/traits/elicitation_traits.rs`

The `ElicitationDefinition` trait only produces `ElicitCreateRequest` for form mode. There is no way to create URL-mode elicitation requests through the builder system.

---

## INFO Issues

### I1. Examples Reference "2025-06-18" Spec Version

Several example servers and documentation references use "MCP 2025-06-18" which was the previous spec version. While these examples work (they import from `turul-mcp-protocol` which now re-exports 2025-11-25 types), the version strings and comments may confuse users:

- `crates/turul-mcp-builders/src/traits/sampling_traits.rs:146` - "MCP 2025-06-18 specification"
- `crates/turul-mcp-builders/src/traits/elicitation_traits.rs:156` - "MCP 2025-06-18 specification"
- `crates/turul-mcp-server/src/handlers/mod.rs:304,333,342,436` - "MCP 2025-06-18" comments
- `crates/turul-mcp-client/src/session.rs:327` - Protocol version "2025-06-18"

### I2. Examples Compile Successfully

All 4 new showcase examples compile fine:
- `examples/icon-showcase/`
- `examples/sampling-with-tools-showcase/`
- `examples/task-types-showcase/`
- `examples/builders-showcase/`

The `turul-mcp-server` and `turul-http-mcp-server` crates compile successfully with `cargo check`.

### I3. Protocol Crate Tests Pass (121 + 2 Doc-Tests)

The `turul-mcp-protocol-2025-11-25` crate's own tests all pass:
```
test result: ok. 121 passed; 0 failed; 0 ignored
Doc-tests turul-mcp-protocol-2025-11-25: 2 passed
```

### I4. Builder `to_tool()`/`to_resource()`/`to_prompt()` Produce Valid JSON

Despite the icon hardcoding issue (C3), the builder trait `to_*()` methods produce correctly structured JSON with proper camelCase field names and proper skip_serializing_if handling.

---

## Cross-Reference: Protocol Crate Issues Still Open

The following issues from the original COMPLIANCE_REPORT.md remain unresolved and affect the full stack:

| ID | Severity | Summary |
|----|----------|---------|
| #1 | CRITICAL | Task types completely wrong (field names, enum values, missing fields) |
| #2 | CRITICAL | Icon type wrong structure (string vs object array) |
| #3 | CRITICAL | Implementation missing `description`, `websiteUrl` fields |
| #4 | CRITICAL | ModelHint is closed enum, should be struct with `name?: string` |
| #5 | CRITICAL | CreateMessageParams missing `toolChoice`, `task` fields |
| #6 | CRITICAL | CreateMessageResult content should support arrays |
| #7 | CRITICAL | Elicitation missing URL mode |
| #8 | MODERATE | ServerCapabilities.tasks wrong sub-fields |
| #9 | MODERATE | Sampling Role includes "System" (not in spec) |
| #10 | MODERATE | StringSchema missing `default` field |
| #11 | MODERATE | Elicitation content should be `string|number|boolean|string[]` not `Value` |
| #12 | MODERATE | Missing TaskStatusNotification type |

---

## Recommended Fix Priority

### P0 (Blocking - Must Fix Before Release)
1. Fix `turul-mcp-client` test compilation (C2) - add `tasks: None` to ServerCapabilities literal
2. Fix Task types in protocol crate (Protocol #1) - field names, enum values, required fields
3. Fix Icon type structure (Protocol #2) - `IconUrl` -> `Icon` struct, singular -> plural array

### P1 (High - Required for 2025-11-25 Compliance)
4. Add task handlers to server (C1) - implement `tasks/get`, `tasks/cancel`, `tasks/list`, `tasks/result`
5. Add `HasIcon` trait to builders (C3) - enable icon data through trait system
6. Fix dispatch error code (W1) - add `MethodNotFound` variant to McpError -> maps to `-32601`
7. Fix `ServerCapabilities.tasks` sub-fields (Protocol #8 + C5)
8. Add `notifications/tasks/status` handler (W3)
9. Fix ModelHint type (Protocol #4)
10. Fix Implementation missing fields (Protocol #3)

### P2 (Medium)
11. Add task builder traits (W5)
12. Add sampling tools trait (C4)
13. Add URL elicitation mode to builders (W6)
14. Add ToolChoice to CreateMessageParams (Protocol #5)
15. Fix client protocol version (W4)
16. Update "2025-06-18" references (I1)

### P3 (Low)
17. Fix content array support (Protocol #6)
18. Fix sampling Role enum (Protocol #9)
19. Fix elicitation content types (Protocol #11)
20. Add StringSchema.default (Protocol #10)
