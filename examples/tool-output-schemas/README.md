# Tool Output Schemas Example

Demonstrates automatic schema generation using `schemars` derive macros.

## What This Example Shows

1. **Flat Structures** - Simple output types with basic fields
2. **Nested Objects** - Complex types with nested structs
3. **Arrays of Objects** - Vec<T> with detailed item schemas
4. **Optional Fields** - Proper handling of Option<T> types

## Running the Example

```bash
cargo run --package tool-output-schemas
```

The server provides three tools:
- `calculator_derive` - Flat structure (value, operation)
- `calculator_function` - Function macro example
- `analyze_data` - Nested structure with arrays and optional fields

## Output Schema Examples

### Flat Structure (calculator_derive)

```json
{
  "outputSchema": {
    "type": "object",
    "properties": {
      "result": {
        "type": "object",
        "properties": {
          "value": { "type": "number" },
          "operation": { "type": "string" }
        },
        "required": ["value", "operation"]
      }
    }
  }
}
```

### Nested Structure (analyze_data)

```json
{
  "outputSchema": {
    "type": "object",
    "properties": {
      "result": {
        "type": "object",
        "properties": {
          "dataset": { "type": "string" },
          "stats": {
            "type": "object",
            "properties": {
              "min": { "type": "number" },
              "max": { "type": "number" },
              "mean": { "type": "number" },
              "count": { "type": "integer" }
            }
          },
          "points": {
            "type": "array",
            "items": {
              "type": "object",
              "properties": {
                "timestamp": { "type": "string" },
                "value": { "type": "number" }
              }
            }
          }
        }
      }
    }
  }
}
```

## Optional Fields Pattern

For `Option<T>` fields, use `skip_serializing_if` to omit None values:

```rust
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct Output {
    pub required_field: String,

    // ✅ CORRECT: Omitted when None, not serialized as null
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optional_field: Option<String>,
}
```

## Known Limitations

- **HashMap/BTreeMap**: May show as generic `{type: "object"}` without value type details
- **Complex $refs**: Unresolvable references fall back to generic object schema
- **Deep Nesting**: Tested to 2 levels; deeper nesting may have edge cases

## See Also

- [ADR-014: Schemars Schema Generation](../../docs/adr/014-schemars-schema-generation.md) - Architecture decision
- [Schemars Integration Tests](../../crates/turul-mcp-derive/tests/schemars_integration_test.rs) - Comprehensive test coverage
