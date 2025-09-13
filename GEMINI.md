# Turul MCP Framework

## Project Overview

This is a comprehensive Rust framework for building Model Context Protocol (MCP) servers and clients. It is fully compliant with the MCP 2025-06-18 specification. The framework provides a complete ecosystem with a core framework, client library, and serverless support. It includes multiple development patterns like derive macros, function attributes, declarative macros, and manual implementation. It supports various transports like HTTP/1.1, HTTP/2, WebSocket, SSE, and stdio. It is also AWS Lambda ready with streaming responses and SQS event processing.

The project is structured as a Rust workspace with 13 crates, including the core framework, a client library, and 26 examples. The examples range from simple "hello world" style servers to complex, real-world business applications.

## My Role: Critical Analyst and Planner

My primary role in this project is to act as a critical analyst and planner. I will:

*   **Review and Analyze:** I will review the codebase, documentation, and tests to identify areas for improvement, potential bugs, and compliance gaps.
*   **Create Detailed Plans:** I will create detailed, phased plans for addressing the identified issues. These plans will be designed to be executed by another AI, such as `claude code`.
*   **Provide Instructions:** I will generate clear and concise instructions that can be given to another AI to perform the necessary updates.
*   **Maintain Documentation:** I will keep the `GEMINI.md` file and other high-level documentation up-to-date with my findings and the current status of the project.

I will **not** directly modify the code or create files myself. My role is to provide the strategy and the plan, not to execute it.

## Framework Compliance and Design Analysis

### Executive Summary

The Turul MCP Framework is a production-ready, comprehensively tested implementation of the Model Context Protocol (MCP) 2025-06-18 specification. It provides a robust and idiomatic Rust solution for building MCP servers and clients. A full schema-level compliance review confirms that the framework's data structures are a meticulous match for the official specification. The testing strategy is mature, with E2E tests covering all major protocol areas, including advanced concurrency and state-management scenarios. While the protocol implementation is fully compliant, it's important to distinguish this from full feature completeness, as some advertised capabilities in the examples (e.g., `resources/subscribe`) are not yet fully implemented, representing the next frontier for development.

### From TypeScript Inheritance to Rust Traits: A Best-Practice Approach

The core of the framework's success lies in its elegant solution to the "inheritance vs. composition" problem. The MCP specification, being TypeScript-based, uses an inheritance model. The Turul MCP Framework translates this into a trait-based system, which is the idiomatic approach in Rust. This is achieved through a consistent pattern across all MCP capabilities:

1.  **Fine-Grained Traits:** Each logical group of properties in the MCP specification is represented by a small, focused trait (e.g., `HasName`, `HasDescription`).
2.  **Composition Traits:** A "definition" trait (e.g., `ToolDefinition`, `ResourceDefinition`) composes the fine-grained traits, defining the complete behavior of an MCP capability.
3.  **Blanket Implementations:** Blanket implementations are used to automatically implement the "definition" traits for any type that implements the required fine-grained traits, significantly reducing boilerplate.
4.  **Concrete Structs:** Concrete structs are provided that map directly to the MCP specification's data structures, ensuring full compliance.

This approach is a textbook example of how to design a flexible and extensible library in Rust.

### Capability-by-Capability Compliance

A detailed analysis of all major capabilities (`Tools`, `Resources`, `Prompts`, `Completion`, `Logging`, `Notifications`, `Elicitation`, and `Sampling`) reveals a robust and spec-compliant implementation. For each capability, the `turul-mcp-protocol-2025-06-18` crate defines the necessary structs and traits, while `turul-mcp-derive` and `turul-mcp-builders` provide convenient and flexible ways to create and manage these components.

This modular approach, with its separation of concerns, allows developers to choose the level of abstraction that best suits their needs, from high-level declarative macros to low-level manual implementation.

### Testing and Validation Strategy

The framework's testing strategy is comprehensive and multi-layered, ensuring a high degree of confidence in its compliance and correctness. The strategy has successfully guided the project from foundational implementation to a production-ready state.

*   **Protocol-Level Tests:** The framework includes tests that verify the correct serialization and deserialization of MCP data structures, ensuring that the JSON representation matches the official `schema.ts`.
*   **Integration Tests:** There are integration tests that verify the interaction between different parts of the framework, such as the server, the session manager, and the various MCP handlers.
*   **End-to-End (E2E) Tests:** The framework now includes a comprehensive suite of E2E tests that simulate real client-server interactions over HTTP. This suite has been expanded beyond the core capabilities (Tools, Resources, Prompts) to provide full E2E coverage for:
    *   **Sampling Protocol**
    *   **Roots Protocol**
    *   **Elicitation Protocol**
    *   **Advanced Concurrent Sessions:** High-concurrency and resource contention scenarios are tested to ensure stability and session isolation under load.
*   **Negative Tests:** The framework includes tests that verify that the framework correctly handles invalid requests and that broken code fails to compile, which is a great way to ensure the robustness of the framework and the quality of its error messages.

### Compliance vs. Feature Completeness: The Next Step

A key insight from the latest round of reviews is the distinction between protocol compliance and feature completeness. My analysis confirms that the framework is **fully compliant at the protocol level**—it correctly implements the shapes and rules defined in the MCP `schema.ts`.

However, this is different from being **fully feature-complete**. As noted in the `MCP_E2E_COMPLIANCE_TEST_PLAN.md`, some capabilities advertised by the *example servers* are not fully implemented in the framework itself. For instance, the `resources/subscribe` endpoint is defined in the protocol, but the framework does not yet provide a first-class, built-in implementation for handling subscriptions, even though the E2E tests correctly verify that the server advertises this capability as `false`.

This is not a compliance issue, but a matter of scope and maturity. The framework provides the necessary building blocks, but the implementation of certain business logic (like subscription management) is left to the developer. Future development should focus on closing these feature gaps by providing default, production-ready implementations for these remaining capabilities.

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

