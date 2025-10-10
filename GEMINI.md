# Turul MCP Framework

## Project Overview

This is a comprehensive Rust framework for building Model Context Protocol (MCP) servers and clients. It is fully compliant with the MCP 2025-06-18 specification. The framework provides a complete ecosystem with a core framework, a client library, and serverless support. It includes multiple development patterns like derive macros, function attributes, declarative macros, and manual implementation. It supports various transports like HTTP/1.1, HTTP/2, WebSocket, SSE, and stdio. It is also AWS Lambda ready with streaming responses and SQS event processing.

The project is structured as a Rust workspace with 10 crates, including the core framework, a client library, and 42 examples. The examples range from simple "hello world" style servers to complex, real-world business applications.

## My Role: Critical Analyst and Planner

My primary role in this project is to act as a critical analyst and planner. I will:

*   **Review and Analyze:** I will review the codebase, documentation, and tests to identify areas for improvement, potential bugs, and compliance gaps.
*   **Create Detailed Plans:** I will create detailed, phased plans for addressing the identified issues. These plans will be designed to be executed by a code-generating AI assistant.
*   **Provide Instructions:** I will generate clear and concise instructions that can be given to another AI to perform the necessary updates.
*   **Maintain Documentation:** I will keep the `GEMINI.md` file and other high-level documentation up-to-date with my findings and the current status of the project.

I will **not** directly modify the code or create files myself. My role is to provide the strategy and the plan, not to execute it.

## Framework Compliance and Design Analysis

### Executive Summary

The Turul MCP Framework is a production-ready, comprehensively tested implementation of the Model Context Protocol (MCP) 2025-06-18 specification. It provides a robust and idiomatic Rust solution for building MCP servers and clients. A full schema-level compliance review confirms that the framework's data structures are a meticulous match for the official specification. The testing strategy is mature, with E2E tests covering all major protocol areas, including advanced concurrency and state-management scenarios. The `0.2.0` release introduced a powerful, transport-agnostic middleware architecture, and the `0.2.1` release further refines the framework with improved schema generation and protocol purity.

### From TypeScript Inheritance to Rust Traits: A Critical Analysis

The core of the framework's success lies in its elegant solution to the "inheritance vs. composition" problem. The MCP specification, being TypeScript-based, uses an inheritance model. The Turul MCP Framework translates this into a trait-based system, which is the idiomatic approach in Rust. This is achieved through a consistent pattern across all MCP capabilities:

1.  **Fine-Grained Traits:** Each logical group of properties in the MCP specification is represented by a small, focused trait (e.g., `HasName`, `HasDescription`).
2.  **Composition Traits:** A "definition" trait (e.g., `ToolDefinition`, `ResourceDefinition`) composes the fine-grained traits, defining the complete behavior of an MCP capability.
3.  **Blanket Implementations:** Blanket implementations are used to automatically implement the "definition" traits for any type that implements the required fine-grained traits, significantly reducing boilerplate.
4.  **Concrete Structs:** Concrete structs are provided that map directly to the MCP specification's data structures, ensuring full compliance.

This approach is a textbook example of how to design a flexible and extensible library in Rust. It empowers developers to choose their desired level of abstraction, from high-level macros to low-level manual implementation, while ensuring that the framework can handle all implementations uniformly. The `0.2.1` release reinforced this design by moving all framework-specific traits out of the protocol crate and into the builders crate, achieving "protocol crate purity" and further clarifying the separation between the MCP specification and the framework's implementation.

#### A Critical Perspective on the Trait-Based Design

While the trait-based architecture is a significant strength, a critical analysis reveals several trade-offs inherent in this design:

*   **Conceptual Overhead:** The power of the trait system comes at the cost of increased conceptual overhead. To fully leverage the framework, developers must understand not just Rust's trait system, but also the specific patterns of composition and blanket implementations used in this project. For those less familiar with these intermediate-to-advanced Rust features, the "magic" of a struct automatically implementing `ToolDefinition` can be a barrier to understanding and debugging.

*   **Verbose Manual Implementation:** The framework provides derive macros to abstract away the boilerplate of trait implementation. However, when developers need to opt for manual implementation for more control, the verbosity becomes apparent. Implementing each fine-grained trait (`HasBaseMetadata`, `HasDescription`, `HasInputSchema`, etc.) individually can be tedious and error-prone, although it does offer maximum flexibility. This creates a steep jump in complexity when moving from the "easy path" of macros to the "expert path" of manual implementation.

*   **Duality of Trait and Struct:** The parallel existence of the `ToolDefinition` trait and the `Tool` struct is a necessary consequence of this design pattern in Rust. The `to_tool()` method on the trait, which converts the abstract trait object into a concrete, serializable struct, is a clean solution. However, it requires developers to be mindful of whether they are working with an abstract `&dyn ToolDefinition` or a concrete `Tool`, which can be a point of confusion.

In summary, the framework's core design makes a deliberate trade-off in favor of flexibility and power over simplicity. This is a reasonable choice for a framework intended to be comprehensive and extensible, but it's a trade-off that should be acknowledged. The provided macros are essential for mitigating this complexity and making the framework approachable for a wider range of developers.

### Middleware Architecture

The `0.2.0` release introduced a powerful and flexible middleware architecture, designed to handle cross-cutting concerns in a clean and transport-agnostic manner. The same middleware can be used for both HTTP and AWS Lambda transports, ensuring consistent behavior across different deployment environments.

The core of the middleware system is the `McpMiddleware` trait, which defines two key methods:

*   `before_dispatch`: Executed before the MCP request is processed. This allows for request validation, authentication, rate limiting, and session data injection.
*   `after_dispatch`: Executed after the MCP request has been processed. This allows for response modification, logging, and other post-processing tasks.

Middleware is registered on the server builder using the `.middleware()` method, and multiple middleware can be chained together to form a processing pipeline. The framework provides examples for common use cases, including:

*   **Authentication:** `examples/middleware-auth-server` and `examples/middleware-auth-lambda`
*   **Logging:** `examples/middleware-logging-server`
*   **Rate Limiting:** `examples/middleware-rate-limit-server`

This middleware system provides a robust mechanism for adding custom logic to the request/response lifecycle, without cluttering the core MCP implementation.

### Capability-by-Capability Compliance

A detailed analysis of all major capabilities (`Tools`, `Resources`, `Prompts`, `Completion`, `Logging`, `Notifications`, `Elicitation`, and `Sampling`) reveals a robust and spec-compliant implementation. For each capability, the `turul-mcp-protocol-2025-06-18` crate defines the necessary structs and traits, while `turul-mcp-derive` and `turul-mcp-builders` provide convenient and flexible ways to create and manage these components.

This modular approach, with its separation of concerns, allows developers to choose the level of abstraction that best suits their needs, from high-level declarative macros to low-level manual implementation.

### Testing and Validation Strategy: A Critical Review

The framework's testing strategy is its most mature and impressive feature, providing a high degree of confidence in its compliance, correctness, and robustness. The strategy is not just a collection of unit tests, but a multi-layered approach that validates the framework from the protocol level all the way to end-to-end user scenarios. The `E2E_TEST_IMPLEMENTATION_STATUS.md` file serves as a living document that tracks the claim of 100% E2E test coverage for the MCP 2025-06-18 specification.

A critical review of the test suite reveals several key strengths:

*   **Compliance as Code:** The tests in `tests/mcp_specification_compliance.rs` and `tests/mcp_behavioral_compliance.rs` effectively codify the MCP specification. Instead of relying on manual verification, the framework uses executable tests to ensure compliance with everything from endpoint naming (`resources/templates/list` vs `templates/list`) and notification casing (`listChanged` vs `list_changed`) to the correct propagation of `_meta` fields and the truthfulness of capability advertising.

*   **Behavioral Compliance Testing:** The framework goes beyond simple protocol validation to test nuanced behavioral requirements. The tests in `mcp_behavioral_compliance.rs` start a real server to verify complex interactions, such as:
    *   **Strict Lifecycle Enforcement:** The server correctly rejects requests made before the `notifications/initialized` handshake is complete.
    *   **Pagination and DoS Protection:** The server correctly handles `limit` and `cursor` parameters, and clamps large `limit` values to prevent abuse.
    *   **Version Negotiation:** The server correctly negotiates protocol versions with clients.

*   **True End-to-End (E2E) Testing:** The framework includes a suite of true E2E tests that simulate real client-server interactions over the network. The tests in `tests/e2e_sse_notification_roundtrip.rs` are a prime example. They start a server, connect a client via SSE, trigger a tool that sends progress notifications, and verify that the notifications are delivered to the correct client. This provides a high degree of confidence that the entire notification pipeline is working correctly.

*   **Session Isolation Testing:** The E2E test suite also includes tests for critical security and correctness features like session isolation. The `test_sse_notification_session_isolation` test creates multiple concurrent clients and verifies that each client only receives notifications for its own session, proving that the framework can safely handle multiple users at once.

*   **Negative Testing:** The test suite includes a comprehensive set of negative tests that verify the framework's resilience to invalid inputs and error conditions. This includes everything from invalid URI formats to incorrect protocol usage.

The `0.2.1` release significantly improved the verification infrastructure, with 30 out of 31 examples now passing a comprehensive verification suite. This demonstrates a commitment to quality and provides a high degree of confidence in the framework's stability.

### Compliance vs. Behavioral Completeness: The Next Step for 0.2.1 and Beyond

A key insight from the latest round of reviews is the distinction between protocol compliance and behavioral completeness. My analysis confirms that the framework is **fully compliant at the protocol level**—it correctly implements the data structures and rules defined in the MCP `schema.ts`.

However, for the `0.2.1` release, it is important to note this is different from being **fully behavior-complete**. As detailed in the `MCP_E2E_COMPLIANCE_TEST_PLAN.md`, several advanced capabilities, while defined, are not yet fully implemented. These represent the next frontier of development for the framework. Key examples include:

*   **Resource Subscriptions:** The framework does not provide a first-class implementation for the `resources/subscribe` method. It correctly advertises this capability as `false` in the `initialize` handshake, adhering to the protocol's truthfulness requirement. The necessary hooks exist for developers to implement this logic themselves.
*   **Advanced List Endpoint Features:** Some list-based endpoints, such as `tools/list`, do not yet support advanced features like stable sorting, pagination, or the propagation of the `_meta` field from request to response.
*   **Session-Aware Resources:** The `McpResource::read` trait currently lacks access to the session context, meaning resources cannot dynamically change their content based on the session state. The `session://` resource in examples is therefore a simulation of this capability.

These are not compliance bugs but rather represent the current scope of the framework's maturity. They are documented as the immediate focus for future releases. The `0.2.1` release focused on shoring up the existing features and improving the overall stability of the framework, laying a solid foundation for tackling these larger features in the future.

## Release 0.2.1 Highlights

The `0.2.1` release focused on stability, bug fixing, and improving the developer experience. Key highlights include:

*   **Schemars Integration**: A breaking change now requires tool output types to derive `schemars::JsonSchema`, enabling the generation of detailed, accurate schemas for all tools in the `tools/list` endpoint.
*   **Protocol Crate Purity**: All framework-specific traits have been moved from `turul-mcp-protocol` to `turul-mcp-builders`. This breaking change ensures the protocol crate is a pure, 1-to-1 implementation of the MCP specification.
*   **Notification Payloads Fixed**: A critical regression where notification payloads were not being serialized correctly has been fixed, and 18 new tests have been added to prevent future regressions.
*   **Improved SSE Resumability:** Ensured that SSE keepalive events preserve the `Last-Event-ID`, allowing for proper reconnection.
*   **Enhanced Verification:** The verification infrastructure was significantly improved, with deterministic polling, pre-built binaries, and better error diagnosis. 30 out of 31 examples are now verified.
*   **Code Quality:** Fixed 156 clippy warnings, resulting in a 100% clean codebase.

## Building and Running

The project is built and tested using Cargo.

### Building the project

To build the entire project, run the following command from the root of the repository:

```bash
cargo build --workspace
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

To run the middleware examples:

```bash
cargo run --example middleware-auth-server
cargo run --example middleware-logging-server
cargo run --example middleware-rate-limit-server
```

To run the Lambda middleware example, you will need to use `cargo-lambda`:

```bash
cargo lambda watch --example middleware-auth-lambda
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

The project uses `tracing` for logging and `thiserror` for error handling. Handlers must return domain errors, and new error types should derive `thiserror::Error` and implement `turul_mcp_json_rpc_server::r#async::ToJsonRpcError`.

The project has a comprehensive test suite with over 650 tests. You can run all tests with `cargo test --workspace`.

## Detailed Guidelines

For detailed guidelines on project structure, architecture, development conventions, testing, and MCP specification compliance, please refer to the [AGENTS.md](AGENTS.md) file. This document provides a comprehensive guide for developers and AI agents working on the Turul MCP Framework.
