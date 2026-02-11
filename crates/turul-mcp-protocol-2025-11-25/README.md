# turul-mcp-protocol-2025-11-25

[![Crates.io](https://img.shields.io/crates/v/turul-mcp-protocol-2025-11-25.svg)](https://crates.io/crates/turul-mcp-protocol-2025-11-25)
[![Documentation](https://docs.rs/turul-mcp-protocol-2025-11-25/badge.svg)](https://docs.rs/turul-mcp-protocol-2025-11-25)

Complete Model Context Protocol (MCP) specification implementation for the 2025-11-25 version with spec-pure concrete types, task lifecycle support, and icon metadata.

## Overview

`turul-mcp-protocol-2025-11-25` provides the protocol types for MCP specification version 2025-11-25. This crate extends the 2025-06-18 specification with four new feature areas: Icons, URL Elicitation, Sampling Tools, and Tasks.

For the 2025-06-18 spec, see `turul-mcp-protocol-2025-06-18`.

## New in 2025-11-25

### Icons

Tools, resources, prompts, resource templates, and server implementations can now include icons for display. Icons are represented as `Icon` structs with `src`, `mime_type`, `sizes`, and `theme` fields. The `src` field must be either a `data:` URI (RFC 2397) or an `https://` URL.

```rust
use turul_mcp_protocol_2025_11_25::icons::Icon;

// Icon from an HTTPS URL
let icon = Icon::new("https://example.com/search-icon.png");

// Icon from a data URI
let icon = Icon::data_uri("image/png", "iVBORw0KGgo=");

// Icons are optional arrays on tools, resources, prompts, and implementations
// icons: Option<Vec<Icon>>
```

### URL Elicitation

The elicitation system now supports URL input via the `StringFormat::Uri` format constraint. Use the builder convenience methods for one-call URL collection.

```rust
use turul_mcp_protocol_2025_11_25::{
    ElicitationBuilder, ElicitationSchema, PrimitiveSchemaDefinition,
    StringSchema, StringFormat,
};

// Quick URL input via builder
let request = ElicitationBuilder::url_input(
    "Please enter the repository URL",
    "repo_url",
    "The Git repository URL to clone",
);

// Manual URL schema construction
let schema = ElicitationSchema::new()
    .with_property(
        "website".to_string(),
        PrimitiveSchemaDefinition::url_with_description("Your website URL"),
    )
    .with_required(vec!["website".to_string()]);
```

### Sampling Tools

Sampling requests (`CreateMessageParams`) can now include a list of tools that the LLM may use during sampling. This allows servers to provide tool definitions alongside sampling context.

```rust
use turul_mcp_protocol_2025_11_25::{
    CreateMessageRequest, Tool, ToolSchema,
    sampling::SamplingMessage,
};

let tools = vec![
    Tool::new("web_search", ToolSchema::object())
        .with_description("Search the web"),
];

let request = CreateMessageRequest::new(
    vec![SamplingMessage::user_text("Find information about Rust")],
    1024,
).with_tools(tools);
```

### Tasks

The task system enables tracking of long-running operations. Tasks have a lifecycle defined by `TaskStatus` with full storage, handler, and E2E support.

**Key design**: There is no `tasks/create` method. Tasks are created implicitly when a client sends a task-augmented request (e.g., `tools/call` with a `task` field). The server returns a `CreateTaskResult` instead of the operation's direct result.

```rust
use turul_mcp_protocol_2025_11_25::tasks::{
    Task, TaskStatus, TaskMetadata,
    GetTaskParams, CancelTaskParams, ListTasksParams,
    GetTaskPayloadParams, CreateTaskResult,
};

// Task struct — the core task representation
let task = Task::new(
    "task-123",
    TaskStatus::Working,
    "2024-01-01T00:00:00Z",  // created_at (required)
    "2024-01-01T00:00:00Z",  // last_updated_at (required)
);

// Task-augmented request — add task field to CallToolParams
// task: Some(TaskMetadata { ttl: Some(60000) })

// Get task status
let get_params = GetTaskParams { task_id: "task-123".into(), meta: None };

// Cancel a working task
let cancel_params = CancelTaskParams { task_id: "task-123".into(), meta: None };

// Retrieve the original operation's result
let payload_params = GetTaskPayloadParams { task_id: "task-123".into(), meta: None };
```

#### Task Status Lifecycle

```
Working -> InputRequired  (needs user input)
Working -> Completed      (success)
Working -> Failed         (error)
Working -> Cancelled      (user/system cancellation)
InputRequired -> Working  (input provided)
InputRequired -> Completed | Failed | Cancelled
```

#### Task Operations

| Method | Params Type | Result Type | Notes |
|--------|------------|-------------|-------|
| `tasks/get` | `GetTaskParams` | `GetTaskResult` | Get current task status |
| `tasks/cancel` | `CancelTaskParams` | `CancelTaskResult` | Cancel a working task |
| `tasks/list` | `ListTasksParams` | `ListTasksResult` | Paginated task listing |
| `tasks/result` | `GetTaskPayloadParams` | Original result | Blocks until terminal; returns original operation's result |

#### Task Support on Tools

Tools declare their task support via `ToolExecution.task_support`:

| Value | Meaning |
|-------|---------|
| `TaskSupport::Required` | Clients MUST use task augmentation |
| `TaskSupport::Optional` | Clients MAY use task augmentation |
| `TaskSupport::Forbidden` | Clients MUST NOT use task augmentation |

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
turul-mcp-protocol-2025-11-25 = "0.3"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

### Basic Usage

```rust
use turul_mcp_protocol_2025_11_25::{
    Tool, ToolSchema, JsonSchema,
    InitializeRequest, InitializeResult,
    Implementation, ClientCapabilities, ServerCapabilities,
    McpVersion,
};
use turul_mcp_protocol_2025_11_25::icons::Icon;
use std::collections::HashMap;

// Create a tool (icons are optional — most tools don't need them)
let tool = Tool::new("calculator", ToolSchema::object()
    .with_properties(HashMap::from([
        ("a".to_string(), JsonSchema::number()),
        ("b".to_string(), JsonSchema::number()),
    ]))
    .with_required(vec!["a".to_string(), "b".to_string()])
)
.with_description("Add two numbers");

// Initialize with 2025-11-25 protocol version
let init_request = InitializeRequest::new(
    McpVersion::V2025_11_25,
    ClientCapabilities::default(),
    Implementation::new("my-client", "1.0.0"),
);

println!("Protocol version: {}", McpVersion::V2025_11_25);
```

## Version Capability Detection

```rust
use turul_mcp_protocol_2025_11_25::McpVersion;

let version = McpVersion::V2025_11_25;

assert!(version.supports_tasks());
assert!(version.supports_icons());
assert!(version.supports_url_elicitation());
assert!(version.supports_sampling_tools());
assert!(version.supports_elicitation());
assert!(version.supports_streamable_http());
assert!(version.supports_meta_fields());

// Feature list
let features = version.supported_features();
// ["streamable-http", "_meta-fields", "progress-token", "cursor",
//  "elicitation", "tasks", "icons", "url-elicitation", "sampling-tools"]
```

## MCP Message Coverage

### Client-to-Server Messages

| Message Type | Rust Struct | Module |
|:---|:---|:---|
| `initialize` | `InitializeRequest` | `initialize.rs` |
| `ping` | `PingRequest` | `ping.rs` |
| `tools/list` | `ListToolsRequest` | `tools.rs` |
| `tools/call` | `CallToolRequest` | `tools.rs` |
| `resources/list` | `ListResourcesRequest` | `resources.rs` |
| `resources/read` | `ReadResourceRequest` | `resources.rs` |
| `resources/subscribe` | `SubscribeRequest` | `resources.rs` |
| `resources/unsubscribe` | `UnsubscribeRequest` | `resources.rs` |
| `prompts/list` | `ListPromptsRequest` | `prompts.rs` |
| `prompts/get` | `GetPromptRequest` | `prompts.rs` |
| `tasks/get` | `GetTaskRequest` | `tasks.rs` |
| `tasks/cancel` | `CancelTaskRequest` | `tasks.rs` |
| `tasks/list` | `ListTasksRequest` | `tasks.rs` |
| `tasks/result` | `GetTaskPayloadRequest` | `tasks.rs` |
| `completion/complete` | `CompleteRequest` | `completion.rs` |
| `logging/setLevel` | `SetLevelRequest` | `logging.rs` |

### Server-to-Client Messages

| Message Type | Rust Struct | Module |
|:---|:---|:---|
| `sampling/createMessage` | `CreateMessageRequest` | `sampling.rs` |
| `elicitation/create` | `ElicitCreateRequest` | `elicitation.rs` |
| `roots/list` | `ListRootsRequest` | `roots.rs` |
| Various notifications | `*Notification` types | `notifications.rs` |

## Error Handling

```rust
use turul_mcp_protocol_2025_11_25::{McpError, McpResult};

fn process_task() -> McpResult<String> {
    // Protocol-level errors
    Err(McpError::InvalidParameters("Task ID is required".to_string()))
}

fn find_tool() -> McpResult<String> {
    Err(McpError::ToolNotFound("unknown_tool".to_string()))
}
```

## Testing

```bash
# Run all protocol tests
cargo test --package turul-mcp-protocol-2025-11-25

# Test specific areas
cargo test --package turul-mcp-protocol-2025-11-25 tasks
cargo test --package turul-mcp-protocol-2025-11-25 tools
cargo test --package turul-mcp-protocol-2025-11-25 elicitation
cargo test --package turul-mcp-protocol-2025-11-25 sampling
```

## Architecture

This crate follows the same spec-pure design as `turul-mcp-protocol-2025-06-18`:

- Zero framework features -- only official MCP 2025-11-25 types
- Identical module structure for cross-version familiarity
- Same trait hierarchy (HasMethod, HasParams, HasData, HasMeta, RpcResult)
- Independent test suite (150+ tests)

For the design rationale behind having separate crates per spec version, see
[ADR 015: MCP 2025-11-25 Protocol Crate Strategy](../../docs/adr/015-mcp-2025-11-25-protocol-crate.md).

## License

Licensed under the MIT License. See [LICENSE](../../LICENSE) for details.
