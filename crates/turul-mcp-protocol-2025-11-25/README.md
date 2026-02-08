# turul-mcp-protocol-2025-11-25

[![Crates.io](https://img.shields.io/crates/v/turul-mcp-protocol-2025-11-25.svg)](https://crates.io/crates/turul-mcp-protocol-2025-11-25)
[![Documentation](https://docs.rs/turul-mcp-protocol-2025-11-25/badge.svg)](https://docs.rs/turul-mcp-protocol-2025-11-25)

Complete Model Context Protocol (MCP) specification implementation for the 2025-11-25 version with spec-pure concrete types, task lifecycle support, and icon metadata.

## Overview

`turul-mcp-protocol-2025-11-25` provides the protocol types for MCP specification version 2025-11-25. This crate extends the 2025-06-18 specification with four new feature areas: Icons, URL Elicitation, Sampling Tools, and Tasks.

For the 2025-06-18 spec, see `turul-mcp-protocol-2025-06-18`.

## New in 2025-11-25

### Icons

Tools, resources, prompts, resource templates, and server implementations can now include an icon for display. Icons are represented as `IconUrl` values that must be either a `data:` URI (RFC 2397) or an `https://` URL.

```rust
use turul_mcp_protocol_2025_11_25::{Tool, ToolSchema, IconUrl};

// Icon from an HTTPS URL
let tool = Tool::new("search", ToolSchema::object())
    .with_description("Search the web")
    .with_icon(IconUrl::https("https://example.com/search-icon.png"));

// Icon from a data URI
let tool = Tool::new("calculator", ToolSchema::object())
    .with_icon(IconUrl::data_uri("image/png", "iVBORw0KGgo="));

// Icons on resources
use turul_mcp_protocol_2025_11_25::Resource;
let resource = Resource::new("file:///config.json", "app_config")
    .with_icon(IconUrl::https("https://example.com/config-icon.png"));

// Icons on server implementation
use turul_mcp_protocol_2025_11_25::Implementation;
let server = Implementation::new("my-server", "1.0.0")
    .with_title("My MCP Server")
    .with_icon(IconUrl::https("https://example.com/server-icon.png"));
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

### Tasks (Experimental)

The task system enables tracking of long-running operations. Tasks have a lifecycle defined by `TaskStatus` and support progress reporting.

```rust
use turul_mcp_protocol_2025_11_25::{
    TaskInfo, TaskStatus, TaskProgress,
    CreateTaskRequest, GetTaskRequest,
    CancelTaskRequest, ListTasksRequest,
};

// Create a task
let request = CreateTaskRequest::new()
    .with_message("Processing large dataset");

// Task with progress tracking
let task = TaskInfo::new("task-123", TaskStatus::Running)
    .with_message("Processing batch 3 of 10")
    .with_progress(TaskProgress::new(30).with_total(100));

// Get task status
let get_request = GetTaskRequest::new("task-123");

// Cancel a running task
let cancel_request = CancelTaskRequest::new("task-123");

// List all tasks with pagination
let list_request = ListTasksRequest::new().with_limit(25);
```

#### Task Status Lifecycle

```
Running -> Completed   (success)
Running -> Failed      (error)
Running -> Cancelled   (user/system cancellation)
```

#### Task CRUD Operations

| Method | Request Type | Result Type |
|--------|-------------|-------------|
| `tasks/create` | `CreateTaskRequest` | `CreateTaskResult` |
| `tasks/get` | `GetTaskRequest` | `GetTaskResult` |
| `tasks/cancel` | `CancelTaskRequest` | `CancelTaskResult` |
| `tasks/list` | `ListTasksRequest` | `ListTasksResult` |

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
turul-mcp-protocol-2025-11-25 = "0.2"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

### Basic Usage

```rust
use turul_mcp_protocol_2025_11_25::{
    Tool, ToolSchema, JsonSchema, IconUrl,
    InitializeRequest, InitializeResult,
    Implementation, ClientCapabilities, ServerCapabilities,
    McpVersion,
};
use std::collections::HashMap;

// Create a tool with icon
let tool = Tool::new("calculator", ToolSchema::object()
    .with_properties(HashMap::from([
        ("a".to_string(), JsonSchema::number()),
        ("b".to_string(), JsonSchema::number()),
    ]))
    .with_required(vec!["a".to_string(), "b".to_string()])
)
.with_description("Add two numbers")
.with_icon(IconUrl::https("https://example.com/calc.png"));

// Initialize with 2025-11-25 protocol version
let init_request = InitializeRequest::new(
    McpVersion::V2025_11_25,
    ClientCapabilities::default(),
    Implementation::new("my-client", "1.0.0"),
);

println!("Protocol version: {}", McpVersion::V2025_11_25);
println!("Tool: {} (icon: {})", tool.name, tool.icon.is_some());
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
| `tasks/create` | `CreateTaskRequest` | `tasks.rs` |
| `tasks/get` | `GetTaskRequest` | `tasks.rs` |
| `tasks/cancel` | `CancelTaskRequest` | `tasks.rs` |
| `tasks/list` | `ListTasksRequest` | `tasks.rs` |
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
- Independent test suite (121+ tests)

For the design rationale behind having separate crates per spec version, see
[ADR 015: MCP 2025-11-25 Protocol Crate Strategy](../../docs/adr/015-mcp-2025-11-25-protocol-crate.md).

## License

Licensed under the MIT License. See [LICENSE](../../LICENSE) for details.
