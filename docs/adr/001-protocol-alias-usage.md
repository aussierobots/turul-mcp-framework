# ADR-001: turul_mcp_protocol Alias Usage

**Status**: MANDATORY  
**Date**: 2024-01-01  
**Decision Makers**: Framework Team

## Context

The framework uses protocol versioning (currently `turul-mcp-protocol-2025-11-25`) but needs future-proofing and consistency across all code.

## Problem

- Direct versioned imports create coupling to specific protocol versions
- Future protocol updates would require massive code changes
- Inconsistent import patterns across the framework

## Decision

ALL code in the turul-mcp-framework MUST use the `turul_mcp_protocol` alias, never direct versioned crate paths.

## Implementation

### Cargo.toml Pattern
```toml
[dependencies]
# ✅ CORRECT: Use the turul-mcp-protocol re-export crate
turul-mcp-protocol = { path = "path/to/turul-mcp-protocol" }

# ❌ WRONG: Direct versioned dependency
turul-mcp-protocol-2025-11-25 = { path = "path/to/turul-mcp-protocol-2025-11-25" }
```

### Import Pattern
```rust
// ✅ CORRECT: Protocol types via re-export crate
use turul_mcp_protocol::{Resource, ResourceContent};

// ✅ CORRECT: Framework traits via builders
use turul_mcp_builders::prelude::*;  // HasResourceMetadata, ResourceDefinition, etc.

// ❌ WRONG: Direct versioned import
use turul_mcp_protocol_2025_11_25::{Resource, ResourceContent};

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