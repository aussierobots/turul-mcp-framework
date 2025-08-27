# MCP Protocol Crate (2025-06-18)

This crate provides a complete and robust Rust implementation of the Model Context Protocol (MCP) specification, version `2025-06-18`. It includes all necessary types for requests, responses, and notifications, built with strong typing and `serde` compatibility for seamless integration with any JSON-RPC transport layer.

This implementation has been verified for a high degree of compliance with the official MCP specification.

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