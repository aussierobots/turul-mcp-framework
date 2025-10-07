# ADR-001: turul_mcp_protocol Alias Usage

**Status**: MANDATORY  
**Date**: 2024-01-01  
**Decision Makers**: Framework Team

## Context

The framework uses protocol versioning (`turul-mcp-protocol-2025-06-18`) but needs future-proofing and consistency across all code.

## Problem

- Direct versioned imports create coupling to specific protocol versions
- Future protocol updates would require massive code changes
- Inconsistent import patterns across the framework

## Decision

ALL code in the turul-mcp-framework MUST use the `turul_mcp_protocol` alias, never direct `turul_mcp_protocol_2025_06_18` paths.

## Implementation

### Cargo.toml Pattern
```toml
[dependencies]
# ✅ CORRECT: Use turul-mcp-protocol as alias
turul-mcp-protocol = { path = "path/to/turul-mcp-protocol-2025-06-18", package = "turul-mcp-protocol-2025-06-18" }

# ❌ WRONG: Direct versioned dependency  
turul-mcp-protocol-2025-06-18 = { path = "path/to/turul-mcp-protocol-2025-06-18" }
```

### Import Pattern
```rust
// ✅ CORRECT: Protocol types via alias
use turul_mcp_protocol::{Resource, ResourceContent};

// ✅ CORRECT: Framework traits via builders
use turul_mcp_builders::prelude::*;  // HasResourceMetadata, ResourceDefinition, etc.

// ❌ WRONG: Direct versioned import
use turul_mcp_protocol_2025_06_18::{Resource, ResourceContent};

// ❌ WRONG: Framework traits from protocol crate (no longer exist there)
use turul_mcp_protocol::{HasResourceMetadata, ResourceDefinition};
```

## Enforcement

This rule applies to:
- All example code
- Macro-generated code  
- Test code
- Documentation code samples
- Derive macro implementations

## Consequences

- **Positive**: Future-proofed against protocol version changes
- **Positive**: Consistent import patterns across codebase
- **Positive**: Easier protocol upgrades
- **Risk**: Must maintain strict import discipline