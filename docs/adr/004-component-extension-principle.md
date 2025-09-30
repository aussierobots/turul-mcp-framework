# ADR-004: Component Extension Principle

**Status**: MANDATORY  
**Date**: 2024-01-01  
**Decision Makers**: Framework Team

## Context

Framework components should be extensible without creating parallel "enhanced" or "advanced" versions that fragment the API and create choice paralysis.

## Problem

- Multiple similar components confuse users
- API fragmentation increases maintenance burden
- Choice paralysis reduces adoption
- Inconsistent patterns across framework

## Decision

**ABSOLUTE RULE**: Extend existing components by adding capabilities. NEVER create parallel "enhanced" or "advanced" versions that fragment the API.

## Implementation Pattern

### ✅ CORRECT - Single component with pluggable backend
```rust
pub struct SessionMcpHandler<S: SessionStorage = InMemorySessionStorage> {
    storage: Arc<S>,
    // ... other fields
}

// Zero-config constructor (defaults to InMemory)
impl SessionMcpHandler<InMemorySessionStorage> {
    pub fn new(config: ServerConfig, dispatcher: Arc<JsonRpcDispatcher>) -> Self {
        let storage = Arc::new(InMemorySessionStorage::new());
        Self::with_storage(config, dispatcher, storage)
    }
}

// Extensible constructor for other storage backends
impl<S: SessionStorage + 'static> SessionMcpHandler<S> {
    pub fn with_storage(config: ServerConfig, dispatcher: Arc<JsonRpcDispatcher>, storage: Arc<S>) -> Self {
        Self { config, dispatcher, storage }
    }
}
```

### ❌ WRONG - Creating parallel components
```rust
// ❌ DON'T DO THIS
struct BasicSessionHandler { /* basic implementation */ }
struct AdvancedSessionHandler { /* enhanced implementation */ }
struct EnterpriseSessionHandler { /* enterprise features */ }
```

## Benefits

- Prevents API fragmentation and choice paralysis
- Maintains zero-configuration while allowing extensibility  
- Reduces maintenance burden
- Aligns with framework philosophy of "one way to do things"

## Consequences

- **Positive**: Consistent API experience
- **Positive**: Zero configuration with extensibility
- **Positive**: Reduced maintenance burden
- **Risk**: Must design extensibility points carefully
- **Risk**: More complex generic implementations