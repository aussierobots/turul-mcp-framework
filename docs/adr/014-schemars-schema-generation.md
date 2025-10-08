# ADR-014: Schemars Schema Generation

**Status**: Accepted

**Date**: 2025-10-09

## Context

Tool output schemas enable MCP clients (like AI models) to understand the structure of tool responses, improving their ability to work with complex structured data. However, manually building `JsonSchema` objects is verbose and error-prone, especially for nested structures with many fields.

### Problem

```rust
// Manual schema building is verbose
impl HasOutputSchema for MyTool {
    fn output_schema(&self) -> Option<JsonSchema> {
        Some(JsonSchema::Object {
            properties: Some({
                let mut props = HashMap::new();
                props.insert("result".to_string(), JsonSchema::Number {
                    description: Some("Calculation result".to_string()),
                    ..Default::default()
                });
                props.insert("operation".to_string(), JsonSchema::String {
                    description: Some("Operation performed".to_string()),
                    ..Default::default()
                });
                props  // 20+ lines for 2 fields!
            }),
            required: Some(vec!["result".to_string(), "operation".to_string()]),
            ..Default::default()
        })
    }
}
```

### Requirements

1. **Automatic Schema Generation**: Derive schemas from Rust types
2. **Type Safety**: Schemas auto-sync with code changes
3. **Optional**: No breaking changes to existing manual approach
4. **MCP Compliance**: Generated schemas must match MCP spec format
5. **Nested Support**: Handle nested objects, arrays, and optional fields

## Decision

Integrate `schemars` (JSON Schema generation library) with a **safe converter** pattern:

### Architecture

```rust
// 1. User defines output type with JsonSchema derive
#[derive(Serialize, Deserialize, JsonSchema)]
struct CalculationOutput {
    /// Calculation result
    result: f64,
    /// Operation performed
    operation: String,
}

// 2. Macro generates converter code
impl HasOutputSchema for MyTool {
    fn output_schema(&self) -> Option<JsonSchema> {
        static SCHEMA: OnceLock<JsonSchema> = OnceLock::new();
        Some(SCHEMA.get_or_init(|| {
            // Generate schemars schema
            let schemars_schema = schemars::schema_for!(CalculationOutput);

            // Convert to MCP JsonSchema
            let json_value = serde_json::to_value(&schemars_schema).unwrap();
            convert_value_to_json_schema_with_defs(&json_value, &definitions)
        }))
    }
}
```

### Safe Converter Design

**File**: `crates/turul-mcp-builders/src/schemars_helpers.rs`

**Key Features**:
1. **$ref Resolution**: Handles both `#/$defs/` (JSON Schema 2020-12) and `#/definitions/` (draft-07)
2. **Type Array Extraction**: Converts `["string", "null"]` → `"string"` for Option types
3. **Recursive Conversion**: Handles nested objects and arrays properly
4. **Lossy-but-Safe Fallback**: Complex patterns fall back to generic `{type: "object"}` instead of panicking

**Example Conversion**:
```json
// Schemars generates (JSON Schema 2020-12):
{
  "type": "object",
  "properties": {
    "stats": { "$ref": "#/$defs/Statistics" }
  },
  "$defs": {
    "Statistics": {
      "type": "object",
      "properties": {
        "min": { "type": "number" },
        "max": { "type": "number" }
      }
    }
  }
}

// Converter produces (MCP JsonSchema):
{
  "type": "object",
  "properties": {
    "stats": {
      "type": "object",
      "properties": {
        "min": { "type": "number" },
        "max": { "type": "number" }
      }
    }
  }
}
```

### Optional Fields Pattern

For `Option<T>` fields, schemars generates `"type": ["string", "null"]`. To avoid validation errors when `None` is serialized as `null`, use:

```rust
#[derive(Serialize, Deserialize, JsonSchema)]
struct Output {
    required_field: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    optional_field: Option<String>,  // Omitted from JSON when None
}
```

## Consequences

### Positive

- ✅ **Auto-Sync**: Schemas automatically match Rust types
- ✅ **Type Safety**: Compiler enforces schema correctness
- ✅ **Doc Comments**: Schemars includes Rust doc comments in schemas
- ✅ **Less Boilerplate**: 3 lines vs 20+ for simple schemas
- ✅ **Nested Structures**: Handles nested objects and arrays automatically
- ✅ **Zero Breaking Changes**: Manual schemas still work

### Negative

- ❌ **$ref Fallback Not Tested**: Unresolvable references silently become generic objects
- ❌ **HashMap/BTreeMap Limitations**: May show as generic `{type: "object"}` without value type
- ⚠️ **Dependency**: Adds schemars to derive crate (compile-time only)
- ⚠️ **Complexity**: Conversion logic is non-trivial (200+ lines)

### Risks and Mitigations

**Risk**: Complex JSON Schema patterns not supported by converter

**Mitigation**:
1. Converter uses "lossy-but-safe" pattern - never panics
2. Falls back to generic object schema
3. Documented limitations in README and docs

**Risk**: Schema drift if converter has bugs

**Mitigation**:
1. Comprehensive test coverage (5 unit tests + 2 integration tests)
2. Examples demonstrate nested structures and optional fields
3. Tests verify tools/list and tools/call consistency

## Implementation

### Converter Functions

```rust
// Simple conversion (no $refs)
pub fn convert_value_to_json_schema(value: &Value) -> JsonSchema

// With $ref resolution (recommended)
pub fn convert_value_to_json_schema_with_defs(
    value: &Value,
    definitions: &HashMap<String, Value>
) -> JsonSchema
```

### Macro Code Generation

**File**: `crates/turul-mcp-derive/src/utils.rs:1209-1240`

```rust
// 1. Generate schemars schema
let schemars_schema = schema_for!(#ty);

// 2. Extract definitions for $ref resolution
let definitions = /* extract from schema_value["$defs"] or ["definitions"] */;

// 3. Convert with safe converter
let inner_schema = convert_value_to_json_schema_with_defs(&schema_value, &definitions);

// 4. Wrap in output field
JsonSchema::Object {
    properties: hashmap!{ #output_field_name => inner_schema },
    required: vec![#output_field_name],
    ...
}
```

### Test Coverage

**Unit Tests** (`crates/turul-mcp-derive/tests/schemars_integration_test.rs`):
- ✅ Flat structures (3 fields)
- ✅ Nested objects (2 levels deep)
- ✅ Arrays of objects
- ✅ Optional fields (type extraction)
- ✅ Optional field serialization (skip_serializing_if pattern)

**Integration Tests**:
- ✅ `custom_output_field_test.rs` - Custom output field names
- ✅ `tool-output-schemas` example - All patterns demonstrated

**Coverage Gaps** (documented in TODO_TRACKER.md):
- ❌ HashMap/BTreeMap fields not explicitly tested
- ❌ $ref fallback behavior not verified

## Alternatives Considered

### Alternative 1: Generic Type Introspection

Use Rust's `std::any::type_name()` to generate schemas at runtime.

**Rejected**: Cannot access field names, types, or doc comments at runtime without proc macros.

### Alternative 2: Manual-Only Approach

Keep only manual schema building.

**Rejected**: Too verbose for complex nested structures. Users requested automatic generation.

### Alternative 3: JSON Schema Draft-07 Only

Only support `#/definitions/` (draft-07), ignore `#/$defs/`.

**Rejected**: Schemars 1.0+ uses JSON Schema 2020-12 with `$defs`. Would break with current schemars versions.

## References

- **Schemars**: https://docs.rs/schemars/1.0.4/schemars/
- **JSON Schema 2020-12**: https://json-schema.org/draft/2020-12/release-notes
- **MCP Tool Output Schemas**: https://spec.modelcontextprotocol.io/specification/2025-06-18/tools/
- **Implementation**: `crates/turul-mcp-builders/src/schemars_helpers.rs`
- **Tests**: `crates/turul-mcp-derive/tests/schemars_integration_test.rs`
- **Example**: `examples/tool-output-schemas/`

## See Also

- [ADR-002: Compile-time Schema Generation](./002-compile-time-schema-generation.md) - Input schema generation
- [ADR-003: JsonSchema Standardization](./003-jsonschema-standardization.md) - Framework-wide schema types
- [Tool Output Schemas Example](../../examples/tool-output-schemas/) - Demonstrates all patterns
