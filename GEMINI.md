# Turul MCP Framework

## Project Overview

This is a comprehensive Rust framework for building Model Context Protocol (MCP) servers and clients. It is fully compliant with the MCP 2025-06-18 specification. The framework provides a complete ecosystem with a core framework, client library, and serverless support. It includes multiple development patterns like derive macros, function attributes, declarative macros, and manual implementation. It supports various transports like HTTP/1.1, HTTP/2, WebSocket, SSE, and stdio. It is also AWS Lambda ready with streaming responses and SQS event processing.

The project is structured as a Rust workspace with 37 crates, including the core framework, a client library, and 26 examples. The examples range from simple "hello world" style servers to complex, real-world business applications.

## Building and Running

The project is built and tested using Cargo.

### Building the project

To build the entire project, run the following command from the root of the repository:

```bash
cargo build
```

### Running the examples

To run an example, use the `cargo run` command and specify the example name. For example, to run the minimal server:

```bash
cargo run --example minimal-server
```

To run the comprehensive server:

```bash
cargo run --example comprehensive-server
```

### Running the tests

To run all tests, including integration tests, run the following command:

```bash
cargo test --workspace
```

## Development Conventions

The project follows standard Rust conventions. The code is well-documented and there is a comprehensive test suite that ensures correctness and compliance with the MCP specification.

The framework offers four levels of abstraction for creating tools:

1.  **Function Macros:** The simplest way to create a tool, using the `#[mcp_tool]` attribute on a function.
2.  **Derive Macros:** A struct-based approach using `#[derive(McpTool)]`.
3.  **Builder Pattern:** A runtime-flexible way to build tools.
4.  **Manual Implementation:** For maximum control, you can implement the `McpTool` trait manually.

The project uses `tracing` for logging.
