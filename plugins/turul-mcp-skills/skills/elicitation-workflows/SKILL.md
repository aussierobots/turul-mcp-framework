---
name: elicitation-workflows
description: >
  This skill should be used when the user asks about "elicitation",
  "ElicitationBuilder", "elicit", "ElicitResult", "ElicitAction",
  "elicitation/create", "ElicitationProvider", "PrimitiveSchemaDefinition",
  "ElicitationSchema", "schema-driven form", "user input form",
  "with_elicitation", "with_elicitation_provider", "DynamicElicitation",
  "ElicitResultBuilder", "elicitation validation", or "multi-step elicitation".
  Covers MCP elicitation for collecting structured user input via
  primitive schemas in the Turul MCP Framework (Rust).
---

# Elicitation Workflows — Turul MCP Framework

Elicitation lets the server request structured input from the user/client. The server sends a schema-driven form; the client presents it and returns the response. This is an MCP 2025-11-25 feature — schemas are restricted to **primitive types only** (no nesting).

## When to Use Elicitation

```
Need user input during tool execution?
├─ Single value (text, number, boolean) ────→ Convenience constructor (text_input, confirm, choice)
├─ Multiple fields in one form ─────────────→ ElicitationBuilder::form() with field methods
├─ Sequential forms with state ─────────────→ Multi-step workflow (session state between steps)
└─ Custom UI (CLI, web, desktop) ───────────→ Implement ElicitationProvider trait
```

**Elicitation is a client capability.** The server requests it; the client decides whether to support it.

## Schema Primitives

MCP elicitation schemas are restricted to flat objects with primitive fields. No nesting, no arrays, no `$ref`.

| Type | Rust Type | Builder Method | Variants |
|---|---|---|---|
| String | `StringSchema` | `.string_field()` | `_with_length()`, `_with_format()` |
| Number | `NumberSchema` | `.number_field()` | `_with_range()`, integer variants |
| Boolean | `BooleanSchema` | `.boolean_field()` | `_with_default()` |
| Enum | `EnumSchema` | `.enum_field()` | `_with_names()` (display names) |

**String formats**: `StringFormat::Email`, `Uri`, `Date`, `DateTime`

**Number types**: `.number_field()` (float), `.integer_field()` (integer with `schema_type: "integer"`)

## ElicitationBuilder

The builder constructs `ElicitCreateRequest` objects with validated schemas.

```rust
// turul-mcp-server v0.3
use turul_mcp_builders::ElicitationBuilder;
use turul_mcp_protocol::elicitation::StringFormat;

let request = ElicitationBuilder::new("Please provide your contact details")
    .title("Contact Form")
    .string_field("name", "Your full name")
    .string_field_with_format("email", "Email address", StringFormat::Email)
    .number_field_with_range("age", "Your age", Some(18.0), Some(120.0))
    .enum_field(
        "department",
        "Your department",
        vec!["engineering".into(), "sales".into(), "support".into()],
    )
    .boolean_field_with_default("newsletter", "Subscribe to newsletter", false)
    .require_fields(vec!["name".into(), "email".into()])
    .build();
```

**Key methods:**

| Method | Purpose |
|---|---|
| `new(message)` | Create builder with the user-facing message |
| `.title(title)` | Optional dialog title |
| `.string_field(name, desc)` | Add a string field |
| `.string_field_with_length(name, desc, min, max)` | String with length constraints |
| `.string_field_with_format(name, desc, format)` | String with format (email, uri, date) |
| `.number_field(name, desc)` | Add a float field |
| `.integer_field(name, desc)` | Add an integer field |
| `.number_field_with_range(name, desc, min, max)` | Number with min/max constraints |
| `.boolean_field(name, desc)` | Add a boolean field |
| `.boolean_field_with_default(name, desc, default)` | Boolean with default value |
| `.enum_field(name, desc, values)` | Add an enum (string with predefined values) |
| `.enum_field_with_names(name, desc, values, display_names)` | Enum with display labels |
| `.require_field(name)` / `.require_fields(names)` | Mark fields as required |
| `.meta_value(key, value)` | Add metadata key-value pair |
| `.build()` | Build `ElicitCreateRequest` |
| `.build_dynamic()` | Build `DynamicElicitation` (with validation traits) |

**See:** `references/elicitation-builder-reference.md` for the full API reference.

## Convenience Constructors

One-liner shortcuts for common patterns:

```rust
// turul-mcp-server v0.3
use turul_mcp_builders::ElicitationBuilder;

// Simple text input (required)
let req = ElicitationBuilder::text_input("Enter your name", "name", "Full name").build();

// Number with range
let req = ElicitationBuilder::number_input("Enter score", "score", "Score (0-100)", Some(0.0), Some(100.0)).build();

// Yes/no confirmation
let req = ElicitationBuilder::confirm("Do you agree to the terms?").build();

// Multiple choice
let req = ElicitationBuilder::choice(
    "Select priority",
    "priority",
    "Task priority",
    vec!["low".into(), "medium".into(), "high".into()],
).build();

// Email input
let req = ElicitationBuilder::email_input("Enter email", "email", "Contact email").build();

// URL input
let req = ElicitationBuilder::url_input("Enter website", "url", "Website URL").build();

// Complex form (chain field methods)
let req = ElicitationBuilder::form("Complete your profile")
    .string_field("name", "Full name")
    .enum_field("role", "Role", vec!["admin".into(), "user".into()])
    .require_fields(vec!["name".into(), "role".into()])
    .build();
```

## Handling Responses

`ElicitResult` has three actions: `Accept` (user provided input), `Decline` (user refused), `Cancel` (user cancelled).

```rust
// turul-mcp-server v0.3
use turul_mcp_protocol::elicitation::{ElicitResult, ElicitAction};

fn handle_elicitation_result(result: ElicitResult) -> McpResult<String> {
    match result.action {
        ElicitAction::Accept => {
            // content is only present on Accept
            let content = result.content.unwrap_or_default();
            let name = content.get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            Ok(format!("Hello, {}!", name))
        }
        ElicitAction::Decline => {
            Ok("User declined the request.".to_string())
        }
        ElicitAction::Cancel => {
            Ok("User cancelled the operation.".to_string())
        }
    }
}
```

**`ElicitResultBuilder`** — For constructing test responses:

```rust
use turul_mcp_builders::ElicitResultBuilder;

let accept = ElicitResultBuilder::accept_single("name", json!("Alice"));
let accept_multi = ElicitResultBuilder::accept_fields(vec![
    ("name".into(), json!("Alice")),
    ("age".into(), json!(30)),
]);
let decline = ElicitResultBuilder::decline();
let cancel = ElicitResultBuilder::cancel();
```

## Server Setup

Enable elicitation on the server builder:

```rust
// turul-mcp-server v0.3
use turul_mcp_server::McpServer;

// Development/testing — uses MockElicitationProvider
// Auto-accepts, declines if message contains "decline", cancels if "cancel"
let server = McpServer::builder()
    .name("my-server")
    .with_elicitation()  // Mock provider
    .tool(MyTool::default())
    .build()?;

// Production — custom provider for real UI
let server = McpServer::builder()
    .name("my-server")
    .with_elicitation_provider(MyCustomProvider)
    .tool(MyTool::default())
    .build()?;
```

## Custom ElicitationProvider

Implement the `ElicitationProvider` trait to present custom UI for elicitation requests.

```rust
// turul-mcp-server v0.3
use turul_mcp_server::handlers::ElicitationProvider;
use turul_mcp_protocol::elicitation::{ElicitCreateRequest, ElicitResult};
use turul_mcp_protocol::McpError;
use async_trait::async_trait;

struct WebFormProvider {
    base_url: String,
}

#[async_trait]
impl ElicitationProvider for WebFormProvider {
    async fn elicit(
        &self,
        request: &ElicitCreateRequest,
    ) -> Result<ElicitResult, McpError> {
        // Present the form via your UI mechanism
        // Return the user's response as ElicitResult
        let response = present_web_form(&self.base_url, request).await
            .map_err(|e| McpError::tool_execution(e.to_string()))?;
        Ok(response)
    }
}
```

**See:** `examples/custom-elicitation-provider.rs` for a complete example.

## Multi-Step Workflows

Chain elicitations by accumulating state between steps using session state.

```rust
// turul-mcp-server v0.3 — Pattern: multi-step elicitation
// Step 1: Collect basic info → store in session → Step 2: Collect details

async fn execute(&self, session: Option<SessionContext>) -> McpResult<String> {
    let session = session.ok_or(McpError::tool_execution("Session required"))?;

    // Check which step we're on
    let step: u32 = session.get_typed_state("onboarding_step").await
        .unwrap_or(1);

    match step {
        1 => {
            // First elicitation: collect name + email
            let request = ElicitationBuilder::form("Enter your basic information")
                .string_field("name", "Full name")
                .string_field_with_format("email", "Email", StringFormat::Email)
                .require_fields(vec!["name".into(), "email".into()])
                .build();

            // ... send request, handle response, store in session
            session.set_typed_state("onboarding_step", 2).await?;
            Ok("Step 1 complete. Run again for step 2.".to_string())
        }
        2 => {
            // Second elicitation: collect role + preferences
            let request = ElicitationBuilder::form("Choose your preferences")
                .enum_field("role", "Role", vec!["admin".into(), "user".into()])
                .boolean_field_with_default("notifications", "Enable notifications", true)
                .require_field("role")
                .build();

            // ... send request, handle response
            session.set_typed_state("onboarding_step", 3).await?;
            Ok("Onboarding complete!".to_string())
        }
        _ => Ok("Already completed onboarding.".to_string()),
    }
}
```

**See:** `examples/multi-step-workflow.rs` for a complete example.

## Validation

`DynamicElicitation` (from `.build_dynamic()`) provides automatic validation via `HasElicitationHandling`:

- **`validate_content(content)`** — Checks required fields present, types match schema, enum values valid
- **`process_content(content)`** — Validates + normalizes (enforces length constraints, range limits)

```rust
// turul-mcp-server v0.3
let elicitation = ElicitationBuilder::new("Create account")
    .string_field_with_length("username", "Username", Some(3), Some(20))
    .number_field_with_range("age", "Age", Some(18.0), Some(120.0))
    .require_fields(vec!["username".into(), "age".into()])
    .build_dynamic();

// Validate user input
let mut content = HashMap::new();
content.insert("username".into(), json!("Al"));  // Too short!
content.insert("age".into(), json!(25));

let result = elicitation.process_content(content);
assert!(result.is_err());  // "Field 'username' must be at least 3 characters long"
```

## Common Mistakes

1. **Nested schemas** — MCP spec restricts elicitation to primitive types only. No nested objects, arrays, or `$ref`. Use multiple sequential elicitations for complex data.

2. **Forgetting `.with_elicitation()` on server builder** — Without it, elicitation requests have no provider and will fail at runtime. Add `.with_elicitation()` (dev) or `.with_elicitation_provider(custom)` (prod).

3. **Reading `content` without checking `action`** — `content` is only `Some` when `action == Accept`. Always match on the action first.

4. **Using raw protocol types instead of builder** — `ElicitationBuilder` handles schema construction, required fields, and format constraints. Don't construct `ElicitationSchema` manually unless you need trait-level control.

5. **Not testing decline/cancel paths** — `MockElicitationProvider` can simulate all three actions. Test all paths: messages containing "decline" trigger `Decline`, "cancel" triggers `Cancel`, everything else triggers `Accept`.

## Beyond This Skill

**Error handling in elicitation tools?** → See the `error-handling-patterns` skill for `McpError` variants and tool execution error patterns.

**Combining elicitation with tasks?** → See the `task-patterns` skill for long-running tools that collect input mid-execution.

**Testing elicitation workflows?** → See the `testing-patterns` skill for `McpTestClient`, E2E test setup, and compliance assertions.

**Creating the tool that uses elicitation?** → See the `tool-creation-patterns` skill for `#[mcp_tool]`, `#[derive(McpTool)]`, and `ToolBuilder`.

**Builder API reference?** → See `references/elicitation-builder-reference.md` for the complete `ElicitationBuilder` and `ElicitResultBuilder` API.
