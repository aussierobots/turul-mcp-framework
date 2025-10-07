# turul-mcp-protocol-2025-06-18

[![Crates.io](https://img.shields.io/crates/v/turul-mcp-protocol-2025-06-18.svg)](https://crates.io/crates/turul-mcp-protocol-2025-06-18)
[![Documentation](https://docs.rs/turul-mcp-protocol-2025-06-18/badge.svg)](https://docs.rs/turul-mcp-protocol-2025-06-18)

Complete Model Context Protocol (MCP) specification implementation for the 2025-06-18 version with comprehensive trait-based architecture and type-safe protocol compliance.

## Overview

`turul-mcp-protocol-2025-06-18` provides the foundational protocol implementation for the turul-mcp-framework, including all request/response types, notifications, and trait definitions for the MCP 2025-06-18 specification.

## Features

- ✅ **Complete MCP 2025-06-18 Implementation** - All protocol messages and types
- ✅ **Trait-Based Architecture** - Fine-grained traits for composable implementations  
- ✅ **Type Safety** - Strong typing with comprehensive compile-time validation
- ✅ **Serde Integration** - Full JSON serialization/deserialization support
- ✅ **JSON-RPC 2.0 Compliance** - Proper JSON-RPC message wrapping
- ✅ **Schema Generation** - Built-in JSON Schema support for tool parameters
- ✅ **Meta Field Support** - Complete `_meta` field implementation with cursor and progress tokens

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
turul-mcp-protocol-2025-06-18 = "0.2.0"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

**Note**: Most users should use `turul-mcp-protocol` (the version alias) instead of importing this crate directly.

**Important**: These types represent the MCP protocol messages themselves. JSON-RPC 2.0 wrapping is handled by the transport layer and framework - see `turul-http-mcp-server` for complete server implementation.

### Basic Usage

```rust
use turul_mcp_protocol_2025_06_18::{
    Tool, ToolSchema, JsonSchema,
    InitializeRequest, InitializeResult,
    CallToolRequest, CallToolResult,
    Implementation, ClientCapabilities, ServerCapabilities
};
use serde_json::json;
use std::collections::HashMap;

// Create a tool definition
let tool = Tool::new("calculator", ToolSchema::object()
    .with_properties(HashMap::from([
        ("operation".to_string(), JsonSchema::string()),
        ("a".to_string(), JsonSchema::number()),
        ("b".to_string(), JsonSchema::number()),
    ]))
    .with_required(vec!["operation".to_string(), "a".to_string(), "b".to_string()])
).with_description("Perform basic math operations");

// Create initialization structures  
let client_info = Implementation::new("My MCP Client", "1.0.0");
let server_info = Implementation::new("My MCP Server", "1.0.0");

println!("Tool name: {}", tool.name);
println!("Tool description: {}", tool.description.as_deref().unwrap_or("No description"));
```

## Protocol Architecture

### Trait-Based Design Pattern

The crate provides concrete MCP specification types that can be serialized and deserialized:

```rust
use turul_mcp_protocol_2025_06_18::{
    // Concrete protocol types
    Tool, ToolSchema, Resource, Prompt,
    // Request/Response types
    CallToolRequest, ListResourcesRequest, GetPromptRequest,
    // Common types
    McpError, McpResult
};

// Create protocol types directly
let tool = Tool::new("calculator", ToolSchema::object())
    .with_description("Perform calculations");

// Serialize for JSON-RPC transport
let json = serde_json::to_string(&tool)?;

// For framework traits like ToolDefinition, ResourceDefinition:
// use turul_mcp_builders::prelude::*;
```

### JSON-RPC 2.0 Integration

All MCP messages follow proper JSON-RPC 2.0 patterns:

```rust
use turul_mcp_protocol_2025_06_18::{
    InitializeRequest, CallToolRequest,
    ClientCapabilities, Implementation,
    McpVersion
};

// Request pattern: InitializeRequest struct
let init_request = InitializeRequest::new(
    McpVersion::V2025_06_18,
    ClientCapabilities::default(),
    Implementation::new("Test Client", "1.0.0"),
);

// Tool call request
let tool_request = CallToolRequest::new("calculator")
    .with_arguments(HashMap::from([
        ("operation".to_string(), json!("add")),
        ("a".to_string(), json!(5)),
        ("b".to_string(), json!(3)),
    ]));

println!("Initialize protocol version: {}", init_request.protocol_version);
println!("Tool name: {}", tool_request.params.name);
```

## Protocol Types

### Core Initialization

```rust
use turul_mcp_protocol_2025_06_18::{
    InitializeRequest, InitializeResult,
    ClientCapabilities, ServerCapabilities,
    Implementation, McpVersion
};

// Client initialization
let capabilities = ClientCapabilities::default();

let init_request = InitializeRequest::new(
    McpVersion::V2025_06_18,
    capabilities,
    Implementation::new("My MCP Client", "1.0.0"),
);

// Server response
let server_impl = Implementation::new("My Server", "1.0.0");
let server_capabilities = ServerCapabilities::default();

let init_result = InitializeResult {
    protocol_version: McpVersion::V2025_06_18.as_str().to_string(),
    capabilities: server_capabilities,
    server_info: server_impl,
    instructions: None,
    meta: None,
};
```

### Tools Implementation

```rust
use turul_mcp_protocol_2025_06_18::{
    Tool, ToolSchema, JsonSchema, ToolAnnotations,
    CallToolRequest, CallToolResult, ToolResult
};
use std::collections::HashMap;

// Create a tool with comprehensive schema
let calculator_tool = Tool::new(
    "calculator",
    ToolSchema::object()
        .with_properties(HashMap::from([
            ("operation".to_string(), JsonSchema::string()),
            ("a".to_string(), JsonSchema::number()),
            ("b".to_string(), JsonSchema::number()),
        ]))
        .with_required(vec!["operation".to_string(), "a".to_string(), "b".to_string()])
)
.with_description("Perform basic mathematical operations")
.with_title("Calculator Tool");

// Tool call request
let tool_request = CallToolRequest::new("calculator")
    .with_arguments(HashMap::from([
        ("operation".to_string(), serde_json::json!("add")),
        ("a".to_string(), serde_json::json!(5)),
        ("b".to_string(), serde_json::json!(3)),
    ]));

// Tool result - CallToolResult with optional is_error and structured_content
let tool_result = CallToolResult::success(vec![
    ToolResult::text("8"),
    ToolResult::image("base64data", "image/png"),
]);

// Or create with error flag
let error_result = CallToolResult::error(vec![
    ToolResult::text("Calculation failed")
]).with_error_flag(true);

// Note: structured_content is optional and used for schema-compliant output
// Serializes as "structuredContent" (camelCase) in JSON per MCP specification
let structured_result = CallToolResult::success(vec![
    ToolResult::text("8")
]).with_structured_content(serde_json::json!({"result": 8}));
```

### Resources Implementation

```rust
use turul_mcp_protocol_2025_06_18::{
    Resource, ResourceTemplate,
    ReadResourceRequest, ReadResourceResult,
    ResourceContent
};

// Define a resource
let config_resource = Resource::new(
    "file:///app/config.json",
    "app_config"
)
.with_description("Application configuration file")
.with_mime_type("application/json");

// Resource template for dynamic resources
let log_template = ResourceTemplate::new(
    "daily_logs",
    "file:///logs/{date}.log"
)
.with_description("Daily log files")
.with_mime_type("text/plain");

// Read resource content
let resource_content = ReadResourceResult::single(
    ResourceContent::text(
        "file:///config.json",
        r#"{"debug": true, "port": 8080}"#
    )
);
```

### Prompts Implementation

```rust
use turul_mcp_protocol_2025_06_18::{
    Prompt, PromptArgument, PromptMessage,
    GetPromptRequest, GetPromptResult,
    Role, ContentBlock
};

// Define a prompt template
let greeting_prompt = Prompt::new("greeting")
    .with_description("Generate personalized greetings")
    .with_arguments(vec![
        PromptArgument::new("name")
            .with_description("Person to greet")
            .with_required(true),
        PromptArgument::new("style")
            .with_description("Greeting style")
            .with_required(false),
    ]);

// Get prompt result with rendered messages
let prompt_result = GetPromptResult {
    description: Some("Personalized greeting prompt".to_string()),
    messages: vec![
        PromptMessage::user_text("Please create a friendly greeting for Alice"),
        PromptMessage::assistant_text("Hello Alice! Welcome to our application. How can I help you today?"),
    ],
    meta: None,
};
```

### Notifications

**Important**: All notification method names follow exact MCP specification with camelCase where specified (e.g., "notifications/resources/listChanged").

```rust
use turul_mcp_protocol_2025_06_18::{
    ProgressNotification, ProgressNotificationParams,
    LoggingMessageNotification, LoggingMessageNotificationParams,
    ResourceUpdatedNotification, ResourceUpdatedNotificationParams,
    ProgressToken, LogLevel
};

let mut progress = ProgressNotification::new("task-123".to_string(), 75);
progress.total = Some(100);
progress.message = Some("Processing...".to_string());
progress._meta = Some(json!({ "source": "my-app" }));

let mut log = LoggingMessageNotification::new(
    LoggingLevel::Error,
    json!({ "error": "Connection failed", "retry_count": 3 })
);
log.logger = Some("database".to_string());
log._meta = Some(json!({ "request_id": "xyz-123" }));

let mut resource_change = ResourceListChangedNotification::default();
resource_change._meta = Some(json!({ "reason": "file-watcher" }));
```

## Schema Generation

### Built-in JSON Schema Support

```rust
use turul_mcp_protocol_2025_06_18::{JsonSchema, ToolSchema};
use std::collections::HashMap;

// Create comprehensive schemas
let user_schema = JsonSchema::object()
    .with_properties(HashMap::from([
        ("id".to_string(), JsonSchema::integer()
            .with_minimum(1.0)
            .with_description("User ID")),
        ("name".to_string(), JsonSchema::string()
            .with_min_length(1)
            .with_max_length(100)
            .with_description("Full name")),
        ("email".to_string(), JsonSchema::string()
            .with_format("email")
            .with_description("Email address")),
        ("age".to_string(), JsonSchema::integer()
            .with_minimum(0.0)
            .with_maximum(120.0)
            .with_description("Age in years")),
        ("tags".to_string(), JsonSchema::array()
            .with_items(JsonSchema::string())
            .with_description("User tags")),
        ("active".to_string(), JsonSchema::boolean()
            .with_description("Account status")),
    ]))
    .with_required(vec!["id".to_string(), "name".to_string(), "email".to_string()])
    .with_additional_properties(false);

// Enum schema
let status_schema = JsonSchema::string()
    .with_enum(vec!["pending", "active", "suspended", "deleted"])
    .with_description("Account status");

// Array with constrained items
let coordinates_schema = JsonSchema::array()
    .with_items(JsonSchema::number().with_minimum(-180.0).with_maximum(180.0))
    .with_min_items(2)
    .with_max_items(2)
    .with_description("GPS coordinates [longitude, latitude]");
```

## Meta Fields and Advanced Features

### Progress Tracking

```rust
use turul_mcp_protocol_2025_06_18::{
    ProgressToken, CallToolRequest, CallToolResult
};
use std::collections::HashMap;

// Request with progress token
let tool_request = CallToolRequest::new("long_calculation")
    .with_arguments(HashMap::from([
        ("iterations".to_string(), serde_json::json!(1000)),
    ]));

// Response with progress updates  
let tool_result = CallToolResult::success(vec![
    ToolResult::text("Calculation completed")
]);
```

### Cursor-Based Pagination

```rust
use turul_mcp_protocol_2025_06_18::{Cursor, ListResourcesResult};

// Response with cursor for pagination  
// Note: _meta field supports round-trip - request meta can be included in response meta
// Serializes as "nextCursor" and "_meta" fields (camelCase) in JSON per MCP specification
let resources_result = ListResourcesResult::new(vec![/* resources */])
    .with_next_cursor(Cursor("page_2_token_abc123".to_string()));
```

## MCP Message Coverage

The crate provides full coverage for all specified client and server messages.

### Client-to-Server Messages

These are messages sent from the client (e.g., an IDE) to the server.

| Message Type | Rust Struct | Defining File |
| :--- | :--- | :--- |
| `PingRequest` | `PingRequest` | [`ping.rs`](./src/ping.rs) |
| `InitializeRequest` | `InitializeRequest` | [`initialize.rs`](./src/initialize.rs) |
| `CompleteRequest` | `CompleteRequest` | [`completion.rs`](./src/completion.rs) |
| `SetLevelRequest` | `SetLevelRequest` | [`logging.rs`](./src/logging.rs) |
| `GetPromptRequest` | `GetPromptRequest` | [`prompts.rs`](./src/prompts.rs) |
| `ListPromptsRequest` | `ListPromptsRequest` | [`prompts.rs`](./src/prompts.rs) |
| `ListResourcesRequest` | `ListResourcesRequest` | [`resources.rs`](./src/resources.rs) |
| `ListResourceTemplatesRequest` | `ListResourceTemplatesRequest` | [`resources.rs`](./src/resources.rs) |
| `ReadResourceRequest` | `ReadResourceRequest` | [`resources.rs`](./src/resources.rs) |
| `SubscribeRequest` | `SubscribeRequest` | [`resources.rs`](./src/resources.rs) |
| `UnsubscribeRequest` | `UnsubscribeRequest` | [`resources.rs`](./src/resources.rs) |
| `CallToolRequest` | `CallToolRequest` | [`tools.rs`](./src/tools.rs) |
| `ListToolsRequest` | `ListToolsRequest` | [`tools.rs`](./src/tools.rs) |
| `CancelledNotification` | `CancelledNotification` | [`notifications.rs`](./src/notifications.rs) |
| `ProgressNotification` | `ProgressNotification` | [`notifications.rs`](./src/notifications.rs) |
| `InitializedNotification` | `InitializedNotification` | [`notifications.rs`](./src/notifications.rs) |
| `RootsListChangedNotification` | `RootsListChangedNotification` | [`notifications.rs`](./src/notifications.rs) |

### Server-to-Client Messages

These are messages sent from the server to the client.

| Message Type | Rust Struct | Defining File |
| :--- | :--- | :--- |
| `PingRequest` | `PingRequest` | [`ping.rs`](./src/ping.rs) |
| `CreateMessageRequest` | `CreateMessageRequest` | [`sampling.rs`](./src/sampling.rs) |
| `ListRootsRequest` | `ListRootsRequest` | [`roots.rs`](./src/roots.rs) |
| `ElicitRequest` | `ElicitCreateRequest` | [`elicitation.rs`](./src/elicitation.rs) |
| `CancelledNotification` | `CancelledNotification` | [`notifications.rs`](./src/notifications.rs) |
| `ProgressNotification` | `ProgressNotification` | [`notifications.rs`](./src/notifications.rs) |
| `LoggingMessageNotification` | `LoggingMessageNotification` | [`notifications.rs`](./src/notifications.rs) |
| `ResourceUpdatedNotification` | `ResourceUpdatedNotification` | [`notifications.rs`](./src/notifications.rs) |
| `ResourceListChangedNotification` | `ResourceListChangedNotification` | [`notifications.rs`](./src/notifications.rs) |
| `ToolListChangedNotification` | `ToolListChangedNotification` | [`notifications.rs`](./src/notifications.rs) |
| `PromptListChangedNotification` | `PromptListChangedNotification` | [`notifications.rs`](./src/notifications.rs) |

## Protocol Helpers and Utilities

This crate also provides a number of helper modules that define the core primitives and architectural patterns used by the protocol types.

*   **[`meta.rs`](./src/meta.rs)**: Defines structured `_meta` field types, including `Cursor` and `ProgressToken`.
*   **[`schema.rs`](./src/schema.rs)**: Provides a `JsonSchema` enum for building tool schemas.
*   **[`traits.rs`](./src/traits.rs)**: A comprehensive set of internal traits that ensure consistency across all protocol types.
*   **[`json_rpc.rs`](./src/json_rpc.rs)**: Defines the core JSON-RPC 2.0 request, response, and notification wrappers.
*   **[`version.rs`](./src/version.rs)**: Manages MCP versioning and capabilities.
*   **[`param_extraction.rs`](./src/param_extraction.rs)**: Utilities for safely extracting parameters from requests.

For complete details on the specification that these types implement, please refer to the official [MCP 2025-06-18 TypeScript Schema](https://github.com/metacall-protocol/mcp-spec/blob/main/mcp-2025-06-18.ts).

## Using Protocol Types

This crate provides the concrete MCP specification types for serialization and protocol compliance.

### Core Protocol Types

| MCP Capability | Core Type | Purpose |
| :--- | :--- | :--- |
| **Tools** | `Tool`, `ToolSchema` | Define tool metadata and input/output schemas |
| **Resources** | `Resource`, `ResourceContent` | Define resources and their content |
| **Prompts** | `Prompt`, `PromptMessage` | Define prompts and messages |
| **Roots** | `Root` | Define file system roots |
| **Requests** | `*Request` types | Protocol request structures |
| **Results** | `*Result` types | Protocol response structures |

### Building Framework Features

**For framework traits and builders** (ToolDefinition, ResourceDefinition, etc.), use the `turul-mcp-builders` crate:

```rust
// Protocol types (this crate)
use turul_mcp_protocol::Tool;

// Framework traits (builders crate)
use turul_mcp_builders::prelude::*;

// Now you have both protocol types and framework traits
```

See [`turul-mcp-builders`](../turul-mcp-builders/README.md) for trait-based construction patterns

## Testing and Validation

### Protocol Compliance Testing

```rust
#[cfg(test)]
mod tests {
    use turul_mcp_protocol_2025_06_18::*;

    #[test]
    fn test_protocol_types_serialization() {
        let tool = Tool::new("test", ToolSchema::object());
        let json = serde_json::to_string(&tool).unwrap();
        let deserialized: Tool = serde_json::from_str(&json).unwrap();
        assert_eq!(tool.name, deserialized.name);
    }

    #[test] 
    fn test_json_rpc_compliance() {
        let request = InitializeRequest::new(
            McpVersion::V2025_06_18,
            ClientCapabilities::default(),
            Implementation::new("test-client", "1.0.0"),
        );
        
        // Should serialize with proper structure
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"protocolVersion\":\"2025-06-18\""));
    }

    #[test]
    fn test_protocol_types() {
        let tool = Tool::new("test", ToolSchema::object());

        // Test protocol type fields
        assert_eq!(tool.name, "test");
        assert!(tool.description.is_none());

        // Protocol types serialize to JSON
        let json = serde_json::to_string(&tool).unwrap();
        assert!(json.contains("\"name\":\"test\""));
    }
}
```

### Schema Validation

```bash
# Run all protocol tests
cargo test --package turul-mcp-protocol-2025-06-18

# Test specific protocol areas
cargo test --package turul-mcp-protocol-2025-06-18 tools
cargo test --package turul-mcp-protocol-2025-06-18 resources  
cargo test --package turul-mcp-protocol-2025-06-18 sampling
```

## Framework Integration

### Usage with turul-mcp-server

```rust
use turul_mcp_server::prelude::*;
use turul_mcp_derive::mcp_tool;

// Use derive macro for automatic trait implementation
#[mcp_tool(name = "my_tool", description = "My tool")]
async fn my_tool(input: String) -> McpResult<String> {
    Ok(format!("Processed: {}", input))
}

let server = McpServer::builder()
    .name("My Server")
    .version("1.0.0")
    .tool_fn(my_tool)
    .build()?;
```

### Direct Protocol Usage

```rust
use turul_mcp_protocol_2025_06_18::*;

async fn handle_mcp_request(request_json: &str) -> Result<String, Box<dyn std::error::Error>> {
    let request: serde_json::Value = serde_json::from_str(request_json)?;
    
    match request["method"].as_str() {
        Some("initialize") => {
            let init_request: InitializeRequest = serde_json::from_value(request)?;
            
            let response = InitializeResult {
                protocol_version: "2025-06-18".to_string(),
                capabilities: ServerCapabilities::default(),
                server_info: Implementation::new("My Server", "1.0.0"),
                instructions: None,
                meta: None,
            };
            
            Ok(serde_json::to_string(&response)?)
        }
        Some("tools/call") => {
            let tool_request: CallToolRequest = serde_json::from_value(request)?;
            
            let response = CallToolResult::success(vec![
                ToolResult::text("Tool executed successfully")
            ]);
            
            Ok(serde_json::to_string(&response)?)
        }
        _ => {
            let error = McpError::ToolExecutionError("Method not found".to_string());
            Ok(serde_json::to_string(&error.to_json_rpc_error())?)
        }
    }
}
```

## Error Handling

### Protocol Error Types

```rust
use turul_mcp_protocol_2025_06_18::{McpError, McpResult};

fn handle_protocol_errors() -> McpResult<String> {
    // MCP protocol errors  
    let validation_error = McpError::InvalidParameters("Missing required field".to_string());
    let param_error = McpError::invalid_param_type("age", "number", "string");
    let execution_error = McpError::ToolExecutionError("Calculation failed".to_string());
    
    // Combined error handling
    match some_operation() {
        Ok(result) => Ok(format!("Success: {:?}", result)),
        Err(McpError::InvalidParameters(e)) => Err(McpError::InvalidParameters(e)),
        Err(McpError::ToolExecutionError(e)) => Err(McpError::ToolExecutionError(e)),
        Err(e) => Err(e),
    }
}

fn some_operation() -> McpResult<String> {
    Ok("Success".to_string())
}
```

## Feature Flags

```toml
[dependencies]
turul-mcp-protocol-2025-06-18 = { version = "0.2.0", features = ["server"] }
```

Available features:
- `default` - Core protocol types and traits
- `server` - Server-specific functionality and optimizations
- `client` - Client-specific helper functions and utilities

## Architectural Decisions

### Why Trait-Based Architecture?

1. **TypeScript Alignment**: Perfect 1:1 mapping with MCP TypeScript interfaces
2. **Composability**: Fine-grained traits can be mixed and matched as needed
3. **Framework Flexibility**: Same trait interfaces work for both concrete and dynamic implementations
4. **Type Safety**: Compile-time guarantees for protocol compliance
5. **Extensibility**: Easy to add new traits and compose them automatically

### Protocol Version Guarantees

- **Specification Compliance**: All types exactly match MCP 2025-06-18 TypeScript schema
- **Field Naming**: Exact camelCase field names from specification  
- **Optional Fields**: Proper `Option<T>` with `skip_serializing_if` for TypeScript optional fields
- **Inheritance Patterns**: Rust struct composition replicates TypeScript interface inheritance
- **JSON-RPC Compliance**: All messages follow proper JSON-RPC 2.0 patterns

## Performance Notes

- **Zero-Cost Abstractions**: Trait dispatch optimized away at compile time where possible
- **Efficient Serialization**: Optimized `serde` implementations with minimal allocations
- **Schema Caching**: JSON schemas can be cached for repeated use
- **Memory Layout**: Structs optimized for minimal memory usage

## Contributing

When contributing to this crate:

1. **Specification Compliance**: All changes must maintain exact TypeScript schema compatibility
2. **Trait Consistency**: Follow the fine-grained trait → composed trait → concrete implementation pattern
3. **Test Coverage**: Add tests for all new protocol types and trait implementations
4. **Documentation**: Update examples and documentation for new features

For the complete TypeScript specification, see: [MCP 2025-06-18 Schema](https://github.com/metacall-protocol/mcp-spec/blob/main/mcp-2025-06-18.ts)

## License

Licensed under the MIT License. See [LICENSE](../../LICENSE) for details.