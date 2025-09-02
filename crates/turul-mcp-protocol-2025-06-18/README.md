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
turul-mcp-protocol-2025-06-18 = "0.1"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

**Note**: Most users should use `turul-mcp-protocol` (the version alias) instead of importing this crate directly.

### Basic Usage

```rust
use turul_mcp_protocol_2025_06_18::{
    Tool, ToolSchema, JsonSchema,
    InitializeRequest, InitializeResult,
    CallToolRequest, CallToolResult,
    ClientInfo, ServerInfo, Implementation
};
use serde_json::json;

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
let client_info = ClientInfo {
    name: "My MCP Client".to_string(),
    version: "1.0.0".to_string(),
};

let server_info = Implementation::new("My MCP Server", "1.0.0");

println!("Tool name: {}", tool.name());
println!("Tool description: {}", tool.description().unwrap_or("No description"));
```

## Protocol Architecture

### Trait-Based Design Pattern

The crate implements a comprehensive trait-based architecture that mirrors the TypeScript specification:

```rust
use turul_mcp_protocol_2025_06_18::{
    // Fine-grained traits
    HasBaseMetadata, HasDescription, HasInputSchema,
    // Composed definition traits  
    ToolDefinition,
    // Concrete protocol types
    Tool, ToolSchema
};

// All protocol types implement corresponding definition traits
fn process_tool(tool: &dyn ToolDefinition) {
    println!("Tool: {}", tool.name());
    if let Some(desc) = tool.description() {
        println!("Description: {}", desc);
    }
}
```

### JSON-RPC 2.0 Integration

All MCP messages follow proper JSON-RPC 2.0 patterns:

```rust
use turul_mcp_protocol_2025_06_18::{
    InitializeRequest, InitializeParams,
    CallToolRequest, CallToolParams,
    JsonRpcRequestTrait, HasMethod, HasParams
};

// Request pattern: { method, params, id?, jsonrpc }
let init_request = InitializeRequest {
    method: "initialize".to_string(),  // Auto-determined by framework
    params: InitializeParams {
        protocol_version: "2025-06-18".to_string(),
        capabilities: Default::default(),
        client_info: ClientInfo {
            name: "Test Client".to_string(),
            version: "1.0.0".to_string(),
        },
        meta: None, // Optional _meta field
    },
};

// All requests implement JsonRpcRequestTrait
assert_eq!(init_request.method(), "initialize");
```

## Protocol Types

### Core Initialization

```rust
use turul_mcp_protocol_2025_06_18::{
    InitializeRequest, InitializeResult,
    ClientCapabilities, ServerCapabilities,
    Implementation
};

// Client initialization
let capabilities = ClientCapabilities {
    experimental: Some(HashMap::new()),
    sampling: None,
    roots: Some(RootsCapability { list_changed: true }),
};

let init_params = InitializeParams {
    protocol_version: "2025-06-18".to_string(),
    capabilities,
    client_info: ClientInfo {
        name: "My MCP Client".to_string(),
        version: "1.0.0".to_string(),
    },
    meta: None,
};

// Server response
let server_impl = Implementation::new("My Server", "1.0.0");
let server_capabilities = ServerCapabilities {
    experimental: Some(HashMap::new()),
    logging: Some(LoggingCapability {}),
    prompts: Some(PromptsCapability { list_changed: true }),
    resources: Some(ResourcesCapability { 
        subscribe: true, 
        list_changed: true 
    }),
    tools: Some(ToolsCapability { list_changed: true }),
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
            ("operation".to_string(), JsonSchema::string()
                .with_enum(vec!["add", "subtract", "multiply", "divide"])),
            ("a".to_string(), JsonSchema::number()
                .with_description("First operand")),
            ("b".to_string(), JsonSchema::number()
                .with_description("Second operand")),
        ]))
        .with_required(vec!["operation".to_string(), "a".to_string(), "b".to_string()])
        .with_description("Tool input schema")
)
.with_description("Perform basic mathematical operations")
.with_title("Calculator Tool");

// Tool call request
let tool_request = CallToolRequest {
    method: "tools/call".to_string(),
    params: CallToolParams {
        name: "calculator".to_string(),
        arguments: Some(serde_json::json!({
            "operation": "add",
            "a": 5,
            "b": 3
        })),
        meta: None,
    },
};

// Tool result
let tool_result = CallToolResult {
    content: vec![
        ToolResult::text("8"),
        ToolResult::image_data("image/png", "base64data"),
        ToolResult::resource_reference("file:///result.json"),
    ],
    is_error: false,
    meta: None,
};
```

### Resources Implementation

```rust
use turul_mcp_protocol_2025_06_18::{
    Resource, ResourceAnnotations, ResourceTemplate,
    ReadResourceRequest, ReadResourceResult,
    ResourceContent, TextResourceContent, ImageResourceContent
};

// Define a resource
let config_resource = Resource::new(
    "file:///app/config.json",
    Some("Application configuration file")
)
.with_name("app_config")
.with_mime_type("application/json");

// Resource template for dynamic resources
let log_template = ResourceTemplate::new(
    "file:///logs/{date}.log",
    Some("Daily log files")
)
.with_name("daily_logs")
.with_mime_type("text/plain");

// Read resource content
let resource_content = ReadResourceResult {
    contents: vec![
        ResourceContent::Text(TextResourceContent {
            uri: "file:///config.json".to_string(),
            mime_type: Some("application/json".to_string()),
            text: r#"{"debug": true, "port": 8080}"#.to_string(),
        }),
        ResourceContent::Blob(BlobResourceContent {
            uri: "file:///logo.png".to_string(), 
            mime_type: Some("image/png".to_string()),
            blob: "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==".to_string(),
        }),
    ],
    meta: None,
};
```

### Prompts Implementation

```rust
use turul_mcp_protocol_2025_06_18::{
    Prompt, PromptArgument, PromptMessage,
    GetPromptRequest, GetPromptResult,
    Role, MessageContent, TextContent
};

// Define a prompt template
let greeting_prompt = Prompt::new("greeting", Some("Generate personalized greetings"))
    .with_arguments(vec![
        PromptArgument::new("name", Some("Person to greet"))
            .with_required(true),
        PromptArgument::new("style", Some("Greeting style"))
            .with_required(false),
    ]);

// Get prompt result with rendered messages
let prompt_result = GetPromptResult {
    description: Some("Personalized greeting prompt".to_string()),
    messages: vec![
        PromptMessage {
            role: Role::User,
            content: MessageContent::Text(TextContent {
                text: "Please create a friendly greeting for Alice".to_string(),
            }),
        },
        PromptMessage {
            role: Role::Assistant, 
            content: MessageContent::Text(TextContent {
                text: "Hello Alice! Welcome to our application. How can I help you today?".to_string(),
            }),
        },
    ],
    meta: None,
};
```

### Sampling Implementation

```rust
use turul_mcp_protocol_2025_06_18::{
    CreateMessageRequest, CreateMessageResult,
    SamplingMessage, ModelPreferences
};

// Sampling request for message generation
let sampling_request = CreateMessageRequest {
    method: "sampling/createMessage".to_string(),
    params: CreateMessageParams {
        messages: vec![
            SamplingMessage {
                role: Role::User,
                content: MessageContent::Text(TextContent {
                    text: "What is the capital of France?".to_string(),
                }),
            }
        ],
        model_preferences: Some(ModelPreferences {
            hints: Some(vec![
                ModelHint {
                    name: Some("claude-3-opus".to_string()),
                },
            ]),
            cost_priority: Some(0.5),
            speed_priority: Some(0.7),
            intelligence_priority: Some(0.9),
        }),
        system_prompt: Some("You are a helpful geography assistant.".to_string()),
        include_context: Some("auto".to_string()),
        temperature: Some(0.1),
        max_tokens: Some(150),
        stop_sequences: Some(vec!["\n\n".to_string()]),
        meta: None,
    },
};

// Sampling result
let sampling_result = CreateMessageResult {
    role: Role::Assistant,
    content: MessageContent::Text(TextContent {
        text: "The capital of France is Paris.".to_string(),
    }),
    model: Some("claude-3-opus-20240229".to_string()),
    stop_reason: Some("end_turn".to_string()),
    meta: None,
};
```

### Notifications

```rust
use turul_mcp_protocol_2025_06_18::{
    ProgressNotification, LoggingNotification,
    ResourceUpdatedNotification, ToolListChangedNotification
};

// Progress notification
let progress = ProgressNotification {
    method: "notifications/progress".to_string(),
    params: Some(ProgressNotificationParams {
        progress_token: ProgressToken("task-123".to_string()),
        progress: 75.0,
        total: Some(100.0),
        meta: None,
    }),
};

// Logging notification
let log_notification = LoggingNotification {
    method: "notifications/message".to_string(),
    params: Some(LoggingNotificationParams {
        level: LogLevel::Info,
        data: serde_json::json!({
            "message": "Task completed successfully",
            "duration_ms": 1250,
            "items_processed": 42
        }),
        logger: Some("task_processor".to_string()),
        meta: None,
    }),
};

// Resource update notification
let resource_updated = ResourceUpdatedNotification {
    method: "notifications/resources/updated".to_string(),
    params: Some(ResourceUpdatedNotificationParams {
        uri: "file:///data/config.json".to_string(),
        meta: None,
    }),
};
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
    ProgressToken, CallToolParams, CallToolResult
};

// Request with progress token
let tool_request = CallToolRequest {
    method: "tools/call".to_string(),
    params: CallToolParams {
        name: "long_calculation".to_string(),
        arguments: Some(serde_json::json!({"iterations": 1000})),
        meta: Some(HashMap::from([
            ("progressToken".to_string(), serde_json::json!("calc-2024-001")),
            ("timeout".to_string(), serde_json::json!(30000)),
        ])),
    },
};

// Response with progress updates
let tool_result = CallToolResult {
    content: vec![ToolResult::text("Calculation completed")],
    is_error: false,
    meta: Some(HashMap::from([
        ("duration_ms".to_string(), serde_json::json!(25000)),
        ("iterations_completed".to_string(), serde_json::json!(1000)),
    ])),
};
```

### Cursor-Based Pagination

```rust
use turul_mcp_protocol_2025_06_18::{Cursor, ListResourcesResult};

// Response with cursor for pagination
let resources_result = ListResourcesResult {
    resources: vec![/* ... */],
    next_cursor: Some(Cursor("page_2_token_abc123".to_string())),
    meta: Some(HashMap::from([
        ("total_count".to_string(), serde_json::json!(250)),
        ("page_size".to_string(), serde_json::json!(50)),
    ])),
};
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
| `EmptyResult` | `EmptyResult` | [`ping.rs`](./src/ping.rs) |
| `CreateMessageResult` | `CreateMessageResult` | [`sampling.rs`](./src/sampling.rs) |
| `ListRootsResult` | `ListRootsResult` | [`roots.rs`](./src/roots.rs) |
| `ElicitResult` | `ElicitResult` | [`elicitation.rs`](./src/elicitation.rs) |

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
| `EmptyResult` | `EmptyResult` | [`ping.rs`](./src/ping.rs) |
| `InitializeResult` | `InitializeResult` | [`initialize.rs`](./src/initialize.rs) |
| `CompleteResult` | `CompleteResult` | [`completion.rs`](./src/completion.rs) |
| `GetPromptResult` | `GetPromptResult` | [`prompts.rs`](./src/prompts.rs) |
| `ListPromptsResult` | `ListPromptsResult` | [`prompts.rs`](./src/prompts.rs) |
| `ListResourceTemplatesResult` | `ListResourceTemplatesResult` | [`resources.rs`](./src/resources.rs) |
| `ListResourcesResult` | `ListResourcesResult` | [`resources.rs`](./src/resources.rs) |
| `ReadResourceResult` | `ReadResourceResult` | [`resources.rs`](./src/resources.rs) |
| `CallToolResult` | `CallToolResult` | [`tools.rs`](./src/tools.rs) |
| `ListToolsResult` | `ListToolsResult` | [`tools.rs`](./src/tools.rs) |

## Protocol Helpers and Utilities

This crate also provides a number of helper modules that define the core primitives and architectural patterns used by the protocol types.

*   **[`meta.rs`](./src/meta.rs)**: Defines structured `_meta` field types, including `Cursor` and `ProgressToken`.
*   **[`schema.rs`](./src/schema.rs)**: Provides a `JsonSchema` enum for building tool schemas.
*   **[`traits.rs`](./src/traits.rs)**: A comprehensive set of internal traits that ensure consistency across all protocol types.
*   **[`json_rpc.rs`](./src/json_rpc.rs)**: Defines the core JSON-RPC 2.0 request, response, and notification wrappers.
*   **[`version.rs`](./src/version.rs)**: Manages MCP versioning and capabilities.
*   **[`param_extraction.rs`](./src/param_extraction.rs)**: Utilities for safely extracting parameters from requests.

For complete details on the specification that these types implement, please refer to the official [MCP 2025-06-18 TypeScript Schema](https://github.com/metacall-protocol/mcp-spec/blob/main/mcp-2025-06-18.ts).

## Implementing MCP Features with Traits

This crate uses a trait-based architecture to allow for flexible and type-safe implementation of MCP features. To create your own custom logic for a specific MCP capability, you can implement its corresponding "definition" trait.

Here is a high-level guide to the primary traits for each MCP area:

| MCP Capability | Core Rust Trait | Purpose |
| :--- | :--- | :--- |
| **Tools** | `ToolDefinition` | Implement to define a new tool that the server can execute. |
| **Resources** | `ResourceDefinition` | Implement to define a new resource that the server can provide. |
| **Prompts** | `PromptDefinition` | Implement to define a new prompt or prompt template. |
| **Roots** | `RootDefinition` | Implement to define a new file system root that can be exposed. |
| **Elicitation** | `ElicitationDefinition` | Implement to define a new structured input elicitation flow. |
| **Sampling** | `SamplingDefinition` | Implement to define custom logic for `sampling/createMessage` requests. |
| **Logging** | `LoggerDefinition` | Implement to define custom logging behavior. |

### Granular Trait Details

Each of these "definition" traits is composed of smaller, more granular traits that allow for precise control over each part of the definition.

#### Tools ([`tools.rs`](./src/tools.rs))
- **Core Trait**: `ToolDefinition`
- **Component Traits**:
    - `HasBaseMetadata` (name, title)
    - `HasDescription`
    - `HasInputSchema`
    - `HasOutputSchema`
    - `HasAnnotations`
    - `HasToolMeta`

#### Resources ([`resources.rs`](./src/resources.rs))
- **Core Trait**: `ResourceDefinition`
- **Component Traits**:
    - `HasResourceMetadata` (name, title)
    - `HasResourceDescription`
    - `HasResourceUri`
    - `HasResourceMimeType`
    - `HasResourceSize`
    - `HasResourceAnnotations`
    - `HasResourceMeta`

#### Prompts ([`prompts.rs`](./src/prompts.rs))
- **Core Trait**: `PromptDefinition`
- **Component Traits**:
    - `HasPromptMetadata` (name, title)
    - `HasPromptDescription`
    - `HasPromptArguments`
    - `HasPromptAnnotations`
    - `HasPromptMeta`

#### Roots ([`roots.rs`](./src/roots.rs))
- **Core Trait**: `RootDefinition`
- **Component Traits**:
    - `HasRootMetadata` (uri, name)
    - `HasRootPermissions`
    - `HasRootFiltering`
    - `HasRootAnnotations`

#### Elicitation ([`elicitation.rs`](./src/elicitation.rs))
- **Core Trait**: `ElicitationDefinition`
- **Component Traits**:
    - `HasElicitationMetadata` (message, title)
    - `HasElicitationSchema`
    - `HasElicitationHandling`

#### Sampling ([`sampling.rs`](./src/sampling.rs))
- **Core Trait**: `SamplingDefinition`
- **Component Traits**:
    - `HasSamplingConfig` (max_tokens, temperature)
    - `HasSamplingContext` (messages, system_prompt)
    - `HasModelPreferences`

#### Logging ([`logging.rs`](./src/logging.rs))
- **Core Trait**: `LoggerDefinition`
- **Component Traits**:
    - `HasLoggingMetadata`
    - `HasLogLevel`
    - `HasLogFormat`
    - `HasLogTransport`

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
        assert_eq!(tool.name(), deserialized.name());
    }

    #[test] 
    fn test_json_rpc_compliance() {
        let request = InitializeRequest {
            method: "initialize".to_string(),
            params: InitializeParams::default(),
        };
        
        // Should serialize with proper JSON-RPC 2.0 structure
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"method\":\"initialize\""));
    }

    #[test]
    fn test_trait_implementations() {
        let tool = Tool::new("test", ToolSchema::object());
        
        // Test fine-grained traits
        assert_eq!(tool.name(), "test");
        assert!(tool.description().is_none());
        
        // Test composed trait
        fn accepts_tool_definition(_tool: &dyn ToolDefinition) {}
        accepts_tool_definition(&tool);
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
use turul_mcp_server::McpServer;
use turul_mcp_protocol_2025_06_18::{Tool, ToolDefinition, HasBaseMetadata};

struct MyTool;

impl HasBaseMetadata for MyTool {
    fn name(&self) -> &str { "my_tool" }
}

// Implement other required traits...

let server = McpServer::builder()
    .name("My Server")
    .version("1.0.0")
    .tool(MyTool)  // Protocol types work directly with framework
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
            
            let response = CallToolResult {
                content: vec![ToolResult::text("Tool executed successfully")],
                is_error: false,
                meta: None,
            };
            
            Ok(serde_json::to_string(&response)?)
        }
        _ => {
            let error = JsonRpcError::method_not_found(&request["method"].to_string());
            Ok(serde_json::to_string(&error)?)
        }
    }
}
```

## Error Handling

### Protocol Error Types

```rust
use turul_mcp_protocol_2025_06_18::{McpError, JsonRpcError, ProtocolError};

fn handle_protocol_errors() {
    // JSON-RPC errors
    let json_rpc_error = JsonRpcError::new(-32601, "Method not found", None);
    
    // MCP protocol errors  
    let protocol_error = ProtocolError::InvalidCapabilities("Missing required capability".to_string());
    
    // Validation errors
    let validation_error = ValidationError::MissingField("name".to_string());
    
    // Combined error handling
    match some_operation() {
        Ok(result) => println!("Success: {:?}", result),
        Err(McpError::JsonRpc(e)) => println!("JSON-RPC error: {}", e),
        Err(McpError::Protocol(e)) => println!("Protocol error: {}", e),
        Err(McpError::Validation(e)) => println!("Validation error: {}", e),
        Err(e) => println!("Other error: {}", e),
    }
}
```

## Feature Flags

```toml
[dependencies]
turul-mcp-protocol-2025-06-18 = { version = "0.1", features = ["server"] }
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