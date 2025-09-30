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

## Implementation Status: ✅ COMPLETE with Known Limitations

- Type mapping fixed (f64→number, String→string, bool→boolean)
- Zero-config uses `additionalProperties: true` for flexibility
- Explicit types use detailed property schemas (for self-referential types only)
- Tool descriptions use readable format ("Struct Calculator")
- All MCP Inspector validation errors resolved
- Custom output field names working (`output_field = "tileMetadata"`)

### Current Limitations

#### Detailed Schema Generation for External Types
**Status**: ⚠️ **PARTIAL IMPLEMENTATION**

While `#[tool(output = Type)]` syntax exists, detailed schema generation only works when `output_type == tool_struct` (self-referential). For external struct types like `TileMetadata`, the framework falls back to generic object schemas:

```rust
// ❌ This generates generic schema despite explicit output type
#[derive(McpTool, Clone)]
#[tool(name = "get_tile", output = TileMetadata, output_field = "tileMetadata")]
struct GetTileTool { ... }

// Generates: {"tileMetadata": {"type": "object", "additionalProperties": null}}
// Expected: {"tileMetadata": {"type": "object", "properties": {"tile_id": {...}, ...}}}
```

**Root Cause**: Rust procedural macros cannot introspect external struct definitions during compilation. The `generate_enhanced_output_schema` function can only analyze the struct being derived on.

**Current Workaround**: Use `additionalProperties: true` for flexibility, which works at runtime but provides less precise validation for MCP clients.

#### Proposed Solution: Runtime Schema Generation

**Approach**: Generate detailed schemas on first tool execution based on actual returned data structure.

**Benefits**:
- Works for any struct type (including external crates)
- Accurate schemas based on real data
- No compile-time limitations
- Better MCP client validation

**Implementation Plan**:
1. Analyze JSON structure on first execution
2. Generate detailed schema with proper field types
3. Cache for subsequent calls
4. Fallback to compile-time schema if analysis fails

**Impact**: This would solve the MCP Inspector validation issue for complex struct return types while maintaining backward compatibility.