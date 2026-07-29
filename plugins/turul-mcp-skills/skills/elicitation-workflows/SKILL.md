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

**Spec lane: MCP 2026-07-28 (current default).** The elicitation *schema* (primitive-only fields, `ElicitationBuilder`'s field methods, `ElicitResult`/`ElicitAction`) is unchanged from 2025-11-25 and works on both lanes. **The transport mechanism is completely different**: 2025-11-25 held the connection open for a synchronous server→client `elicitation/create` request (`ElicitationProvider` trait, `.with_elicitation()`); 2026-07-28's stateless core has no connection to hold open, so it uses MRTR (Multi-Round-Trip, SEP-2322) instead — the server returns `InputRequiredResult` from the *original* request, and the client re-issues that same call with `inputResponses`. `ElicitationProvider`/`.with_elicitation()`/`.with_elicitation_provider()` are `#[cfg(feature = "protocol-2025-11-25")]`-gated and don't exist on a default 2026-07-28 build — see the MRTR sections below for the replacement, and the tail of this skill for the frozen 2025-11-25 mechanism.

Elicitation lets the server request structured input from the user/client. Schemas are restricted to **primitive types only** (no nesting).

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
// turul-mcp-server v0.4
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
| `.build()` | Build `ElicitCreateRequest` — **2025-11-25 only** (`#[cfg(feature = "protocol-2025-11-25")]`); doesn't exist on 2026-07-28 |
| `.build_dynamic()` | Build `DynamicElicitation` (with validation traits) — works on both lanes; use `.message()` / `.requested_schema()` (from `HasElicitationMetadata`/`HasElicitationSchema`) to feed a 2026-07-28 `ElicitRequest::new_form()` — see [MRTR](#mrtr-multi-round-trip-2026-07-28) below |

**See:** `references/elicitation-builder-reference.md` for the full API reference.

## Convenience Constructors

One-liner shortcuts for common patterns. **The `.build()` calls below are 2025-11-25 only** — on 2026-07-28, swap the trailing `.build()` for `.build_dynamic()` and feed `.message()`/`.requested_schema()` into `ElicitRequest::new_form()` (see [MRTR](#mrtr-multi-round-trip-2026-07-28)).

```rust
// turul-mcp-server v0.4
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
// turul-mcp-server v0.4
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

## MRTR (Multi-Round-Trip, 2026-07-28)

No server builder opt-in is needed — MRTR is core dispatcher behavior on `tools/call`, `resources/read`, and `prompts/get`. A tool that needs input returns `Err(McpError::InputRequired { .. })`; the dispatcher converts that into a successful `InputRequiredResult` for the client, after checking the client declared the matching capability in `_meta.clientCapabilities` (undeclared → `-32021 MissingRequiredClientCapability`).

```rust
// turul-mcp-server v0.4 (feature = "protocol-2026-07-28")
use turul_mcp_builders::ElicitationBuilder;
use turul_mcp_protocol::input_required::{InputRequest, InputRequests, InputResponse};
use turul_mcp_protocol::elicitation::{ElicitRequest, ElicitAction};
use turul_mcp_protocol::McpError;
use std::collections::HashMap;

async fn execute(&self, session: Option<SessionContext>) -> McpResult<String> {
    let session = session.ok_or_else(|| McpError::tool_execution("Session required"))?;

    // Retry leg: the client already answered — inputResponses is present.
    if let Some(mut responses) = session.input_responses() {
        let name = match responses.remove("contact_form") {
            Some(InputResponse::Elicit(result)) if result.action == ElicitAction::Accept => {
                result.content
                    .and_then(|c| c.get("name").and_then(|v| v.as_str()).map(str::to_string))
                    .unwrap_or_else(|| "unknown".to_string())
            }
            _ => return Ok("User declined or cancelled.".to_string()),
        };
        return Ok(format!("Hello, {name}!"));
    }

    // First leg: build the schema (spec-agnostic), then wrap it in a 2026-07-28
    // ElicitRequest — build_dynamic()/.message()/.requested_schema() work on
    // both lanes; .build() (→ ElicitCreateRequest) does not.
    let elicitation = ElicitationBuilder::new("Please provide your contact details")
        .string_field("name", "Your full name")
        .require_fields(vec!["name".into()])
        .build_dynamic();

    let request = ElicitRequest::new_form(
        elicitation.message().to_string(),
        elicitation.requested_schema().clone(),
    );

    let mut input_requests = HashMap::new();
    input_requests.insert("contact_form".to_string(), InputRequest::Elicit(request));

    Err(McpError::InputRequired {
        input_requests: Some(InputRequests(input_requests)),
        request_state: None, // Some(opaque_state) to carry your own state through the retry
    })
}
```

**Reading `request_state` back**: `session.mrtr_request_state() -> Option<String>` on the retry leg — the client echoes it verbatim. Treat it as attacker-controlled (verify integrity, e.g. HMAC, before letting it influence authorization decisions) — it never touches server-side storage.

**Multi-step forms**: chain by encoding a step marker into `request_state` rather than session state — each retry is a fresh request against a (possibly different) server instance; there is no cross-request session to accumulate state in the way 2025-11-25's session-state pattern did.

**Sampling and roots too**: `InputRequest` is `Elicit(ElicitRequest) | CreateMessage(CreateMessageRequest) | ListRoots(ListRootsRequest)` — the same MRTR mechanism replaces server-initiated sampling and roots requests, not just elicitation. (`CreateMessage`/`ListRoots` are themselves deprecated per SEP-2577, but remain valid `InputRequest` variants during the 12-month migration window.)

## Validation

`DynamicElicitation` (from `.build_dynamic()`) provides automatic validation via `HasElicitationHandling`:

- **`validate_content(content)`** — Checks required fields present, types match schema, enum values valid
- **`process_content(content)`** — Validates + normalizes (enforces length constraints, range limits)

```rust
// turul-mcp-server v0.4
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

2. **Reaching for `.with_elicitation()` / `ElicitationProvider` on 2026-07-28** — neither exists on a default build (`#[cfg(feature = "protocol-2025-11-25")]`-gated). MRTR needs no server-builder opt-in; a tool returning `McpError::InputRequired` is sufficient.

3. **Reading `content` without checking `action`** — `content` is only `Some` when `action == Accept`. Always match on the action first.

4. **Using raw protocol types instead of builder** — `ElicitationBuilder` handles schema construction, required fields, and format constraints. Don't construct `ElicitationSchema` manually unless you need trait-level control.

5. **Accumulating multi-step state in session state under 2026-07-28** — there is no cross-request session on the stateless core. Encode step state into MRTR's `request_state` instead (integrity-checked, since the client echoes it back verbatim).

6. **Forgetting the capability-declaration check** — a client that didn't declare `elicitation` (or `sampling`/`roots` for those `InputRequest` variants) in `_meta.clientCapabilities` gets `-32021 MissingRequiredClientCapability`, not the elicitation prompt. This is dispatcher-enforced, not something you check manually.

## Beyond This Skill

**Error handling in elicitation tools?** → See the `error-handling-patterns` skill for `McpError` variants and tool execution error patterns.

**Combining elicitation with tasks?** → See the `task-patterns` skill for long-running tools that collect input mid-execution.

**Testing elicitation workflows?** → See the `testing-patterns` skill for `McpTestClient`, E2E test setup, and compliance assertions.

**Creating the tool that uses elicitation?** → See the `tool-creation-patterns` skill for `#[mcp_tool]`, `#[derive(McpTool)]`, and `ToolBuilder`.

**Builder API reference?** → See `references/elicitation-builder-reference.md` for the complete `ElicitationBuilder` and `ElicitResultBuilder` API.

---

## 2025-11-25 Synchronous Elicitation (frozen, `--no-default-features --features protocol-2025-11-25`)

The pre-MRTR mechanism: the server holds the connection and sends a synchronous `elicitation/create` request; the client answers on the same connection.

- Server opt-in: `.with_elicitation()` (mock provider — auto-accepts, declines if the message contains "decline", cancels if "cancel") or `.with_elicitation_provider(MyProvider)` (custom `ElicitationProvider` trait impl) on `McpServer::builder()`. Both are `#[cfg(feature = "protocol-2025-11-25")]`-gated.
- `ElicitationProvider` trait: `async fn elicit(&self, request: &ElicitCreateRequest) -> Result<ElicitResult, McpError>` — present the form via your UI, return the response.
- `ElicitationBuilder::build()` (also 2025-11-25-only) produces the `ElicitCreateRequest` this trait consumes.
- Multi-step forms: accumulate state across steps via `session.get_typed_state()`/`set_typed_state()` — valid because a 2025-11-25 session persists across requests, unlike 2026-07-28's ephemeral per-request session.

**See:** `examples/custom-elicitation-provider.rs` and `examples/multi-step-workflow.rs` for worked examples of this frozen mechanism.
