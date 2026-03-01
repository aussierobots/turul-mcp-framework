# Resource Pattern Reference — Turul MCP Framework

Deep-dive reference for all four resource creation patterns. See the `resource-prompt-patterns` skill SKILL.md for the decision flowchart and quick-start examples.

## Function Macro: `#[mcp_resource]`

### Attributes

| Attribute | Type | Required | Default |
|---|---|---|---|
| `uri` | String literal | Yes | — |
| `name` | String literal | No | Function name |
| `description` | String literal | No | `"Resource: {name}"` |
| `mime_type` | String literal | No | None |

### URI Template Variables

Template variables in the URI (`{id}`) are extracted from `params["template_variables"]["id"]`. The function macro auto-generates extraction code for typed parameters:

```rust
// turul-mcp-server v0.3
#[mcp_resource(uri = "file:///data/{id}.json", name = "data_item")]
async fn data_item(id: String) -> McpResult<Vec<ResourceContent>> {
    // `id` is auto-extracted from params.template_variables.id
    Ok(vec![ResourceContent::text(&format!("file:///data/{id}.json"), load(id).await?)])
}
```

For raw access to the full params object, use `params: Value`:

```rust
#[mcp_resource(uri = "file:///data/{id}.json", name = "data_item")]
async fn data_item(params: Value) -> McpResult<Vec<ResourceContent>> {
    let id = params.get("template_variables")
        .and_then(|tv| tv.get("id"))
        .and_then(|v| v.as_str())
        .ok_or(McpError::invalid_params("Missing id"))?;
    // ...
}
```

### Session Access

Add `session: Option<&SessionContext>` as the second parameter:

```rust
#[mcp_resource(uri = "file:///profile.json", name = "profile")]
async fn profile(params: Value, session: Option<&SessionContext>) -> McpResult<Vec<ResourceContent>> {
    if let Some(s) = session {
        let user: String = s.get_typed_state("user_id").await.unwrap_or_default();
        // ...
    }
    // ...
}
```

### Generated Code

The macro creates a struct `{FunctionName}ResourceImpl` and a constructor function that returns it. It implements all 8 metadata traits plus `McpResource` (with the user function body as `read()`).

### Registration

```rust
McpServer::builder().resource_fn(data_item)   // .resource_fn() required
```

---

## Derive Macro: `#[derive(McpResource)]`

### Struct-Level Attributes

```rust
#[derive(McpResource, Clone)]
#[resource(
    name = "my_resource",      // Required
    uri = "file:///data.json", // Required (supports {template} vars)
    description = "...",       // Required
    mime_type = "text/plain"   // Optional
)]
struct MyResource;
```

### What Is Generated

Derive generates **metadata traits only** (8 traits):
- `HasResourceMetadata`, `HasResourceDescription`, `HasResourceUri`, `HasResourceMimeType`
- `HasResourceSize`, `HasResourceAnnotations`, `HasResourceMeta`, `HasIcons`

The blanket `ResourceDefinition` impl is provided automatically.

### What Is NOT Generated

`McpResource::read()` — you must implement it manually:

```rust
#[async_trait]
impl McpResource for MyResource {
    async fn read(&self, params: Option<Value>, session: Option<&SessionContext>)
        -> McpResult<Vec<ResourceContent>> {
        // Your implementation here
        Ok(vec![ResourceContent::text("file:///data.json", "content".to_string())])
    }
}
```

### Field Attributes (Caveat)

`#[content]` and `#[content_type]` are accepted by the parser but are **NOT consumed for code generation**. They do not auto-generate `read()` logic. Always implement `read()` explicitly.

### Registration

```rust
McpServer::builder().resource(MyResource)   // .resource() for derive
```

---

## Declarative Macro: `resource!{}`

### Syntax

```rust
let my_resource = resource! {
    uri: "file:///data.json",
    name: "data",
    description: "Data resource",
    content: |params, session| async move {
        Ok(vec![ResourceContent::text("file:///data.json", "content".to_string())])
    }
};
```

### Fields

| Field | Type | Required |
|---|---|---|
| `uri` | String literal | Yes |
| `name` | String literal | Yes |
| `description` | String literal | Yes |
| `content` | Async closure | Yes |

### Closure Signature

```rust
|params: Option<Value>, session: Option<&SessionContext>| async move {
    -> Result<Vec<ResourceContent>, McpError>
}
```

### Generated Code

Creates a hidden struct `{CapitalizedName}Resource` with all 8 metadata traits AND `McpResource` impl. Returns an initialized instance directly — no manual trait implementation needed.

Source: `crates/turul-mcp-derive/src/macros/resource.rs`

### Registration

```rust
McpServer::builder().resource(my_resource)   // .resource()
```

---

## Builder: `ResourceBuilder`

### Constructor & Chain

```rust
ResourceBuilder::new("file:///status.json")  // URI required
    .name("status")                           // Optional (auto-extracted from URI)
    .title("System Status")                   // Optional display name
    .description("Current system status")     // Optional
    .mime_type("application/json")            // Optional
    .size(1024)                               // Optional (u64)
    .annotations(annotations)                 // Optional Annotations struct
    .annotation_audience(vec!["admin".into()]) // Convenience
    .annotation_priority(0.8)                 // Convenience (f64)
    .icons(vec![icon])                        // Optional Vec<Icon>
    .meta(hashmap)                            // Optional HashMap<String, Value>
```

### Static Content

```rust
.text_content("plain text content")
.json_content(serde_json::json!({"key": "value"}))
.blob_content("base64data...", "image/png")
```

### Dynamic Content

```rust
// Returns ResourceContent directly
.read(|uri: String| async move {
    Ok(ResourceContent::text(&uri, fetch_content(&uri).await))
})

// Returns String, auto-wrapped in ResourceContent::text
.read_text(|uri: String| async move {
    Ok(fetch_text(&uri).await)
})
```

### Build & Register

```rust
let resource = builder.build()?;   // Result<DynamicResource, String>
McpServer::builder().resource(resource)
```

---

## ResourceContent Constructors

| Constructor | Parameters | Content Type |
|---|---|---|
| `ResourceContent::text(uri, text)` | `(&str, String)` | Plain text |
| `ResourceContent::blob(uri, data, mime)` | `(&str, String, String)` | Base64-encoded binary |

A single `read()` can return multiple `ResourceContent` items in the `Vec`.

---

## Framework Traits

Fine-grained traits (all auto-implemented by macros/derive/builder):

| Trait | Key Method | Notes |
|---|---|---|
| `HasResourceMetadata` | `name() -> &str` | Required; also `title() -> Option<&str>` |
| `HasResourceDescription` | `description() -> Option<&str>` | |
| `HasResourceUri` | `uri() -> &str` | Required |
| `HasResourceMimeType` | `mime_type() -> Option<&str>` | |
| `HasResourceSize` | `size() -> Option<u64>` | |
| `HasResourceAnnotations` | `annotations() -> Option<&Annotations>` | |
| `HasResourceMeta` | `resource_meta() -> Option<&HashMap<String, Value>>` | |
| `HasIcons` | `icons() -> Option<&Vec<Icon>>` | |

Blanket `ResourceDefinition` impl provides:
- `display_name() -> &str` — returns `title()` if present, else `name()`
- `to_resource() -> Resource` — converts to MCP protocol `Resource` struct
