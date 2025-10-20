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
**Status**: ✅ **RESOLVED via Schemars Integration**

The `#[tool(output = Type)]` syntax now supports three schema generation paths:

1. **Schemars integration** (with `#[derive(JsonSchema)]`): Detailed schemas for any external type
2. **Self introspection** (when `output = Self` or omitted): Detailed schemas via compile-time analysis
3. **Fallback** (external types without JsonSchema): Generic object schemas

```rust
// ✅ Detailed schema with schemars
#[derive(Serialize, Deserialize, JsonSchema)]  // Add JsonSchema derive
pub struct TileMetadata { ... }

#[derive(McpTool, Clone)]
#[tool(name = "get_tile", output = TileMetadata, output_field = "tileMetadata")]
struct GetTileTool { ... }

// Generates: {"tileMetadata": {"type": "object", "properties": {"tile_id": {...}, ...}}}

// ✅ Detailed schema with Self introspection (no schemars needed)
#[derive(McpTool, Clone, Serialize, Deserialize)]
#[tool(name = "calculator")]  // No output specified = defaults to Self
struct Calculator {
    #[param] pub a: f64,
    #[param] pub b: f64,
    pub result: f64,  // Output field
}

// ⚠️ Generic fallback (external type without JsonSchema)
#[derive(Serialize, Deserialize)]  // NO JsonSchema
pub struct TileMetadata { ... }

// Generates: {"output": {"type": "object", "additionalProperties": true}}
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