# ADR: Compile-Time Schema Generation for MCP Tools

## Status: MANDATORY ✅ COMPLETE

## Core Rules

1. **All schemas**: `type: "object"` at root level (MCP requirement)
2. **Explicit types**: Use `#[tool(output = Type)]` for detailed schemas  
3. **Zero-config**: Use `additionalProperties: true` for flexibility
4. **Type mapping**: f64→number, String→string, bool→boolean, structs→object
5. **Field names**: primitives→"output", structs→camelCase, custom→user-specified

## Schema Examples

### String Output (Zero-config)
```rust
// Schema: {"type": "object", "additionalProperties": true}  
// Output: {"output": "Hello, Alice!"}
```

### Struct Output (Explicit type)
```rust
#[tool(output = CalculationResult)]
// Schema: {"type": "object", "properties": {"calculationResult": {"type": "object", "properties": {...}}}}
// Output: {"calculationResult": {"sum": 42.0, "operation": "addition"}}
```

### Struct Output (Zero-config) 
```rust
// Schema: {"type": "object", "additionalProperties": true}
// Output: {"calculationResult": {"sum": 42.0, "operation": "addition"}}
```

## Implementation Status: ✅ COMPLETE

- Type mapping fixed (f64→number, String→string, bool→boolean)
- Zero-config uses `additionalProperties: true` for flexibility
- Explicit types use detailed property schemas  
- Tool descriptions use readable format ("Struct Calculator")
- All MCP Inspector validation errors resolved