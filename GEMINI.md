# Gemini Code Assistant Context

## Project Overview

This project provides a Rust framework for building Model Context Protocol (MCP) servers. It aims to be compliant with the MCP 2025-06-18 specification and offers various development patterns to simplify server creation.

The project is a Rust workspace with the following key crates:

*   **`mcp-json-rpc-server`**: A transport-agnostic JSON-RPC 2.0 server implementation.
*   **`mcp-protocol-2025-06-18`**: An implementation of the MCP 2025-06-18 specification.
*   **`mcp-protocol`**: A crate that aliases the current MCP version.
*   **`http-mcp-server`**: An HTTP transport layer with CORS and Server-Sent Events (SSE) support.
*   **`mcp-server`**: A high-level framework for building MCP servers.
*   **`mcp-derive`**: Procedural and declarative macros for simplified development.
*   **`mcp-client`**: A client library for interacting with MCP servers.
*   **`mcp-builders`**: A collection of builders for programmatically constructing MCP messages and requests.
*   **`mcp-session-storage`**: A crate that provides abstractions for session storage.

## Specification Compliance

While the project implements a significant portion of the MCP 2025-06-18 specification, the claim of being "fully compliant" is not entirely accurate. The framework has a strong focus on the "tools" capability, and other features, while present in the protocol crate, are not as well-integrated into the high-level server framework.

### Implemented Features
- **✅ Tools**: Comprehensive support for defining and using tools.
- **✅ Session Management**: Basic session management is in place.
- **✅ Notifications**: Support for real-time notifications via SSE.
- **✅ Development Patterns**: Multiple development patterns are supported (derive macros, function attributes, etc.).

### Partially Implemented or Missing Features
- **⚠️ Resources**: The `resources` capability is only partially implemented. While the protocol crate has been updated to include the `subscribe` capability, the `mcp-server` crate does not yet support it.
- **⚠️ Dynamic Capabilities**: The framework does not support dynamic capabilities, where the server's capabilities can change at runtime.

## Architecture Review

The framework is generally well-structured, with a clear separation of concerns between the protocol implementation and the server framework. However, a detailed review has identified several architectural flaws and areas for improvement:

*   **Inconsistent Feature Enablement:** The `McpServerBuilder` API is inconsistent. Some features are enabled with `with_...` methods, while others are not. This can be confusing for developers.
*   **Lack of Clear Separation between Core and Optional Features:** The builder mixes core and optional MCP features, making it difficult to create minimal, compliant servers.
*   **Incomplete `resources` Implementation:** The `resources` feature is not fully implemented, lacking support for subscriptions and real-time updates.
*   **No Support for Dynamic Capabilities:** The framework does not support dynamic capabilities, which is a key feature of the MCP specification.
*   **`initialize` Request Handler:** The `mcp-server` framework has a dedicated handler for the `initialize` request, `SessionAwareInitializeHandler`.

## Recommendations

To improve the framework and move closer to full compliance with the MCP 2025-06-18 specification, the following actions are recommended:

*   **Improve the `McpServerBuilder` API:** Create a more consistent and intuitive API for enabling and configuring MCP features.
*   **Separate Core and Optional Features:** Refactor the builder to clearly distinguish between core and optional features.
*   **Complete the `resources` Implementation:** Add support for resource subscriptions and real-time updates to the `mcp-server` crate.
*   **Implement Dynamic Capabilities:** Allow the server's capabilities to be updated at runtime.

## TypeScript Schema Compliance Review

A review of the official MCP 2025-06-18 TypeScript schema has revealed the following discrepancies with the Rust implementation:

*   **`resources.subscribe` Capability Partially Implemented:** The `resources.subscribe` capability has been added to the `mcp-protocol-2025-06-18` crate, but it is not yet supported by the `mcp-server` crate. The `with_resources` method in the `McpServerBuilder` still hardcodes the `subscribe` capability to `false`.
*   **JSON-RPC 2.0 Fields:** The Rust `InitializeRequest` and `InitializeResult` structs do not include the `jsonrpc`, `id`, `method`, and `result` fields. These fields are part of the JSON-RPC 2.0 protocol and are handled by the `json-rpc-server` crate. While this is a reasonable design choice, it is a difference between the Rust implementation and the TypeScript schema.

### Recommendations

To achieve full compliance with the TypeScript schema, the following actions are recommended:

*   **Implement the `resources.subscribe` Capability in `mcp-server`:** Update the `mcp-server` crate to support the `resources.subscribe` capability. This will involve updating the `McpServerBuilder`, the `ResourcesHandler`, and the `McpResource` trait.
*   **Clarify JSON-RPC 2.0 Field Handling:** Add a note to the documentation explaining that the JSON-RPC 2.0 fields are handled by the `json-rpc-server` crate and are not part of the MCP-specific structs.

## Development Patterns

The framework supports multiple development patterns to cater to different needs, from rapid prototyping to fine-grained control.

1.  **Derive Macros (Recommended)**: Best for type safety, IDE support, and compile-time validation. Ideal for most use cases.
    *   **Examples**: `derive-macro-server`, `macro-calculator`, `enhanced-tool-macro-test`

2.  **Function Attributes**: For simple functions and minimal boilerplate, offering a natural function-based syntax.
    *   **Example**: `function-macro-server`

3.  **Declarative Macros**: Ultra-concise for inline tool/resource creation with minimal boilerplate.
    *   **Examples**: `tool-macro-example`, `resource-macro-example`

4.  **Manual Implementation**: Provides maximum control for complex logic, custom schemas, and advanced validation.
    *   **Examples**: `minimal-server`, `calculator-server`, `manual-tools-server`

## Examples

The project includes a comprehensive set of 25 examples that demonstrate various features and development patterns. For a detailed overview, see `EXAMPLES_OVERVIEW.md` (business-oriented) and `EXAMPLES_SUMMARY.md` (technical deep-dive).

Here are some key examples:

*   **`minimal-server`**: The simplest possible MCP server.
*   **`comprehensive-server`**: A demo of all MCP features in one server.
*   **`derive-macro-server`**: Demonstrates the recommended `#[derive(McpTool)]` pattern.
*   **`stateful-server`**: An example of stateful operations and session management.
*   **`notification-server`**: Demonstrates real-time updates via Server-Sent Events (SSE).
*   **`roots-server`**: Showcases secure file system access with security boundaries.
*   **`elicitation-server`**: Demonstrates structured user input collection with interactive forms.

## Building and Running

### Building the Project

To build the entire project, run the following command from the root directory:

```bash
cargo build
```

### Running Examples

To run an example, use the `-p` flag with `cargo run` followed by the example name.

```bash
# Run the minimal server
cargo run -p minimal-server

# Run the comprehensive server demonstrating all features
cargo run -p comprehensive-server

# Run the stateful server
cargo run -p stateful-server
```

### Running Tests

To run all tests, use the following command:

```bash
cargo test
```

All tests are currently passing.

To run only the integration tests, use:

```bash
cargo test --test integration_tests
```

## Contribution Guidelines

Contributions are welcome. The process is as follows:

1.  Fork the repository.
2.  Create a feature branch.
3.  Make your changes with tests.
4.  Run the test suite (`cargo test`).
5.  Commit your changes.
6.  Push to the branch.
7.  Open a Pull Request.
