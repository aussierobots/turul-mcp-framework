# Builder Pattern Guide — `ToolBuilder`

The builder pattern constructs MCP tools at runtime. Use it when tool definitions are not known at compile time — dynamic tools, plugin systems, or configuration-driven servers.

## Basic Usage

```rust
// turul-mcp-server v0.3.0
use serde_json::json;
use turul_mcp_server::ToolBuilder;

let tool = ToolBuilder::new("add")
    .description("Add two numbers")
    .number_param("a", "First number")
    .number_param("b", "Second number")
    .number_output()
    .execute(|args| async move {
        let a = args.get("a").and_then(|v| v.as_f64())
            .ok_or("Missing parameter 'a'")?;
        let b = args.get("b").and_then(|v| v.as_f64())
            .ok_or("Missing parameter 'b'")?;
        Ok(json!({"result": a + b}))
    })
    .build()
    .map_err(|e| format!("Failed to build tool: {}", e))?;
```

## Builder Methods

### Tool Identity

| Method | Description |
|---|---|
| `ToolBuilder::new(name)` | Create builder with tool name |
| `.description(desc)` | Set human-readable description |

### Parameter Definition

| Method | JSON Schema Type | Description |
|---|---|---|
| `.string_param(name, desc)` | `string` | Add a string parameter |
| `.number_param(name, desc)` | `number` | Add a number parameter |
| `.boolean_param(name, desc)` | `boolean` | Add a boolean parameter |
| `.integer_param(name, desc)` | `integer` | Add an integer parameter |
| `.optional_string_param(name, desc)` | `string` (optional) | Non-required string |
| `.optional_number_param(name, desc)` | `number` (optional) | Non-required number |

### Output Schema

| Method | Output Schema |
|---|---|
| `.number_output()` | `{"result": number}` |
| `.string_output()` | `{"result": string}` |
| `.boolean_output()` | `{"result": boolean}` |
| `.object_output()` | `{"result": object}` |
| `.custom_output_schema(schema)` | Custom JSON schema |

### Task Support

| Method | Description |
|---|---|
| `.execution(ToolExecution)` | Set per-tool task support configuration |

### Execution

| Method | Description |
|---|---|
| `.execute(closure)` | Set the execution closure |
| `.build()` | Finalize and return the tool |

## Parameter Extraction

Unlike macros, builder tools extract parameters manually from `serde_json::Map`:

```rust
.execute(|args| async move {
    // Required parameters — return error if missing
    let name = args.get("name")
        .and_then(|v| v.as_str())
        .ok_or("Missing parameter 'name'")?;

    // Optional parameters — use default
    let verbose = args.get("verbose")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // Number parameters — always use as_f64()
    let count = args.get("count")
        .and_then(|v| v.as_f64())
        .ok_or("Missing parameter 'count'")?;

    Ok(json!({"result": format!("Hello, {name}")}))
})
```

**Important:** For number fields, always use `as_f64()`. The MCP protocol transmits all numbers as JSON floats — `as_u64()` returns `None` for float values.

## Custom Output Schemas

For complex output types, define the schema manually:

```rust
use serde_json::json;

let schema = json!({
    "type": "object",
    "properties": {
        "items": {
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "name": {"type": "string"},
                    "score": {"type": "number"}
                },
                "required": ["name", "score"]
            }
        },
        "total": {"type": "integer"}
    },
    "required": ["items", "total"]
});

let tool = ToolBuilder::new("search")
    .description("Search items")
    .string_param("query", "Search query")
    .custom_output_schema(schema)
    .execute(|args| async move {
        // ...
        Ok(json!({"items": [], "total": 0}))
    })
    .build()?;
```

## Registration

Builder tools use `.tool()`:

```rust
let server = McpServer::builder()
    .name("my-server")
    .tool(add_tool)
    .tool(search_tool)
    .build()?;
```

## Dynamic Tool Loading Example

```rust
// Load tool definitions from a config file
let config: Vec<ToolConfig> = load_config("tools.json")?;

let mut builder = McpServer::builder().name("dynamic-server");

for tool_def in config {
    let tool = ToolBuilder::new(&tool_def.name)
        .description(&tool_def.description)
        .string_param("input", "Tool input")
        .string_output()
        .execute(move |args| {
            let def = tool_def.clone();
            async move {
                // Dynamic execution based on config
                Ok(json!({"result": "processed"}))
            }
        })
        .build()?;

    builder = builder.tool(tool);
}

let server = builder.build()?;
```

## Error Handling

Builder execute closures return `Result<Value, E>` where `E: Into<String>`:

```rust
.execute(|args| async move {
    let input = args.get("input")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'input'")?;  // String error, auto-converted

    if input.is_empty() {
        return Err("Input cannot be empty".into());
    }

    Ok(json!({"result": input.to_uppercase()}))
})
```

The framework wraps these into `McpError::ToolExecutionError` automatically. See: [CLAUDE.md — Critical Error Handling Rules](https://github.com/aussierobots/turul-mcp-framework/blob/main/CLAUDE.md#critical-error-handling-rules)

## Complete Example

See: `examples/builder-tool.rs` in this skill, or the full server at [`examples/calculator-add-builder-server/src/main.rs`](https://github.com/aussierobots/turul-mcp-framework/blob/main/examples/calculator-add-builder-server/src/main.rs) in the framework repository.

## Task Support

Declare task support via `.execution()`:

```rust
use turul_mcp_protocol::tools::{ToolExecution, TaskSupport};

let tool = ToolBuilder::new("slow_process")
    .description("Long-running process")
    .string_param("input", "Data to process")
    .string_output()
    .execution(ToolExecution { task_support: Some(TaskSupport::Optional) })
    .execute(|args| async move {
        // Long-running work...
        Ok(json!({"result": "done"}))
    })
    .build()?;
```

**Values:** `TaskSupport::Optional`, `TaskSupport::Required`, `TaskSupport::Forbidden`.

**Server requirement:** The server must have `.with_task_storage()` configured for tools with task support.

## Trade-offs vs Macros

| Aspect | Builder | Macros |
|---|---|---|
| Type safety | Manual (runtime errors) | Compile-time |
| Boilerplate | More (parameter extraction) | Less |
| Flexibility | Can construct tools at runtime | Fixed at compile time |
| Session | No direct access | Available (derive only) |
| Task support | `.execution()` | `task_support = "..."` |
| Best for | Plugin systems, dynamic config | Application tools |

Use macros (Level 1 or 2) unless you specifically need runtime flexibility.
