# Schemars Integration

The Turul MCP Framework uses [schemars](https://docs.rs/schemars) to generate detailed JSON schemas for tool outputs. When your output type derives `schemars::JsonSchema`, the framework auto-detects it and produces a rich schema with property descriptions, nested objects, arrays, and optional field handling.

## Setup

Add schemars to your `Cargo.toml`:

```toml
[dependencies]
schemars = "0.8"
```

## Deriving JsonSchema

```rust
use schemars::JsonSchema;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WeatherReport {
    /// Current temperature in Celsius
    pub temperature: f64,
    /// Weather condition description
    pub condition: String,
    /// Humidity percentage (0-100)
    pub humidity: u32,
}
```

Doc comments (`///`) on fields become `description` in the JSON schema. This is the primary way to document your output fields.

## Nested Types

Schemars handles nested types automatically. All nested types must also derive `JsonSchema`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Statistics {
    pub min: f64,
    pub max: f64,
    pub mean: f64,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DataPoint {
    pub timestamp: String,
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AnalysisResult {
    /// Name of the dataset analyzed
    pub dataset: String,
    /// Summary statistics (nested object)
    pub stats: Statistics,
    /// Individual data points (array of objects)
    pub points: Vec<DataPoint>,
}
```

The generated schema resolves `$ref` references inline, producing a self-contained schema that MCP clients can consume without reference resolution.

## Optional Fields

Use `Option<T>` with `skip_serializing_if` for optional output fields:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchResult {
    pub title: String,
    pub score: f64,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub snippet: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
}
```

The schema marks these fields as not required. The `skip_serializing_if` annotation ensures `null` values are omitted entirely from the JSON output (not serialized as `"snippet": null`).

## Enum Outputs

Schemars supports enums with multiple variants:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type")]
pub enum ConversionResult {
    Success { value: f64, unit: String },
    Error { message: String },
}
```

The `#[serde(tag = "type")]` produces an internally-tagged enum in the schema.

## camelCase Field Names

MCP requires camelCase in JSON. Use `#[serde(rename)]` for fields with underscores:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ServerInfo {
    #[serde(rename = "serverName")]
    pub server_name: String,

    #[serde(rename = "isRunning")]
    pub is_running: bool,

    #[serde(rename = "uptimeSeconds")]
    pub uptime_seconds: u64,
}
```

See: [CLAUDE.md — JSON Naming: camelCase ONLY](https://github.com/aussierobots/turul-mcp-framework/blob/main/CLAUDE.md#json-naming-camelcase-only)

## Compile-Time Errors

If you forget to derive `JsonSchema` on a type used in `output = Type`:

```
error[E0277]: the trait bound `MyType: JsonSchema` is not satisfied
```

Fix: Add `schemars::JsonSchema` to the derive list on `MyType` and all nested types.

## Testing Schemas

Verify your schemas are correct:

```rust
// In a test
use schemars::schema_for;

#[test]
fn test_schema() {
    let schema = schema_for!(WeatherReport);
    let json = serde_json::to_string_pretty(&schema).unwrap();
    println!("{}", json);
    // Verify it contains expected properties
    assert!(json.contains("temperature"));
    assert!(json.contains("condition"));
}
```

For release validation, the framework requires:
- `cargo test -p turul-mcp-derive schemars_integration_test`
- `cargo test --test schema_tests mcp_vec_result_schema_test`

See: [AGENTS.md — Release Readiness Notes](https://github.com/aussierobots/turul-mcp-framework/blob/main/AGENTS.md#release-readiness-notes-2025-10-01)
