# ADR-001: turul_mcp_protocol Alias Usage

**Status**: MANDATORY  
**Date**: 2024-01-01  
**Decision Makers**: Framework Team

## Context

The framework uses protocol versioning but needs future-proofing and consistency across all code. The `turul-mcp-protocol` alias now defaults to `turul-mcp-protocol-2026-07-28` (the `protocol-2025-11-25` cargo feature is the opt-in escape hatch to the prior spec). See ADR-029 for the feature-gated coexistence mechanism.

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
## Revision log

- **2026-06-07** — **Third exception added: `turul-mcp-client` may import both versioned protocol crates directly.** The bilingual client links `turul-mcp-protocol-2025-11-25` and `turul-mcp-protocol-2026-07-28` directly (gated by the `client-bilingual` / `client-2025-11-25-only` / `client-2026-07-28-only` features) so one client can negotiate and speak either wire spec per connection; it does NOT route through the `turul-mcp-protocol` alias. This is the only consumer-side exception to the rule above. See ADR-030 (client bilingual coexistence). The alias rule remains MANDATORY for every other consumer (server, http-server, builders, aws-lambda, derive, examples).

- **2026-06-07** — **Alias default flipped to `protocol-2026-07-28`.** The `turul-mcp-protocol` re-export now defaults to the 2026-07-28 crate (it previously re-exported `turul-mcp-protocol-2025-11-25` unconditionally). `protocol-2025-11-25` is the opt-in feature for consumers who still need the prior spec. Mechanism and cascade in ADR-029. Landed on `2026-07-28-MCP-Specification` (not merged to `main`).

- **2026-06-07** — **Maintainer decision: the `turul-mcp-protocol` alias is a 0.4 transition mechanism, slated for retirement in 0.5.** The alias exists to give framework crates one re-export point during the 2025-11-25 → 2026-07-28 spec transition. Once the transition settles, the alias is to be deprecated/retired in the 0.5 line in favor of framework crates depending on the versioned protocol crate (`turul-mcp-protocol-2026-07-28`) directly. The MANDATORY-alias rule above governs the 0.4 line; it is expected to be superseded in 0.5.
