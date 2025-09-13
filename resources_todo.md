# Resources Compliance TODO

## Current State
- Protocol types for resources (list/read/templates, subscribe/unsubscribe, notifications) are implemented in `turul-mcp-protocol-2025-06-18`.
- HTTP SSE bridge supports resource notifications.
- Server builder registers `resources/list` and `resources/templates/list`; a single `ResourcesHandler` claims to support both `resources/list` and `resources/read` but cannot distinguish method.
- Tests cover protocol structs and resource business logic, but not server endpoint behavior nor notifications.

## Gaps / Issues
- `ResourcesHandler` only returns list results; `resources/read` is effectively unimplemented because `McpHandler` has no method parameter and the bridge drops the method string.
- `resources/templates/list` handler returns raw JSON instead of `ListResourceTemplatesResult`.
- Notification naming inconsistency: camelCase `notifications/resources/listChanged` (tests/spec) vs snake_case constants (`list_changed`) in builders.
- No end‑to‑end tests for `resources/list`, `resources/read`, `resources/templates/list`, or notifications.

## Phase Plan (critical, spec-aligned)

Phase 0 — Spec & Naming Alignment (blocker)
- Confirm canonical method names from protocol crate (already camelCase: `notifications/.../listChanged`).
- Identify and fix snake_case usages (docs/constants/tests) to camelCase to avoid interop bugs.
- Add a minimal unit check asserting method strings for resources/prompts/tools/roots notifications.

Files to update for naming consistency (no behavior change):
- `crates/turul-mcp-builders/src/notification.rs` (constants + related tests): use `listChanged`.
- `crates/turul-http-mcp-server/src/notification_bridge.rs` (doc comments): reflect camelCase.
- `AGENTS.md`, `GEMINI.md`, `docs/adr/005-mcp-message-notifications-architecture.md`, `WORKING_MEMORY.md`: update examples to camelCase.

Phase 1 — Handler Separation & Routing (correctness)
- Replace multi‑method `ResourcesHandler` with:
  - `ResourcesListHandler` → supports only `resources/list`.
  - `ResourcesReadHandler` → supports only `resources/read`; invokes `McpResource.read(params)`; returns `ReadResourceResult`.
- Keep `ResourceTemplatesHandler` but return protocol `ListResourceTemplatesResult` (not raw JSON).
- Builder wiring: register distinct handlers; `.with_resources()` attaches registered resources to `ResourcesReadHandler` and uses `resource_to_descriptor` for `list`.

Phase 2 — Dynamic URI Templates (robustness, best practice)
- Model dynamic resources via `ResourceTemplate` (RFC 6570) and resolve at `resources/read` time.
- Introduce a template registry: compile templates to safe matchers; map to resolver functions.
- Strict variable validation:
  - `user_id`: `^[A-Za-z0-9_-]{1,128}$` (example; reject ambiguous Unicode/non‑printables).
  - `image_format`: allowlist `{png,jpg,jpeg,webp,svg}` only.
- MIME strategy: extension→MIME map (json/pdf/images); reject unknowns with MCP validation error.
- Do NOT enumerate dynamic instances in `resources/list`; publish via `resources/templates/list`.

Phase 3 — Security Controls (non‑negotiable)
- Enforce absolute URIs and scheme policy (treat `file://` as a virtual namespace unless a root explicitly allows file access).
- Root scoping: any real FS access must be under configured Roots; forbid path traversal; normalize and validate paths.
- Response hardening: size caps for Blob/Text; include optional `_meta` such as size/etag/lastModified when available.
- Failure modes: use MCP error taxonomy (Invalid params, Validation, Resource execution) with clear messages; never `unwrap()` in hot paths.

Phase 4 — Notifications & Capabilities (truthful signaling)
- Emit `notifications/resources/updated` on content change (concrete URI), and `notifications/resources/listChanged` only when the available set changes.
- Set `ServerCapabilities.resources.listChanged = true` only when change events are actually wired (SSE enabled + change source connected). Keep `subscribe=false` until subscribe/unsubscribe endpoints exist.

Phase 5 — Pagination & _meta (scalability)
- Implement cursor semantics for `resources/list` (stable ordering by URI; opaque cursor token).
- Preserve/propagate optional `_meta` in requests and results consistently.

Phase 6 — Test Coverage (end‑to‑end)
- Endpoint integration: `resources/list`, `resources/read` (Text/Blob), `resources/templates/list`; validate `uri`, `mimeType`, `_meta`.
- Dynamic URIs: `file:///user-{user_id}.json` / `.pdf` / `user-profile-{user_id}.{image_format}` — happy paths and validation failures.
- Notifications: SSE delivery of `resources/updated` and `.../listChanged` with camelCase methods.
- Pagination: cursor round‑trip across multiple pages.
- Negative tests: invalid user_id, unsupported image_format, unknown URI → MCP errors.

## Priorities & Timeline
- Week 1 (Critical fixes):
  - Day 1: Phase 0 (naming) + Phase 1 (handler separation, builder wiring)
  - Day 2–3: Phase 2 (dynamic URI templates, validation, MIME mapping)
  - Day 4–5: Phase 3 (security controls, error taxonomy), basic endpoint integration tests
- Week 2 (Enhancements & coverage):
  - Day 1–2: Phase 4 (notifications wiring) when SSE path is ready
  - Day 3: Phase 5 (cursor pagination)
  - Day 4–5: Phase 6 (comprehensive tests: notifications, pagination, negatives)

## Tests To Add/Adjust
- New integration tests under `tests/resources/tests/`:
  - `resources_endpoints_integration.rs`: start a server with `.with_resources()`, POST JSON‑RPC for `resources/list`, `resources/read` (Text and Blob), `resources/templates/list`; assert shapes, `mimeType`, absolute `uri`, optional `_meta`.
  - `resources_notifications.rs`: enable SSE, trigger a resource change hook (or stub), assert receipt of `notifications/resources/listChanged` and `notifications/resources/updated` with camelCase method names.
  - `resources_pagination.rs`: validate `cursor`/`nextCursor` handling (can be deterministic two‑page stub).
- Update any builder notification constant tests to camelCase.

## Acceptance Criteria
- `resources/list`, `resources/read`, `resources/templates/list` return spec‑compliant results for registered resources.
- Notification names and capabilities align with MCP 2025‑06‑18 (camelCase listChanged).
- Tests pass for protocol structures and new endpoint/notification coverage.

## Open Questions
- Do we need `resources/subscribe` now? If yes, specify backend/state for subscriptions; otherwise keep capability off.
- For large lists, define a stable cursor strategy (e.g., lexicographic by URI).

- Is there a need to support `resources/subscribe` in near term, or keep capability off until a concrete backend and state model exists?
- For very large catalogs, confirm cursor stability requirements (e.g., lexicographic by URI + opaque token).

## Notes on Dynamic Resource Patterns (design decisions)
- Prefer templates + read‑time resolution over listing millions of instances; aligns with MCP templates and avoids noisy lists.
- Keep MIME explicit and correct; do not infer from content alone; validate extensions strictly.
- Treat `file://` URIs as logical identifiers unless a Root explicitly maps them to disk; never allow arbitrary disk reads by default.
- Treat `file://` URIs as logical identifiers unless a Root explicitly maps them to disk; never allow arbitrary disk reads by default.

## Impact Assessment
- Critical (must fix): naming inconsistency (camelCase vs snake_case); `resources/read` unimplemented; lack of URI template resolution.
- Security (should fix): missing path/roots validation; no content size caps; weak input validation (user_id, image_format).
- Enhancements (nice to have): pagination for large lists; listChanged notifications; future subscribe/unsubscribe support.

## Working Memory Update
The notification naming issue is an interoperability blocker; handler separation is required for basic correctness; and security controls are non‑negotiable for production. Prioritize Phases 0–3, then proceed with Phases 4–6 once the core is correct and secure.
