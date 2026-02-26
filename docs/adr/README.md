# Architecture Decision Records (ADRs)

This directory contains Architecture Decision Records (ADRs) that document important architectural decisions made during the development of the turul-mcp-framework.

## ADR Format

Each ADR follows the standard format:
- **Status**: Current state (Accepted, Superseded, Deprecated)
- **Context**: Problem description and constraints
- **Decision**: What was decided and why
- **Consequences**: Positive and negative outcomes

## Current ADRs

| Number | Title | Status | Date | Description |
|--------|-------|--------|------|-------------|
| [001](./001-session-storage-architecture.md) | Session Storage Architecture | Accepted | 2025-08-29 | Pluggable session storage backend design |
| [001](./001-lambda-mcp-integration-architecture.md) | Lambda MCP Integration Architecture | Accepted | 2025-08-31 | AWS Lambda integration architecture for MCP servers |
| [001](./001-protocol-alias-usage.md) | Protocol Alias Usage | Mandatory | 2024-01-01 | Always use `turul-mcp-protocol` re-export, never versioned crates directly |
| [002](./002-compile-time-schema-generation.md) | Compile-time Schema Generation | Accepted | 2025-08-28 | Automatic JSON schema generation from Rust types |
| [002](./002-resources-integration-pattern.md) | Resources Integration Pattern | Mandatory | 2024-01-01 | Standard pattern for integrating MCP resources |
| [003](./003-jsonschema-standardization.md) | JsonSchema Standardization | Accepted | 2025-08-28 | Framework-wide JsonSchema type usage |
| [003](./003-zero-configuration-principle.md) | Zero-Configuration Design Principle | Mandatory | 2024-01-01 | Users never specify method strings; framework auto-determines |
| [004](./004-sessioncontext-macro-support.md) | SessionContext Macro Support | Accepted | 2025-08-28 | Automatic SessionContext injection in macros |
| [004](./004-component-extension-principle.md) | Component Extension Principle | Mandatory | 2024-01-01 | Extend existing components, never create "enhanced" versions |
| [005](./005-mcp-message-notifications-architecture.md) | MCP Message Notifications Architecture | Accepted | 2025-09-02 | Dual-stream SSE notification delivery and event type formatting |
| [005](./005-trait-based-architecture.md) | Trait-Based Architecture Pattern | Mandatory | 2024-01-01 | Trait-based design for extensibility and testability |
| [006](./006-streamable-http-compatibility.md) | Streamable HTTP Compatibility | Accepted | 2025-09-15 | SSE streaming and chunked transfer encoding for progress notifications |
| [007](./007-auto-detection-resource-security.md) | Auto-detection Resource Security | Accepted | 2025-09-18 | Framework auto-detects resource implementations to prevent security issues |
| [008](./008-documentation-accuracy-verification.md) | Documentation Accuracy Verification Process | Accepted | 2025-09-20 | Critical verification methodology for documentation accuracy |
| [009](./009-protocol-based-handler-routing.md) | Protocol-based Handler Routing | Accepted | 2025-09-22 | Route requests based on protocol version negotiation |
| [010](./010-architectural-guidelines.md) | Architectural Guidelines | Accepted | 2025-09-25 | Core principles for framework design and maintenance |
| [011](./011-lambda-streaming-incompatibility.md) | Lambda Streaming Incompatibility | Accepted | 2025-09-28 | AWS Lambda limitations with SSE streaming |
| [012](./012-middleware-architecture.md) | Middleware Architecture | Accepted | 2025-10-05 | Before/after hooks for auth, logging, and custom logic |
| [013](./013-lambda-authorizer-integration.md) | Lambda Authorizer Integration | Accepted | 2025-10-06 | Lambda authorizer pattern for API Gateway authentication |
| [014](./014-schemars-schema-generation.md) | Schemars Schema Generation | Accepted | 2025-10-09 | Optional schemars integration for automatic tool output schemas |
| [015](./015-mcp-2025-11-25-protocol-crate.md) | MCP 2025-11-25 Protocol Crate | Accepted | 2026-02-07 | Separate crate strategy for 2025-11-25 spec support |
| [016](./016-task-storage-architecture.md) | Task Storage Architecture | Accepted | 2026-02-11 | Pluggable task storage with 4 backends and parity test suite |
| [017](./017-task-runtime-executor-boundary.md) | Task Runtime-Executor Boundary | Accepted | 2026-02-11 | Three-layer split: storage / executor / runtime |
| [018](./018-task-pagination-cursor-contract.md) | Task Pagination Cursor Contract | Accepted | 2026-02-11 | Deterministic cursor-based pagination across backends |

## Tasks Architecture ADRs

ADRs 015–018 form a cohesive cluster documenting the Tasks subsystem (an experimental MCP 2025-11-25 capability with full framework implementation support):

| ADR | Focus |
|-----|-------|
| [015](./015-mcp-2025-11-25-protocol-crate.md) | Protocol crate strategy — separate crate for 2025-11-25 spec types including Tasks |
| [016](./016-task-storage-architecture.md) | Storage — `TaskStorage` trait, 4 backends, state machine, parity test suite |
| [017](./017-task-runtime-executor-boundary.md) | Runtime — three-layer split: storage / executor / runtime coordination |
| [018](./018-task-pagination-cursor-contract.md) | Pagination — deterministic cursor contract across backends, DynamoDB exception |

## Adding New ADRs

When adding a new ADR:

1. Use the next sequential number (019, 020, etc.). Note: some legacy ADRs share a number prefix (e.g., multiple 001-* files covering different topics) — each is a separate accepted decision distinguished by its full title and filename.
2. Use kebab-case for the filename: `NNN-short-descriptive-title.md`
3. Follow the standard ADR template
4. Update this README with the new entry
5. Reference the ADR in relevant documentation

## ADR Template

```markdown
# ADR-NNN: Title

**Status**: Accepted | Superseded | Deprecated

**Date**: YYYY-MM-DD

## Context

Description of the problem, constraints, and requirements that led to this decision.

## Decision

What was decided, including alternatives considered and rationale.

## Consequences

### Positive
- List positive outcomes

### Negative  
- List negative outcomes or trade-offs

### Risks
- List potential risks or mitigation strategies

## Implementation

Key implementation details and patterns to follow.
```

## See Also

- [Framework Architecture](../../CLAUDE.md) - Main architectural documentation
- [Working Memory](../../WORKING_MEMORY.md) - Current development status
- [Examples](../../examples/) - Code examples implementing these decisions