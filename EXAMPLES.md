# MCP Framework Examples

This document provides a comprehensive overview of all **54 active examples** in the MCP Framework (plus `examples/archived/` — grown by 5 in the 2026-06-12 archive slice; the disposition review lives in git history), organized by learning progression from basic concepts to advanced implementations.

**✅ All active examples compile under their lane's CI gates** (2026-07-28 default lane
+ per-manifest 2025-11-25 pins). Per-example functional re-verification against the
2026 stateless core was reviewed example-by-example in the 2026-06-12 disposition slices (review doc in git history) — the
2026-02-26 "phases 1-5" sweep predates the 2026 cutover and no longer applies as stated.
**Last verified**: 2026-06-12 (v0.4.0 branch — MCP 2026-07-28 default, 2025-11-25 opt-in)

## Client ↔ Server Example Pairs

Every client example has a corresponding server example **on the same spec
lane** — run the server first, then point the client at it.

| Client example | Lane | Corresponding server | Notes |
|---|---|---|---|
| `streamable-http-client` | **2026-07-28** | `minimal-server` (port 8641); point at `notification-server` (port 8005) for live subscription deliveries | The canonical 2026 stateless pair: `connect()` negotiation, discover retention, `call_tool`, request-scoped progress, and the ack-first `subscriptions_listen` stream |
| `mrtr-elicitation-client` (in `mrtr-elicitation-server`) | **2026-07-28** | `mrtr-elicitation-server` (port 8642) | The MRTR round trip: `input_required` → answer → `call_tool_with_input_responses` |
| `ext-tasks-client` (in `ext-tasks-server`) | **2026-07-28** | `ext-tasks-server` (port 8645) | SEP-2663 task lifecycle: `call_tool_or_task` → poll → `tasks/update` → completed; sync fallback for undeclared clients |
| `bilingual-fleet-client` | **both** | any mix (demo: `minimal-server` + `client-initialise-server`) | One binary sweeping a mixed 2025/2026 fleet — per-connection negotiation |
| `streamable-http-client-2025-11-25` | 2025-11-25 | `client-initialise-server` (alternatives: any 2025-pinned server) | Hand-parsed 2025 POST SSE framing |
| `client-initialise-report` | 2025-11-25 (wire-pinned) | `client-initialise-server` | Raw-wire lifecycle compliance probe |
| `session-management-compliance-test` | 2025-11-25 (wire-pinned) | `client-initialise-server` | Session-contract regression client |
| `logging-test-client` | 2025-11-25 | `logging-test-server` | `logging/setLevel` wire contract |
| `tasks-e2e-inmemory-client` | 2025-11-25 | `tasks-e2e-inmemory-server` | 2025 task lifecycle driver |
| `lambda-mcp-client` | 2025-11-25 (via negotiation) | `lambda-mcp-server` (2025-pinned) | The bilingual client falls back to 2025 against this peer |

**Legend**:
- ✅ **Verified Working** - Tested and confirmed functional
- ⚙️ **Requires Setup** - External dependencies needed
- 🎓 **Educational** - Teaches manual implementation patterns
- 🚀 **Production Ready** - Uses optimized macros
- 🔧 **Builder Pattern** - Runtime construction

## 🟢 **GETTING STARTED** (5 examples) - Start Here

**Complete Calculator Learning Suite** - Four progressive levels of MCP tool implementation:

| Example | Port | Status | Learning Level | Description |
|---------|------|--------|----------------|-------------|
| **minimal-server** 🚀 | 8641 | ✅ WORKING | Foundation | Simplest possible MCP server with echo tool |
| **calculator-add-function-server** 🚀 | 8648 | ✅ WORKING | Level 1 - Ultra Simple | Function macro `#[mcp_tool]` |
| **calculator-add-simple-server-derive** 🚀 | 8647 | ✅ WORKING | Level 2 - Most Common | Derive macro `#[derive(McpTool)]` |
| **calculator-add-builder-server** 🔧 | 8649 | ✅ WORKING | Level 3 - Runtime | Builder pattern construction |
| **calculator-add-manual-server** 🎓 | 8646 | ✅ WORKING | Level 4 - Full Control | Manual trait implementation |

**Quick Start Command**:
```bash
# Start with the minimal server
cargo run -p minimal-server
# Server: http://127.0.0.1:8641/mcp
```

## 🟢 **SESSION STORAGE** (3 examples) - Persistent State

| Example | Port | Status | Description | Use Case |
|---------|------|--------|-------------|----------|
| **simple-sqlite-session** | 8061 | ✅ WORKING | File-based persistence | Single-instance deployments |
| **simple-postgres-session** | 8060 | ⚙️ REQUIRES_SETUP | Database-backed sessions | Production multi-instance |
| **simple-dynamodb-session** | 8062 | ⚙️ REQUIRES_SETUP | AWS cloud sessions | Serverless deployments |

**Setup Requirements**:
- **PostgreSQL**: Requires Docker container (instructions in example)
- **DynamoDB**: Requires AWS credentials configuration

## 🟡 **RESOURCE SERVERS** - Resource Handling & Session-Aware (2025 lane)

| Example | Port | Status | Description | Key Features |
|---------|------|--------|-------------|--------------|
| **resource-server** | 8007 | ✅ VALIDATED | Resource macros | `#[derive(McpResource)]` with session context |
| **resources-server** | 8041 | ✅ VALIDATED | Multiple resource types | Resource handling patterns |
| **resource-test-server** | 8043 | ✅ VALIDATED | Resource testing | Resource validation framework |
| **function-resource-server** | 8008 | ✅ VALIDATED | Function-based resources | Resource function patterns |
| **session-aware-resource-server** | 8008 | ✅ VALIDATED | Session-aware resources (2025-11-25 pinned) | Session context integration on the stateful lane |

## 🟢 **FEATURE-SPECIFIC SERVERS** (14 examples) - Specialized MCP Features

| Example | Port | Status | Description | Key Features |
|---------|------|--------|-------------|--------------|
| **prompts-server** | 8006 | ✅ VALIDATED | Prompt handling | MCP prompts feature demonstration |
| **prompts-test-server** | 8046 | ✅ VALIDATED | Prompt validation | Prompts testing and validation |
| **completion-server** | 8042 | ✅ VALIDATED | Text completion | IDE completion integration |
| **sampling-server** | 8044 | ✅ VALIDATED | Data sampling | LLM sampling feature support |
| **elicitation-server** | 8047 | ✅ VALIDATED | Information gathering | User input elicitation patterns |
| **pagination-server** | 8044 | ✅ VALIDATED | Result pagination | Large dataset pagination support |
| **notification-server** | 8005 | ✅ VALIDATED | Notifications (2026: subscriptions/listen stream) | Real-time notification patterns |
| **roots-server** | 8050 | ✅ VALIDATED | Root directories | MCP roots/list endpoint demonstration |
| **mrtr-elicitation-server** | 8642 | ✅ VALIDATED | MRTR elicitation round trip (2026) | `InputRequired` → retry with `inputResponses`; paired client bin |
| **origin-policy-server** | 8643 | ✅ VALIDATED | Origin validation / DNS-rebinding (2026) | `OriginPolicy` default/AllowList/Disabled; 403 matrix verified live |
| **header-bound-tools-server** | 8644 | ✅ VALIDATED | SEP-2243 header-bound tool params (2026) | `x-mcp-header` → `Mcp-Param-*` mirroring; -32020 contract verified live |
| **ext-tasks-server** | 8645 | ✅ VALIDATED | Tasks extension (SEP-2663, 2026) | Task election + polling + `tasks/update` mid-task input; paired client bin; live-verified |
| **oauth-resource-server** | 8080 | ✅ VALIDATED | OAuth 2.1 Resource Server (RFC 9728) | JWKS Bearer validation, PRM well-known routes, `--required-scope` → 403 insufficient_scope |
| **dynamic-tools-server** | 8484 | ✅ VALIDATED | Dynamic tool registration (2025-pinned) | `ToolRegistry` + `notifications/tools/list_changed` on the stateful lane |

## 🔵 **ADVANCED/COMPOSITE SERVERS** (2 examples) - Complex Functionality

| Example | Port | Status | Description | Advanced Features |
|---------|------|--------|-------------|-------------------|
| **audit-trail-server** | 8009 | ✅ VALIDATED | Audit logging | Comprehensive audit logging system |
| **zero-config-getting-started** | 8641 | ✅ VALIDATED | Zero-configuration setup | Getting started tutorial server |

## 🔴 **SESSION & STATE** (3 examples) - Advanced State Handling (2025-pinned lane)

| Example | Port | Status | Description | Session Features |
|---------|------|--------|-------------|------------------|
| **stateful-server** | 8006 | ✅ VALIDATED | Advanced stateful operations (2025-11-25 pinned) | Session state management on the stateful lane |
| **session-logging-proof-test** | 8001 | ✅ VALIDATED | Session logging validation | Session-based logging verification |
| **logging-test-server** | 8052 | ✅ VALIDATED | Logging test suite | Comprehensive logging test suite |

## 🟠 **CLIENT EXAMPLES** (7 entries) - Client Implementation

| Example | Type | Status | Description | Purpose |
|---------|------|--------|-------------|---------|
| **client-initialise-server** | Server | ✅ VALIDATED | Client connectivity test server | MCP session initialization testing |
| **client-initialise-report** | Client | ✅ VALIDATED | MCP client implementation | Tests server initialization |
| **streamable-http-client** | Client | ✅ VALIDATED | 2026-07-28 stateless client (pairs with minimal-server) | connect() negotiation, discover retention, request-scoped progress API |
| **streamable-http-client-2025-11-25** | Client | ✅ VALIDATED | Hand-parsed 2025 Streamable HTTP client | MCP 2025-11-25 streaming demo |
| **bilingual-fleet-client** | Client | ✅ VALIDATED | Mixed 2025/2026 fleet sweep | Per-connection negotiation (server/discover → initialize fallback) |
| **logging-test-client** | Client | ✅ VALIDATED | Logging client | Tests logging functionality |
| **session-management-compliance-test** | Combined | ✅ VALIDATED | Session compliance testing | MCP session spec compliance |

**Client Testing**:
```bash
# Start the test server
cargo run -p client-initialise-server

# Test with client (in another terminal)
cargo run -p client-initialise-report -- --url http://127.0.0.1:8641/mcp
```

## ☁️ **AWS LAMBDA** (3 examples) - Serverless Deployment (middleware-auth-lambda is listed under Middleware)

| Example | Type | Status | Description | AWS Features |
|---------|------|--------|-------------|--------------|
| **lambda-mcp-server** | Lambda | ✅ VALIDATED | Serverless MCP server | Basic Lambda deployment |
| **lambda-mcp-client** | Lambda Client | ✅ VALIDATED | Lambda MCP client | AWS Lambda client integration |
| **lambda-authorizer** | Lambda | ✅ VALIDATED | API Gateway authorizer | REQUEST authorizer with wildcard methodArn for MCP |

## 🟣 **TOOL CREATION & OUTPUT SCHEMAS** (5 examples) - Tool Patterns

| Example | Port | Status | Description | Key Features |
|---------|------|--------|-------------|--------------|
| **derive-macro-server** | 8765 | ✅ VALIDATED | Derive macro tools | `#[derive(McpTool)]` with code generation tools |
| **function-macro-server** | 8003 | ✅ VALIDATED | Function macro tools | `#[mcp_tool]` attribute macro patterns |
| **tools-test-server** | random | ✅ VALIDATED | Comprehensive tool testing | All MCP tool patterns and edge cases |
| **tool-output-introspection** | 8641 | ✅ VALIDATED | Output schema via introspection | Automatic field-level output schema generation |
| **tool-output-schemas** | 8641 | ✅ VALIDATED | Output schema via schemars | `schemars::JsonSchema` derive for JSON Schema output |

## 🛡️ **MIDDLEWARE** (4 examples) - Request Processing Pipelines

| Example | Port | Status | Description | Middleware Pattern |
|---------|------|--------|-------------|-------------------|
| **middleware-auth-server** | 8080 | ✅ VALIDATED | API key authentication | `before_dispatch` header extraction |
| **middleware-logging-server** | 8670 | ✅ VALIDATED | Request timing/tracing | Request duration logging in `after_dispatch` |
| **middleware-rate-limit-server** | 8671 | ✅ VALIDATED | Rate limiting | Per-session request counting |
| **middleware-auth-lambda** | Lambda | ✅ VALIDATED | Lambda auth middleware | API Gateway authorizer context (V1 nested, V1 flat, V2) with Streamable HTTP (REST API V1) |

## 🔄 **TASKS (MCP 2025-11-25)** (2 examples) - Long-Running Operations

| Example | Type | Status | Description | Task Features |
|---------|------|--------|-------------|---------------|
| **tasks-e2e-inmemory-server** | Server | ✅ VALIDATED | Task-enabled MCP server | `slow_add` tool with configurable delay, InMemory storage |
| **tasks-e2e-inmemory-client** | Client | ✅ VALIDATED | Task lifecycle client | Full task lifecycle: create, poll, cancel, result |

**Task E2E Testing**:
```bash
# Start the task-enabled server
cargo run -p tasks-e2e-inmemory-server

# Run the client test suite (in another terminal)
cargo run -p tasks-e2e-inmemory-client -- --url http://127.0.0.1:8080/mcp
```

## 📖 **TYPE SHOWCASES** (1 example) - Print-Only Demonstrations

These examples demonstrate MCP 2025-11-25 type construction without starting a server:

| Example | Type | Status | Description | Types Demonstrated |
|---------|------|--------|-------------|-------------------|
| **icon-showcase** | Demo | ✅ VALIDATED | Icon support | `Icon` struct on tools, resources, prompts |

## 🚨 **COMPREHENSIVE VALIDATION RESULTS**

### ✅ **ALL 54 ACTIVE EXAMPLES COMPILE UNDER THEIR LANE'S CI GATES**
**0.4.0 branch (MCP 2026-07-28 default, 2025-11-25 opt-in) — last reconciled 2026-06-12.**
The per-example functional verification ledger lives in
the 2026-06-12 review (in git history); the migrate slice
re-verified its six examples live on the wire.

- **Getting Started** - 5 examples (all tool creation levels)
- **Session Storage** - 3 examples (SQLite, PostgreSQL, DynamoDB)
- **Resource Servers** - 6 examples (session-aware resources)
- **Feature-Specific** - 14 examples (prompts, sampling, elicitation, MRTR, tasks extension, origin policy, header binding, OAuth RS, etc.)
- **Advanced/Composite** - audit-trail, pagination, icon showcase
- **Session & State** - stateful-server + session-aware-resource-server (2025-pinned) and the storage-backend trio
- **Client Examples** - see the pairs table above (9 pairing rows)
- **AWS Lambda** - 3 examples (server, client, authorizer) + middleware-auth-lambda under Middleware
- **Tool Creation & Schemas** - 5 examples (macro patterns + output schemas)
- **Middleware** - 4 examples (auth, logging, rate-limiting, Lambda auth)
- **Tasks** - 2 examples (MCP 2025-11-25 task lifecycle)
- **Type Showcases** - 1 example (print-only type demonstrations)

> Lane-by-lane compilation is enforced by CI (`scripts/ci-gates.sh all`); the
> 2026-06-12 disposition slices live-verified every migrated and new example.
> Historical 0.3-era per-example verification notes live in the v0.3.x tags
> (`EXAMPLE_VERIFICATION_LOG.md`, deleted in the 0.4 docs purge).

### 🎯 **CURRENT STATE**
- **2026-07-28 default lane**: stateless core (per-request `_meta`, POST-only, `subscriptions/listen`, MRTR)
- **2025-11-25 opt-in lane**: stateful session examples pinned per-manifest, feeding the regression gates
- **Live-verified**: migrated and new examples were verified by running them and executing their own printed commands

### 📊 **Statistics**
- **Total Examples**: 54 active (29 archived in `examples/archived/`)
- **Session state (2025-pinned lane)**: stateful-server, session-aware-resource-server, and the logging/session test fixtures demonstrate cross-request session state on the opt-in lane; 2026-default examples use request-scoped context or app-owned storage instead
- **Client-Server Pairs**: 9 pairing-table rows validating communication patterns
- **Task Support**: 3 examples demonstrating MCP 2025-11-25 task lifecycle (InMemory storage)
- **Middleware**: 4 examples (HTTP auth, logging, rate-limiting, Lambda auth)
- **Storage Backends**: All 4 session backends (InMemory, SQLite, PostgreSQL, DynamoDB) working
- **AWS Lambda Integration**: 3 examples (server, client, API Gateway authorizer) + middleware-auth-lambda

### 🔧 **Running Examples**

**Basic Pattern**:
```bash
# Run any example (examples are packages with [[bin]] targets — use -p)
cargo run -p <example-name>

# Examples with custom ports
cargo run -p client-initialise-server -- --port 8641
```

**With Features** (for PostgreSQL/DynamoDB examples):
```bash
cargo run --features postgres --example simple-postgres-session
cargo run --features dynamodb --example simple-dynamodb-session
```

> **Note**: Some examples share default ports (e.g., 8641, 8008). Run only one example per port at a time.

### 📝 **Development Notes**
- All examples use the latest framework patterns
- Session management is enabled by default
- SSE notifications available on all HTTP servers
- Error handling demonstrates proper MCP error types

---

**📋 Framework Status**: 0.4.0 branch (`feat/turul-mcp-protocol-2026-07-28`) — MCP 2026-07-28 stateless core by default with the 2025-11-25 stateful line as a per-manifest opt-in. Registered spec-gap status lives in `docs/plans/2026-07-28-spec-compliance.md`.