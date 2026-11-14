# ADR-037: Macros declare every per-object spec field they can know at compile time

**Status:** Accepted
**Date:** 2026-11-15
**Crates:** `turul-mcp-derive` (attribute parsing and emitted trait impls),
`turul-mcp-server` (`resources/templates/list`)
**Related:** ADR-036 (sorted wire maps), MCP Apps 2026-01-26 (`_meta.ui`)

## Context

The 2026-07-28 schema lets a server set `_meta`, `title`, `icons` and
`annotations` on Tool, Resource, ResourceTemplate and Prompt. The builders
(`ToolBuilder`, `ResourceBuilder`, `PromptBuilder`) can set all of these.
The macros could not set several of them, because they emitted the carrying
trait with a hard-coded `None`:

| Field | `#[mcp_tool]` | `derive(McpTool)` | `#[mcp_resource]` | `derive(McpResource)` | `derive(McpPrompt)` |
|---|---|---|---|---|---|
| `_meta` | None | None | None | None | None |
| `icons` | settable | settable | none | none | none |
| `annotations` | settable | settable | None | None | n/a |
| `title` | settable | settable | settable | settable | absent |

A tool that opens an MCP App must carry `_meta.ui.resourceUri`. With no
`_meta` attribute, every Apps tool had to fall back to `ToolBuilder` or to a
hand-written `HasToolMeta` impl. A hand-written impl beside a derive is
impossible: the derive already emits the impl, so the two conflict (E0119).

`resources/templates/list` built each `ResourceTemplate` from only name,
description and mime type. It dropped title, icons, annotations and `_meta`,
even when the registered resource declared them. That is non-compliant
output, whatever form declared the resource.

A `resources/read` content item carries `_meta`, which is where MCP Apps
puts `UiResourceMeta`. Every resource form's handler returns its content
items, so it already sets that `_meta` by building `TextResourceContents` or
`BlobResourceContents`; no new surface is needed there.

## Decision

1. **`meta = <expr>`** on `#[mcp_tool]`, `#[tool(...)]` (`derive(McpTool)`),
   `#[mcp_resource]`, `#[resource(...)]` (`derive(McpResource)`) and
   `#[prompt(...)]` (`derive(McpPrompt)`), and `meta: <expr>` on `tool!`. `<expr>` has type
   `HashMap<String, serde_json::Value>`. The macro emits the carrying trait
   (`HasToolMeta`, `HasResourceMeta`, `HasPromptMeta`) returning a value
   built once per type into a `OnceLock`, the same pattern `icons` already
   uses. The typed form for MCP Apps is the extension's own constructor:
   `meta = UiToolMeta { .. }.to_meta()`. The derive crate gains no
   dependency on `turul-mcp-ext-apps`. That crate owns the `ui` key, and the
   macro stays generic.
2. **`icons = <expr>`** (`Vec<Icon>`) on the two resource forms and on
   `derive(McpPrompt)`, and `icons: <expr>` on `tool!`, reusing the tool
   macros' `generate_icons_impl`.
3. **`annotations = <expr>`** (`Annotations`) on the two resource forms.
4. **`title = "..."`** on `derive(McpPrompt)`.
5. `resources/templates/list` copies title, icons, annotations and `_meta`
   from the registered resource, as `resources/list` already does.
6. `#[mcp_resource(mime_type = ...)]` accepts any `&'static str`
   expression, so `mime_type = MCP_APP_HTML_MIME` names the extension's
   constant instead of repeating its value. A string literal still works.
   (`uri` stays a literal: template variables are parsed from it at compile
   time.)

`derive(McpPrompt)` on a struct with no fields failed to compile (the
emitted empty `vec![]` had no inferable type). The emitted argument list is
now typed, which the new prompt test exercises.

With none of the new attributes in use, the emitted impls are identical to
before, so no served byte changes. The one exception is item 5: a template
resource that already declared title, icons, annotations or `_meta` now
serves them. That is a spec-compliance fix.

### Deliberate omissions (documented in docs/compliance/server-features.md)

- **`resource!` and `prompt!` declarative macros**: these are shorthands
  and stay minimal. Use an attribute macro, a derive or a builder.
- **Resource `size` on macros**: `size` describes the content, which a
  static attribute cannot know. Use `ResourceBuilder::size` or a manual
  `HasResourceSize` impl.
- **Result `_meta` on tools and prompts**: a macro-authored tool returns a
  value that the framework wraps into `CallToolResult`. The handler can
  shape `structuredContent` and `isError` through its return type and
  `ToolExecutionError`, but cannot set result `_meta`. A manual `McpTool`
  impl returns `CallToolResult` directly and can. `McpPrompt::render`
  returns messages, so `GetPromptResult._meta` is likewise manual-only.
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
