# ADR: Compile-Time Schema Generation for MCP Tools

## Status: MANDATORY

## Context

The MCP framework requires **compile-time schema generation** for proper MCP Inspector compatibility and protocol compliance. Schema generation must be available immediately when `tools/list` is called, before any tool execution.

## Problem Statement

1. **MCP Inspector Validation**: Output schemas must exactly match the JSON types being returned
2. **Compile-Time Requirement**: Input and output schemas must be available at compile time
3. **Type Safety**: Schema types must align with actual Rust return types
4. **Zero-Config vs Explicit**: Support both zero-configuration and explicit type specification

## MCP TypeScript Specification Alignment

```typescript
export interface Tool extends BaseMetadata {
  description?: string;  // LLM-friendly description (no "zero-config" text)
  inputSchema: {
    type: "object";
    properties?: { [key: string]: object };
    required?: string[];
  };
  outputSchema?: {
    type: "object";  // MUST be object at root level
    properties?: { [key: string]: object };  // Property types must match actual values
    required?: string[];
  };
}
```

## Core Issue Identified

The validation errors are caused by **JSON Schema type mismatches**:
- ALL schemas MUST be `type: "object"` at root level per MCP spec
- Properties within the object must have correct JSON Schema types
- Example: `{"output": 42}` requires schema `{type: "object", properties: {"output": {type: "number"}}}`

## Architecture Decision

### 1. Maintain Full Attribute Support

```rust
// Explicit output type and field name
#[derive(McpTool)]
#[tool(name = "calculator", description = "Add numbers", output = f64, field = "sum")]
struct Calculator { a: f64, b: f64 }

// Zero-config with custom field name
#[derive(McpTool)]
#[tool(field = "result")]
struct SimpleCalc { x: f64, y: f64 }

// Pure zero-config
#[derive(McpTool)]
struct AutoCalc { a: f64, b: f64 }
```

### 2. Complete Schema Examples

#### String Output Example:
```rust
#[derive(McpTool)]
struct Greeter { name: String }

impl Greeter {
    async fn execute(&self) -> McpResult<String> {
        Ok(format!("Hello, {}!", self.name))
    }
}

// Generated Schema:
{
  "type": "object",
  "properties": {
    "output": { "type": "string" }
  },
  "required": ["output"]
}

// Actual Output:
{"output": "Hello, Alice!"}
```

#### Struct Output Example:
```rust
#[derive(Serialize)]
struct CalculationResult {
    sum: f64,
    operation: String,
    timestamp: u64,
}

#[derive(McpTool)]
struct StructCalculator { a: f64, b: f64 }

impl StructCalculator {
    async fn execute(&self) -> McpResult<CalculationResult> {
        Ok(CalculationResult { ... })
    }
}

// Generated Schema:
{
  "type": "object",
  "properties": {
    "calculationResult": {
      "type": "object",
      "properties": {
        "sum": { "type": "number" },
        "operation": { "type": "string" },
        "timestamp": { "type": "integer" }
      },
      "required": ["sum", "operation", "timestamp"]
    }
  },
  "required": ["calculationResult"]
}

// Actual Output:
{
  "calculationResult": {
    "sum": 42.0,
    "operation": "addition", 
    "timestamp": 1234567890
  }
}
```

#### Nested Struct Example:
```rust
#[derive(Serialize)]
struct Address {
    street: String,
    city: String,
    coordinates: Coordinates,
}

#[derive(Serialize)]  
struct Coordinates {
    lat: f64,
    lng: f64,
}

// Generated Schema (nested properties):
{
  "type": "object",
  "properties": {
    "address": {
      "type": "object",
      "properties": {
        "street": { "type": "string" },
        "city": { "type": "string" },
        "coordinates": {
          "type": "object",
          "properties": {
            "lat": { "type": "number" },
            "lng": { "type": "number" }
          },
          "required": ["lat", "lng"]
        }
      },
      "required": ["street", "city", "coordinates"]
    }
  },
  "required": ["address"]
}
```

### 3. JSON Schema Type Mapping

| Rust Type | JSON Schema Type | Example Output |
|-----------|------------------|----------------|
| `f64`, `f32` | `"number"` | `{"output": 42.5}` |
| `i32`, `i64`, etc | `"integer"` | `{"output": 42}` |
| `String`, `&str` | `"string"` | `{"output": "hello"}` |
| `bool` | `"boolean"` | `{"output": true}` |
| `Vec<T>` | `"array"` | `{"output": [1,2,3]}` |
| Custom structs | `"object"` | `{"calculationResult": {...}}` |

### 4. Schema Generation Strategy

1. **Explicit Types**: Use `#[tool(output = Type)]` to generate accurate schema
2. **Zero-Config**: Require type hints or use compile-time detection
3. **Struct Introspection**: Generate detailed object schemas with nested property definitions
4. **No Runtime Detection**: All schemas must be available at compile time

### 5. Field Name Resolution

| Scenario | Field Name | Example |
|----------|------------|---------|
| Custom specified | User value | `#[tool(field = "result")]` → `"result"` |
| Primitive types | `"output"` | `f64` → `{"output": 42.0}` |
| Struct types | camelCase struct name | `CalculationResult` → `{"calculationResult": {...}}` |

### 6. Implementation Requirements

#### Core Functions (in utils.rs)
- `generate_enhanced_output_schema()` - Main schema generator with struct introspection
- `determine_output_field_name()` - Field name resolution logic
- `type_to_schema()` - **FIX THIS**: Must generate correct JSON Schema types
- `rust_type_to_json_schema()` - **NEW**: Direct Rust type → JSON Schema mapping

#### Derive Macro (tool_derive.rs)
- Maintain all existing attribute parsing
- Support both explicit and zero-config modes
- Generate compile-time schemas (no runtime detection for now)

## What Went Wrong

1. **Kept changing approaches** instead of fixing the core type mapping issue
2. **Removed working functionality** instead of improving it
3. **Focused on runtime solutions** when compile-time is required
4. **Made schemas generic** instead of type-specific

## Immediate Actions

1. **Restore full attribute parsing** - `#[tool(output = Type, field = "name")]`
2. **Fix type mapping functions** - Generate correct JSON Schema types
3. **Test with MCP Inspector** - Ensure no validation errors
4. **Preserve zero-config** - But with proper type inference

## Success Criteria

- ✅ MCP Inspector shows no validation errors
- ✅ All tool attributes work as designed
- ✅ Schemas match actual return types exactly
- ✅ Zero-config tools work without configuration
- ✅ Struct property introspection functions
- ✅ Custom field names work correctly

## Non-Goals

- Runtime schema detection (compile-time only)
- Generic/flexible schemas (must be type-specific)
- Complex unsafe code (keep it simple)

## Implementation Priority

1. **HIGH**: Fix `type_to_schema()` and related functions with correct JSON types
2. **HIGH**: Restore all removed attribute parsing functionality  
3. **MEDIUM**: Test all scenarios with MCP Inspector
4. **LOW**: Enhance struct property introspection (already mostly working)

This ADR prevents further architectural thrashing and ensures we build on the working foundation instead of constantly redesigning.