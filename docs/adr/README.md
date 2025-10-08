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
| [002](./002-compile-time-schema-generation.md) | Compile-time Schema Generation | Accepted | 2025-08-28 | Automatic JSON schema generation from Rust types |
| [003](./003-jsonschema-standardization.md) | JsonSchema Standardization | Accepted | 2025-08-28 | Framework-wide JsonSchema type usage |
| [004](./004-sessioncontext-macro-support.md) | SessionContext Macro Support | Accepted | 2025-08-28 | Automatic SessionContext injection in macros |
| [005](./005-mcp-message-notifications-architecture.md) | MCP Message Notifications Architecture | Accepted | 2025-09-02 | Dual-stream SSE notification delivery and event type formatting |
| [006](./006-streamable-http-compatibility.md) | Streamable HTTP Compatibility | Accepted | 2025-09-15 | SSE streaming and chunked transfer encoding for progress notifications |
| [007](./007-auto-detection-resource-security.md) | Auto-detection Resource Security | Accepted | 2025-09-18 | Framework auto-detects resource implementations to prevent security issues |
| [008](./008-documentation-accuracy-verification.md) | Documentation Accuracy Verification Process | Accepted | 2025-09-20 | Critical verification methodology for documentation accuracy |
| [009](./009-protocol-based-handler-routing.md) | Protocol-based Handler Routing | Accepted | 2025-09-22 | Route requests based on protocol version negotiation |
| [010](./010-architectural-guidelines.md) | Architectural Guidelines | Accepted | 2025-09-25 | Core principles for framework design and maintenance |
| [011](./011-lambda-streaming-incompatibility.md) | Lambda Streaming Incompatibility | Accepted | 2025-09-28 | AWS Lambda limitations with SSE streaming |
| [012](./012-middleware-architecture.md) | Middleware Architecture | Accepted | 2025-10-05 | Before/after hooks for auth, logging, and custom logic |
| [013](./013-lambda-authorizer-integration.md) | Lambda Authorizer Integration | Accepted | 2025-10-06 | Lambda authorizer pattern for API Gateway authentication |
| [014](./014-schemars-schema-generation.md) | Schemars Schema Generation | Accepted | 2025-10-09 | Optional schemars integration for automatic tool output schemas |

## Adding New ADRs

When adding a new ADR:

1. Use the next sequential number (005, 006, etc.)
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