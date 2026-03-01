# Prompt Pattern Reference — Turul MCP Framework

Deep-dive reference for all three prompt creation patterns. See the `resource-prompt-patterns` skill SKILL.md for the decision flowchart and quick-start examples.

## Derive Macro: `#[derive(McpPrompt)]`

### Struct-Level Attributes

```rust
#[derive(McpPrompt)]
#[prompt(
    name = "my_prompt",        // Required
    description = "..."        // Required
)]
struct MyPrompt {
    #[argument(description = "Argument description")]
    field_name: String,
}
```

### Field-Level Attributes

| Attribute | Required | Default |
|---|---|---|
| `#[argument(description = "...")]` | No | `"No description"` |

Field names become argument names. All field types are treated as strings in the MCP argument definition.

### What Is Generated

Derive generates **metadata traits only** (6 traits + HasIcons):
- `HasPromptMetadata`, `HasPromptDescription`, `HasPromptArguments`
- `HasPromptAnnotations`, `HasPromptMeta`, `HasIcons`

Arguments are lazily initialized in a static `OnceLock` (allocated once per runtime).

The blanket `PromptDefinition` impl is provided automatically.

### What Is NOT Generated

`McpPrompt::render()` — you must implement it manually:

```rust
#[async_trait]
impl McpPrompt for MyPrompt {
    async fn render(&self, args: Option<HashMap<String, Value>>)
        -> McpResult<Vec<PromptMessage>> {
        let field = args.as_ref()
            .and_then(|a| a.get("field_name"))
            .and_then(|v| v.as_str())
            .unwrap_or("default");
        Ok(vec![PromptMessage::user_text(format!("Process: {field}"))])
    }
}
```

### Session Access

`McpPrompt::render()` has **no session parameter**. For session-dependent prompts, use `OnceLock` or capture external state:

```rust
static SESSION_DATA: OnceLock<Arc<Mutex<HashMap<String, String>>>> = OnceLock::new();
```

### Registration

```rust
McpServer::builder().prompt(MyPrompt { field_name: String::new() })
```

---

## Declarative Macro: `prompt!{}`

### Syntax

```rust
let my_prompt = prompt! {
    name: "greet",
    description: "Greeting prompt",
    arguments: {
        name: String => "Person to greet", required
    },
    template: |args| async move {
        let name = args.as_ref()
            .and_then(|a| a.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or("World");
        Ok(vec![PromptMessage::user_text(format!("Hello, {name}!"))])
    }
};
```

### Fields

| Field | Type | Required |
|---|---|---|
| `name` | String literal | Yes |
| `title` | String literal | No |
| `description` | String literal | No |
| `arguments` | Block | No |
| `template` | Async closure | Yes |

### Arguments Block Syntax

```rust
arguments: {
    arg_name: Type => "Description", required    // Required argument
    other_arg: String => "Other description"     // Optional argument (no `required`)
}
```

**Type mapping:** `String`/`string` → "string", `i32`/`i64`/... → "integer", `f32`/`f64`/... → "number", `bool`/`boolean` → "boolean". Other types default to "string".

### Template Closure Signature

```rust
|args: Option<HashMap<String, Value>>| async move {
    -> Result<Vec<PromptMessage>, McpError>
}
```

### Generated Code

Creates a hidden `GeneratedPrompt` struct with all metadata traits AND `McpPrompt::render()` impl. Returns an initialized instance — no manual trait implementation needed.

Source: `crates/turul-mcp-derive/src/macros/prompt.rs`

### Registration

```rust
McpServer::builder().prompt(my_prompt)
```

---

## Builder: `PromptBuilder`

### Constructor & Chain

```rust
PromptBuilder::new("summarize")              // Name required
    .title("Summarize Text")                  // Optional display name
    .description("Summarize provided text")   // Optional
    .icons(vec![icon])                        // Optional Vec<Icon>
    .meta(hashmap)                            // Optional HashMap<String, Value>
```

### Argument Methods

```rust
.string_argument("text", "Text to summarize")         // Required string arg
.optional_string_argument("style", "Summary style")   // Optional string arg
.argument(PromptArgument::new("custom").with_description("Custom arg").required())
```

### Message Methods

**Static messages:**
```rust
.user_message("Fixed user message")
.assistant_message("Fixed assistant message")
.user_image(base64_data, "image/png")
.message(PromptMessage::user_text("Custom"))
```

**Template messages** (with `{arg_name}` placeholder replacement):
```rust
.template_user_message("Summarize in {style} style: {text}")
.template_assistant_message("Here is the {style} summary:")
```

**Dynamic generation** (overrides all messages/templates):
```rust
.get(|args: HashMap<String, String>| async move {
    let text = args.get("text").cloned().unwrap_or_default();
    Ok(GetPromptResult::new(vec![
        PromptMessage::user_text(format!("Summarize: {text}"))
    ]))
})
```

### Build & Register

```rust
let prompt = builder.build()?;   // Result<DynamicPrompt, String>
McpServer::builder().prompt(prompt)
```

### Template Processing

When using `.template_user_message()` / `.template_assistant_message()` without `.get()`, the builder creates a default template processor that:
1. Replaces `{arg_name}` placeholders with argument values
2. Passes image content through unchanged
3. Returns `GetPromptResult` with the processed messages

---

## PromptMessage Constructors

| Constructor | Parameters | Role |
|---|---|---|
| `PromptMessage::user_text(text)` | `impl Into<String>` | User |
| `PromptMessage::assistant_text(text)` | `impl Into<String>` | Assistant |
| `PromptMessage::user_image(data, mime)` | `(impl Into<String>, impl Into<String>)` | User |
| `PromptMessage::text(text)` | `impl Into<String>` | User (default) |

**Important:** MCP has only `User` and `Assistant` roles. There is no `System` role in the protocol.

---

## PromptArgument API

```rust
PromptArgument::new("arg_name")
    .with_description("Description text")
    .with_title("Display Title")     // Optional
    .required()                       // Sets required: Some(true)
    .optional()                       // Sets required: Some(false)
```

Fields: `name: String`, `title: Option<String>`, `description: Option<String>`, `required: Option<bool>`.

---

## GetPromptResult

```rust
GetPromptResult::new(vec![
    PromptMessage::user_text("message"),
])
.with_description("Optional result description")
.with_meta(hashmap)
```

---

## Framework Traits

Fine-grained traits (all auto-implemented by macros/derive/builder):

| Trait | Key Method | Notes |
|---|---|---|
| `HasPromptMetadata` | `name() -> &str` | Required; also `title() -> Option<&str>` |
| `HasPromptDescription` | `description() -> Option<&str>` | |
| `HasPromptArguments` | `arguments() -> Option<&Vec<PromptArgument>>` | |
| `HasPromptAnnotations` | `annotations() -> Option<&PromptAnnotations>` | |
| `HasPromptMeta` | `prompt_meta() -> Option<&HashMap<String, Value>>` | |
| `HasIcons` | `icons() -> Option<&Vec<Icon>>` | |

Blanket `PromptDefinition` impl provides:
- `display_name() -> &str` — returns `title()` if present, else `name()`
- `to_prompt() -> Prompt` — converts to MCP protocol `Prompt` struct
