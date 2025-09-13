# Prompts Compliance TODO

## Current State
- Protocol types for prompts (list/get, messages, arguments, notifications) are implemented and covered by unit tests.
- Server builder registers `prompts/list` and `prompts/get`; a single `PromptsHandler` claims both but only lists, due to handler interface not receiving the method name.
- Example prompts server uses McpPrompt traits; SSE bridge supports prompt listChanged notifications.

## Gaps / Issues
- `prompts/get` not implemented in the generic handler; no dispatch to `McpPrompt` implementations to produce `GetPromptResult`.
- Argument typing gap: protocol expects `GetPromptParams.arguments: Option<HashMap<String, String>>`, while `McpPrompt.render` currently uses `HashMap<String, Value>`.
- Notification naming inconsistency: tests/spec use `notifications/prompts/listChanged` (camelCase) vs `list_changed` constants in builders.
- No end‑to‑end server tests for prompts endpoints or notifications.

## Phase Plan (critical, spec-aligned)

Phase 0 — Spec & Naming Alignment (blocker)
- Confirm canonical naming from the protocol crate (camelCase `notifications/prompts/listChanged`).
- Locate and fix any snake_case usages to camelCase to avoid interop mismatches.
- Add a unit check asserting prompt notification method string is exactly `notifications/prompts/listChanged`.

Files to update for naming consistency (no behavior change):
- `crates/turul-mcp-builders/src/notification.rs` (constants + tests) → camelCase `listChanged`.
- `crates/turul-http-mcp-server/src/notification_bridge.rs` (doc comments) → camelCase.
- `AGENTS.md`, `GEMINI.md`, `docs/adr/005-mcp-message-notifications-architecture.md`, `WORKING_MEMORY.md` → examples use camelCase.

Phase 1 — Handler Separation & Routing (correctness)
- Replace multi‑method `PromptsHandler` with:
  - `PromptsListHandler` → supports only `prompts/list` and returns `ListPromptsResult`.
  - `PromptsGetHandler` → supports only `prompts/get` and returns `GetPromptResult`.
- Builder wiring: register distinct handlers; `.with_prompts()` attaches registered prompts to both handlers.

Phase 2 — Arguments & Validation (MCP semantics)
- Parse `GetPromptRequest` where `arguments` is `HashMap<String, String>` (per MCP).
- Validate required args against `PromptDefinition.arguments` (required true); emit MCP Validation errors if missing.
- If internal render uses `HashMap<String, Value>`, convert strings to `Value::String` safely.
- Ensure no System role in messages; only `user` and `assistant` per spec.

Phase 3 — Response Construction (shape fidelity)
- `ListPromptsResult`: include `nextCursor` and optional `_meta` when available.
- `GetPromptResult`: include `description` when available and optional `_meta`; messages must use spec `ContentBlock` variants.
- Never `unwrap()` on hot paths; use structured MCP errors.

Phase 4 — Notifications & Capabilities (truthful signaling)
- Emit `notifications/prompts/listChanged` only when the prompt set actually changes.
- Set `ServerCapabilities.prompts.listChanged = true` only when change events are wired (e.g., hot‑reload/config change path exists and SSE enabled).

Phase 5 — Pagination & _meta (scalability)
- Implement cursor semantics for `prompts/list` (stable ordering by prompt name; opaque cursor token).
- Propagate optional `_meta` in requests and results.

Phase 6 — Test Coverage (end‑to‑end)
- Endpoint integration: `prompts/list` and `prompts/get` with/without required args; validate roles, message content, and optional `_meta`.
- Notifications: SSE delivery of `notifications/prompts/listChanged` with camelCase.
- Pagination: cursor round‑trip across multiple pages.
- Negative tests: missing required args → MCP validation error; unknown prompt → MCP prompt‑not‑found error code.

## Tests To Add/Adjust
- New integration tests under `tests/prompts/tests/`:
  - `prompts_endpoints_integration.rs`: start a server with `.with_prompts()`, POST JSON‑RPC for `prompts/list` and `prompts/get` (with and without required args); assert response shapes, roles, `_meta`.
  - `prompts_notifications.rs`: enable SSE, mutate prompt set (or stub), assert `notifications/prompts/listChanged` delivery with camelCase method.
  - `prompts_arguments_validation.rs`: verify MCP error codes and messages when required args are missing/invalid.
- Update builder notification constant tests to camelCase.

## Acceptance Criteria
- `prompts/list` and `prompts/get` return spec‑compliant results; arguments validated per `PromptArgument` schema.
- Notification names and capabilities align with MCP 2025‑06‑18 (camelCase listChanged).
- Integration tests cover endpoints, validation, and notifications; all existing protocol tests continue to pass.

## Open Questions
- Should `McpPrompt.render` accept `HashMap<String, String>` to match protocol directly? If not, keep a conversion layer in the handler.
- Define when prompts set changes and how to trigger listChanged (e.g., hot‑reload, admin API, config changes).

## Notes on Prompt Design (best practices)
- Keep prompt names stable and human‑readable; include descriptions for discoverability.
- Arguments should be documented with `PromptArgument` names, descriptions, and `required` flags.
- Messages should be strictly `user`/`assistant` roles with `Text`/`Image`/`Resource` content blocks; avoid ad‑hoc fields.
