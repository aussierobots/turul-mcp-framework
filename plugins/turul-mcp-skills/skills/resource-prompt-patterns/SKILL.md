---
name: resource-prompt-patterns
description: >
  This skill should be used when the user asks to "create a resource",
  "add a resource", "MCP resource", "McpResource", "mcp_resource macro",
  "#[derive(McpResource)]", "ResourceBuilder", "resource URI",
  "URI template", "ResourceContent", "dynamic resource", "resource!",
  "create a prompt", "add a prompt", "MCP prompt", "McpPrompt",
  "#[derive(McpPrompt)]", "PromptBuilder", "prompt arguments",
  "PromptMessage", "prompt template", "GetPromptResult", "prompt!",
  "resources/read", "prompts/get", or "resource vs prompt".
  Covers creating MCP resources (function macro, derive, resource!{},
  builder) and MCP prompts (derive, prompt!{}, builder) in the Turul
  MCP Framework (Rust).
---

# Resource & Prompt Patterns — Turul MCP Framework

Resources and prompts are two of the three core MCP primitives (alongside tools). Resources expose **data for clients to read**; prompts expose **conversational templates with arguments**.

## When to Use Resources vs Prompts

```
What are you providing?
├─ Data that clients READ (files, configs, API responses) ──→ Resource
└─ Conversational templates with ARGUMENTS ─────────────────→ Prompt
```

---

## Resource Decision Flowchart

The framework provides four approaches to creating MCP resources:

```
Need a resource?
├─ Definitions known at compile time? ───→ Use macros
│   ├─ Need session context or complex read logic? ──→ Derive (#[derive(McpResource)]) + manual impl
│   ├─ Inline one-off with closure body? ────────────→ Declarative (resource!{})
│   └─ Otherwise ────────────────────────────────────→ Function Macro (#[mcp_resource])  ← DEFAULT
└─ Resources loaded from config/DB at runtime? ──────→ Builder (ResourceBuilder)
```

### Pattern 1: Function Macro `#[mcp_resource]` (Start Here)

**Best for:** Most resources. Typed parameters, auto-generated `McpResource` impl.

```rust
// turul-mcp-server v0.3
use turul_mcp_derive::mcp_resource;
use turul_mcp_server::prelude::*;

#[mcp_resource(
    uri = "file:///logs/{service}.log",
    name = "service_log",
    description = "Recent log entries for a service",
    mime_type = "text/plain"
)]
async fn service_log(service: String) -> McpResult<Vec<ResourceContent>> {
    let content = read_log_file(&service).await?;
    Ok(vec![ResourceContent::text(
        &format!("file:///logs/{service}.log"),
        content,
    )])
}

let server = McpServer::builder()
    .resource_fn(service_log)   // Note: .resource_fn() for function macros
    .build()?;
```

**Key points:**
- URI template variables (`{service}`) become function parameters automatically
- `uri` is required; `name`, `description`, `mime_type` are optional
- Register with `.resource_fn()` (NOT `.resource()`)
- Session access available — add `session: Option<&SessionContext>` as second parameter

**See:** `references/resource-guide.md` for full attribute reference.

### Pattern 2: Derive Macro `#[derive(McpResource)]`

**Best for:** Resources needing session access or complex read logic with a named struct.

```rust
// turul-mcp-server v0.3
use turul_mcp_derive::McpResource;
use turul_mcp_server::prelude::*;

#[derive(McpResource, Clone)]
#[resource(name = "profile", uri = "file:///users/{id}.json", description = "User profile")]
struct ProfileResource;

#[async_trait]
impl McpResource for ProfileResource {
    async fn read(&self, params: Option<Value>, session: Option<&SessionContext>)
        -> McpResult<Vec<ResourceContent>> {
        let id = params.as_ref()
            .and_then(|p| p.get("template_variables"))
            .and_then(|tv| tv.get("id"))
            .and_then(|v| v.as_str())
            .ok_or(McpError::invalid_params("Missing id"))?;
        Ok(vec![ResourceContent::text(
            &format!("file:///users/{id}.json"),
            get_profile(id).await?,
        )])
    }
}

let server = McpServer::builder()
    .resource(ProfileResource)   // Note: .resource() for derive
    .build()?;
```

**Key points:**
- Derive generates metadata traits ONLY — you MUST implement `McpResource::read()` manually
- `name`, `uri`, `description` are required struct-level attributes; `mime_type` is optional
- `#[content]`/`#[content_type]` field attributes are accepted syntactically but NOT used for code generation — always implement `read()` explicitly
- Session access is available in the manual `read()` impl

### Pattern 3: Declarative Macro `resource!{}`

**Best for:** Inline one-off resources with closure body. Generates full struct + all traits + `McpResource` impl.

```rust
// turul-mcp-server v0.3
use turul_mcp_server::prelude::*;

let config = resource! {
    uri: "file:///config.json",
    name: "config",
    description: "Application configuration",
    content: |_params, _session| async move {
        Ok(vec![ResourceContent::text("file:///config.json", load_config().await)])
    }
};

let server = McpServer::builder()
    .resource(config)   // .resource() for declarative macro
    .build()?;
```

**Key points:**
- `content:` takes `|params: Option<Value>, session: Option<&SessionContext>| async { ... }`
- All four fields (`uri`, `name`, `description`, `content`) are required
- Generates a hidden struct with all trait impls — no manual implementation needed
- Register with `.resource()`

### Pattern 4: Builder `ResourceBuilder`

**Best for:** Resources whose definitions are unknown at compile time (loaded from config/DB).

```rust
// turul-mcp-server v0.3
use turul_mcp_server::prelude::*;

let resource = ResourceBuilder::new("file:///status.json")
    .description("System status")
    .mime_type("application/json")
    .read_text(|uri| async move {
        Ok(get_status_json())
    })
    .build()?;

let server = McpServer::builder()
    .resource(resource)
    .build()?;
```

**Key points:**
- `.text_content()` / `.json_content()` / `.blob_content()` for static content
- `.read()` / `.read_text()` for dynamic callbacks (receive URI string)
- Name auto-extracted from URI if not set with `.name()`
- Register with `.resource()`

### ResourceContent Constructors

```rust
ResourceContent::text(uri, text)              // Plain text
ResourceContent::blob(uri, base64_data, mime) // Binary (base64-encoded)
```

Multiple content items can be returned from a single `read()` call.

---

## Prompt Decision Flowchart

The framework provides three approaches to creating MCP prompts:

```
Need a prompt?
├─ Definition known at compile time? ───→ Use macros
│   ├─ Need custom struct with fields? ──→ Derive (#[derive(McpPrompt)]) + manual render()
│   └─ Inline one-off with closure? ─────→ Declarative (prompt!{})
└─ Prompts loaded at runtime? ───────────→ Builder (PromptBuilder)

Note: There is no #[mcp_prompt] function macro (unlike tools and resources).
```

### Pattern 1: Derive Macro `#[derive(McpPrompt)]`

**Best for:** Prompts with typed fields that become MCP arguments.

```rust
// turul-mcp-server v0.3
use turul_mcp_derive::McpPrompt;
use turul_mcp_server::prelude::*;

#[derive(McpPrompt)]
#[prompt(name = "code_review", description = "Review code for issues")]
struct CodeReviewPrompt {
    #[argument(description = "Programming language")]
    language: String,
    #[argument(description = "Code to review")]
    code: String,
}

#[async_trait]
impl McpPrompt for CodeReviewPrompt {
    async fn render(&self, args: Option<HashMap<String, Value>>)
        -> McpResult<Vec<PromptMessage>> {
        let lang = args.as_ref()
            .and_then(|a| a.get("language"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let code = args.as_ref()
            .and_then(|a| a.get("code"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        Ok(vec![PromptMessage::user_text(
            format!("Review this {lang} code for bugs and improvements:\n\n```{lang}\n{code}\n```")
        )])
    }
}

let server = McpServer::builder()
    .prompt(CodeReviewPrompt { language: String::new(), code: String::new() })
    .build()?;
```

**Key points:**
- `#[argument(description = "...")]` on fields generates `PromptArgument` entries
- Derive generates metadata traits ONLY — you MUST implement `McpPrompt::render()` manually
- `render()` signature: `(&self, args: Option<HashMap<String, Value>>) -> McpResult<Vec<PromptMessage>>` — **no session parameter**
- For session-dependent prompts, use `OnceLock` or capture external state

### Pattern 2: Declarative Macro `prompt!{}`

**Best for:** Inline one-off prompts with closure body. Generates full struct + all traits + `McpPrompt` impl.

```rust
// turul-mcp-server v0.3
use turul_mcp_server::prelude::*;

let greet = prompt! {
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

let server = McpServer::builder()
    .prompt(greet)
    .build()?;
```

**Key points:**
- `arguments:` block syntax: `name: Type => "description"` with optional `required` flag
- `template:` takes `|args: Option<HashMap<String, Value>>| async { ... }`
- Generates a hidden struct with all trait impls including `McpPrompt` — no manual implementation
- `name` and `template` are required; `description`, `title`, `arguments` are optional

### Pattern 3: Builder `PromptBuilder`

**Best for:** Runtime-defined prompts with built-in template processing.

```rust
// turul-mcp-server v0.3
use turul_mcp_server::prelude::*;

let prompt = PromptBuilder::new("summarize")
    .description("Summarize text")
    .string_argument("text", "Text to summarize")
    .optional_string_argument("style", "Summary style")
    .template_user_message("Summarize in {style} style:\n\n{text}")
    .build()?;

let server = McpServer::builder()
    .prompt(prompt)
    .build()?;
```

**Key points:**
- `{arg_name}` placeholders auto-replaced in template messages
- `.string_argument()` creates required arg; `.optional_string_argument()` creates optional
- `.get(callback)` for fully dynamic generation (receives `HashMap<String, String>`, returns `GetPromptResult`)
- `.user_message()` / `.assistant_message()` for static messages; `.template_user_message()` / `.template_assistant_message()` for templates

### PromptMessage Constructors

```rust
PromptMessage::user_text("content")       // User role
PromptMessage::assistant_text("content")  // Assistant role
PromptMessage::user_image(data, mime)     // User image (base64)
```

**Important:** MCP has only `User` and `Assistant` roles — there is no `System` role.

---

## Quick Comparison

| Feature | Resource fn macro | Resource derive | `resource!{}` | Resource builder | Prompt derive | `prompt!{}` | Prompt builder |
|---|---|---|---|---|---|---|---|
| Compile-time | Yes | Yes | Yes | No | Yes | Yes | No |
| Trait auto-gen | Full | Metadata only | Full | N/A | Metadata only | Full | N/A |
| Session access | Yes | Yes (manual) | Yes (closure) | No | **No** | **No** | No |
| Registration | `.resource_fn()` | `.resource()` | `.resource()` | `.resource()` | `.prompt()` | `.prompt()` | `.prompt()` |
| Best for | Default | Session + complex | Inline one-off | Runtime-defined | Struct w/ fields | Inline one-off | Runtime-defined |

## Common Mistakes

1. **Using `ResourceBuilder` when `#[mcp_resource]` suffices** — builder is for runtime-defined resources only
2. **Expecting `#[mcp_prompt]` function macro to exist** — prompts have derive, `prompt!{}`, and builder, but no function macro
3. **Expecting `#[derive(McpResource)]` to auto-generate `read()`** — derive generates metadata traits only; implement `read()` manually (use `resource!{}` if you want auto-generation)
4. **Same for `#[derive(McpPrompt)]`** — must implement `McpPrompt::render()` manually (use `prompt!{}` if you want auto-generation)
5. **Forgetting `.resource_fn()` vs `.resource()`** — function macros use `_fn` variant; all other patterns use `.resource()` / `.prompt()`
6. **Using `Role::System` in `PromptMessage`** — MCP spec has only `User` and `Assistant`
7. **Expecting session access in `McpPrompt::render()`** — `render()` takes `args` only, no session parameter; use `OnceLock` for session-dependent logic
8. **Treating `#[content]`/`#[content_type]` as functional on derive** — these are accepted syntactically but do not generate runtime content; always implement `read()` explicitly

## Beyond This Skill

**Tool creation?** → See the `tool-creation-patterns` skill.

**Output schemas, schemars, structuredContent?** → See the `output-schemas` skill.

**Client-side resource/prompt calls?** → See the `mcp-client-patterns` skill.

**Session state?** Use `session.get_typed_state(key).await` / `session.set_typed_state(key, value).await?`. See: [CLAUDE.md — API Conventions](https://github.com/aussierobots/turul-mcp-framework/blob/main/CLAUDE.md#api-conventions)

**Error handling?** Return `McpResult<T>` (alias for `Result<T, McpError>`). Never create `JsonRpcError` in handlers. See: [CLAUDE.md — Critical Error Handling Rules](https://github.com/aussierobots/turul-mcp-framework/blob/main/CLAUDE.md#critical-error-handling-rules)
