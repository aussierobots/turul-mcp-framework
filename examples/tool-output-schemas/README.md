# Tool Output Schemas Example

This example demonstrates using the optional `schemars` feature to auto-generate tool output schemas.

## Usage

**Manual Schema Approach:**
```rust
use turul_mcp_protocol::{ToolSchema, schema::JsonSchema};
use std::sync::OnceLock;
use std::collections::HashMap;

impl HasOutputSchema for MyTool {
    fn output_schema(&self) -> Option<&ToolSchema> {
        static SCHEMA: OnceLock<ToolSchema> = OnceLock::new();
        Some(SCHEMA.get_or_init(|| {
            ToolSchema {
                schema_type: "object".to_string(),
                properties: Some({
                    let mut props = HashMap::new();
                    props.insert(
                        "result".to_string(),
                        JsonSchema::number().with_description("Calculation result".to_string()),
                    );
                    props
                }),
                required: Some(vec!["result".to_string()]),
                additional: HashMap::new(),
            }
        }))
    }
}
```

**Schemars Approach (requires `--features schemars`):**
```rust
use schemars::{JsonSchema, schema_for};
use turul_mcp_builders::ToolSchemaExt;

#[derive(Serialize, JsonSchema)]
struct MyOutput {
    /// Result of calculation
    result: f64,
}

impl HasOutputSchema for MyTool {
    fn output_schema(&self) -> Option<&ToolSchema> {
        static SCHEMA: OnceLock<ToolSchema> = OnceLock::new();
        Some(SCHEMA.get_or_init(|| {
            let json_schema = schema_for!(MyOutput);
            ToolSchema::from_schemars(json_schema).expect("Valid schema")
        }))
    }
}
```

## Benefits

- **Manual**: Full control, works always
- **Schemars**: Auto-syncs with Rust types, includes doc comments

## Limitations

- Complex schemas with `anyOf`/`oneOf` (from `Option` types) may not convert
- Keep output schemas simple for best results
