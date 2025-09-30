# ADR-002: Resources Integration Pattern

**Status**: MANDATORY  
**Date**: 2024-01-01  
**Decision Makers**: Framework Team

## Context

Resources integration has been problematic due to:
- Trait implementation conflicts between derive macros and blanket implementations
- Missing required trait imports causing method resolution failures
- Framework version mismatches creating incompatible trait definitions
- Unclear error handling patterns for resource operations

## Decision

MCP Resources MUST be implemented using the framework's trait-based pattern with proper import discipline and derive macro usage.

## Implementation

### ✅ CORRECT Resource Implementation Pattern
```rust
use turul_mcp_protocol::resources::{
    HasResourceMetadata, HasResourceUri, HasResourceDescription, 
    ResourceDefinition, McpResource
};
use turul_mcp_derive::McpResource;
use serde::{Serialize, Deserialize};

#[derive(McpResource, Clone, Serialize, Deserialize)]
#[resource(
    name = "ticker_announcements",
    description = "ASX ticker announcements and market data"
)]
pub struct TickerAnnouncementsResource {
    pub ticker: String,
    pub date_range: Option<String>,
}

impl TickerAnnouncementsResource {
    // Business logic methods here
    pub async fn fetch_announcements(&self) -> McpResult<Vec<Announcement>> {
        // Implementation
    }
}
```

### Required Import Pattern
```rust
// MANDATORY: All resource implementations must import these traits
use turul_mcp_protocol::resources::{
    HasResourceMetadata,    // Provides name(), title(), description() 
    HasResourceUri,         // Provides uri() method
    HasResourceDescription, // Provides description() method  
    ResourceDefinition,     // Composed trait for all resource operations
    McpResource             // Execution trait for resource calls
};
```

### Error Handling Pattern
```rust
use turul_mcp_protocol::errors::McpError;

impl McpResource for MyResource {
    async fn read(&self, uri: &str) -> McpResult<ResourceContent> {
        self.fetch_data().await
            .map_err(|e| McpError::internal_error(&format!("Resource fetch failed: {}", e)))
    }
}

// Available error constructors:
// - McpError::internal_error(msg)
// - McpError::invalid_params(msg) 
// - McpError::method_not_found(method)
// - McpError::invalid_request(msg)
```

## ❌ WRONG Approaches - DO NOT USE
```rust
// ❌ Manual trait implementation (creates conflicts)
impl ResourceDefinition for MyResource { /* manual impl */ }

// ❌ Missing required imports (method resolution fails)
use turul_mcp_derive::McpResource; // Missing HasResourceUri, etc.

// ❌ Direct protocol version imports (version mismatch)
use turul_mcp_protocol_2025_06_18::resources::ResourceDefinition;
```

## Consequences

- **Positive**: Eliminates trait conflicts and import issues
- **Positive**: Consistent error handling across all resources
- **Positive**: Framework auto-determines resource URIs and methods
- **Risk**: Must maintain strict import discipline across all resource implementations