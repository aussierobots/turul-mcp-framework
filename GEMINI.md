# Gemini Code Assistant Context

## Project Overview

This project is a production-ready Rust framework for building Model Context Protocol (MCP) servers. It is designed to be fully compliant with the MCP 2025-06-18 specification. The framework provides automatic session management, real-time notifications, and multiple development patterns to simplify the creation of MCP servers.

The project is structured as a Rust workspace containing several crates:

*   **`json-rpc-server`**: A transport-agnostic JSON-RPC 2.0 server implementation.
*   **`mcp-protocol-2025-06-18`**: A complete implementation of the MCP 2025-06-18 specification.
*   **`mcp-protocol`**: An alias crate for the current MCP version.
*   **`http-mcp-server`**: An HTTP transport layer with CORS and Server-Sent Events (SSE) support.
*   **`mcp-server`**: A high-level framework with a builder pattern for easy server creation.
*   **`mcp-derive`**: Procedural and declarative macros for simplified development.

The framework is built on top of popular Rust libraries like Tokio, Hyper, and Serde.

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

## MCP Protocol Features Demonstrated

The examples in this repository cover the entire MCP 2025-06-18 specification.

### Core Features
- **✅ Tools**: All examples demonstrate tool implementation.
- **✅ Resources**: Static and dynamic content serving (`resources-server`, `resource-server`, `dynamic-resource-server`).
- **✅ Prompts**: AI interaction templates (`prompts-server`).
- **✅ Completion**: Text completion and suggestions (`completion-server`).
- **✅ Logging**: Dynamic log level management (`logging-server`).
- **✅ Notifications**: Real-time SSE updates (`notification-server`).
- **✅ Roots**: File system security boundaries (`roots-server`).
- **✅ Sampling**: AI model integration (`sampling-server`).
- **✅ Elicitation**: Structured user input collection (`elicitation-server`).

### Advanced Features
- **✅ Session Management**: State persistence across requests (`stateful-server`).
- **✅ Progress Tracking**: Real-time operation monitoring (`notification-server`, `elicitation-server`).
- **✅ Pagination**: Cursor-based navigation for large datasets (`pagination-server`).
- **✅ Version Negotiation**: Protocol compatibility handling (`version-negotiation-server`).
- **✅ Performance Testing**: Load and stress testing capabilities (`performance-testing`).

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