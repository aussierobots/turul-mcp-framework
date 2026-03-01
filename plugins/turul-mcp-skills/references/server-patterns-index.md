# Server Patterns Index

Pointer index to authoritative documentation for common Turul MCP Framework patterns. This is a reference table — follow the links for full guidance.

| Topic | Brief | Authoritative Link |
|---|---|---|
| Import hierarchy | Prefer `turul_mcp_server::prelude::*`; never reference versioned protocol crates directly. | [CLAUDE.md — Protocol Re-export Rule](https://github.com/aussierobots/turul-mcp-framework/blob/main/CLAUDE.md#protocol-re-export-rule-mandatory) |
| Error handling | Return `McpResult<T>` from handlers; never create `JsonRpcError` directly. | [CLAUDE.md — Critical Error Handling Rules](https://github.com/aussierobots/turul-mcp-framework/blob/main/CLAUDE.md#critical-error-handling-rules) |
| Session API | Use `get_typed_state(key).await` / `set_typed_state(key, value).await?`. | [CLAUDE.md — API Conventions](https://github.com/aussierobots/turul-mcp-framework/blob/main/CLAUDE.md#api-conventions) |
| camelCase JSON | All JSON fields must use `#[serde(rename = "camelCase")]` per MCP spec. | [CLAUDE.md — JSON Naming](https://github.com/aussierobots/turul-mcp-framework/blob/main/CLAUDE.md#json-naming-camelcase-only) |
| Zero-config design | No method strings in user code; framework auto-determines from types. | [CLAUDE.md — Zero-Configuration Design](https://github.com/aussierobots/turul-mcp-framework/blob/main/CLAUDE.md#zero-configuration-design) |
| Tool output schemas | Tools with `outputSchema` must provide `structuredContent` (auto-generated). | [CLAUDE.md — MCP Tool Output Compliance](https://github.com/aussierobots/turul-mcp-framework/blob/main/CLAUDE.md#mcp-tool-output-compliance) |
| Server builder | Use `McpServer::builder()` to configure and start servers. | [CLAUDE.md — Basic Server](https://github.com/aussierobots/turul-mcp-framework/blob/main/CLAUDE.md#basic-server) |
| Middleware | Pre/post dispatch hooks for auth, logging, rate limiting. | [examples/middleware-auth-server](https://github.com/aussierobots/turul-mcp-framework/tree/main/examples/middleware-auth-server) |
| Lambda deployment | `LambdaMcpServerBuilder` for AWS Lambda integration. | [examples/lambda-mcp-server](https://github.com/aussierobots/turul-mcp-framework/tree/main/examples/lambda-mcp-server) |
| Session storage | InMemory (default), SQLite, PostgreSQL, DynamoDB backends. | [examples/simple-sqlite-session](https://github.com/aussierobots/turul-mcp-framework/tree/main/examples/simple-sqlite-session) |
| Task storage | Durable task backends for long-running operations. | [turul-mcp-task-storage crate](https://github.com/aussierobots/turul-mcp-framework/tree/main/crates/turul-mcp-task-storage) |
| Storage backend matrix | Feature flags, Cargo.toml patterns, config structs, and environment guidance for all storage backends. | [storage-backend-matrix.md](./storage-backend-matrix.md) |
| MCP compliance | Run `cargo test --test compliance` for specification compliance tests. | [AGENTS.md — MCP Specification Compliance](https://github.com/aussierobots/turul-mcp-framework/blob/main/AGENTS.md#mcp-specification-compliance) |
| Release gates | 7 named test suites covering compliance, schemas, lifecycle, capabilities. | [AGENTS.md — Release Readiness Notes](https://github.com/aussierobots/turul-mcp-framework/blob/main/AGENTS.md#release-readiness-notes-2025-10-01) |
| Streamable HTTP | Accept headers, session handshake, SSE notifications. | [CLAUDE.md — Streamable HTTP Requirements](https://github.com/aussierobots/turul-mcp-framework/blob/main/CLAUDE.md#streamable-http-requirements) |
| MCP 2025-11-25 | Notification naming, progress fields, Role enum, structuredContent. | [CLAUDE.md — MCP 2025-11-25 Compliance](https://github.com/aussierobots/turul-mcp-framework/blob/main/CLAUDE.md#mcp-2025-11-25-compliance) |
| Resource creation | Function macro, derive, declarative macro, builder patterns for MCP resources. | [resource-prompt-patterns skill](../skills/resource-prompt-patterns/SKILL.md) |
| Prompt creation | Derive, declarative macro, builder patterns for MCP prompts. | [resource-prompt-patterns skill](../skills/resource-prompt-patterns/SKILL.md) |
| MCP Client | Transport selection, connection lifecycle, tool invocation, task workflows, error handling. | [mcp-client-patterns skill](../skills/mcp-client-patterns/SKILL.md) |
