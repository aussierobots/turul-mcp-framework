# ADR-005: Trait-Based Architecture Pattern

**Status**: MANDATORY  
**Date**: 2024-01-01  
**Decision Makers**: Framework Team

## Context

The framework needs a consistent, composable architecture pattern for implementing MCP specification components across all areas (tools, resources, prompts, etc.).

## Decision

Follow a consistent trait-based architecture pattern across all MCP specification implementations using fine-grained, composable traits.

## Pattern: Concrete Struct + Trait Interface

### 1. Fine-Grained Traits
Break down TypeScript interfaces into focused, composable Rust traits:

```rust
pub trait HasBaseMetadata {
    fn name(&self) -> &str;
    fn title(&self) -> Option<&str> { None }
}

pub trait HasInputSchema {
    fn input_schema(&self) -> &ToolSchema;
}
```

### 2. Composed Definition Trait
Combine fine-grained traits into complete definition interfaces:

```rust
pub trait ToolDefinition: 
    HasBaseMetadata + HasInputSchema + /* ... */ + Send + Sync 
{
    // Convenience methods using composed traits
    fn display_name(&self) -> &str {
        // TypeScript spec: Tool.title > annotations.title > Tool.name
        if let Some(title) = self.title() {
            title
        } else if let Some(annotations) = self.annotations() {
            if let Some(title) = &annotations.title {
                title
            } else {
                self.name()
            }
        } else {
            self.name()
        }
    }
}
```

### 3. Concrete Struct Implementation
Protocol structs implement the definition traits:

```rust
impl HasBaseMetadata for Tool {
    fn name(&self) -> &str { &self.name }
    fn title(&self) -> Option<&str> { self.title.as_deref() }
}

impl HasInputSchema for Tool {
    fn input_schema(&self) -> &ToolSchema { &self.input_schema }
}
```

### 4. Dynamic Implementation
Runtime implementations also implement the same definition traits.

### 5. Framework Interface Usage
All framework code uses trait interfaces, not concrete types.

## Application Areas

Apply this pattern consistently across:

- **Tools**: `Tool` struct + `ToolDefinition` trait + `McpTool` trait
- **Resources**: `Resource` struct + `ResourceDefinition` trait + `McpResource` trait  
- **Prompts**: `Prompt` struct + `PromptDefinition` trait + `McpPrompt` trait
- **Sampling**: Message types + definition traits + handler traits
- **Completion**: Completion types + definition traits + provider traits

## Consequences

- **Positive**: Consistent patterns across all MCP areas
- **Positive**: Composable, fine-grained traits
- **Positive**: TypeScript specification compliance
- **Positive**: Runtime and compile-time implementations use same interfaces
- **Risk**: More complex trait hierarchies
- **Risk**: Requires discipline to maintain consistency