# MCP 2025-11-25 Spec Compliance Report

**Date**: 2026-02-07
**Reviewer**: spec-compliance agent
**Crate**: `turul-mcp-protocol-2025-11-25`
**Tests passing**: 121/121 + 2 doc-tests
**Reference**: https://github.com/modelcontextprotocol/specification/blob/main/schema/2025-11-25/schema.ts

---

## Summary

The crate has **significant deviations** from the official MCP 2025-11-25 TypeScript schema. While existing features (tools, resources, prompts, elicitation basics) are well-implemented and pass camelCase serialization, the newly added 2025-11-25 features (Tasks, Icons, Sampling additions) diverge from the spec in several critical ways.

**Severity Rating**: 7 issues CRITICAL, 5 issues MODERATE, 3 issues MINOR

---

## CRITICAL Issues (Must Fix)

### 1. Task Types: Completely Wrong Schema

**File**: `src/tasks.rs`

The `TaskInfo` / `TaskStatus` / `TaskProgress` types do NOT match the spec's `Task` type at all.

| Field | Spec (`Task`) | Our Implementation (`TaskInfo`) | Status |
|-------|--------------|-------------------------------|--------|
| `taskId` | `string` (REQUIRED) | `id: String` | WRONG FIELD NAME |
| `status` | `TaskStatus` union | `status: TaskStatus` | WRONG VALUES (see below) |
| `statusMessage` | `string?` (optional) | `message: Option<String>` | WRONG FIELD NAME |
| `createdAt` | `string` (REQUIRED, ISO 8601) | MISSING | MISSING |
| `lastUpdatedAt` | `string` (REQUIRED, ISO 8601) | MISSING | MISSING |
| `ttl` | `number \| null` (REQUIRED) | MISSING | MISSING |
| `pollInterval` | `number?` (optional) | MISSING | MISSING |
| `progress` | N/A | `Option<TaskProgress>` | NOT IN SPEC |
| `metadata` | N/A | `Option<HashMap>` | NOT IN SPEC |

**TaskStatus enum values are WRONG**:
- Spec: `"working"`, `"input_required"`, `"completed"`, `"failed"`, `"cancelled"`
- Ours: `"running"`, `"completed"`, `"failed"`, `"cancelled"`
- Missing: `"input_required"` (critical for elicitation flow)
- Wrong: `"running"` should be `"working"`

**TaskProgress type does not exist in spec** - progress is not a separate object in the Task schema.

**Params field names wrong**:
- Spec uses `taskId` everywhere; we use `id`
- `GetTaskRequest.params.taskId` -> our `GetTaskParams.id`
- `CancelTaskRequest.params.taskId` -> our `CancelTaskParams.id`

**Missing request type**: `GetTaskPayloadRequest` (method `"tasks/result"`) is not implemented.

**Missing result shapes**: In the spec, `GetTaskResult` and `CancelTaskResult` extend `Result & Task` directly (task fields are at the top level), not wrapped in a `task` field.

### 2. Icon Type: Wrong Structure

**File**: `src/tools.rs` (IconUrl type)

The spec defines `Icon` as an **object** with multiple fields, NOT a plain string:

| Field | Spec (`Icon`) | Our Implementation (`IconUrl`) | Status |
|-------|--------------|-------------------------------|--------|
| `src` | `string` (REQUIRED) | Entire type is a transparent string | WRONG STRUCTURE |
| `mimeType` | `string?` (optional) | MISSING | MISSING |
| `sizes` | `string[]?` (optional) | MISSING | MISSING |
| `theme` | `"light" \| "dark"?` (optional) | MISSING | MISSING |

Additionally, the spec uses an **`Icons` interface** with `icons?: Icon[]` (plural array), not a single `icon?: string` field.

**Affected structs** (all have wrong icon field type):
- `Tool.icon` -> should be `icons?: Icon[]`
- `Resource.icon` -> should be `icons?: Icon[]`
- `Prompt.icon` -> should be `icons?: Icon[]`
- `ResourceTemplate.icon` -> should be `icons?: Icon[]`
- `Implementation.icon` -> should be `icons?: Icon[]`

### 3. Implementation Type: Missing Fields

**File**: `src/initialize.rs`

| Field | Spec | Our Implementation | Status |
|-------|------|--------------------|--------|
| `name` | `string` (REQUIRED) | `name: String` | OK |
| `version` | `string` (REQUIRED) | `version: String` | OK |
| `title` | `string?` (optional) | `title: Option<String>` | OK |
| `description` | `string?` (optional) | MISSING | MISSING |
| `websiteUrl` | `string?` (optional, URI) | MISSING | MISSING |
| `icons` | `Icon[]?` (optional) | `icon: Option<IconUrl>` (WRONG) | WRONG TYPE & NAME |

### 4. ModelHint Type: Wrong Structure

**File**: `src/sampling.rs`

The spec defines `ModelHint` as an **object** with `name?: string`, NOT a fixed enum of model names:

```typescript
// Spec
interface ModelHint {
  name?: string; // substring to match against model names
}
```

Our implementation uses a closed enum with hardcoded model names (`Claude35Sonnet20241022`, `Gpt4o`, etc.), which is completely wrong. `ModelHint` should be a struct with an optional `name` field.

### 5. CreateMessageParams: Missing Fields

**File**: `src/sampling.rs`

| Field | Spec | Our Implementation | Status |
|-------|------|--------------------|--------|
| `messages` | `SamplingMessage[]` | OK | OK |
| `modelPreferences` | `ModelPreferences?` | OK | OK |
| `systemPrompt` | `string?` | OK | OK |
| `includeContext` | `"none" \| "thisServer" \| "allServers"?` | `Option<String>` | SHOULD BE ENUM |
| `temperature` | `number?` | OK | OK |
| `maxTokens` | `number` | OK | OK |
| `stopSequences` | `string[]?` | OK | OK |
| `metadata` | `object?` | OK | OK |
| `tools` | `Tool[]?` | OK | OK |
| `toolChoice` | `ToolChoice?` | MISSING | MISSING |
| `task` | `TaskMetadata?` | MISSING | MISSING |
| `_meta` | `object?` | OK | OK |

**Missing types**: `ToolChoice` with `mode?: "auto" | "required" | "none"` is not implemented.

### 6. CreateMessageResult: Content Can Be Array

**File**: `src/sampling.rs`

The spec says `content: SamplingMessageContentBlock | SamplingMessageContentBlock[]` - it can be either a single content block OR an array. Our `SamplingMessage` uses `content: ContentBlock` (single only).

### 7. Elicitation: Missing URL Mode

**File**: `src/elicitation.rs`

The spec has TWO elicitation modes:
1. **Form mode** (`ElicitRequestFormParams`): `mode?: "form"`, `message`, `requestedSchema` - Partially implemented
2. **URL mode** (`ElicitRequestURLParams`): `mode: "url"`, `message`, `elicitationId`, `url` - NOT implemented

Our `ElicitCreateParams` only supports form mode. The `ElicitRequestParams` should be a union of form and URL params. The `mode` field is missing from our form params.

---

## MODERATE Issues

### 8. ServerCapabilities.tasks: Wrong Sub-Fields

**File**: `src/initialize.rs`

The spec's `tasks` capability has specific sub-fields:
```typescript
tasks?: {
  list?: {};     // supports tasks/list
  cancel?: {};   // supports tasks/cancel
  requests?: {}; // supports tasks/result (payload retrieval)
}
```

Our `TasksCapabilities` only has `listChanged: Option<bool>`, which doesn't match.

### 9. Sampling Role Enum: Includes "System"

**File**: `src/sampling.rs`

The spec's `Role` for sampling is `"user" | "assistant"` (no `"system"`). Our `Role` enum includes `System`, which is not in the MCP sampling spec. System prompts go in the `systemPrompt` field, not as a message role.

### 10. StringSchema: Missing `default` Field

**File**: `src/elicitation.rs`

The spec's `StringSchema` has an optional `default?: string` field that is missing from our implementation.

### 11. Elicitation Content Type

The spec says `ElicitResult.content` values can be `string | number | boolean | string[]`, not `Value` (arbitrary JSON).

### 12. Missing TaskStatusNotification

**File**: `src/notifications.rs`

The spec defines `notifications/tasks/status` with `TaskStatusNotificationParams` (extends Task). This notification type is not implemented at all.

---

## MINOR Issues

### 13. Elicitation Form Mode Field

The form mode params should have `mode?: "form"` as an optional field to distinguish from URL mode.

### 14. Tasks Create Method

The spec doesn't actually define `tasks/create` as a standalone method. Task creation happens implicitly when a request returns a Task result. The spec only defines `tasks/get`, `tasks/result`, `tasks/cancel`, and `tasks/list`. Our `CreateTaskRequest`/`CreateTaskResult` types may not be needed.

### 15. MCP_VERSION Constant Correctness

`MCP_VERSION = "2025-11-25"` and `McpVersion::CURRENT = V2025_11_25` are correct and match the spec.

---

## Passing Areas (No Issues)

The following areas are correctly implemented:

1. **camelCase serialization**: All structs use `#[serde(rename_all = "camelCase")]` or explicit `#[serde(rename = "...")]` annotations. No snake_case JSON keys detected.

2. **Optional field handling**: All optional fields use `#[serde(skip_serializing_if = "Option::is_none")]` correctly.

3. **_meta field handling**: All types properly use `#[serde(rename = "_meta")]` for the meta field.

4. **Tool types** (excluding icons): `Tool`, `ToolAnnotations`, `ToolSchema`, `CallToolRequest/Result`, `ListToolsRequest/Result` all match the spec correctly.

5. **Resource types** (excluding icons): `Resource`, `ResourceTemplate`, list/read request/result types match.

6. **Prompt types** (excluding icons): `Prompt`, `PromptArgument`, list/get request/result types match.

7. **Elicitation basics**: Form mode schema types (`StringSchema`, `NumberSchema`, `BooleanSchema`, `EnumSchema`, `PrimitiveSchemaDefinition`) match the spec.

8. **StringFormat enum**: `"email" | "uri" | "date" | "date-time"` values are correct.

9. **ElicitAction enum**: `"accept" | "decline" | "cancel"` values are correct.

10. **Version negotiation**: `McpVersion` enum and parsing are correct.

11. **Content types**: `ContentBlock` variants (text, image, audio, resource_link, resource) match the spec.

12. **Trait implementations**: All existing types have complete `HasMethod`, `HasParams`, `HasData`, `HasMeta` implementations.

---

## Recommended Fix Priority

1. **P0 (Blocking)**: Fix Task types (#1) - wrong field names, wrong enum values, missing required fields
2. **P0 (Blocking)**: Fix Icon type (#2) - completely wrong structure (string vs object array)
3. **P1 (High)**: Fix ModelHint (#4) - enum vs struct mismatch
4. **P1 (High)**: Fix Implementation fields (#3) - missing description, websiteUrl
5. **P1 (High)**: Add TaskStatusNotification (#12)
6. **P1 (High)**: Fix ServerCapabilities.tasks (#8)
7. **P2 (Medium)**: Add ToolChoice, task field to CreateMessageParams (#5)
8. **P2 (Medium)**: Add URL elicitation mode (#7)
9. **P2 (Medium)**: Fix sampling Role enum (#9)
10. **P3 (Low)**: Fix content array support in CreateMessageResult (#6)
11. **P3 (Low)**: Add StringSchema.default (#10)
12. **P3 (Low)**: Fix elicitation content types (#11)
