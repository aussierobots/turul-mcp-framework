# ADR-037: Macros declare every per-object spec field they can know at compile time

**Status:** Accepted
**Date:** 2026-11-15
**Crates:** `turul-mcp-derive` (attribute parsing and emitted trait impls),
`turul-mcp-server` (`resources/templates/list`, `prompts/get`)
**Related:** ADR-036 (sorted wire maps), MCP Apps 2026-01-26 (`_meta.ui`)

## Context

The 2026-07-28 schema lets a server set `_meta`, `title`, `icons` and
`annotations` on Tool, Resource, ResourceTemplate and Prompt. The builders
(`ToolBuilder`, `ResourceBuilder`, `PromptBuilder`) can set all of these.
The macros could not set several of them, because they emitted the carrying
trait with a hard-coded `None`:

| Field | `#[mcp_tool]` | `derive(McpTool)` | `tool!` | `#[mcp_resource]` | `derive(McpResource)` | `resource!` | `derive(McpPrompt)` | `prompt!` |
|---|---|---|---|---|---|---|---|---|
| `_meta` | None | None | None | None | None | None | None | None |
| `icons` | settable | settable | none | none | none | none | none | none |
| `annotations` | settable | settable | settable | None | None | None | n/a | n/a |
| `title` | settable | settable | settable | settable | settable | settable | absent | settable |
| `mimeType` | n/a | n/a | n/a | literal | literal | None | n/a | n/a |
| `size` | n/a | n/a | n/a | None | None | None | n/a | n/a |

A tool that opens an MCP App must carry `_meta.ui.resourceUri`. With no
`_meta` attribute, every Apps tool had to fall back to `ToolBuilder` or to a
hand-written `HasToolMeta` impl. A hand-written impl beside a derive is
impossible: the derive already emits the impl, so the two conflict (E0119).

`resources/templates/list` built each `ResourceTemplate` from only name,
description and mime type. It dropped title, icons, annotations and `_meta`,
even when the registered resource declared them. That is non-compliant
output, whatever form declared the resource.

`size` is knowable at compile time whenever the content is: a resource
serving an embedded document (`include_str!`) knows its byte length.

`prompts/get` built `GetPromptResult` itself from `McpPrompt::render`,
bypassing `McpPrompt::get_response`, the trait's own result constructor. So
no prompt of any form could set result `_meta`, and the trait's
`validate_args` and `transform_messages` hooks never ran on the wire: an
override of either was dead code.

A `resources/read` content item carries `_meta`, which is where MCP Apps
puts `UiResourceMeta`. Every resource form's handler returns its content
items, so it already sets that `_meta` by building `TextResourceContents` or
`BlobResourceContents`; no new surface is needed there.

## Decision

1. **`meta = <expr>`** on `#[mcp_tool]`, `#[tool(...)]` (`derive(McpTool)`),
   `#[mcp_resource]`, `#[resource(...)]` (`derive(McpResource)`) and
   `#[prompt(...)]` (`derive(McpPrompt)`), and `meta: <expr>` on `tool!`,
   `resource!` and `prompt!`. `<expr>` has type
   `HashMap<String, serde_json::Value>`. The macro emits the carrying trait
   (`HasToolMeta`, `HasResourceMeta`, `HasPromptMeta`) returning a value
   built once per type into a `OnceLock`, the same pattern `icons` already
   uses. The typed form for MCP Apps is the extension's own constructor:
   `meta = UiToolMeta { .. }.to_meta()`. The derive crate gains no
   dependency on `turul-mcp-ext-apps`. That crate owns the `ui` key, and the
   macro stays generic.
2. **`icons = <expr>`** (`Vec<Icon>`) on the two resource forms and on
   `derive(McpPrompt)`, and `icons: <expr>` on `tool!`, `resource!` and
   `prompt!`, reusing the tool macros' `generate_icons_impl`.
3. **`annotations = <expr>`** (`Annotations`) on the two resource forms, and
   `annotations: <expr>` on `resource!`.
4. **`title = "..."`** on `derive(McpPrompt)`.
5. `resources/templates/list` copies title, icons, annotations and `_meta`
   from the registered resource, as `resources/list` already does.
6. `#[mcp_resource(mime_type = ...)]` accepts any `&'static str`
   expression, so `mime_type = MCP_APP_HTML_MIME` names the extension's
   constant instead of repeating its value. A string literal still works.
   (`uri` stays a literal: template variables are parsed from it at compile
   time.) `resource!` takes `mime_type: <expr>` the same way.
7. **`size = <expr>`** (`u64`) on `#[mcp_resource]` and `#[resource(...)]`,
   and `size: <expr>` on `resource!`, emitting `HasResourceSize`. It is
   served on `resources/list`; `ResourceTemplate` has no `size` field.
8. **`prompts/get` returns `McpPrompt::get_response`**, which runs
   `validate_args`, `render` and `transform_messages`. The handler keeps
   its required-argument check and its MRTR conversion, then adds the
   caller's echoed `_meta` extras to the result without replacing keys the
   prompt set. A prompt sets `GetPromptResult._meta` by overriding
   `get_response`, the same way a tool sets `CallToolResult._meta` by
   implementing `McpTool::call`. The handler's own `GetPromptResult`
   construction is deleted, so there is one.
9. **An argument the macro does not know is a compile error.**
   `#[mcp_tool]` and `#[mcp_resource]` refuse an unknown key, naming the
   accepted ones, so `mimetype = ...` or `readonly = true` fails the build
   instead of being dropped.
10. **`task_support` on the 2026-07-28 lane is a compile error naming the
    Tasks extension.** `ToolExecution` exists only in the 2025-11-25 schema.
    The derive crate's lane feature decides, through one emitter
    (`generate_execution_impl` in `utils.rs`) shared by `#[mcp_tool]` and
    `derive(McpTool)`. Before, the emitted paths failed to resolve with
    `cannot find TaskSupport`.

Every declarative field takes an expression, as `tool!` does. Each shared
emitter (`generate_meta_impl`, `generate_icons_impl`,
`generate_resource_annotations_impl`, `generate_resource_size_impl` in
`tool_derive.rs`) is the one source of its trait impl for every form.

`derive(McpPrompt)` on a struct with no fields failed to compile (the
emitted empty `vec![]` had no inferable type). The emitted argument list is
now typed, which the new prompt test exercises.

With none of the new attributes in use, the emitted impls are identical to
before, so no served byte changes. The one exception is item 5: a template
resource that already declared title, icons, annotations or `_meta` now
serves them. That is a spec-compliance fix.

A `validate_args` error now reaches the client as a JSON-RPC error. That
is the contract the hook always documented; before, it was never called.

### Not macro attributes (documented in docs/compliance/server-features.md)

- **Result `_meta`**: a per-call value, not a per-object declaration, so no
  static attribute can carry it. A tool sets it by implementing `McpTool`
  (its `call` returns `CallToolResult`); a prompt by overriding
  `McpPrompt::get_response`. Both work for every form of prompt; a
  macro-authored tool keeps its framework-built result.
- **Prompt `annotations`**: not a field of `Prompt` in 2026-07-28.

`resultType` is set by the protocol layer on every result and needs no
declaration.

Rejected:
- A dedicated `ui(resource_uri = "...", visibility = [...])` attribute.
  It would duplicate `UiToolMeta` inside the derive crate, giving two
  definitions of one vocabulary.
- Allowing a hand-written `HasToolMeta` beside the derive by not emitting
  it when unset. That changes trait coherence for every existing derive,
  and an attribute covers the need.

## Tests

- `crates/turul-mcp-server/tests/macro_spec_fields_2026.rs`: the served
  `tools/list`, `resources/list`, `resources/templates/list` and
  `prompts/list` JSON for each form with the new attributes equals the exact
  expected objects. A form without them serves no `_meta`, `icons` or
  `annotations` key.
- The test file is the compile-pass example for every new attribute.
  `examples/ext-apps-server` declares its `ui://` view with `#[mcp_resource]`
  (`mime_type = MCP_APP_HTML_MIME`), setting `_meta.ui` on the content item it
  returns; its
  served `tools/list`, `resources/list`, `resources/read` and `tools/call`
  JSON is byte-identical to the manual impl it replaced, apart from
  `serverInfo.version`. Its `get_weather` tool stays a manual impl because
  the view displays the result text, and a macro tool returning `String`
  serves `structuredContent` with JSON text (ADR-010).

## Revision log

- **2026-11-15**: Initial, Accepted.
- **2026-11-15**: `resource!` and `prompt!` take `meta`, `icons` (and
  `resource!` `annotations`, `mime_type`); resource forms take `size`;
  `prompts/get` returns `McpPrompt::get_response`, so prompt result
  `_meta`, `validate_args` and `transform_messages` work. Withdraws the
  omissions for the declarative macros and for `size`.
- **2026-11-17**: Items 9 and 10. Unknown `#[mcp_tool]` / `#[mcp_resource]`
  arguments are refused; `task_support` on 2026-07-28 fails with a message
  naming the Tasks extension; the two copies of the execution-impl `match`
  become `generate_execution_impl`. Tests: `unknown_argument_is_refused_*` in
  `tool_attr.rs` and `resource_attr.rs`, `execution_impl_tests` in `utils.rs`.
