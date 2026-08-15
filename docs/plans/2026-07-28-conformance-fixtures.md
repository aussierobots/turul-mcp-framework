# MCP conformance — server fixture requirements (harvested)

**Machine-harvested, not hand-written.** Every block below is the verbatim
"Server Implementation Requirements" text that
`@modelcontextprotocol/conformance@0.2.0-alpha.11` prints for each of its **62
server scenarios**. Regenerate rather than edit:

```bash
npx -y @modelcontextprotocol/conformance@0.2.0-alpha.11 list        # scenario names
npx -y @modelcontextprotocol/conformance@0.2.0-alpha.11 server \
    --url http://127.0.0.1:1/mcp --scenario <name>                 # its requirements
```

## Why this file exists

Upstream ships a conformance suite that runs against any server URL and carries
**75 scenarios tagged 2026-07-28** plus 10 for the tasks extension. It is the
strongest compliance instrument available to this project, because the scenarios
are authored by the spec maintainers rather than by us — unlike every test in
`tests/`, it can disagree with our reading of the spec.

> ## STATUS 2026-08-15 — superseded in part. Read this before the rest.
>
> The fixture server exists: `examples/conformance-fixture-server`. It passes
> **37 of 37 scored scenarios** for `--requirements 2026-07-28`. The score is
> recorded in [`docs/compliance/README.md`](../compliance/README.md), which
> retires the "do not record a conformance score anywhere" instruction that
> used to sit here — it was correct until the server existed and is now
> satisfied.
>
> **Three corrections to this file, which is harvested and therefore partial.**
> Prefer the harness over anything below it:
>
> 1. **The inventory of "27 fixtures" is wrong — the harness references 44.**
>    Extract them with
>    `grep -o 'test_[a-z_0-9]*' package/dist/index.js | sort -u` after
>    `npm pack @modelcontextprotocol/conformance@0.2.0-alpha.11`.
> 2. **Some names below are wrong.** This file says `test_tool_with_logging`;
>    the harness wants `test_logging_tool`. When a scenario fails with
>    `Unknown tool: X`, **X is authoritative.**
> 3. **`dns-rebinding-protection` was NOT a harness artifact.** This file
>    recorded it as one, on the strength of a hand-check that varied `Origin`
>    while leaving `Host` truthful — the single combination that was rejected.
>    Setting *both* to `evil.example.com` returned **200**. It was a real
>    DNS-rebinding vulnerability, fixed in `683b925`, and the mistaken
>    all-clear here is exactly why a hand-check is not a substitute for the
>    suite.
>
> **Where the real ground truth lives:** the published package's
> `dist/index.js` carries every scenario's assertions inline, including for the
> nine that print no requirements. Read it rather than inferring.

Pointing the suite at `minimal-server` on 2026-08-15 produced 101 passed / 68
failed, and that number meant nothing — most failures were absent fixtures.

This file remains useful as the verbatim requirement text for the scenarios
that print it.

## Caveat: 9 scenarios print no requirements

`server-sse-polling`, `server-sse-multiple-streams`, `dns-rebinding-protection`,
`tasks-capability-negotiation`, `tasks-status-notifications`,
`input-required-result-missing-input-response`,
`input-required-result-unsupported-methods`,
`input-required-result-ignore-extra-params`, `input-required-result-validate-input`.

These test transport or protocol behaviour rather than fixture content. Their
expectations must be read from the scenario source in the conformance repo, not
inferred from silence here.

---

## `server-initialize`

Endpoint: initialize

Requirements:
- Accept initialize request with client info and capabilities
- Return valid initialize response with server info, protocol version, and capabilities
- Accept initialized notification from client after handshake
- If a session ID is assigned, it MUST only contain visible ASCII characters (0x21 to 0x7E)

This test verifies the server can complete the two-phase initialization handshake successfully,
and validates session ID format if one is assigned.

## `server-session-lifecycle`

- Accept an HTTP DELETE to the MCP endpoint that carries the issued
  Mcp-Session-Id header (or respond 405 if the server does not support
  explicit termination). The spec does not pin a success status, so any
  other response is reported as a warning rather than a failure.
- After such a DELETE, return HTTP 404 Not Found for subsequent requests
  bearing the terminated session ID.

Servers without session management (stateless) are reported as SKIPPED.

## `server-stateless`

Endpoints:
- server/discover: Returns supportedVersions and capabilities; SHOULD identify itself via _meta['io.modelcontextprotocol/serverInfo'] (spec PR #3002).
- tools/call: Implement structural test tools like test_missing_capability requiring explicit capabilities in _meta.

Grouped Specification Requirements:

1. Per-Request _meta Validation (4 Checks)
   - Rejects requests missing _meta or lacking structural required internal subfields (protocolVersion, clientCapabilities) with a JSON-RPC -32602 Invalid params error signature and an HTTP status code 400 Bad Request.
   - Serves requests whose _meta omits clientInfo (a SHOULD since spec PR #3002 — servers MUST NOT require it).
2. Discovery & Capabilities (3 Checks)
   - Implements server/discover mapping exact mandatory protocol elements.
   - Dynamically checks prompt capability declaration constraints, validates that active RPC handlers match advertised discovery capacities.
3. Version Negotiation & Headers (3 Checks)
   - Mismatched or unknown protocol versions must return an UnsupportedProtocolVersionError (HTTP status code 400 Bad Request) carrying precise version tracking arrays.
   - Absent or altered protocol version header metadata must trigger a -32020 Header Mismatch error with an HTTP 400 boundary state.
4. Client Capability Constraints (2 Checks)
   - Accessing platform capabilities without explicit declaration drops requests with a -32021 MissingRequiredClientCapabilityError returning an HTTP status code 400 Bad Request. Its error.data.requiredCapabilities is a ClientCapabilities object keyed by the missing capability (e.g. { "sampling": {} }), not an array of names.
5. Methods & Routing Mechanics (5 Checks)
   - Removed legacy endpoints (initialize, ping, logging/setLevel, etc.) or generic unknown methods must cleanly yield an HTTP status code 404 Not Found alongside a JSON-RPC -32601 Method not found payload. All error returns must preserve original request ID mappings.
   - Validates response streams contain only IncompleteResult chunks and never independent top-level JSON-RPC requests, while enforcing that no log messages are emitted when _meta.../logLevel is omitted.
6. Subscription Streams & Filtering (3 Checks)
   - Mandates that notifications/subscriptions/acknowledged is the first message on a subscriptions/listen stream, and that subsequent notifications carry a matching _meta.../subscriptionId.
   - Verifies strict containment where servers do not dispatch notification types that fall outside the client's explicit requested subscription filter list.
7. Dynamic List Mutations (2 Checks)
   - Evaluates that list-changed capable servers notify active listen streams with promptsListChanged: true or toolsListChanged: true upon live configuration or capability modifications.  

  - HttpServerMetaInvalid400: Rejections of requests missing required _meta fields use HTTP 400 Bad Request.
    Error: _meta validation probe failed completely

Test stateless MCP server architecture (SEP-2575).



Endpoints:
- server/discover: Returns supportedVersions and capabilities; SHOULD identify itself via _meta['io.modelcontextprotocol/serverInfo'] (spec PR #3002).
- tools/call: Implement structural test tools like test_missing_capability requiring explicit capabilities in _meta.

Grouped Specification Requirements:

1. Per-Request _meta Validation (4 Checks)
   - Rejects requests missing _meta or lacking structural required internal subfields (protocolVersion, clientCapabilities) with a JSON-RPC -32602 Invalid params error signature and an HTTP status code 400 Bad Request.
   - Serves requests whose _meta omits clientInfo (a SHOULD since spec PR #3002 — servers MUST NOT require it).
2. Discovery & Capabilities (3 Checks)
   - Implements server/discover mapping exact mandatory protocol elements.
   - Dynamically checks prompt capability declaration constraints, validates that active RPC handlers match advertised discovery capacities.
3. Version Negotiation & Headers (3 Checks)
   - Mismatched or unknown protocol versions must return an UnsupportedProtocolVersionError (HTTP status code 400 Bad Request) carrying precise version tracking arrays.
   - Absent or altered protocol version header metadata must trigger a -32020 Header Mismatch error with an HTTP 400 boundary state.
4. Client Capability Constraints (2 Checks)
   - Accessing platform capabilities without explicit declaration drops requests with a -32021 MissingRequiredClientCapabilityError returning an HTTP status code 400 Bad Request. Its error.data.requiredCapabilities is a ClientCapabilities object keyed by the missing capability (e.g. { "sampling": {} }), not an array of names.
5. Methods & Routing Mechanics (5 Checks)
   - Removed legacy endpoints (initialize, ping, logging/setLevel, etc.) or generic unknown methods must cleanly yield an HTTP status code 404 Not Found alongside a JSON-RPC -32601 Method not found payload. All error returns must preserve original request ID mappings.
   - Validates response streams contain only IncompleteResult chunks and never independent top-level JSON-RPC requests, while enforcing that no log messages are emitted when _meta.../logLevel is omitted.
6. Subscription Streams & Filtering (3 Checks)
   - Mandates that notifications/subscriptions/acknowledged is the first message on a subscriptions/listen stream, and that subsequent notifications carry a matching _meta.../subscriptionId.
   - Verifies strict containment where servers do not dispatch notification types that fall outside the client's explicit requested subscription filter list.
7. Dynamic List Mutations (2 Checks)
   - Evaluates that list-changed capable servers notify active listen streams with promptsListChanged: true or toolsListChanged: true upon live configuration or capability modifications.  

  - RequestMetaInvalid: Rejects request with _meta missing io.modelcontextprotocol/protocolVersion
    Error: _meta validation probe failed completely

Test stateless MCP server architecture (SEP-2575).



Endpoints:
- server/discover: Returns supportedVersions and capabilities; SHOULD identify itself via _meta['io.modelcontextprotocol/serverInfo'] (spec PR #3002).
- tools/call: Implement structural test tools like test_missing_capability requiring explicit capabilities in _meta.

Grouped Specification Requirements:

1. Per-Request _meta Validation (4 Checks)
   - Rejects requests missing _meta or lacking structural required internal subfields (protocolVersion, clientCapabilities) with a JSON-RPC -32602 Invalid params error signature and an HTTP status code 400 Bad Request.
   - Serves requests whose _meta omits clientInfo (a SHOULD since spec PR #3002 — servers MUST NOT require it).
2. Discovery & Capabilities (3 Checks)
   - Implements server/discover mapping exact mandatory protocol elements.
   - Dynamically checks prompt capability declaration constraints, validates that active RPC handlers match advertised discovery capacities.
3. Version Negotiation & Headers (3 Checks)
   - Mismatched or unknown protocol versions must return an UnsupportedProtocolVersionError (HTTP status code 400 Bad Request) carrying precise version tracking arrays.
   - Absent or altered protocol version header metadata must trigger a -32020 Header Mismatch error with an HTTP 400 boundary state.
4. Client Capability Constraints (2 Checks)
   - Accessing platform capabilities without explicit declaration drops requests with a -32021 MissingRequiredClientCapabilityError returning an HTTP status code 400 Bad Request. Its error.data.requiredCapabilities is a ClientCapabilities object keyed by the missing capability (e.g. { "sampling": {} }), not an array of names.
5. Methods & Routing Mechanics (5 Checks)
   - Removed legacy endpoints (initialize, ping, logging/setLevel, etc.) or generic unknown methods must cleanly yield an HTTP status code 404 Not Found alongside a JSON-RPC -32601 Method not found payload. All error returns must preserve original request ID mappings.
   - Validates response streams contain only IncompleteResult chunks and never independent top-level JSON-RPC requests, while enforcing that no log messages are emitted when _meta.../logLevel is omitted.
6. Subscription Streams & Filtering (3 Checks)
   - Mandates that notifications/subscriptions/acknowledged is the first message on a subscriptions/listen stream, and that subsequent notifications carry a matching _meta.../subscriptionId.
   - Verifies strict containment where servers do not dispatch notification types that fall outside the client's explicit requested subscription filter list.
7. Dynamic List Mutations (2 Checks)
   - Evaluates that list-changed capable servers notify active listen streams with promptsListChanged: true or toolsListChanged: true upon live configuration or capability modifications.  

  - HttpServerMetaInvalid400: Rejections of requests missing required _meta fields use HTTP 400 Bad Request.
    Error: _meta validation probe failed completely

Test stateless MCP server architecture (SEP-2575).



Endpoints:
- server/discover: Returns supportedVersions and capabilities; SHOULD identify itself via _meta['io.modelcontextprotocol/serverInfo'] (spec PR #3002).
- tools/call: Implement structural test tools like test_missing_capability requiring explicit capabilities in _meta.

Grouped Specification Requirements:

1. Per-Request _meta Validation (4 Checks)
   - Rejects requests missing _meta or lacking structural required internal subfields (protocolVersion, clientCapabilities) with a JSON-RPC -32602 Invalid params error signature and an HTTP status code 400 Bad Request.
   - Serves requests whose _meta omits clientInfo (a SHOULD since spec PR #3002 — servers MUST NOT require it).
2. Discovery & Capabilities (3 Checks)
   - Implements server/discover mapping exact mandatory protocol elements.
   - Dynamically checks prompt capability declaration constraints, validates that active RPC handlers match advertised discovery capacities.
3. Version Negotiation & Headers (3 Checks)
   - Mismatched or unknown protocol versions must return an UnsupportedProtocolVersionError (HTTP status code 400 Bad Request) carrying precise version tracking arrays.
   - Absent or altered protocol version header metadata must trigger a -32020 Header Mismatch error with an HTTP 400 boundary state.
4. Client Capability Constraints (2 Checks)
   - Accessing platform capabilities without explicit declaration drops requests with a -32021 MissingRequiredClientCapabilityError returning an HTTP status code 400 Bad Request. Its error.data.requiredCapabilities is a ClientCapabilities object keyed by the missing capability (e.g. { "sampling": {} }), not an array of names.
5. Methods & Routing Mechanics (5 Checks)
   - Removed legacy endpoints (initialize, ping, logging/setLevel, etc.) or generic unknown methods must cleanly yield an HTTP status code 404 Not Found alongside a JSON-RPC -32601 Method not found payload. All error returns must preserve original request ID mappings.
   - Validates response streams contain only IncompleteResult chunks and never independent top-level JSON-RPC requests, while enforcing that no log messages are emitted when _meta.../logLevel is omitted.
6. Subscription Streams & Filtering (3 Checks)
   - Mandates that notifications/subscriptions/acknowledged is the first message on a subscriptions/listen stream, and that subsequent notifications carry a matching _meta.../subscriptionId.
   - Verifies strict containment where servers do not dispatch notification types that fall outside the client's explicit requested subscription filter list.
7. Dynamic List Mutations (2 Checks)
   - Evaluates that list-changed capable servers notify active listen streams with promptsListChanged: true or toolsListChanged: true upon live configuration or capability modifications.  

  - RequestMetaInvalid: Rejects request with _meta missing io.modelcontextprotocol/clientCapabilities
    Error: _meta validation probe failed completely

Test stateless MCP server architecture (SEP-2575).



Endpoints:
- server/discover: Returns supportedVersions and capabilities; SHOULD identify itself via _meta['io.modelcontextprotocol/serverInfo'] (spec PR #3002).
- tools/call: Implement structural test tools like test_missing_capability requiring explicit capabilities in _meta.

Grouped Specification Requirements:

1. Per-Request _meta Validation (4 Checks)
   - Rejects requests missing _meta or lacking structural required internal subfields (protocolVersion, clientCapabilities) with a JSON-RPC -32602 Invalid params error signature and an HTTP status code 400 Bad Request.
   - Serves requests whose _meta omits clientInfo (a SHOULD since spec PR #3002 — servers MUST NOT require it).
2. Discovery & Capabilities (3 Checks)
   - Implements server/discover mapping exact mandatory protocol elements.
   - Dynamically checks prompt capability declaration constraints, validates that active RPC handlers match advertised discovery capacities.
3. Version Negotiation & Headers (3 Checks)
   - Mismatched or unknown protocol versions must return an UnsupportedProtocolVersionError (HTTP status code 400 Bad Request) carrying precise version tracking arrays.
   - Absent or altered protocol version header metadata must trigger a -32020 Header Mismatch error with an HTTP 400 boundary state.
4. Client Capability Constraints (2 Checks)
   - Accessing platform capabilities without explicit declaration drops requests with a -32021 MissingRequiredClientCapabilityError returning an HTTP status code 400 Bad Request. Its error.data.requiredCapabilities is a ClientCapabilities object keyed by the missing capability (e.g. { "sampling": {} }), not an array of names.
5. Methods & Routing Mechanics (5 Checks)
   - Removed legacy endpoints (initialize, ping, logging/setLevel, etc.) or generic unknown methods must cleanly yield an HTTP status code 404 Not Found alongside a JSON-RPC -32601 Method not found payload. All error returns must preserve original request ID mappings.
   - Validates response streams contain only IncompleteResult chunks and never independent top-level JSON-RPC requests, while enforcing that no log messages are emitted when _meta.../logLevel is omitted.
6. Subscription Streams & Filtering (3 Checks)
   - Mandates that notifications/subscriptions/acknowledged is the first message on a subscriptions/listen stream, and that subsequent notifications carry a matching _meta.../subscriptionId.
   - Verifies strict containment where servers do not dispatch notification types that fall outside the client's explicit requested subscription filter list.
7. Dynamic List Mutations (2 Checks)
   - Evaluates that list-changed capable servers notify active listen streams with promptsListChanged: true or toolsListChanged: true upon live configuration or capability modifications.  

  - HttpServerMetaInvalid400: Rejections of requests missing required _meta fields use HTTP 400 Bad Request.
    Error: _meta validation probe failed completely

Test stateless MCP server architecture (SEP-2575).



Endpoints:
- server/discover: Returns supportedVersions and capabilities; SHOULD identify itself via _meta['io.modelcontextprotocol/serverInfo'] (spec PR #3002).
- tools/call: Implement structural test tools like test_missing_capability requiring explicit capabilities in _meta.

Grouped Specification Requirements:

1. Per-Request _meta Validation (4 Checks)
   - Rejects requests missing _meta or lacking structural required internal subfields (protocolVersion, clientCapabilities) with a JSON-RPC -32602 Invalid params error signature and an HTTP status code 400 Bad Request.
   - Serves requests whose _meta omits clientInfo (a SHOULD since spec PR #3002 — servers MUST NOT require it).
2. Discovery & Capabilities (3 Checks)
   - Implements server/discover mapping exact mandatory protocol elements.
   - Dynamically checks prompt capability declaration constraints, validates that active RPC handlers match advertised discovery capacities.
3. Version Negotiation & Headers (3 Checks)
   - Mismatched or unknown protocol versions must return an UnsupportedProtocolVersionError (HTTP status code 400 Bad Request) carrying precise version tracking arrays.
   - Absent or altered protocol version header metadata must trigger a -32020 Header Mismatch error with an HTTP 400 boundary state.
4. Client Capability Constraints (2 Checks)
   - Accessing platform capabilities without explicit declaration drops requests with a -32021 MissingRequiredClientCapabilityError returning an HTTP status code 400 Bad Request. Its error.data.requiredCapabilities is a ClientCapabilities object keyed by the missing capability (e.g. { "sampling": {} }), not an array of names.
5. Methods & Routing Mechanics (5 Checks)
   - Removed legacy endpoints (initialize, ping, logging/setLevel, etc.) or generic unknown methods must cleanly yield an HTTP status code 404 Not Found alongside a JSON-RPC -32601 Method not found payload. All error returns must preserve original request ID mappings.
   - Validates response streams contain only IncompleteResult chunks and never independent top-level JSON-RPC requests, while enforcing that no log messages are emitted when _meta.../logLevel is omitted.
6. Subscription Streams & Filtering (3 Checks)
   - Mandates that notifications/subscriptions/acknowledged is the first message on a subscriptions/listen stream, and that subsequent notifications carry a matching _meta.../subscriptionId.
   - Verifies strict containment where servers do not dispatch notification types that fall outside the client's explicit requested subscription filter list.
7. Dynamic List Mutations (2 Checks)
   - Evaluates that list-changed capable servers notify active listen streams with promptsListChanged: true or toolsListChanged: true upon live configuration or capability modifications.  

  - RequestMetaClientInfoOptional: Serves requests whose _meta omits io.modelcontextprotocol/clientInfo (clientInfo is a SHOULD).
    Error: clientInfo-less probe failed completely

Test stateless MCP server architecture (SEP-2575).



Endpoints:
- server/discover: Returns supportedVersions and capabilities; SHOULD identify itself via _meta['io.modelcontextprotocol/serverInfo'] (spec PR #3002).
- tools/call: Implement structural test tools like test_missing_capability requiring explicit capabilities in _meta.

Grouped Specification Requirements:

1. Per-Request _meta Validation (4 Checks)
   - Rejects requests missing _meta or lacking structural required internal subfields (protocolVersion, clientCapabilities) with a JSON-RPC -32602 Invalid params error signature and an HTTP status code 400 Bad Request.
   - Serves requests whose _meta omits clientInfo (a SHOULD since spec PR #3002 — servers MUST NOT require it).
2. Discovery & Capabilities (3 Checks)
   - Implements server/discover mapping exact mandatory protocol elements.
   - Dynamically checks prompt capability declaration constraints, validates that active RPC handlers match advertised discovery capacities.
3. Version Negotiation & Headers (3 Checks)
   - Mismatched or unknown protocol versions must return an UnsupportedProtocolVersionError (HTTP status code 400 Bad Request) carrying precise version tracking arrays.
   - Absent or altered protocol version header metadata must trigger a -32020 Header Mismatch error with an HTTP 400 boundary state.
4. Client Capability Constraints (2 Checks)
   - Accessing platform capabilities without explicit declaration drops requests with a -32021 MissingRequiredClientCapabilityError returning an HTTP status code 400 Bad Request. Its error.data.requiredCapabilities is a ClientCapabilities object keyed by the missing capability (e.g. { "sampling": {} }), not an array of names.
5. Methods & Routing Mechanics (5 Checks)
   - Removed legacy endpoints (initialize, ping, logging/setLevel, etc.) or generic unknown methods must cleanly yield an HTTP status code 404 Not Found alongside a JSON-RPC -32601 Method not found payload. All error returns must preserve original request ID mappings.
   - Validates response streams contain only IncompleteResult chunks and never independent top-level JSON-RPC requests, while enforcing that no log messages are emitted when _meta.../logLevel is omitted.
6. Subscription Streams & Filtering (3 Checks)
   - Mandates that notifications/subscriptions/acknowledged is the first message on a subscriptions/listen stream, and that subsequent notifications carry a matching _meta.../subscriptionId.
   - Verifies strict containment where servers do not dispatch notification types that fall outside the client's explicit requested subscription filter list.
7. Dynamic List Mutations (2 Checks)
   - Evaluates that list-changed capable servers notify active listen streams with promptsListChanged: true or toolsListChanged: true upon live configuration or capability modifications.  

  - ServerImplementsDiscover: Servers MUST implement server/discover.
    Error: Discovery failed: fetch failed

Test stateless MCP server architecture (SEP-2575).



Endpoints:
- server/discover: Returns supportedVersions and capabilities; SHOULD identify itself via _meta['io.modelcontextprotocol/serverInfo'] (spec PR #3002).
- tools/call: Implement structural test tools like test_missing_capability requiring explicit capabilities in _meta.

Grouped Specification Requirements:

1. Per-Request _meta Validation (4 Checks)
   - Rejects requests missing _meta or lacking structural required internal subfields (protocolVersion, clientCapabilities) with a JSON-RPC -32602 Invalid params error signature and an HTTP status code 400 Bad Request.
   - Serves requests whose _meta omits clientInfo (a SHOULD since spec PR #3002 — servers MUST NOT require it).
2. Discovery & Capabilities (3 Checks)
   - Implements server/discover mapping exact mandatory protocol elements.
   - Dynamically checks prompt capability declaration constraints, validates that active RPC handlers match advertised discovery capacities.
3. Version Negotiation & Headers (3 Checks)
   - Mismatched or unknown protocol versions must return an UnsupportedProtocolVersionError (HTTP status code 400 Bad Request) carrying precise version tracking arrays.
   - Absent or altered protocol version header metadata must trigger a -32020 Header Mismatch error with an HTTP 400 boundary state.
4. Client Capability Constraints (2 Checks)
   - Accessing platform capabilities without explicit declaration drops requests with a -32021 MissingRequiredClientCapabilityError returning an HTTP status code 400 Bad Request. Its error.data.requiredCapabilities is a ClientCapabilities object keyed by the missing capability (e.g. { "sampling": {} }), not an array of names.
5. Methods & Routing Mechanics (5 Checks)
   - Removed legacy endpoints (initialize, ping, logging/setLevel, etc.) or generic unknown methods must cleanly yield an HTTP status code 404 Not Found alongside a JSON-RPC -32601 Method not found payload. All error returns must preserve original request ID mappings.
   - Validates response streams contain only IncompleteResult chunks and never independent top-level JSON-RPC requests, while enforcing that no log messages are emitted when _meta.../logLevel is omitted.
6. Subscription Streams & Filtering (3 Checks)
   - Mandates that notifications/subscriptions/acknowledged is the first message on a subscriptions/listen stream, and that subsequent notifications carry a matching _meta.../subscriptionId.
   - Verifies strict containment where servers do not dispatch notification types that fall outside the client's explicit requested subscription filter list.
7. Dynamic List Mutations (2 Checks)
   - Evaluates that list-changed capable servers notify active listen streams with promptsListChanged: true or toolsListChanged: true upon live configuration or capability modifications.  

  - ServerDeclaresPromptsInDiscover: Servers that support prompts MUST declare the prompts capability in their DiscoverResult.
    Error: Prerequisite missing: fetch failed

Test stateless MCP server architecture (SEP-2575).



Endpoints:
- server/discover: Returns supportedVersions and capabilities; SHOULD identify itself via _meta['io.modelcontextprotocol/serverInfo'] (spec PR #3002).
- tools/call: Implement structural test tools like test_missing_capability requiring explicit capabilities in _meta.

Grouped Specification Requirements:

1. Per-Request _meta Validation (4 Checks)
   - Rejects requests missing _meta or lacking structural required internal subfields (protocolVersion, clientCapabilities) with a JSON-RPC -32602 Invalid params error signature and an HTTP status code 400 Bad Request.
   - Serves requests whose _meta omits clientInfo (a SHOULD since spec PR #3002 — servers MUST NOT require it).
2. Discovery & Capabilities (3 Checks)
   - Implements server/discover mapping exact mandatory protocol elements.
   - Dynamically checks prompt capability declaration constraints, validates that active RPC handlers match advertised discovery capacities.
3. Version Negotiation & Headers (3 Checks)
   - Mismatched or unknown protocol versions must return an UnsupportedProtocolVersionError (HTTP status code 400 Bad Request) carrying precise version tracking arrays.
   - Absent or altered protocol version header metadata must trigger a -32020 Header Mismatch error with an HTTP 400 boundary state.
4. Client Capability Constraints (2 Checks)
   - Accessing platform capabilities without explicit declaration drops requests with a -32021 MissingRequiredClientCapabilityError returning an HTTP status code 400 Bad Request. Its error.data.requiredCapabilities is a ClientCapabilities object keyed by the missing capability (e.g. { "sampling": {} }), not an array of names.
5. Methods & Routing Mechanics (5 Checks)
   - Removed legacy endpoints (initialize, ping, logging/setLevel, etc.) or generic unknown methods must cleanly yield an HTTP status code 404 Not Found alongside a JSON-RPC -32601 Method not found payload. All error returns must preserve original request ID mappings.
   - Validates response streams contain only IncompleteResult chunks and never independent top-level JSON-RPC requests, while enforcing that no log messages are emitted when _meta.../logLevel is omitted.
6. Subscription Streams & Filtering (3 Checks)
   - Mandates that notifications/subscriptions/acknowledged is the first message on a subscriptions/listen stream, and that subsequent notifications carry a matching _meta.../subscriptionId.
   - Verifies strict containment where servers do not dispatch notification types that fall outside the client's explicit requested subscription filter list.
7. Dynamic List Mutations (2 Checks)
   - Evaluates that list-changed capable servers notify active listen streams with promptsListChanged: true or toolsListChanged: true upon live configuration or capability modifications.  

  - DiscoverCapabilitiesMatchHandlers: capabilities matches what the server honors on real RPC calls
    Error: Discovery runtime check failed: fetch failed

Test stateless MCP server architecture (SEP-2575).



Endpoints:
- server/discover: Returns supportedVersions and capabilities; SHOULD identify itself via _meta['io.modelcontextprotocol/serverInfo'] (spec PR #3002).
- tools/call: Implement structural test tools like test_missing_capability requiring explicit capabilities in _meta.

Grouped Specification Requirements:

1. Per-Request _meta Validation (4 Checks)
   - Rejects requests missing _meta or lacking structural required internal subfields (protocolVersion, clientCapabilities) with a JSON-RPC -32602 Invalid params error signature and an HTTP status code 400 Bad Request.
   - Serves requests whose _meta omits clientInfo (a SHOULD since spec PR #3002 — servers MUST NOT require it).
2. Discovery & Capabilities (3 Checks)
   - Implements server/discover mapping exact mandatory protocol elements.
   - Dynamically checks prompt capability declaration constraints, validates that active RPC handlers match advertised discovery capacities.
3. Version Negotiation & Headers (3 Checks)
   - Mismatched or unknown protocol versions must return an UnsupportedProtocolVersionError (HTTP status code 400 Bad Request) carrying precise version tracking arrays.
   - Absent or altered protocol version header metadata must trigger a -32020 Header Mismatch error with an HTTP 400 boundary state.
4. Client Capability Constraints (2 Checks)
   - Accessing platform capabilities without explicit declaration drops requests with a -32021 MissingRequiredClientCapabilityError returning an HTTP status code 400 Bad Request. Its error.data.requiredCapabilities is a ClientCapabilities object keyed by the missing capability (e.g. { "sampling": {} }), not an array of names.
5. Methods & Routing Mechanics (5 Checks)
   - Removed legacy endpoints (initialize, ping, logging/setLevel, etc.) or generic unknown methods must cleanly yield an HTTP status code 404 Not Found alongside a JSON-RPC -32601 Method not found payload. All error returns must preserve original request ID mappings.
   - Validates response streams contain only IncompleteResult chunks and never independent top-level JSON-RPC requests, while enforcing that no log messages are emitted when _meta.../logLevel is omitted.
6. Subscription Streams & Filtering (3 Checks)
   - Mandates that notifications/subscriptions/acknowledged is the first message on a subscriptions/listen stream, and that subsequent notifications carry a matching _meta.../subscriptionId.
   - Verifies strict containment where servers do not dispatch notification types that fall outside the client's explicit requested subscription filter list.
7. Dynamic List Mutations (2 Checks)
   - Evaluates that list-changed capable servers notify active listen streams with promptsListChanged: true or toolsListChanged: true upon live configuration or capability modifications.  

  - ServerUnsupportedVersionError: If the server does not implement the requested version (whether the version is unknown to the server, or is a known version the server has chosen not to support), it MUST respond with an UnsupportedProtocolVersionError listing the versions it does support; the error data carries the supported versions and echoes the requested version.
    Error: Unsupported version invocation failed completely

Test stateless MCP server architecture (SEP-2575).



Endpoints:
- server/discover: Returns supportedVersions and capabilities; SHOULD identify itself via _meta['io.modelcontextprotocol/serverInfo'] (spec PR #3002).
- tools/call: Implement structural test tools like test_missing_capability requiring explicit capabilities in _meta.

Grouped Specification Requirements:

1. Per-Request _meta Validation (4 Checks)
   - Rejects requests missing _meta or lacking structural required internal subfields (protocolVersion, clientCapabilities) with a JSON-RPC -32602 Invalid params error signature and an HTTP status code 400 Bad Request.
   - Serves requests whose _meta omits clientInfo (a SHOULD since spec PR #3002 — servers MUST NOT require it).
2. Discovery & Capabilities (3 Checks)
   - Implements server/discover mapping exact mandatory protocol elements.
   - Dynamically checks prompt capability declaration constraints, validates that active RPC handlers match advertised discovery capacities.
3. Version Negotiation & Headers (3 Checks)
   - Mismatched or unknown protocol versions must return an UnsupportedProtocolVersionError (HTTP status code 400 Bad Request) carrying precise version tracking arrays.
   - Absent or altered protocol version header metadata must trigger a -32020 Header Mismatch error with an HTTP 400 boundary state.
4. Client Capability Constraints (2 Checks)
   - Accessing platform capabilities without explicit declaration drops requests with a -32021 MissingRequiredClientCapabilityError returning an HTTP status code 400 Bad Request. Its error.data.requiredCapabilities is a ClientCapabilities object keyed by the missing capability (e.g. { "sampling": {} }), not an array of names.
5. Methods & Routing Mechanics (5 Checks)
   - Removed legacy endpoints (initialize, ping, logging/setLevel, etc.) or generic unknown methods must cleanly yield an HTTP status code 404 Not Found alongside a JSON-RPC -32601 Method not found payload. All error returns must preserve original request ID mappings.
   - Validates response streams contain only IncompleteResult chunks and never independent top-level JSON-RPC requests, while enforcing that no log messages are emitted when _meta.../logLevel is omitted.
6. Subscription Streams & Filtering (3 Checks)
   - Mandates that notifications/subscriptions/acknowledged is the first message on a subscriptions/listen stream, and that subsequent notifications carry a matching _meta.../subscriptionId.
   - Verifies strict containment where servers do not dispatch notification types that fall outside the client's explicit requested subscription filter list.
7. Dynamic List Mutations (2 Checks)
   - Evaluates that list-changed capable servers notify active listen streams with promptsListChanged: true or toolsListChanged: true upon live configuration or capability modifications.  

  - HttpServerUnsupportedVersion400: If the server does not implement the requested protocol version, it MUST respond with 400 Bad Request and an UnsupportedProtocolVersionError listing its supported versions.
    Error: Network transaction context unavailable

Test stateless MCP server architecture (SEP-2575).



Endpoints:
- server/discover: Returns supportedVersions and capabilities; SHOULD identify itself via _meta['io.modelcontextprotocol/serverInfo'] (spec PR #3002).
- tools/call: Implement structural test tools like test_missing_capability requiring explicit capabilities in _meta.

Grouped Specification Requirements:

1. Per-Request _meta Validation (4 Checks)
   - Rejects requests missing _meta or lacking structural required internal subfields (protocolVersion, clientCapabilities) with a JSON-RPC -32602 Invalid params error signature and an HTTP status code 400 Bad Request.
   - Serves requests whose _meta omits clientInfo (a SHOULD since spec PR #3002 — servers MUST NOT require it).
2. Discovery & Capabilities (3 Checks)
   - Implements server/discover mapping exact mandatory protocol elements.
   - Dynamically checks prompt capability declaration constraints, validates that active RPC handlers match advertised discovery capacities.
3. Version Negotiation & Headers (3 Checks)
   - Mismatched or unknown protocol versions must return an UnsupportedProtocolVersionError (HTTP status code 400 Bad Request) carrying precise version tracking arrays.
   - Absent or altered protocol version header metadata must trigger a -32020 Header Mismatch error with an HTTP 400 boundary state.
4. Client Capability Constraints (2 Checks)
   - Accessing platform capabilities without explicit declaration drops requests with a -32021 MissingRequiredClientCapabilityError returning an HTTP status code 400 Bad Request. Its error.data.requiredCapabilities is a ClientCapabilities object keyed by the missing capability (e.g. { "sampling": {} }), not an array of names.
5. Methods & Routing Mechanics (5 Checks)
   - Removed legacy endpoints (initialize, ping, logging/setLevel, etc.) or generic unknown methods must cleanly yield an HTTP status code 404 Not Found alongside a JSON-RPC -32601 Method not found payload. All error returns must preserve original request ID mappings.
   - Validates response streams contain only IncompleteResult chunks and never independent top-level JSON-RPC requests, while enforcing that no log messages are emitted when _meta.../logLevel is omitted.
6. Subscription Streams & Filtering (3 Checks)
   - Mandates that notifications/subscriptions/acknowledged is the first message on a subscriptions/listen stream, and that subsequent notifications carry a matching _meta.../subscriptionId.
   - Verifies strict containment where servers do not dispatch notification types that fall outside the client's explicit requested subscription filter list.
7. Dynamic List Mutations (2 Checks)
   - Evaluates that list-changed capable servers notify active listen streams with promptsListChanged: true or toolsListChanged: true upon live configuration or capability modifications.  

  - HttpServerHeaderMismatch400: If the values do not match, the server MUST reject the request with 400 Bad Request and a HeaderMismatch JSON-RPC error.
    Error: Header verification endpoint network hit failed

Test stateless MCP server architecture (SEP-2575).



Endpoints:
- server/discover: Returns supportedVersions and capabilities; SHOULD identify itself via _meta['io.modelcontextprotocol/serverInfo'] (spec PR #3002).
- tools/call: Implement structural test tools like test_missing_capability requiring explicit capabilities in _meta.

Grouped Specification Requirements:

1. Per-Request _meta Validation (4 Checks)
   - Rejects requests missing _meta or lacking structural required internal subfields (protocolVersion, clientCapabilities) with a JSON-RPC -32602 Invalid params error signature and an HTTP status code 400 Bad Request.
   - Serves requests whose _meta omits clientInfo (a SHOULD since spec PR #3002 — servers MUST NOT require it).
2. Discovery & Capabilities (3 Checks)
   - Implements server/discover mapping exact mandatory protocol elements.
   - Dynamically checks prompt capability declaration constraints, validates that active RPC handlers match advertised discovery capacities.
3. Version Negotiation & Headers (3 Checks)
   - Mismatched or unknown protocol versions must return an UnsupportedProtocolVersionError (HTTP status code 400 Bad Request) carrying precise version tracking arrays.
   - Absent or altered protocol version header metadata must trigger a -32020 Header Mismatch error with an HTTP 400 boundary state.
4. Client Capability Constraints (2 Checks)
   - Accessing platform capabilities without explicit declaration drops requests with a -32021 MissingRequiredClientCapabilityError returning an HTTP status code 400 Bad Request. Its error.data.requiredCapabilities is a ClientCapabilities object keyed by the missing capability (e.g. { "sampling": {} }), not an array of names.
5. Methods & Routing Mechanics (5 Checks)
   - Removed legacy endpoints (initialize, ping, logging/setLevel, etc.) or generic unknown methods must cleanly yield an HTTP status code 404 Not Found alongside a JSON-RPC -32601 Method not found payload. All error returns must preserve original request ID mappings.
   - Validates response streams contain only IncompleteResult chunks and never independent top-level JSON-RPC requests, while enforcing that no log messages are emitted when _meta.../logLevel is omitted.
6. Subscription Streams & Filtering (3 Checks)
   - Mandates that notifications/subscriptions/acknowledged is the first message on a subscriptions/listen stream, and that subsequent notifications carry a matching _meta.../subscriptionId.
   - Verifies strict containment where servers do not dispatch notification types that fall outside the client's explicit requested subscription filter list.
7. Dynamic List Mutations (2 Checks)
   - Evaluates that list-changed capable servers notify active listen streams with promptsListChanged: true or toolsListChanged: true upon live configuration or capability modifications.  

  - ServerRejectsUndeclaredCapability: A server MUST NOT rely on capabilities the client has not declared. If processing a request requires a capability the client did not include in io.modelcontextprotocol/clientCapabilities, the server MUST return a MissingRequiredClientCapabilityError (-32021).
    Error: Capability checking call sequence timed out or dropped connection

Test stateless MCP server architecture (SEP-2575).



Endpoints:
- server/discover: Returns supportedVersions and capabilities; SHOULD identify itself via _meta['io.modelcontextprotocol/serverInfo'] (spec PR #3002).
- tools/call: Implement structural test tools like test_missing_capability requiring explicit capabilities in _meta.

Grouped Specification Requirements:

1. Per-Request _meta Validation (4 Checks)
   - Rejects requests missing _meta or lacking structural required internal subfields (protocolVersion, clientCapabilities) with a JSON-RPC -32602 Invalid params error signature and an HTTP status code 400 Bad Request.
   - Serves requests whose _meta omits clientInfo (a SHOULD since spec PR #3002 — servers MUST NOT require it).
2. Discovery & Capabilities (3 Checks)
   - Implements server/discover mapping exact mandatory protocol elements.
   - Dynamically checks prompt capability declaration constraints, validates that active RPC handlers match advertised discovery capacities.
3. Version Negotiation & Headers (3 Checks)
   - Mismatched or unknown protocol versions must return an UnsupportedProtocolVersionError (HTTP status code 400 Bad Request) carrying precise version tracking arrays.
   - Absent or altered protocol version header metadata must trigger a -32020 Header Mismatch error with an HTTP 400 boundary state.
4. Client Capability Constraints (2 Checks)
   - Accessing platform capabilities without explicit declaration drops requests with a -32021 MissingRequiredClientCapabilityError returning an HTTP status code 400 Bad Request. Its error.data.requiredCapabilities is a ClientCapabilities object keyed by the missing capability (e.g. { "sampling": {} }), not an array of names.
5. Methods & Routing Mechanics (5 Checks)
   - Removed legacy endpoints (initialize, ping, logging/setLevel, etc.) or generic unknown methods must cleanly yield an HTTP status code 404 Not Found alongside a JSON-RPC -32601 Method not found payload. All error returns must preserve original request ID mappings.
   - Validates response streams contain only IncompleteResult chunks and never independent top-level JSON-RPC requests, while enforcing that no log messages are emitted when _meta.../logLevel is omitted.
6. Subscription Streams & Filtering (3 Checks)
   - Mandates that notifications/subscriptions/acknowledged is the first message on a subscriptions/listen stream, and that subsequent notifications carry a matching _meta.../subscriptionId.
   - Verifies strict containment where servers do not dispatch notification types that fall outside the client's explicit requested subscription filter list.
7. Dynamic List Mutations (2 Checks)
   - Evaluates that list-changed capable servers notify active listen streams with promptsListChanged: true or toolsListChanged: true upon live configuration or capability modifications.  

  - MissingCapabilityHttp400: On HTTP, the response status MUST be 400 Bad Request [for MissingRequiredClientCapabilityError].
    Error: Network transport layer layer context failed to instantiate

Test stateless MCP server architecture (SEP-2575).



Endpoints:
- server/discover: Returns supportedVersions and capabilities; SHOULD identify itself via _meta['io.modelcontextprotocol/serverInfo'] (spec PR #3002).
- tools/call: Implement structural test tools like test_missing_capability requiring explicit capabilities in _meta.

Grouped Specification Requirements:

1. Per-Request _meta Validation (4 Checks)
   - Rejects requests missing _meta or lacking structural required internal subfields (protocolVersion, clientCapabilities) with a JSON-RPC -32602 Invalid params error signature and an HTTP status code 400 Bad Request.
   - Serves requests whose _meta omits clientInfo (a SHOULD since spec PR #3002 — servers MUST NOT require it).
2. Discovery & Capabilities (3 Checks)
   - Implements server/discover mapping exact mandatory protocol elements.
   - Dynamically checks prompt capability declaration constraints, validates that active RPC handlers match advertised discovery capacities.
3. Version Negotiation & Headers (3 Checks)
   - Mismatched or unknown protocol versions must return an UnsupportedProtocolVersionError (HTTP status code 400 Bad Request) carrying precise version tracking arrays.
   - Absent or altered protocol version header metadata must trigger a -32020 Header Mismatch error with an HTTP 400 boundary state.
4. Client Capability Constraints (2 Checks)
   - Accessing platform capabilities without explicit declaration drops requests with a -32021 MissingRequiredClientCapabilityError returning an HTTP status code 400 Bad Request. Its error.data.requiredCapabilities is a ClientCapabilities object keyed by the missing capability (e.g. { "sampling": {} }), not an array of names.
5. Methods & Routing Mechanics (5 Checks)
   - Removed legacy endpoints (initialize, ping, logging/setLevel, etc.) or generic unknown methods must cleanly yield an HTTP status code 404 Not Found alongside a JSON-RPC -32601 Method not found payload. All error returns must preserve original request ID mappings.
   - Validates response streams contain only IncompleteResult chunks and never independent top-level JSON-RPC requests, while enforcing that no log messages are emitted when _meta.../logLevel is omitted.
6. Subscription Streams & Filtering (3 Checks)
   - Mandates that notifications/subscriptions/acknowledged is the first message on a subscriptions/listen stream, and that subsequent notifications carry a matching _meta.../subscriptionId.
   - Verifies strict containment where servers do not dispatch notification types that fall outside the client's explicit requested subscription filter list.
7. Dynamic List Mutations (2 Checks)
   - Evaluates that list-changed capable servers notify active listen streams with promptsListChanged: true or toolsListChanged: true upon live configuration or capability modifications.  

  - HttpServerMethodNotFound404initialize: If the server does not implement the removed RPC method 'initialize', it MUST respond with 404 Not Found and a JSON-RPC error with code -32601 (Method not found).
    Error: Removed method validation hit dropped connections unexpectedly

Test stateless MCP server architecture (SEP-2575).



Endpoints:
- server/discover: Returns supportedVersions and capabilities; SHOULD identify itself via _meta['io.modelcontextprotocol/serverInfo'] (spec PR #3002).
- tools/call: Implement structural test tools like test_missing_capability requiring explicit capabilities in _meta.

Grouped Specification Requirements:

1. Per-Request _meta Validation (4 Checks)
   - Rejects requests missing _meta or lacking structural required internal subfields (protocolVersion, clientCapabilities) with a JSON-RPC -32602 Invalid params error signature and an HTTP status code 400 Bad Request.
   - Serves requests whose _meta omits clientInfo (a SHOULD since spec PR #3002 — servers MUST NOT require it).
2. Discovery & Capabilities (3 Checks)
   - Implements server/discover mapping exact mandatory protocol elements.
   - Dynamically checks prompt capability declaration constraints, validates that active RPC handlers match advertised discovery capacities.
3. Version Negotiation & Headers (3 Checks)
   - Mismatched or unknown protocol versions must return an UnsupportedProtocolVersionError (HTTP status code 400 Bad Request) carrying precise version tracking arrays.
   - Absent or altered protocol version header metadata must trigger a -32020 Header Mismatch error with an HTTP 400 boundary state.
4. Client Capability Constraints (2 Checks)
   - Accessing platform capabilities without explicit declaration drops requests with a -32021 MissingRequiredClientCapabilityError returning an HTTP status code 400 Bad Request. Its error.data.requiredCapabilities is a ClientCapabilities object keyed by the missing capability (e.g. { "sampling": {} }), not an array of names.
5. Methods & Routing Mechanics (5 Checks)
   - Removed legacy endpoints (initialize, ping, logging/setLevel, etc.) or generic unknown methods must cleanly yield an HTTP status code 404 Not Found alongside a JSON-RPC -32601 Method not found payload. All error returns must preserve original request ID mappings.
   - Validates response streams contain only IncompleteResult chunks and never independent top-level JSON-RPC requests, while enforcing that no log messages are emitted when _meta.../logLevel is omitted.
6. Subscription Streams & Filtering (3 Checks)
   - Mandates that notifications/subscriptions/acknowledged is the first message on a subscriptions/listen stream, and that subsequent notifications carry a matching _meta.../subscriptionId.
   - Verifies strict containment where servers do not dispatch notification types that fall outside the client's explicit requested subscription filter list.
7. Dynamic List Mutations (2 Checks)
   - Evaluates that list-changed capable servers notify active listen streams with promptsListChanged: true or toolsListChanged: true upon live configuration or capability modifications.  

  - HttpServerMethodNotFound404ping: If the server does not implement the removed RPC method 'ping', it MUST respond with 404 Not Found and a JSON-RPC error with code -32601 (Method not found).
    Error: Removed method validation hit dropped connections unexpectedly

Test stateless MCP server architecture (SEP-2575).



Endpoints:
- server/discover: Returns supportedVersions and capabilities; SHOULD identify itself via _meta['io.modelcontextprotocol/serverInfo'] (spec PR #3002).
- tools/call: Implement structural test tools like test_missing_capability requiring explicit capabilities in _meta.

Grouped Specification Requirements:

1. Per-Request _meta Validation (4 Checks)
   - Rejects requests missing _meta or lacking structural required internal subfields (protocolVersion, clientCapabilities) with a JSON-RPC -32602 Invalid params error signature and an HTTP status code 400 Bad Request.
   - Serves requests whose _meta omits clientInfo (a SHOULD since spec PR #3002 — servers MUST NOT require it).
2. Discovery & Capabilities (3 Checks)
   - Implements server/discover mapping exact mandatory protocol elements.
   - Dynamically checks prompt capability declaration constraints, validates that active RPC handlers match advertised discovery capacities.
3. Version Negotiation & Headers (3 Checks)
   - Mismatched or unknown protocol versions must return an UnsupportedProtocolVersionError (HTTP status code 400 Bad Request) carrying precise version tracking arrays.
   - Absent or altered protocol version header metadata must trigger a -32020 Header Mismatch error with an HTTP 400 boundary state.
4. Client Capability Constraints (2 Checks)
   - Accessing platform capabilities without explicit declaration drops requests with a -32021 MissingRequiredClientCapabilityError returning an HTTP status code 400 Bad Request. Its error.data.requiredCapabilities is a ClientCapabilities object keyed by the missing capability (e.g. { "sampling": {} }), not an array of names.
5. Methods & Routing Mechanics (5 Checks)
   - Removed legacy endpoints (initialize, ping, logging/setLevel, etc.) or generic unknown methods must cleanly yield an HTTP status code 404 Not Found alongside a JSON-RPC -32601 Method not found payload. All error returns must preserve original request ID mappings.
   - Validates response streams contain only IncompleteResult chunks and never independent top-level JSON-RPC requests, while enforcing that no log messages are emitted when _meta.../logLevel is omitted.
6. Subscription Streams & Filtering (3 Checks)
   - Mandates that notifications/subscriptions/acknowledged is the first message on a subscriptions/listen stream, and that subsequent notifications carry a matching _meta.../subscriptionId.
   - Verifies strict containment where servers do not dispatch notification types that fall outside the client's explicit requested subscription filter list.
7. Dynamic List Mutations (2 Checks)
   - Evaluates that list-changed capable servers notify active listen streams with promptsListChanged: true or toolsListChanged: true upon live configuration or capability modifications.  

  - HttpServerMethodNotFound404loggingsetLevel: If the server does not implement the removed RPC method 'logging/setLevel', it MUST respond with 404 Not Found and a JSON-RPC error with code -32601 (Method not found).
    Error: Removed method validation hit dropped connections unexpectedly

Test stateless MCP server architecture (SEP-2575).



Endpoints:
- server/discover: Returns supportedVersions and capabilities; SHOULD identify itself via _meta['io.modelcontextprotocol/serverInfo'] (spec PR #3002).
- tools/call: Implement structural test tools like test_missing_capability requiring explicit capabilities in _meta.

Grouped Specification Requirements:

1. Per-Request _meta Validation (4 Checks)
   - Rejects requests missing _meta or lacking structural required internal subfields (protocolVersion, clientCapabilities) with a JSON-RPC -32602 Invalid params error signature and an HTTP status code 400 Bad Request.
   - Serves requests whose _meta omits clientInfo (a SHOULD since spec PR #3002 — servers MUST NOT require it).
2. Discovery & Capabilities (3 Checks)
   - Implements server/discover mapping exact mandatory protocol elements.
   - Dynamically checks prompt capability declaration constraints, validates that active RPC handlers match advertised discovery capacities.
3. Version Negotiation & Headers (3 Checks)
   - Mismatched or unknown protocol versions must return an UnsupportedProtocolVersionError (HTTP status code 400 Bad Request) carrying precise version tracking arrays.
   - Absent or altered protocol version header metadata must trigger a -32020 Header Mismatch error with an HTTP 400 boundary state.
4. Client Capability Constraints (2 Checks)
   - Accessing platform capabilities without explicit declaration drops requests with a -32021 MissingRequiredClientCapabilityError returning an HTTP status code 400 Bad Request. Its error.data.requiredCapabilities is a ClientCapabilities object keyed by the missing capability (e.g. { "sampling": {} }), not an array of names.
5. Methods & Routing Mechanics (5 Checks)
   - Removed legacy endpoints (initialize, ping, logging/setLevel, etc.) or generic unknown methods must cleanly yield an HTTP status code 404 Not Found alongside a JSON-RPC -32601 Method not found payload. All error returns must preserve original request ID mappings.
   - Validates response streams contain only IncompleteResult chunks and never independent top-level JSON-RPC requests, while enforcing that no log messages are emitted when _meta.../logLevel is omitted.
6. Subscription Streams & Filtering (3 Checks)
   - Mandates that notifications/subscriptions/acknowledged is the first message on a subscriptions/listen stream, and that subsequent notifications carry a matching _meta.../subscriptionId.
   - Verifies strict containment where servers do not dispatch notification types that fall outside the client's explicit requested subscription filter list.
7. Dynamic List Mutations (2 Checks)
   - Evaluates that list-changed capable servers notify active listen streams with promptsListChanged: true or toolsListChanged: true upon live configuration or capability modifications.  

  - HttpServerMethodNotFound404resourcessubscribe: If the server does not implement the removed RPC method 'resources/subscribe', it MUST respond with 404 Not Found and a JSON-RPC error with code -32601 (Method not found).
    Error: Removed method validation hit dropped connections unexpectedly

Test stateless MCP server architecture (SEP-2575).



Endpoints:
- server/discover: Returns supportedVersions and capabilities; SHOULD identify itself via _meta['io.modelcontextprotocol/serverInfo'] (spec PR #3002).
- tools/call: Implement structural test tools like test_missing_capability requiring explicit capabilities in _meta.

Grouped Specification Requirements:

1. Per-Request _meta Validation (4 Checks)
   - Rejects requests missing _meta or lacking structural required internal subfields (protocolVersion, clientCapabilities) with a JSON-RPC -32602 Invalid params error signature and an HTTP status code 400 Bad Request.
   - Serves requests whose _meta omits clientInfo (a SHOULD since spec PR #3002 — servers MUST NOT require it).
2. Discovery & Capabilities (3 Checks)
   - Implements server/discover mapping exact mandatory protocol elements.
   - Dynamically checks prompt capability declaration constraints, validates that active RPC handlers match advertised discovery capacities.
3. Version Negotiation & Headers (3 Checks)
   - Mismatched or unknown protocol versions must return an UnsupportedProtocolVersionError (HTTP status code 400 Bad Request) carrying precise version tracking arrays.
   - Absent or altered protocol version header metadata must trigger a -32020 Header Mismatch error with an HTTP 400 boundary state.
4. Client Capability Constraints (2 Checks)
   - Accessing platform capabilities without explicit declaration drops requests with a -32021 MissingRequiredClientCapabilityError returning an HTTP status code 400 Bad Request. Its error.data.requiredCapabilities is a ClientCapabilities object keyed by the missing capability (e.g. { "sampling": {} }), not an array of names.
5. Methods & Routing Mechanics (5 Checks)
   - Removed legacy endpoints (initialize, ping, logging/setLevel, etc.) or generic unknown methods must cleanly yield an HTTP status code 404 Not Found alongside a JSON-RPC -32601 Method not found payload. All error returns must preserve original request ID mappings.
   - Validates response streams contain only IncompleteResult chunks and never independent top-level JSON-RPC requests, while enforcing that no log messages are emitted when _meta.../logLevel is omitted.
6. Subscription Streams & Filtering (3 Checks)
   - Mandates that notifications/subscriptions/acknowledged is the first message on a subscriptions/listen stream, and that subsequent notifications carry a matching _meta.../subscriptionId.
   - Verifies strict containment where servers do not dispatch notification types that fall outside the client's explicit requested subscription filter list.
7. Dynamic List Mutations (2 Checks)
   - Evaluates that list-changed capable servers notify active listen streams with promptsListChanged: true or toolsListChanged: true upon live configuration or capability modifications.  

  - HttpServerMethodNotFound404resourcesunsubscribe: If the server does not implement the removed RPC method 'resources/unsubscribe', it MUST respond with 404 Not Found and a JSON-RPC error with code -32601 (Method not found).
    Error: Removed method validation hit dropped connections unexpectedly

Test stateless MCP server architecture (SEP-2575).



Endpoints:
- server/discover: Returns supportedVersions and capabilities; SHOULD identify itself via _meta['io.modelcontextprotocol/serverInfo'] (spec PR #3002).
- tools/call: Implement structural test tools like test_missing_capability requiring explicit capabilities in _meta.

Grouped Specification Requirements:

1. Per-Request _meta Validation (4 Checks)
   - Rejects requests missing _meta or lacking structural required internal subfields (protocolVersion, clientCapabilities) with a JSON-RPC -32602 Invalid params error signature and an HTTP status code 400 Bad Request.
   - Serves requests whose _meta omits clientInfo (a SHOULD since spec PR #3002 — servers MUST NOT require it).
2. Discovery & Capabilities (3 Checks)
   - Implements server/discover mapping exact mandatory protocol elements.
   - Dynamically checks prompt capability declaration constraints, validates that active RPC handlers match advertised discovery capacities.
3. Version Negotiation & Headers (3 Checks)
   - Mismatched or unknown protocol versions must return an UnsupportedProtocolVersionError (HTTP status code 400 Bad Request) carrying precise version tracking arrays.
   - Absent or altered protocol version header metadata must trigger a -32020 Header Mismatch error with an HTTP 400 boundary state.
4. Client Capability Constraints (2 Checks)
   - Accessing platform capabilities without explicit declaration drops requests with a -32021 MissingRequiredClientCapabilityError returning an HTTP status code 400 Bad Request. Its error.data.requiredCapabilities is a ClientCapabilities object keyed by the missing capability (e.g. { "sampling": {} }), not an array of names.
5. Methods & Routing Mechanics (5 Checks)
   - Removed legacy endpoints (initialize, ping, logging/setLevel, etc.) or generic unknown methods must cleanly yield an HTTP status code 404 Not Found alongside a JSON-RPC -32601 Method not found payload. All error returns must preserve original request ID mappings.
   - Validates response streams contain only IncompleteResult chunks and never independent top-level JSON-RPC requests, while enforcing that no log messages are emitted when _meta.../logLevel is omitted.
6. Subscription Streams & Filtering (3 Checks)
   - Mandates that notifications/subscriptions/acknowledged is the first message on a subscriptions/listen stream, and that subsequent notifications carry a matching _meta.../subscriptionId.
   - Verifies strict containment where servers do not dispatch notification types that fall outside the client's explicit requested subscription filter list.
7. Dynamic List Mutations (2 Checks)
   - Evaluates that list-changed capable servers notify active listen streams with promptsListChanged: true or toolsListChanged: true upon live configuration or capability modifications.  

  - HttpServerMethodNotFound404: If the server does not implement the requested RPC method, it MUST respond with 404 Not Found and a JSON-RPC error with code -32601 (Method not found).
    Error: Unknown fallback test target returned an invalid layout

Test stateless MCP server architecture (SEP-2575).



Endpoints:
- server/discover: Returns supportedVersions and capabilities; SHOULD identify itself via _meta['io.modelcontextprotocol/serverInfo'] (spec PR #3002).
- tools/call: Implement structural test tools like test_missing_capability requiring explicit capabilities in _meta.

Grouped Specification Requirements:

1. Per-Request _meta Validation (4 Checks)
   - Rejects requests missing _meta or lacking structural required internal subfields (protocolVersion, clientCapabilities) with a JSON-RPC -32602 Invalid params error signature and an HTTP status code 400 Bad Request.
   - Serves requests whose _meta omits clientInfo (a SHOULD since spec PR #3002 — servers MUST NOT require it).
2. Discovery & Capabilities (3 Checks)
   - Implements server/discover mapping exact mandatory protocol elements.
   - Dynamically checks prompt capability declaration constraints, validates that active RPC handlers match advertised discovery capacities.
3. Version Negotiation & Headers (3 Checks)
   - Mismatched or unknown protocol versions must return an UnsupportedProtocolVersionError (HTTP status code 400 Bad Request) carrying precise version tracking arrays.
   - Absent or altered protocol version header metadata must trigger a -32020 Header Mismatch error with an HTTP 400 boundary state.
4. Client Capability Constraints (2 Checks)
   - Accessing platform capabilities without explicit declaration drops requests with a -32021 MissingRequiredClientCapabilityError returning an HTTP status code 400 Bad Request. Its error.data.requiredCapabilities is a ClientCapabilities object keyed by the missing capability (e.g. { "sampling": {} }), not an array of names.
5. Methods & Routing Mechanics (5 Checks)
   - Removed legacy endpoints (initialize, ping, logging/setLevel, etc.) or generic unknown methods must cleanly yield an HTTP status code 404 Not Found alongside a JSON-RPC -32601 Method not found payload. All error returns must preserve original request ID mappings.
   - Validates response streams contain only IncompleteResult chunks and never independent top-level JSON-RPC requests, while enforcing that no log messages are emitted when _meta.../logLevel is omitted.
6. Subscription Streams & Filtering (3 Checks)
   - Mandates that notifications/subscriptions/acknowledged is the first message on a subscriptions/listen stream, and that subsequent notifications carry a matching _meta.../subscriptionId.
   - Verifies strict containment where servers do not dispatch notification types that fall outside the client's explicit requested subscription filter list.
7. Dynamic List Mutations (2 Checks)
   - Evaluates that list-changed capable servers notify active listen streams with promptsListChanged: true or toolsListChanged: true upon live configuration or capability modifications.  

  - HttpServerNoIndependentRequestsOnStream: Request stream contains only IncompleteResult, never independent JSON-RPC requests
    Error: Failed to receive progressive stream chunk execution frames from tools/call handler endpoint

Test stateless MCP server architecture (SEP-2575).



Endpoints:
- server/discover: Returns supportedVersions and capabilities; SHOULD identify itself via _meta['io.modelcontextprotocol/serverInfo'] (spec PR #3002).
- tools/call: Implement structural test tools like test_missing_capability requiring explicit capabilities in _meta.

Grouped Specification Requirements:

1. Per-Request _meta Validation (4 Checks)
   - Rejects requests missing _meta or lacking structural required internal subfields (protocolVersion, clientCapabilities) with a JSON-RPC -32602 Invalid params error signature and an HTTP status code 400 Bad Request.
   - Serves requests whose _meta omits clientInfo (a SHOULD since spec PR #3002 — servers MUST NOT require it).
2. Discovery & Capabilities (3 Checks)
   - Implements server/discover mapping exact mandatory protocol elements.
   - Dynamically checks prompt capability declaration constraints, validates that active RPC handlers match advertised discovery capacities.
3. Version Negotiation & Headers (3 Checks)
   - Mismatched or unknown protocol versions must return an UnsupportedProtocolVersionError (HTTP status code 400 Bad Request) carrying precise version tracking arrays.
   - Absent or altered protocol version header metadata must trigger a -32020 Header Mismatch error with an HTTP 400 boundary state.
4. Client Capability Constraints (2 Checks)
   - Accessing platform capabilities without explicit declaration drops requests with a -32021 MissingRequiredClientCapabilityError returning an HTTP status code 400 Bad Request. Its error.data.requiredCapabilities is a ClientCapabilities object keyed by the missing capability (e.g. { "sampling": {} }), not an array of names.
5. Methods & Routing Mechanics (5 Checks)
   - Removed legacy endpoints (initialize, ping, logging/setLevel, etc.) or generic unknown methods must cleanly yield an HTTP status code 404 Not Found alongside a JSON-RPC -32601 Method not found payload. All error returns must preserve original request ID mappings.
   - Validates response streams contain only IncompleteResult chunks and never independent top-level JSON-RPC requests, while enforcing that no log messages are emitted when _meta.../logLevel is omitted.
6. Subscription Streams & Filtering (3 Checks)
   - Mandates that notifications/subscriptions/acknowledged is the first message on a subscriptions/listen stream, and that subsequent notifications carry a matching _meta.../subscriptionId.
   - Verifies strict containment where servers do not dispatch notification types that fall outside the client's explicit requested subscription filter list.
7. Dynamic List Mutations (2 Checks)
   - Evaluates that list-changed capable servers notify active listen streams with promptsListChanged: true or toolsListChanged: true upon live configuration or capability modifications.  

  - ServerNoLogWithoutLogLevel: No notifications/message for requests that didn't set _meta.../logLevel
    Error: Logging target endpoint context dropped or failed to yield frame structures

Test stateless MCP server architecture (SEP-2575).



Endpoints:
- server/discover: Returns supportedVersions and capabilities; SHOULD identify itself via _meta['io.modelcontextprotocol/serverInfo'] (spec PR #3002).
- tools/call: Implement structural test tools like test_missing_capability requiring explicit capabilities in _meta.

Grouped Specification Requirements:

1. Per-Request _meta Validation (4 Checks)
   - Rejects requests missing _meta or lacking structural required internal subfields (protocolVersion, clientCapabilities) with a JSON-RPC -32602 Invalid params error signature and an HTTP status code 400 Bad Request.
   - Serves requests whose _meta omits clientInfo (a SHOULD since spec PR #3002 — servers MUST NOT require it).
2. Discovery & Capabilities (3 Checks)
   - Implements server/discover mapping exact mandatory protocol elements.
   - Dynamically checks prompt capability declaration constraints, validates that active RPC handlers match advertised discovery capacities.
3. Version Negotiation & Headers (3 Checks)
   - Mismatched or unknown protocol versions must return an UnsupportedProtocolVersionError (HTTP status code 400 Bad Request) carrying precise version tracking arrays.
   - Absent or altered protocol version header metadata must trigger a -32020 Header Mismatch error with an HTTP 400 boundary state.
4. Client Capability Constraints (2 Checks)
   - Accessing platform capabilities without explicit declaration drops requests with a -32021 MissingRequiredClientCapabilityError returning an HTTP status code 400 Bad Request. Its error.data.requiredCapabilities is a ClientCapabilities object keyed by the missing capability (e.g. { "sampling": {} }), not an array of names.
5. Methods & Routing Mechanics (5 Checks)
   - Removed legacy endpoints (initialize, ping, logging/setLevel, etc.) or generic unknown methods must cleanly yield an HTTP status code 404 Not Found alongside a JSON-RPC -32601 Method not found payload. All error returns must preserve original request ID mappings.
   - Validates response streams contain only IncompleteResult chunks and never independent top-level JSON-RPC requests, while enforcing that no log messages are emitted when _meta.../logLevel is omitted.
6. Subscription Streams & Filtering (3 Checks)
   - Mandates that notifications/subscriptions/acknowledged is the first message on a subscriptions/listen stream, and that subsequent notifications carry a matching _meta.../subscriptionId.
   - Verifies strict containment where servers do not dispatch notification types that fall outside the client's explicit requested subscription filter list.
7. Dynamic List Mutations (2 Checks)
   - Evaluates that list-changed capable servers notify active listen streams with promptsListChanged: true or toolsListChanged: true upon live configuration or capability modifications.  

  - ServerSendsSubscriptionAck: notifications/subscriptions/acknowledged is the first message on a subscriptions/listen stream
    Error: Failed to open or receive frames from the subscriptions/listen stream endpoint

Test stateless MCP server architecture (SEP-2575).



Endpoints:
- server/discover: Returns supportedVersions and capabilities; SHOULD identify itself via _meta['io.modelcontextprotocol/serverInfo'] (spec PR #3002).
- tools/call: Implement structural test tools like test_missing_capability requiring explicit capabilities in _meta.

Grouped Specification Requirements:

1. Per-Request _meta Validation (4 Checks)
   - Rejects requests missing _meta or lacking structural required internal subfields (protocolVersion, clientCapabilities) with a JSON-RPC -32602 Invalid params error signature and an HTTP status code 400 Bad Request.
   - Serves requests whose _meta omits clientInfo (a SHOULD since spec PR #3002 — servers MUST NOT require it).
2. Discovery & Capabilities (3 Checks)
   - Implements server/discover mapping exact mandatory protocol elements.
   - Dynamically checks prompt capability declaration constraints, validates that active RPC handlers match advertised discovery capacities.
3. Version Negotiation & Headers (3 Checks)
   - Mismatched or unknown protocol versions must return an UnsupportedProtocolVersionError (HTTP status code 400 Bad Request) carrying precise version tracking arrays.
   - Absent or altered protocol version header metadata must trigger a -32020 Header Mismatch error with an HTTP 400 boundary state.
4. Client Capability Constraints (2 Checks)
   - Accessing platform capabilities without explicit declaration drops requests with a -32021 MissingRequiredClientCapabilityError returning an HTTP status code 400 Bad Request. Its error.data.requiredCapabilities is a ClientCapabilities object keyed by the missing capability (e.g. { "sampling": {} }), not an array of names.
5. Methods & Routing Mechanics (5 Checks)
   - Removed legacy endpoints (initialize, ping, logging/setLevel, etc.) or generic unknown methods must cleanly yield an HTTP status code 404 Not Found alongside a JSON-RPC -32601 Method not found payload. All error returns must preserve original request ID mappings.
   - Validates response streams contain only IncompleteResult chunks and never independent top-level JSON-RPC requests, while enforcing that no log messages are emitted when _meta.../logLevel is omitted.
6. Subscription Streams & Filtering (3 Checks)
   - Mandates that notifications/subscriptions/acknowledged is the first message on a subscriptions/listen stream, and that subsequent notifications carry a matching _meta.../subscriptionId.
   - Verifies strict containment where servers do not dispatch notification types that fall outside the client's explicit requested subscription filter list.
7. Dynamic List Mutations (2 Checks)
   - Evaluates that list-changed capable servers notify active listen streams with promptsListChanged: true or toolsListChanged: true upon live configuration or capability modifications.  

  - ServerTagsSubscriptionId: Listen-stream notifications carry _meta.../subscriptionId
    Error: Failed to open stream line or tracking frames are missing completely

Test stateless MCP server architecture (SEP-2575).



Endpoints:
- server/discover: Returns supportedVersions and capabilities; SHOULD identify itself via _meta['io.modelcontextprotocol/serverInfo'] (spec PR #3002).
- tools/call: Implement structural test tools like test_missing_capability requiring explicit capabilities in _meta.

Grouped Specification Requirements:

1. Per-Request _meta Validation (4 Checks)
   - Rejects requests missing _meta or lacking structural required internal subfields (protocolVersion, clientCapabilities) with a JSON-RPC -32602 Invalid params error signature and an HTTP status code 400 Bad Request.
   - Serves requests whose _meta omits clientInfo (a SHOULD since spec PR #3002 — servers MUST NOT require it).
2. Discovery & Capabilities (3 Checks)
   - Implements server/discover mapping exact mandatory protocol elements.
   - Dynamically checks prompt capability declaration constraints, validates that active RPC handlers match advertised discovery capacities.
3. Version Negotiation & Headers (3 Checks)
   - Mismatched or unknown protocol versions must return an UnsupportedProtocolVersionError (HTTP status code 400 Bad Request) carrying precise version tracking arrays.
   - Absent or altered protocol version header metadata must trigger a -32020 Header Mismatch error with an HTTP 400 boundary state.
4. Client Capability Constraints (2 Checks)
   - Accessing platform capabilities without explicit declaration drops requests with a -32021 MissingRequiredClientCapabilityError returning an HTTP status code 400 Bad Request. Its error.data.requiredCapabilities is a ClientCapabilities object keyed by the missing capability (e.g. { "sampling": {} }), not an array of names.
5. Methods & Routing Mechanics (5 Checks)
   - Removed legacy endpoints (initialize, ping, logging/setLevel, etc.) or generic unknown methods must cleanly yield an HTTP status code 404 Not Found alongside a JSON-RPC -32601 Method not found payload. All error returns must preserve original request ID mappings.
   - Validates response streams contain only IncompleteResult chunks and never independent top-level JSON-RPC requests, while enforcing that no log messages are emitted when _meta.../logLevel is omitted.
6. Subscription Streams & Filtering (3 Checks)
   - Mandates that notifications/subscriptions/acknowledged is the first message on a subscriptions/listen stream, and that subsequent notifications carry a matching _meta.../subscriptionId.
   - Verifies strict containment where servers do not dispatch notification types that fall outside the client's explicit requested subscription filter list.
7. Dynamic List Mutations (2 Checks)
   - Evaluates that list-changed capable servers notify active listen streams with promptsListChanged: true or toolsListChanged: true upon live configuration or capability modifications.  

  - ServerHonorsNotificationFilter: Server doesn't send notification types the client didn't request
    Error: Strict subscription filtering line failed to return communication frames

Test stateless MCP server architecture (SEP-2575).



Endpoints:
- server/discover: Returns supportedVersions and capabilities; SHOULD identify itself via _meta['io.modelcontextprotocol/serverInfo'] (spec PR #3002).
- tools/call: Implement structural test tools like test_missing_capability requiring explicit capabilities in _meta.

Grouped Specification Requirements:

1. Per-Request _meta Validation (4 Checks)
   - Rejects requests missing _meta or lacking structural required internal subfields (protocolVersion, clientCapabilities) with a JSON-RPC -32602 Invalid params error signature and an HTTP status code 400 Bad Request.
   - Serves requests whose _meta omits clientInfo (a SHOULD since spec PR #3002 — servers MUST NOT require it).
2. Discovery & Capabilities (3 Checks)
   - Implements server/discover mapping exact mandatory protocol elements.
   - Dynamically checks prompt capability declaration constraints, validates that active RPC handlers match advertised discovery capacities.
3. Version Negotiation & Headers (3 Checks)
   - Mismatched or unknown protocol versions must return an UnsupportedProtocolVersionError (HTTP status code 400 Bad Request) carrying precise version tracking arrays.
   - Absent or altered protocol version header metadata must trigger a -32020 Header Mismatch error with an HTTP 400 boundary state.
4. Client Capability Constraints (2 Checks)
   - Accessing platform capabilities without explicit declaration drops requests with a -32021 MissingRequiredClientCapabilityError returning an HTTP status code 400 Bad Request. Its error.data.requiredCapabilities is a ClientCapabilities object keyed by the missing capability (e.g. { "sampling": {} }), not an array of names.
5. Methods & Routing Mechanics (5 Checks)
   - Removed legacy endpoints (initialize, ping, logging/setLevel, etc.) or generic unknown methods must cleanly yield an HTTP status code 404 Not Found alongside a JSON-RPC -32601 Method not found payload. All error returns must preserve original request ID mappings.
   - Validates response streams contain only IncompleteResult chunks and never independent top-level JSON-RPC requests, while enforcing that no log messages are emitted when _meta.../logLevel is omitted.
6. Subscription Streams & Filtering (3 Checks)
   - Mandates that notifications/subscriptions/acknowledged is the first message on a subscriptions/listen stream, and that subsequent notifications carry a matching _meta.../subscriptionId.
   - Verifies strict containment where servers do not dispatch notification types that fall outside the client's explicit requested subscription filter list.
7. Dynamic List Mutations (2 Checks)
   - Evaluates that list-changed capable servers notify active listen streams with promptsListChanged: true or toolsListChanged: true upon live configuration or capability modifications.  

  - HttpServerErrorJsonrpcId: All error responses carry the request JSON-RPC id
    Error: Not testable: no error responses were observed across the probes, so the id-echo requirement could not be validated

Test stateless MCP server architecture (SEP-2575).



Endpoints:
- server/discover: Returns supportedVersions and capabilities; SHOULD identify itself via _meta['io.modelcontextprotocol/serverInfo'] (spec PR #3002).
- tools/call: Implement structural test tools like test_missing_capability requiring explicit capabilities in _meta.

Grouped Specification Requirements:

1. Per-Request _meta Validation (4 Checks)
   - Rejects requests missing _meta or lacking structural required internal subfields (protocolVersion, clientCapabilities) with a JSON-RPC -32602 Invalid params error signature and an HTTP status code 400 Bad Request.
   - Serves requests whose _meta omits clientInfo (a SHOULD since spec PR #3002 — servers MUST NOT require it).
2. Discovery & Capabilities (3 Checks)
   - Implements server/discover mapping exact mandatory protocol elements.
   - Dynamically checks prompt capability declaration constraints, validates that active RPC handlers match advertised discovery capacities.
3. Version Negotiation & Headers (3 Checks)
   - Mismatched or unknown protocol versions must return an UnsupportedProtocolVersionError (HTTP status code 400 Bad Request) carrying precise version tracking arrays.
   - Absent or altered protocol version header metadata must trigger a -32020 Header Mismatch error with an HTTP 400 boundary state.
4. Client Capability Constraints (2 Checks)
   - Accessing platform capabilities without explicit declaration drops requests with a -32021 MissingRequiredClientCapabilityError returning an HTTP status code 400 Bad Request. Its error.data.requiredCapabilities is a ClientCapabilities object keyed by the missing capability (e.g. { "sampling": {} }), not an array of names.
5. Methods & Routing Mechanics (5 Checks)
   - Removed legacy endpoints (initialize, ping, logging/setLevel, etc.) or generic unknown methods must cleanly yield an HTTP status code 404 Not Found alongside a JSON-RPC -32601 Method not found payload. All error returns must preserve original request ID mappings.
   - Validates response streams contain only IncompleteResult chunks and never independent top-level JSON-RPC requests, while enforcing that no log messages are emitted when _meta.../logLevel is omitted.
6. Subscription Streams & Filtering (3 Checks)
   - Mandates that notifications/subscriptions/acknowledged is the first message on a subscriptions/listen stream, and that subsequent notifications carry a matching _meta.../subscriptionId.
   - Verifies strict containment where servers do not dispatch notification types that fall outside the client's explicit requested subscription filter list.
7. Dynamic List Mutations (2 Checks)
   - Evaluates that list-changed capable servers notify active listen streams with promptsListChanged: true or toolsListChanged: true upon live configuration or capability modifications.

## `logging-set-level`

Endpoint: logging/setLevel

Requirements:
- Accept log level setting
- Filter subsequent log notifications based on level
- Return empty object {}

Log Levels (in order of severity):
- debug
- info
- notice
- warning
- error
- critical
- alert
- emergency

## `ping`

Endpoint: ping

Requirements:
- Accept ping request with no parameters
- Respond promptly with empty object {}

Request Format:

```json
{
  "jsonrpc": "2.0",
  "id": "123",
  "method": "ping"
}
`

Response Format:

`json
{
  "jsonrpc": "2.0",
  "id": "123",
  "result": {}
}
```

Implementation Note: The ping utility allows either party to verify that their counterpart is still responsive and the connection is alive.

## `completion-complete`

Endpoint: completion/complete

Requirements:
- Accept completion requests for prompt or resource template arguments
- Provide contextual suggestions based on partial input
- Return array of completion values ranked by relevance

Request Format:

```json
{
  "method": "completion/complete",
  "params": {
    "ref": {
      "type": "ref/prompt",
      "name": "test_prompt_with_arguments"
    },
    "argument": {
      "name": "arg1",
      "value": "par"
    }
  }
}
`

Response Format:

`json
{
  "completion": {
    "values": ["paris", "park", "party"],
    "total": 150,
    "hasMore": false
  }
}
```

Implementation Note: For conformance testing, completion support can be minimal or return empty arrays. The capability just needs to be declared and the endpoint must respond correctly.

## `tools-list`

Endpoint: tools/list

Requirements:
- Return array of all available tools
- Each tool MUST have:
  - name (string, 1-64 chars, matching ^[A-Za-z0-9_./-]+$)
  - description (string)
  - inputSchema (valid JSON Schema object)

## `tools-call-simple-text`

Implement tool test_simple_text with no arguments that returns:

```json
{
  "content": [
    {
      "type": "text",
      "text": "This is a simple text response for testing."
    }
  ]
}
``

## `tools-call-image`

Implement tool test_image_content with no arguments that returns:

```json
{
  "content": [
    {
      "type": "image",
      "data": "<base64-encoded-png>",
      "mimeType": "image/png"
    }
  ]
}
```

Implementation Note: Use a minimal test image (e.g., 1x1 red pixel PNG)

## `tools-call-audio`

Implement tool test_audio_content with no arguments that returns:

```json
{
  "content": [
    {
      "type": "audio",
      "data": "<base64-encoded-wav>",
      "mimeType": "audio/wav"
    }
  ]
}
```

Implementation Note: Use a minimal test audio file

## `tools-call-embedded-resource`

Implement tool test_embedded_resource with no arguments that returns:

```json
{
  "content": [
    {
      "type": "resource",
      "resource": {
        "uri": "test://embedded-resource",
        "mimeType": "text/plain",
        "text": "This is an embedded resource content."
      }
    }
  ]
}
``

## `tools-call-mixed-content`

Implement tool test_multiple_content_types with no arguments that returns:

```json
{
  "content": [
    {
      "type": "text",
      "text": "Multiple content types test:"
    },
    {
      "type": "image",
      "data": "<base64>",
      "mimeType": "image/png"
    },
    {
      "type": "resource",
      "resource": {
        "uri": "test://mixed-content-resource",
        "mimeType": "application/json",
        "text": "{"test":"data","value":123}"
      }
    }
  ]
}
``

## `tools-call-with-logging`

Implement tool test_tool_with_logging with no arguments.

Behavior: During execution, send 3 log notifications at info level:
1. "Tool execution started"
2. "Tool processing data" (after ~50ms delay)
3. "Tool execution completed" (after another ~50ms delay)

Returns: Text content confirming execution

Implementation Note: The delays are important to test that clients can receive multiple log notifications during tool execution

## `tools-call-error`

Implement tool test_error_handling with no arguments.

Behavior: Always throw an error

Returns: JSON-RPC response with isError: true

```json
{
  "isError": true,
  "content": [
    {
      "type": "text",
      "text": "This tool intentionally returns an error for testing"
    }
  ]
}
``

## `tools-call-with-progress`

Implement tool test_tool_with_progress with no arguments.

Behavior: If _meta.progressToken is provided in request:
- Send progress notification: 0/100
- Wait ~50ms
- Send progress notification: 50/100
- Wait ~50ms
- Send progress notification: 100/100

If no progress token provided, just execute with delays.

Returns: Text content confirming execution

Progress Notification Format:

```json
{
  "method": "notifications/progress",
  "params": {
    "progressToken": "<from request._meta.progressToken>",
    "progress": 50,
    "total": 100
  }
}
``

## `tools-call-sampling`

Implement tool test_sampling with argument:
- prompt (string, required) - The prompt to send to the LLM

Behavior: Request LLM sampling from the client using sampling/createMessage

Sampling Request:

```json
{
  "method": "sampling/createMessage",
  "params": {
    "messages": [
      {
        "role": "user",
        "content": {
          "type": "text",
          "text": "<prompt from arguments>"
        }
      }
    ],
    "maxTokens": 100
  }
}
`

Returns: Text content with the LLM's response

`json
{
  "content": [
    {
      "type": "text",
      "text": "LLM response: <response from sampling>"
    }
  ]
}
`

Implementation Note: If the client doesn't support sampling (no sampling` capability), return an error.

## `tools-call-elicitation`

Implement tool test_elicitation with argument:
- message (string, required) - The message to show the user

Behavior: Request user input from the client using elicitation/create

Elicitation Request:

```json
{
  "method": "elicitation/create",
  "params": {
    "message": "<message from arguments>",
    "requestedSchema": {
      "type": "object",
      "properties": {
        "username": {
          "type": "string",
          "description": "User's response"
        },
        "email": {
          "type": "string",
          "description": "User's email address"
        }
      },
      "required": ["username", "email"]
    }
  }
}
`

Returns: Text content with the user's response

`json
{
  "content": [
    {
      "type": "text",
      "text": "User response: <action: accept/decline/cancel, content: {...}>"
    }
  ]
}
`

Implementation Note: If the client doesn't support elicitation (no elicitation` capability), return an error.

## `json-schema-2020-12`

Implement tool json_schema_2020_12_tool with inputSchema containing JSON Schema 2020-12 features, including the broader vocabulary permitted by SEP-2106 (an $anchor inside $defs, composition keywords allOf/anyOf, and conditional keywords if/then/else):

```json
{
  "name": "json_schema_2020_12_tool",
  "description": "Tool with JSON Schema 2020-12 features",
  "inputSchema": {
    "$schema": "https://json-schema.org/draft/2020-12/schema",
    "type": "object",
    "$defs": {
      "address": {
        "$anchor": "addressDef",
        "type": "object",
        "properties": {
          "street": {
            "type": "string"
          },
          "city": {
            "type": "string"
          }
        }
      }
    },
    "properties": {
      "name": {
        "type": "string"
      },
      "address": {
        "$ref": "#/$defs/address"
      },
      "contactMethod": {
        "type": "string",
        "enum": [
          "phone",
          "email"
        ]
      },
      "phone": {
        "type": "string"
      },
      "email": {
        "type": "string"
      }
    },
    "allOf": [
      {
        "anyOf": [
          {
            "required": [
              "phone"
            ]
          },
          {
            "required": [
              "email"
            ]
          }
        ]
      }
    ],
    "if": {
      "properties": {
        "contactMethod": {
          "const": "phone"
        }
      },
      "required": [
        "contactMethod"
      ]
    },
    "then": {
      "required": [
        "phone"
      ]
    },
    "else": {
      "required": [
        "email"
      ]
    },
    "additionalProperties": false
  }
}
`

Verification: The test verifies that $schema, $defs, and additionalProperties are preserved (SEP-1613), and that the composition (allOf/anyOf), conditional (if/then/else), and $anchor` keywords are preserved (SEP-2106), in the tool listing response.

## `elicitation-sep1034-defaults`

Implement a tool named test_elicitation_sep1034_defaults (no arguments) that requests elicitation/create from the client with a schema containing default values for all primitive types:
- name (string): default "John Doe"
- age (integer): default 30
- score (number): default 95.5
- status (string enum: ["active", "inactive", "pending"]): default "active"
- verified (boolean): default true

Returns: Text content with the elicitation result

```json
{
  "content": [
    {
      "type": "text",
      "text": "Elicitation completed: action=<accept/decline/cancel>, content={...}"
    }
  ]
}
``

## `elicitation-sep1330-enums`

Implement a tool named test_elicitation_sep1330_enums (no arguments) that requests elicitation/create from the client with a schema containing all 5 enum variants:

1. Untitled single-select: { type: "string", enum: ["option1", "option2", "option3"] }
2. Titled single-select: { type: "string", oneOf: [{ const: "value1", title: "First Option" }, ...] }
3. Legacy titled (deprecated): { type: "string", enum: ["opt1", "opt2", "opt3"], enumNames: ["Option One", "Option Two", "Option Three"] }
4. Untitled multi-select: { type: "array", items: { type: "string", enum: ["option1", "option2", "option3"] } }
5. Titled multi-select: { type: "array", items: { anyOf: [{ const: "value1", title: "First Choice" }, ...] } }

Returns: Text content with the elicitation result

```json
{
  "content": [
    {
      "type": "text",
      "text": "Elicitation completed: action=<accept/decline/cancel>, content={...}"
    }
  ]
}
``

## `resources-list`

Endpoint: resources/list

Requirements:
- Return array of all available direct resources (not templates)
- Each resource MUST have:
  - uri (string)
  - name (string)
  - description (string)
  - mimeType (string, optional)

## `resources-read-text`

Implement resource test://static-text that returns:

```json
{
  "contents": [
    {
      "uri": "test://static-text",
      "mimeType": "text/plain",
      "text": "This is the content of the static text resource."
    }
  ]
}
``

## `resources-read-binary`

Implement resource test://static-binary that returns:

```json
{
  "contents": [
    {
      "uri": "test://static-binary",
      "mimeType": "image/png",
      "blob": "<base64-encoded-png>"
    }
  ]
}
``

## `resources-templates-read`

Implement resource template test://template/{id}/data that substitutes parameters.

Behavior: When client requests test://template/123/data, substitute {id} with 123

Returns (for uri: "test://template/123/data"):

```json
{
  "contents": [
    {
      "uri": "test://template/123/data",
      "mimeType": "application/json",
      "text": "{"id":"123","templateTest":true,"data":"Data for ID: 123"}"
    }
  ]
}
``

## `resources-subscribe`

Endpoint: resources/subscribe

Requirements:
- Accept subscription request with URI
- Track subscribed URIs
- Return empty object {}

Example request:

```json
{
  "method": "resources/subscribe",
  "params": {
    "uri": "test://watched-resource"
  }
}
``

## `resources-unsubscribe`

Endpoint: resources/unsubscribe

Requirements:
- Accept unsubscribe request with URI
- Remove URI from subscriptions
- Stop sending update notifications for that URI
- Return empty object {}

## `sep-2164-resource-not-found`

Endpoint: resources/read

When a client requests a URI that does not correspond to any resource, the server:

- MUST NOT return a result with an empty contents array
- SHOULD return a JSON-RPC error with code -32602 (Invalid Params)
- SHOULD include the requested uri in the error data field

Example error response:

```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "error": {
    "code": -32602,
    "message": "Resource not found",
    "data": {
      "uri": "test://nonexistent-resource-for-conformance-testing"
    }
  }
}
```

This scenario does not require the server to register any specific resource — it tests behavior when reading a URI the server does not recognize.

## `prompts-list`

Endpoint: prompts/list

Requirements:
- Return array of all available prompts
- Each prompt MUST have:
  - name (string)
  - description (string)
  - arguments (array, optional) - list of required arguments

## `prompts-get-simple`

Implement a prompt named test_simple_prompt with no arguments that returns:

```json
{
  "messages": [
    {
      "role": "user",
      "content": {
        "type": "text",
        "text": "This is a simple prompt for testing."
      }
    }
  ]
}
``

## `prompts-get-with-args`

Implement a prompt named test_prompt_with_arguments with arguments:
- arg1 (string, required) - First test argument
- arg2 (string, required) - Second test argument

Returns (with args {arg1: "hello", arg2: "world"}):

```json
{
  "messages": [
    {
      "role": "user",
      "content": {
        "type": "text",
        "text": "Prompt with arguments: arg1='hello', arg2='world'"
      }
    }
  ]
}
``

## `prompts-get-embedded-resource`

Implement a prompt named test_prompt_with_embedded_resource with argument:
- resourceUri (string, required) - URI of the resource to embed

Returns:

```json
{
  "messages": [
    {
      "role": "user",
      "content": {
        "type": "resource",
        "resource": {
          "uri": "<resourceUri from arguments>",
          "mimeType": "text/plain",
          "text": "Embedded resource content for testing."
        }
      }
    },
    {
      "role": "user",
      "content": {
        "type": "text",
        "text": "Please process the embedded resource above."
      }
    }
  ]
}
``

## `prompts-get-with-image`

Implement a prompt named test_prompt_with_image with no arguments that returns:

```json
{
  "messages": [
    {
      "role": "user",
      "content": {
        "type": "image",
        "data": "<base64-encoded-png>",
        "mimeType": "image/png"
      }
    },
    {
      "role": "user",
      "content": {
        "type": "text",
        "text": "Please analyze the image above."
      }
    }
  ]
}
``

## `caching`

Servers MUST include ttlMs (integer >= 0) and cacheScope ("public" or "private") on results from:
- tools/list
- prompts/list
- resources/list
- resources/templates/list
- resources/read

  - PromptsListCachingHints: prompts/list response includes ttlMs and cacheScope caching hints
    Error: prompts/list request failed: fetch failed

Test that servers include caching hints (ttlMs and cacheScope) on cacheable results (SEP-2549).



Servers MUST include ttlMs (integer >= 0) and cacheScope ("public" or "private") on results from:
- tools/list
- prompts/list
- resources/list
- resources/templates/list
- resources/read

  - ResourcesListCachingHints: resources/list response includes ttlMs and cacheScope caching hints
    Error: resources/list request failed: fetch failed

Test that servers include caching hints (ttlMs and cacheScope) on cacheable results (SEP-2549).



Servers MUST include ttlMs (integer >= 0) and cacheScope ("public" or "private") on results from:
- tools/list
- prompts/list
- resources/list
- resources/templates/list
- resources/read

  - ResourcesTemplatesListCachingHints: resources/templates/list response includes ttlMs and cacheScope caching hints
    Error: resources/templates/list request failed: fetch failed

Test that servers include caching hints (ttlMs and cacheScope) on cacheable results (SEP-2549).



Servers MUST include ttlMs (integer >= 0) and cacheScope ("public" or "private") on results from:
- tools/list
- prompts/list
- resources/list
- resources/templates/list
- resources/read

  - TtlNonNegative: All ttlMs values are non-negative integers
    Error: no endpoints returned ttlMs

Test that servers include caching hints (ttlMs and cacheScope) on cacheable results (SEP-2549).



Servers MUST include ttlMs (integer >= 0) and cacheScope ("public" or "private") on results from:
- tools/list
- prompts/list
- resources/list
- resources/templates/list
- resources/read

  - CacheScopeValid: All cacheScope values are "public" or "private"
    Error: no endpoints returned cacheScope

Test that servers include caching hints (ttlMs and cacheScope) on cacheable results (SEP-2549).



Servers MUST include ttlMs (integer >= 0) and cacheScope ("public" or "private") on results from:
- tools/list
- prompts/list
- resources/list
- resources/templates/list
- resources/read

## `http-header-validation`

Endpoint: Streamable HTTP

Requirements:
- Server MUST reject requests where Mcp-Method header doesn't match the body method
- Server MUST reject requests where Mcp-Name header doesn't match the body params.name/uri
- Server MUST accept header names case-insensitively
- Server MUST reject case-mismatched header values (method values are case-sensitive)
- Server MUST accept extra whitespace around header values (per HTTP spec)
- Server MUST return HTTP 400 Bad Request for validation failures
- Server MUST return JSON-RPC error with code -32020 (HeaderMismatch)

## `http-custom-header-server-validation`

Endpoint: Streamable HTTP with at least one tool using x-mcp-header

Requirements:
- Server MUST validate Base64-encoded header values
- Server MUST reject requests with invalid Base64 padding or characters
- Server MUST treat values without =?base64?...?= wrapper as literal
- Server MUST reject requests where custom header is omitted but value is in body

  - NotObserved: Declared check sep-2243-server-decode-base64 was never emitted
    Error: Check was not observed: custom-header validation setup failed before this case ran.

Test server validation of custom Mcp-Param headers and Base64 encoding (SEP-2243).



Endpoint: Streamable HTTP with at least one tool using x-mcp-header

Requirements:
- Server MUST validate Base64-encoded header values
- Server MUST reject requests with invalid Base64 padding or characters
- Server MUST treat values without =?base64?...?= wrapper as literal
- Server MUST reject requests where custom header is omitted but value is in body

  - NotObserved: Declared check sep-2243-server-validate-param-match was never emitted
    Error: Check was not observed: custom-header validation setup failed before this case ran.

Test server validation of custom Mcp-Param headers and Base64 encoding (SEP-2243).



Endpoint: Streamable HTTP with at least one tool using x-mcp-header

Requirements:
- Server MUST validate Base64-encoded header values
- Server MUST reject requests with invalid Base64 padding or characters
- Server MUST treat values without =?base64?...?= wrapper as literal
- Server MUST reject requests where custom header is omitted but value is in body

  - NotObserved: Declared check sep-2243-server-reject-invalid-param-chars was never emitted
    Error: Check was not observed: custom-header validation setup failed before this case ran.

Test server validation of custom Mcp-Param headers and Base64 encoding (SEP-2243).



Endpoint: Streamable HTTP with at least one tool using x-mcp-header

Requirements:
- Server MUST validate Base64-encoded header values
- Server MUST reject requests with invalid Base64 padding or characters
- Server MUST treat values without =?base64?...?= wrapper as literal
- Server MUST reject requests where custom header is omitted but value is in body

  - NotObserved: Declared check sep-2243-server-reject-param-mismatch was never emitted
    Error: Check was not observed: custom-header validation setup failed before this case ran.

Test server validation of custom Mcp-Param headers and Base64 encoding (SEP-2243).



Endpoint: Streamable HTTP with at least one tool using x-mcp-header

Requirements:
- Server MUST validate Base64-encoded header values
- Server MUST reject requests with invalid Base64 padding or characters
- Server MUST treat values without =?base64?...?= wrapper as literal
- Server MUST reject requests where custom header is omitted but value is in body

## `tasks-lifecycle`

Server Implementation Requirements (SEP-2663):

The server MUST advertise io.modelcontextprotocol/tasks under
capabilities.extensions and gate the task surface on negotiation.

Sync dispatch (no task created):
- A tools/call against a sync-only tool MUST return a flat
  ToolResult with resultType:"complete" and a content[] array.
- It MUST NOT carry taskId at the top level (that would imply a
  CreateTaskResult).

Server-directed task creation:
- For task-supporting tools, the server decides whether to create a task —
  the client MUST NOT need to opt in via a request param.
- The response MUST be a CreateTaskResult — a flat Result & Task
  intersection: resultType:"task", plus taskId / status /
  createdAt / lastUpdatedAt / ttlMs at the top level.
  There MUST NOT be a nested task wrapper key.

tasks/get DetailedTask:
- Working tasks return status and basic metadata; result/error are
  absent.
- Completed tasks MUST inline the original tool result under result
  with content[]. There is no separate tasks/result method.

Tool errors vs protocol errors (SEP-2663 §error-semantics):
- A tool that ran but reported an error MUST surface as
  status:"completed" with result.isError:true. The status
  "failed" is reserved for protocol-level errors.
- A protocol-level error (server crash, internal failure) MUST surface
  as status:"failed" with an inlined error object (JSON-RPC
  error shape: code/message/data) and MUST NOT carry result.

Cancellation:
- tasks/cancel MUST return an empty
  {resultType:"complete"} ack — no task envelope (SEP-2322
  discriminator). The cancelled status is observed via the next
  tasks/get.
- tasks/cancel against a terminal task returns the same empty ack
  (idempotent) — the spec reserves -32602 for unknown taskIds only.

Required server fixtures (tools/list MUST include all):
- greet — sync-only, returns Hello, {name}!.
- slow_compute — task-supporting, seconds-second sleep then a
  result. seconds: 0 for the immediate path. MUST settle to
  cancelled (not completed/failed) when tasks/cancel
  arrives while running, so the lifecycle cancel check has a
  deterministic terminal status.
- failing_job — task-supporting, always returns a tool execution
  error after ~1s.
- protocol_error_job — task-supporting, panics into a protocol
  error.

  - TasksServerTaskCreation: Task-supporting tool returns flat CreateTaskResult (no nested `task` wrapper)
    Error: fetch failed

Test SEP-2663 Tasks extension lifecycle on the server.

Server Implementation Requirements (SEP-2663):

The server MUST advertise io.modelcontextprotocol/tasks under
capabilities.extensions and gate the task surface on negotiation.

Sync dispatch (no task created):
- A tools/call against a sync-only tool MUST return a flat
  ToolResult with resultType:"complete" and a content[] array.
- It MUST NOT carry taskId at the top level (that would imply a
  CreateTaskResult).

Server-directed task creation:
- For task-supporting tools, the server decides whether to create a task —
  the client MUST NOT need to opt in via a request param.
- The response MUST be a CreateTaskResult — a flat Result & Task
  intersection: resultType:"task", plus taskId / status /
  createdAt / lastUpdatedAt / ttlMs at the top level.
  There MUST NOT be a nested task wrapper key.

tasks/get DetailedTask:
- Working tasks return status and basic metadata; result/error are
  absent.
- Completed tasks MUST inline the original tool result under result
  with content[]. There is no separate tasks/result method.

Tool errors vs protocol errors (SEP-2663 §error-semantics):
- A tool that ran but reported an error MUST surface as
  status:"completed" with result.isError:true. The status
  "failed" is reserved for protocol-level errors.
- A protocol-level error (server crash, internal failure) MUST surface
  as status:"failed" with an inlined error object (JSON-RPC
  error shape: code/message/data) and MUST NOT carry result.

Cancellation:
- tasks/cancel MUST return an empty
  {resultType:"complete"} ack — no task envelope (SEP-2322
  discriminator). The cancelled status is observed via the next
  tasks/get.
- tasks/cancel against a terminal task returns the same empty ack
  (idempotent) — the spec reserves -32602 for unknown taskIds only.

Required server fixtures (tools/list MUST include all):
- greet — sync-only, returns Hello, {name}!.
- slow_compute — task-supporting, seconds-second sleep then a
  result. seconds: 0 for the immediate path. MUST settle to
  cancelled (not completed/failed) when tasks/cancel
  arrives while running, so the lifecycle cancel check has a
  deterministic terminal status.
- failing_job — task-supporting, always returns a tool execution
  error after ~1s.
- protocol_error_job — task-supporting, panics into a protocol
  error.

  - TasksGetDuringWorking: tasks/get returns status + metadata for an active task
    Error: Not testable: no task was created by the preceding step, so this check could not be exercised

Test SEP-2663 Tasks extension lifecycle on the server.

Server Implementation Requirements (SEP-2663):

The server MUST advertise io.modelcontextprotocol/tasks under
capabilities.extensions and gate the task surface on negotiation.

Sync dispatch (no task created):
- A tools/call against a sync-only tool MUST return a flat
  ToolResult with resultType:"complete" and a content[] array.
- It MUST NOT carry taskId at the top level (that would imply a
  CreateTaskResult).

Server-directed task creation:
- For task-supporting tools, the server decides whether to create a task —
  the client MUST NOT need to opt in via a request param.
- The response MUST be a CreateTaskResult — a flat Result & Task
  intersection: resultType:"task", plus taskId / status /
  createdAt / lastUpdatedAt / ttlMs at the top level.
  There MUST NOT be a nested task wrapper key.

tasks/get DetailedTask:
- Working tasks return status and basic metadata; result/error are
  absent.
- Completed tasks MUST inline the original tool result under result
  with content[]. There is no separate tasks/result method.

Tool errors vs protocol errors (SEP-2663 §error-semantics):
- A tool that ran but reported an error MUST surface as
  status:"completed" with result.isError:true. The status
  "failed" is reserved for protocol-level errors.
- A protocol-level error (server crash, internal failure) MUST surface
  as status:"failed" with an inlined error object (JSON-RPC
  error shape: code/message/data) and MUST NOT carry result.

Cancellation:
- tasks/cancel MUST return an empty
  {resultType:"complete"} ack — no task envelope (SEP-2322
  discriminator). The cancelled status is observed via the next
  tasks/get.
- tasks/cancel against a terminal task returns the same empty ack
  (idempotent) — the spec reserves -32602 for unknown taskIds only.

Required server fixtures (tools/list MUST include all):
- greet — sync-only, returns Hello, {name}!.
- slow_compute — task-supporting, seconds-second sleep then a
  result. seconds: 0 for the immediate path. MUST settle to
  cancelled (not completed/failed) when tasks/cancel
  arrives while running, so the lifecycle cancel check has a
  deterministic terminal status.
- failing_job — task-supporting, always returns a tool execution
  error after ~1s.
- protocol_error_job — task-supporting, panics into a protocol
  error.

  - TasksGetTerminalInlinedResult: Completed task tasks/get inlines result with content[] (no separate tasks/result method)
    Error: Not testable: no task was created by the preceding step, so this check could not be exercised

Test SEP-2663 Tasks extension lifecycle on the server.

Server Implementation Requirements (SEP-2663):

The server MUST advertise io.modelcontextprotocol/tasks under
capabilities.extensions and gate the task surface on negotiation.

Sync dispatch (no task created):
- A tools/call against a sync-only tool MUST return a flat
  ToolResult with resultType:"complete" and a content[] array.
- It MUST NOT carry taskId at the top level (that would imply a
  CreateTaskResult).

Server-directed task creation:
- For task-supporting tools, the server decides whether to create a task —
  the client MUST NOT need to opt in via a request param.
- The response MUST be a CreateTaskResult — a flat Result & Task
  intersection: resultType:"task", plus taskId / status /
  createdAt / lastUpdatedAt / ttlMs at the top level.
  There MUST NOT be a nested task wrapper key.

tasks/get DetailedTask:
- Working tasks return status and basic metadata; result/error are
  absent.
- Completed tasks MUST inline the original tool result under result
  with content[]. There is no separate tasks/result method.

Tool errors vs protocol errors (SEP-2663 §error-semantics):
- A tool that ran but reported an error MUST surface as
  status:"completed" with result.isError:true. The status
  "failed" is reserved for protocol-level errors.
- A protocol-level error (server crash, internal failure) MUST surface
  as status:"failed" with an inlined error object (JSON-RPC
  error shape: code/message/data) and MUST NOT carry result.

Cancellation:
- tasks/cancel MUST return an empty
  {resultType:"complete"} ack — no task envelope (SEP-2322
  discriminator). The cancelled status is observed via the next
  tasks/get.
- tasks/cancel against a terminal task returns the same empty ack
  (idempotent) — the spec reserves -32602 for unknown taskIds only.

Required server fixtures (tools/list MUST include all):
- greet — sync-only, returns Hello, {name}!.
- slow_compute — task-supporting, seconds-second sleep then a
  result. seconds: 0 for the immediate path. MUST settle to
  cancelled (not completed/failed) when tasks/cancel
  arrives while running, so the lifecycle cancel check has a
  deterministic terminal status.
- failing_job — task-supporting, always returns a tool execution
  error after ~1s.
- protocol_error_job — task-supporting, panics into a protocol
  error.

  - TasksToolErrorCompletedIsError: Tool execution error reports as completed + result.isError (NOT failed)
    Error: fetch failed

Test SEP-2663 Tasks extension lifecycle on the server.

Server Implementation Requirements (SEP-2663):

The server MUST advertise io.modelcontextprotocol/tasks under
capabilities.extensions and gate the task surface on negotiation.

Sync dispatch (no task created):
- A tools/call against a sync-only tool MUST return a flat
  ToolResult with resultType:"complete" and a content[] array.
- It MUST NOT carry taskId at the top level (that would imply a
  CreateTaskResult).

Server-directed task creation:
- For task-supporting tools, the server decides whether to create a task —
  the client MUST NOT need to opt in via a request param.
- The response MUST be a CreateTaskResult — a flat Result & Task
  intersection: resultType:"task", plus taskId / status /
  createdAt / lastUpdatedAt / ttlMs at the top level.
  There MUST NOT be a nested task wrapper key.

tasks/get DetailedTask:
- Working tasks return status and basic metadata; result/error are
  absent.
- Completed tasks MUST inline the original tool result under result
  with content[]. There is no separate tasks/result method.

Tool errors vs protocol errors (SEP-2663 §error-semantics):
- A tool that ran but reported an error MUST surface as
  status:"completed" with result.isError:true. The status
  "failed" is reserved for protocol-level errors.
- A protocol-level error (server crash, internal failure) MUST surface
  as status:"failed" with an inlined error object (JSON-RPC
  error shape: code/message/data) and MUST NOT carry result.

Cancellation:
- tasks/cancel MUST return an empty
  {resultType:"complete"} ack — no task envelope (SEP-2322
  discriminator). The cancelled status is observed via the next
  tasks/get.
- tasks/cancel against a terminal task returns the same empty ack
  (idempotent) — the spec reserves -32602 for unknown taskIds only.

Required server fixtures (tools/list MUST include all):
- greet — sync-only, returns Hello, {name}!.
- slow_compute — task-supporting, seconds-second sleep then a
  result. seconds: 0 for the immediate path. MUST settle to
  cancelled (not completed/failed) when tasks/cancel
  arrives while running, so the lifecycle cancel check has a
  deterministic terminal status.
- failing_job — task-supporting, always returns a tool execution
  error after ~1s.
- protocol_error_job — task-supporting, panics into a protocol
  error.

  - TasksProtocolErrorFailedShape: Protocol-level error reports as failed + inlined error{code,message}, no result
    Error: fetch failed

Test SEP-2663 Tasks extension lifecycle on the server.

Server Implementation Requirements (SEP-2663):

The server MUST advertise io.modelcontextprotocol/tasks under
capabilities.extensions and gate the task surface on negotiation.

Sync dispatch (no task created):
- A tools/call against a sync-only tool MUST return a flat
  ToolResult with resultType:"complete" and a content[] array.
- It MUST NOT carry taskId at the top level (that would imply a
  CreateTaskResult).

Server-directed task creation:
- For task-supporting tools, the server decides whether to create a task —
  the client MUST NOT need to opt in via a request param.
- The response MUST be a CreateTaskResult — a flat Result & Task
  intersection: resultType:"task", plus taskId / status /
  createdAt / lastUpdatedAt / ttlMs at the top level.
  There MUST NOT be a nested task wrapper key.

tasks/get DetailedTask:
- Working tasks return status and basic metadata; result/error are
  absent.
- Completed tasks MUST inline the original tool result under result
  with content[]. There is no separate tasks/result method.

Tool errors vs protocol errors (SEP-2663 §error-semantics):
- A tool that ran but reported an error MUST surface as
  status:"completed" with result.isError:true. The status
  "failed" is reserved for protocol-level errors.
- A protocol-level error (server crash, internal failure) MUST surface
  as status:"failed" with an inlined error object (JSON-RPC
  error shape: code/message/data) and MUST NOT carry result.

Cancellation:
- tasks/cancel MUST return an empty
  {resultType:"complete"} ack — no task envelope (SEP-2322
  discriminator). The cancelled status is observed via the next
  tasks/get.
- tasks/cancel against a terminal task returns the same empty ack
  (idempotent) — the spec reserves -32602 for unknown taskIds only.

Required server fixtures (tools/list MUST include all):
- greet — sync-only, returns Hello, {name}!.
- slow_compute — task-supporting, seconds-second sleep then a
  result. seconds: 0 for the immediate path. MUST settle to
  cancelled (not completed/failed) when tasks/cancel
  arrives while running, so the lifecycle cancel check has a
  deterministic terminal status.
- failing_job — task-supporting, always returns a tool execution
  error after ~1s.
- protocol_error_job — task-supporting, panics into a protocol
  error.

  - TasksCancelEmptyAck: tasks/cancel returns {resultType:"complete"} ack; status settles to cancelled
    Error: fetch failed

Test SEP-2663 Tasks extension lifecycle on the server.

Server Implementation Requirements (SEP-2663):

The server MUST advertise io.modelcontextprotocol/tasks under
capabilities.extensions and gate the task surface on negotiation.

Sync dispatch (no task created):
- A tools/call against a sync-only tool MUST return a flat
  ToolResult with resultType:"complete" and a content[] array.
- It MUST NOT carry taskId at the top level (that would imply a
  CreateTaskResult).

Server-directed task creation:
- For task-supporting tools, the server decides whether to create a task —
  the client MUST NOT need to opt in via a request param.
- The response MUST be a CreateTaskResult — a flat Result & Task
  intersection: resultType:"task", plus taskId / status /
  createdAt / lastUpdatedAt / ttlMs at the top level.
  There MUST NOT be a nested task wrapper key.

tasks/get DetailedTask:
- Working tasks return status and basic metadata; result/error are
  absent.
- Completed tasks MUST inline the original tool result under result
  with content[]. There is no separate tasks/result method.

Tool errors vs protocol errors (SEP-2663 §error-semantics):
- A tool that ran but reported an error MUST surface as
  status:"completed" with result.isError:true. The status
  "failed" is reserved for protocol-level errors.
- A protocol-level error (server crash, internal failure) MUST surface
  as status:"failed" with an inlined error object (JSON-RPC
  error shape: code/message/data) and MUST NOT carry result.

Cancellation:
- tasks/cancel MUST return an empty
  {resultType:"complete"} ack — no task envelope (SEP-2322
  discriminator). The cancelled status is observed via the next
  tasks/get.
- tasks/cancel against a terminal task returns the same empty ack
  (idempotent) — the spec reserves -32602 for unknown taskIds only.

Required server fixtures (tools/list MUST include all):
- greet — sync-only, returns Hello, {name}!.
- slow_compute — task-supporting, seconds-second sleep then a
  result. seconds: 0 for the immediate path. MUST settle to
  cancelled (not completed/failed) when tasks/cancel
  arrives while running, so the lifecycle cancel check has a
  deterministic terminal status.
- failing_job — task-supporting, always returns a tool execution
  error after ~1s.
- protocol_error_job — task-supporting, panics into a protocol
  error.

  - TasksCancelTerminalIdempotentAck: tasks/cancel on a terminal task returns the same empty-ack as on an active task (idempotent)
    Error: fetch failed

Test SEP-2663 Tasks extension lifecycle on the server.

Server Implementation Requirements (SEP-2663):

The server MUST advertise io.modelcontextprotocol/tasks under
capabilities.extensions and gate the task surface on negotiation.

Sync dispatch (no task created):
- A tools/call against a sync-only tool MUST return a flat
  ToolResult with resultType:"complete" and a content[] array.
- It MUST NOT carry taskId at the top level (that would imply a
  CreateTaskResult).

Server-directed task creation:
- For task-supporting tools, the server decides whether to create a task —
  the client MUST NOT need to opt in via a request param.
- The response MUST be a CreateTaskResult — a flat Result & Task
  intersection: resultType:"task", plus taskId / status /
  createdAt / lastUpdatedAt / ttlMs at the top level.
  There MUST NOT be a nested task wrapper key.

tasks/get DetailedTask:
- Working tasks return status and basic metadata; result/error are
  absent.
- Completed tasks MUST inline the original tool result under result
  with content[]. There is no separate tasks/result method.

Tool errors vs protocol errors (SEP-2663 §error-semantics):
- A tool that ran but reported an error MUST surface as
  status:"completed" with result.isError:true. The status
  "failed" is reserved for protocol-level errors.
- A protocol-level error (server crash, internal failure) MUST surface
  as status:"failed" with an inlined error object (JSON-RPC
  error shape: code/message/data) and MUST NOT carry result.

Cancellation:
- tasks/cancel MUST return an empty
  {resultType:"complete"} ack — no task envelope (SEP-2322
  discriminator). The cancelled status is observed via the next
  tasks/get.
- tasks/cancel against a terminal task returns the same empty ack
  (idempotent) — the spec reserves -32602 for unknown taskIds only.

Required server fixtures (tools/list MUST include all):
- greet — sync-only, returns Hello, {name}!.
- slow_compute — task-supporting, seconds-second sleep then a
  result. seconds: 0 for the immediate path. MUST settle to
  cancelled (not completed/failed) when tasks/cancel
  arrives while running, so the lifecycle cancel check has a
  deterministic terminal status.
- failing_job — task-supporting, always returns a tool execution
  error after ~1s.
- protocol_error_job — task-supporting, panics into a protocol
  error.

## `tasks-wire-fields`

Wire-field renames (SEP-2663):
- The TTL field is named ttlMs on the wire (the v1 ttl key was
  in milliseconds-by-convention; SEP-2663 puts the unit in the field
  name and standardised on the Ms suffix for all duration fields).
- The poll-interval field is named pollIntervalMs (v1 used
  pollInterval).
- A CreateTaskResult MUST NOT carry the legacy ttl or
  pollInterval keys — clients keying off v1 names on a v2 server
  would silently miss the TTL guidance.
- Both ttlMs and pollIntervalMs are integer milliseconds.
  Servers MUST NOT emit fractional values.

TTL non-expiry (SEP-2663):
- A task MUST remain accessible via tasks/get for the duration of
  its ttlMs; a server MUST NOT expire it earlier.

Inlined-result _meta (SEP-2663):
- The v1 io.modelcontextprotocol/related-task _meta key MUST NOT
  appear on tasks/get's inlined result — the taskId is already
  at the root level of the tasks/get response, so the metadata is
  redundant.

Required server fixtures (tools/list MUST include all):
- slow_compute — task-supporting, seconds-second sleep then a
  result.

  - TasksNoEarlyTtlExpiry: Task remains accessible via tasks/get for the duration of its ttlMs
    Error: Not testable: no task was created by the preceding step, so this check could not be exercised

Test SEP-2663 wire-field renames + TTL semantics.



Wire-field renames (SEP-2663):
- The TTL field is named ttlMs on the wire (the v1 ttl key was
  in milliseconds-by-convention; SEP-2663 puts the unit in the field
  name and standardised on the Ms suffix for all duration fields).
- The poll-interval field is named pollIntervalMs (v1 used
  pollInterval).
- A CreateTaskResult MUST NOT carry the legacy ttl or
  pollInterval keys — clients keying off v1 names on a v2 server
  would silently miss the TTL guidance.
- Both ttlMs and pollIntervalMs are integer milliseconds.
  Servers MUST NOT emit fractional values.

TTL non-expiry (SEP-2663):
- A task MUST remain accessible via tasks/get for the duration of
  its ttlMs; a server MUST NOT expire it earlier.

Inlined-result _meta (SEP-2663):
- The v1 io.modelcontextprotocol/related-task _meta key MUST NOT
  appear on tasks/get's inlined result — the taskId is already
  at the root level of the tasks/get response, so the metadata is
  redundant.

Required server fixtures (tools/list MUST include all):
- slow_compute — task-supporting, seconds-second sleep then a
  result.

  - TasksNoRelatedTaskMetaOnInlinedResult: tasks/get inlined result MUST NOT include the v1 io.modelcontextprotocol/related-task _meta key (taskId is at the root)
    Error: fetch failed

Test SEP-2663 wire-field renames + TTL semantics.



Wire-field renames (SEP-2663):
- The TTL field is named ttlMs on the wire (the v1 ttl key was
  in milliseconds-by-convention; SEP-2663 puts the unit in the field
  name and standardised on the Ms suffix for all duration fields).
- The poll-interval field is named pollIntervalMs (v1 used
  pollInterval).
- A CreateTaskResult MUST NOT carry the legacy ttl or
  pollInterval keys — clients keying off v1 names on a v2 server
  would silently miss the TTL guidance.
- Both ttlMs and pollIntervalMs are integer milliseconds.
  Servers MUST NOT emit fractional values.

TTL non-expiry (SEP-2663):
- A task MUST remain accessible via tasks/get for the duration of
  its ttlMs; a server MUST NOT expire it earlier.

Inlined-result _meta (SEP-2663):
- The v1 io.modelcontextprotocol/related-task _meta key MUST NOT
  appear on tasks/get's inlined result — the taskId is already
  at the root level of the tasks/get response, so the metadata is
  redundant.

Required server fixtures (tools/list MUST include all):
- slow_compute — task-supporting, seconds-second sleep then a
  result.

## `tasks-request-state-removal`

SEP-2663 does not define a requestState field on the Task base
interface, so:

- CreateTaskResult MUST NOT carry requestState on the
  tools/call response that creates a task.
- The tasks/get response (DetailedTask) MUST NOT carry
  requestState for any status (working / input_required /
  completed / cancelled / failed).

SEP-2322's InputRequiredResult does carry requestState — that is
the MRTR multi-round-trip surface, unrelated to the tasks-v2 wire, and
is exercised by mrtr-input.ts. This scenario exists because the two
SEPs put requestState in lexically adjacent positions, making
accidental copy-paste from the MRTR shape into the tasks-v2 shape a
foreseeable mistake for fresh implementations.

Required server fixtures (tools/list MUST include all):
- slow_compute — task-supporting, seconds-second sleep then a
  result.

## `tasks-mrtr-input`

Surfacing inputRequests (SEP-2322):
- A task waiting on client input MUST report status:"input_required"
  on tasks/get and surface a non-empty inputRequests map keyed by
  server-minted opaque ids. Each entry carries the underlying request
  (elicitation/create, sampling/createMessage, etc.).

Resuming via tasks/update (SEP-2663):
- The client delivers responses through tasks/update with
  inputResponses keyed to match the server-emitted ids. The server
  MUST return an empty {resultType:"complete"} ack on the
  tasks/update response — the resulting task state is observed via the
  next tasks/get.
- After the response is delivered, the task MUST resume execution and
  proceed to a terminal state (or back to input_required for another
  round).

Partial fulfillment (SEP-2663):
- A tool that emits multiple simultaneous input requests parks the task
  with multiple keys in inputRequests. A client MAY answer them one
  at a time:
  - tasks/update with a subset of keys MUST be acked.
  - The task MUST stay in input_required until every pending request
    has been answered.
  - tasks/get after a partial update MUST surface only the still-pending
    keys; the answered key MUST be removed.

Required server fixtures (tools/list MUST include all):
- confirm_delete — task-supporting, emits a single
  elicitation/create inputRequest then completes when the response
  arrives.
- multi_input — task-supporting, fans out two elicitation/create
  inputRequests in parallel so two keys are pending at once (used by
  the partial-fulfillment check).

  - TasksMRTRTasksUpdateResumes: tasks/update with matching inputResponses MUST be acked with {resultType:"complete"} and resume the task to a terminal state
    Error: fetch failed

Test SEP-2322 MRTR input flow on the tasks surface.



Surfacing inputRequests (SEP-2322):
- A task waiting on client input MUST report status:"input_required"
  on tasks/get and surface a non-empty inputRequests map keyed by
  server-minted opaque ids. Each entry carries the underlying request
  (elicitation/create, sampling/createMessage, etc.).

Resuming via tasks/update (SEP-2663):
- The client delivers responses through tasks/update with
  inputResponses keyed to match the server-emitted ids. The server
  MUST return an empty {resultType:"complete"} ack on the
  tasks/update response — the resulting task state is observed via the
  next tasks/get.
- After the response is delivered, the task MUST resume execution and
  proceed to a terminal state (or back to input_required for another
  round).

Partial fulfillment (SEP-2663):
- A tool that emits multiple simultaneous input requests parks the task
  with multiple keys in inputRequests. A client MAY answer them one
  at a time:
  - tasks/update with a subset of keys MUST be acked.
  - The task MUST stay in input_required until every pending request
    has been answered.
  - tasks/get after a partial update MUST surface only the still-pending
    keys; the answered key MUST be removed.

Required server fixtures (tools/list MUST include all):
- confirm_delete — task-supporting, emits a single
  elicitation/create inputRequest then completes when the response
  arrives.
- multi_input — task-supporting, fans out two elicitation/create
  inputRequests in parallel so two keys are pending at once (used by
  the partial-fulfillment check).

  - TasksMRTRPartialFulfillment: tasks/update with a subset of keys MUST keep the task in input_required with only the unanswered key remaining
    Error: fetch failed

Test SEP-2322 MRTR input flow on the tasks surface.



Surfacing inputRequests (SEP-2322):
- A task waiting on client input MUST report status:"input_required"
  on tasks/get and surface a non-empty inputRequests map keyed by
  server-minted opaque ids. Each entry carries the underlying request
  (elicitation/create, sampling/createMessage, etc.).

Resuming via tasks/update (SEP-2663):
- The client delivers responses through tasks/update with
  inputResponses keyed to match the server-emitted ids. The server
  MUST return an empty {resultType:"complete"} ack on the
  tasks/update response — the resulting task state is observed via the
  next tasks/get.
- After the response is delivered, the task MUST resume execution and
  proceed to a terminal state (or back to input_required for another
  round).

Partial fulfillment (SEP-2663):
- A tool that emits multiple simultaneous input requests parks the task
  with multiple keys in inputRequests. A client MAY answer them one
  at a time:
  - tasks/update with a subset of keys MUST be acked.
  - The task MUST stay in input_required until every pending request
    has been answered.
  - tasks/get after a partial update MUST surface only the still-pending
    keys; the answered key MUST be removed.

Required server fixtures (tools/list MUST include all):
- confirm_delete — task-supporting, emits a single
  elicitation/create inputRequest then completes when the response
  arrives.
- multi_input — task-supporting, fans out two elicitation/create
  inputRequests in parallel so two keys are pending at once (used by
  the partial-fulfillment check).

## `tasks-request-headers`

SEP-2243 defines two required request headers that mirror body fields
into the HTTP layer for routing intermediaries:

- Mcp-Method: <jsonrpc-method> — REQUIRED on every JSON-RPC request,
  matching the body method.
- Mcp-Name: <name-shaped-identifier> — REQUIRED on requests with a
  name-shaped body field. Per SEP-2243 §"Standard Headers" this covers
  tools/call (params.name), resources/read (params.uri), and
  prompts/get (params.name). SEP-2663 §"Streamable HTTP: Routing
  Headers" extends the requirement to tasks-namespace methods:
  tasks/get, tasks/update, and tasks/cancel MUST carry
  Mcp-Name: <taskId> matching params.taskId.

Per SEP-2243 §"Server Behavior", servers that process the request body
MUST validate that header values match the body. Per its "Validation
Failure Conditions", both missing required headers and mismatched
values trigger rejection with JSON-RPC error code -32020
(HeaderMismatch) and HTTP 400.

Required server fixtures (tools/list MUST include all):
- greet — sync-only, returns Hello, {name}!.
- slow_compute — task-supporting, seconds-second sleep then a
  result.

  - Sep2663RoutingHeadersAcceptedOnTasksGet: Server accepts matched Mcp-Method + Mcp-Name request headers on tasks/get and dispatches normally
    Error: fetch failed

Test SEP-2243 Mcp-Method / Mcp-Name request-header validation, tasks surface.



SEP-2243 defines two required request headers that mirror body fields
into the HTTP layer for routing intermediaries:

- Mcp-Method: <jsonrpc-method> — REQUIRED on every JSON-RPC request,
  matching the body method.
- Mcp-Name: <name-shaped-identifier> — REQUIRED on requests with a
  name-shaped body field. Per SEP-2243 §"Standard Headers" this covers
  tools/call (params.name), resources/read (params.uri), and
  prompts/get (params.name). SEP-2663 §"Streamable HTTP: Routing
  Headers" extends the requirement to tasks-namespace methods:
  tasks/get, tasks/update, and tasks/cancel MUST carry
  Mcp-Name: <taskId> matching params.taskId.

Per SEP-2243 §"Server Behavior", servers that process the request body
MUST validate that header values match the body. Per its "Validation
Failure Conditions", both missing required headers and mismatched
values trigger rejection with JSON-RPC error code -32020
(HeaderMismatch) and HTTP 400.

Required server fixtures (tools/list MUST include all):
- greet — sync-only, returns Hello, {name}!.
- slow_compute — task-supporting, seconds-second sleep then a
  result.

  - TasksHeadersRejectMismatchedMethod: When Mcp-Method header disagrees with body on a tools/call, server MUST reject with -32020 HeaderMismatch (SEP-2243 §Server Validation)
    Error: expected -32020 HeaderMismatch; got fetch failed

Test SEP-2243 Mcp-Method / Mcp-Name request-header validation, tasks surface.



SEP-2243 defines two required request headers that mirror body fields
into the HTTP layer for routing intermediaries:

- Mcp-Method: <jsonrpc-method> — REQUIRED on every JSON-RPC request,
  matching the body method.
- Mcp-Name: <name-shaped-identifier> — REQUIRED on requests with a
  name-shaped body field. Per SEP-2243 §"Standard Headers" this covers
  tools/call (params.name), resources/read (params.uri), and
  prompts/get (params.name). SEP-2663 §"Streamable HTTP: Routing
  Headers" extends the requirement to tasks-namespace methods:
  tasks/get, tasks/update, and tasks/cancel MUST carry
  Mcp-Name: <taskId> matching params.taskId.

Per SEP-2243 §"Server Behavior", servers that process the request body
MUST validate that header values match the body. Per its "Validation
Failure Conditions", both missing required headers and mismatched
values trigger rejection with JSON-RPC error code -32020
(HeaderMismatch) and HTTP 400.

Required server fixtures (tools/list MUST include all):
- greet — sync-only, returns Hello, {name}!.
- slow_compute — task-supporting, seconds-second sleep then a
  result.

  - Sep2663ServerRejectsMismatchedMcpNameOnTasksGet: When Mcp-Name header disagrees with params.taskId on tasks/get, server MUST reject with -32020 HeaderMismatch
    Error: routing-task fixture from Check 2 unavailable; cannot drive negative-path probe

Test SEP-2243 Mcp-Method / Mcp-Name request-header validation, tasks surface.



SEP-2243 defines two required request headers that mirror body fields
into the HTTP layer for routing intermediaries:

- Mcp-Method: <jsonrpc-method> — REQUIRED on every JSON-RPC request,
  matching the body method.
- Mcp-Name: <name-shaped-identifier> — REQUIRED on requests with a
  name-shaped body field. Per SEP-2243 §"Standard Headers" this covers
  tools/call (params.name), resources/read (params.uri), and
  prompts/get (params.name). SEP-2663 §"Streamable HTTP: Routing
  Headers" extends the requirement to tasks-namespace methods:
  tasks/get, tasks/update, and tasks/cancel MUST carry
  Mcp-Name: <taskId> matching params.taskId.

Per SEP-2243 §"Server Behavior", servers that process the request body
MUST validate that header values match the body. Per its "Validation
Failure Conditions", both missing required headers and mismatched
values trigger rejection with JSON-RPC error code -32020
(HeaderMismatch) and HTTP 400.

Required server fixtures (tools/list MUST include all):
- greet — sync-only, returns Hello, {name}!.
- slow_compute — task-supporting, seconds-second sleep then a
  result.

## `tasks-dispatch-and-envelope`

Removed v1 methods (SEP-2663):
- tasks/result is removed in v2 — the result is inlined on
  tasks/get. Servers MUST reject the method with JSON-RPC -32601.
- tasks/list is removed in v2. Servers MUST reject it with
  -32601.

Server-directed task creation (SEP-2663):
- The client does NOT send a task hint param. The server alone
  decides whether to create a task. A tools/call against a
  task-supporting tool MUST produce CreateTaskResult even with no
  client hint.

Legacy task param tolerated (SEP-2663):
- A v1 client may still send task: { ttl, pollInterval } on
  tools/call. The server MUST tolerate it (no error) AND MUST NOT
  promote a sync-only tool to a task on its presence. The body
  arguments + tool registration are authoritative.

Immediate-result shortcut (SEP-2663):
- A server MAY return a sync ToolResult for task-supporting tools
  when the operation completes fast enough. Either return a
  CreateTaskResult (with resultType:"task") or a sync
  ToolResult (with resultType:"complete"); both are valid.

resultType:"complete" on non-task responses (SEP-2322):
- Every JSON-RPC response on the tools+tasks surface other than a
  CreateTaskResult MUST carry resultType:"complete". This applies
  to: sync tools/call, tasks/get, tasks/update ack,
  tasks/cancel ack.

Strong consistency / durable create (SEP-2663):
- A server MUST NOT return CreateTaskResult until the task is
  durably created — that is, until a tasks/get for the returned
  taskId would resolve. Issuing tasks/get immediately after the
  CreateTaskResult arrives MUST succeed, not -32602.

Unknown taskId on tasks/get (SEP-2663):
- tasks/get for a taskId the server doesn't recognize MUST return
  JSON-RPC -32602 (InvalidParams). Mirrors the same rule for
  tasks/cancel (clarified upstream in spec commit d963ad0).

Required server fixtures (tools/list MUST include all):
- greet — sync-only.
- slow_compute — task-supporting (seconds: 0 for the immediate
  shortcut path).
- confirm_delete — task-supporting, parks for elicitation.
- failing_job — task-supporting, returns a tool execution error.

  - TasksRemovedTasksList: tasks/list is removed in v2 and MUST reject with -32601
    Error: expected -32601; got <missing> (if the server is otherwise compliant, verify it does not validate other dimensions — routing headers, _meta, params shape — before method dispatch)

Test SEP-2663 dispatch / envelope rules across the tasks surface.



Removed v1 methods (SEP-2663):
- tasks/result is removed in v2 — the result is inlined on
  tasks/get. Servers MUST reject the method with JSON-RPC -32601.
- tasks/list is removed in v2. Servers MUST reject it with
  -32601.

Server-directed task creation (SEP-2663):
- The client does NOT send a task hint param. The server alone
  decides whether to create a task. A tools/call against a
  task-supporting tool MUST produce CreateTaskResult even with no
  client hint.

Legacy task param tolerated (SEP-2663):
- A v1 client may still send task: { ttl, pollInterval } on
  tools/call. The server MUST tolerate it (no error) AND MUST NOT
  promote a sync-only tool to a task on its presence. The body
  arguments + tool registration are authoritative.

Immediate-result shortcut (SEP-2663):
- A server MAY return a sync ToolResult for task-supporting tools
  when the operation completes fast enough. Either return a
  CreateTaskResult (with resultType:"task") or a sync
  ToolResult (with resultType:"complete"); both are valid.

resultType:"complete" on non-task responses (SEP-2322):
- Every JSON-RPC response on the tools+tasks surface other than a
  CreateTaskResult MUST carry resultType:"complete". This applies
  to: sync tools/call, tasks/get, tasks/update ack,
  tasks/cancel ack.

Strong consistency / durable create (SEP-2663):
- A server MUST NOT return CreateTaskResult until the task is
  durably created — that is, until a tasks/get for the returned
  taskId would resolve. Issuing tasks/get immediately after the
  CreateTaskResult arrives MUST succeed, not -32602.

Unknown taskId on tasks/get (SEP-2663):
- tasks/get for a taskId the server doesn't recognize MUST return
  JSON-RPC -32602 (InvalidParams). Mirrors the same rule for
  tasks/cancel (clarified upstream in spec commit d963ad0).

Required server fixtures (tools/list MUST include all):
- greet — sync-only.
- slow_compute — task-supporting (seconds: 0 for the immediate
  shortcut path).
- confirm_delete — task-supporting, parks for elicitation.
- failing_job — task-supporting, returns a tool execution error.

  - TasksServerDirectedCreationNoHint: tools/call with no client `task` hint param MUST still produce CreateTaskResult for task-supporting tools
    Error: fetch failed

Test SEP-2663 dispatch / envelope rules across the tasks surface.



Removed v1 methods (SEP-2663):
- tasks/result is removed in v2 — the result is inlined on
  tasks/get. Servers MUST reject the method with JSON-RPC -32601.
- tasks/list is removed in v2. Servers MUST reject it with
  -32601.

Server-directed task creation (SEP-2663):
- The client does NOT send a task hint param. The server alone
  decides whether to create a task. A tools/call against a
  task-supporting tool MUST produce CreateTaskResult even with no
  client hint.

Legacy task param tolerated (SEP-2663):
- A v1 client may still send task: { ttl, pollInterval } on
  tools/call. The server MUST tolerate it (no error) AND MUST NOT
  promote a sync-only tool to a task on its presence. The body
  arguments + tool registration are authoritative.

Immediate-result shortcut (SEP-2663):
- A server MAY return a sync ToolResult for task-supporting tools
  when the operation completes fast enough. Either return a
  CreateTaskResult (with resultType:"task") or a sync
  ToolResult (with resultType:"complete"); both are valid.

resultType:"complete" on non-task responses (SEP-2322):
- Every JSON-RPC response on the tools+tasks surface other than a
  CreateTaskResult MUST carry resultType:"complete". This applies
  to: sync tools/call, tasks/get, tasks/update ack,
  tasks/cancel ack.

Strong consistency / durable create (SEP-2663):
- A server MUST NOT return CreateTaskResult until the task is
  durably created — that is, until a tasks/get for the returned
  taskId would resolve. Issuing tasks/get immediately after the
  CreateTaskResult arrives MUST succeed, not -32602.

Unknown taskId on tasks/get (SEP-2663):
- tasks/get for a taskId the server doesn't recognize MUST return
  JSON-RPC -32602 (InvalidParams). Mirrors the same rule for
  tasks/cancel (clarified upstream in spec commit d963ad0).

Required server fixtures (tools/list MUST include all):
- greet — sync-only.
- slow_compute — task-supporting (seconds: 0 for the immediate
  shortcut path).
- confirm_delete — task-supporting, parks for elicitation.
- failing_job — task-supporting, returns a tool execution error.

  - TasksLegacyTaskParamIgnored: tools/call with legacy `task` param against a sync tool MUST NOT error and MUST NOT be promoted to a task
    Error: fetch failed

Test SEP-2663 dispatch / envelope rules across the tasks surface.



Removed v1 methods (SEP-2663):
- tasks/result is removed in v2 — the result is inlined on
  tasks/get. Servers MUST reject the method with JSON-RPC -32601.
- tasks/list is removed in v2. Servers MUST reject it with
  -32601.

Server-directed task creation (SEP-2663):
- The client does NOT send a task hint param. The server alone
  decides whether to create a task. A tools/call against a
  task-supporting tool MUST produce CreateTaskResult even with no
  client hint.

Legacy task param tolerated (SEP-2663):
- A v1 client may still send task: { ttl, pollInterval } on
  tools/call. The server MUST tolerate it (no error) AND MUST NOT
  promote a sync-only tool to a task on its presence. The body
  arguments + tool registration are authoritative.

Immediate-result shortcut (SEP-2663):
- A server MAY return a sync ToolResult for task-supporting tools
  when the operation completes fast enough. Either return a
  CreateTaskResult (with resultType:"task") or a sync
  ToolResult (with resultType:"complete"); both are valid.

resultType:"complete" on non-task responses (SEP-2322):
- Every JSON-RPC response on the tools+tasks surface other than a
  CreateTaskResult MUST carry resultType:"complete". This applies
  to: sync tools/call, tasks/get, tasks/update ack,
  tasks/cancel ack.

Strong consistency / durable create (SEP-2663):
- A server MUST NOT return CreateTaskResult until the task is
  durably created — that is, until a tasks/get for the returned
  taskId would resolve. Issuing tasks/get immediately after the
  CreateTaskResult arrives MUST succeed, not -32602.

Unknown taskId on tasks/get (SEP-2663):
- tasks/get for a taskId the server doesn't recognize MUST return
  JSON-RPC -32602 (InvalidParams). Mirrors the same rule for
  tasks/cancel (clarified upstream in spec commit d963ad0).

Required server fixtures (tools/list MUST include all):
- greet — sync-only.
- slow_compute — task-supporting (seconds: 0 for the immediate
  shortcut path).
- confirm_delete — task-supporting, parks for elicitation.
- failing_job — task-supporting, returns a tool execution error.

  - TasksImmediateResultShortcut: For a fast operation, a task-supporting tool MAY skip task creation and return a sync ToolResult; either path is valid
    Error: fetch failed

Test SEP-2663 dispatch / envelope rules across the tasks surface.



Removed v1 methods (SEP-2663):
- tasks/result is removed in v2 — the result is inlined on
  tasks/get. Servers MUST reject the method with JSON-RPC -32601.
- tasks/list is removed in v2. Servers MUST reject it with
  -32601.

Server-directed task creation (SEP-2663):
- The client does NOT send a task hint param. The server alone
  decides whether to create a task. A tools/call against a
  task-supporting tool MUST produce CreateTaskResult even with no
  client hint.

Legacy task param tolerated (SEP-2663):
- A v1 client may still send task: { ttl, pollInterval } on
  tools/call. The server MUST tolerate it (no error) AND MUST NOT
  promote a sync-only tool to a task on its presence. The body
  arguments + tool registration are authoritative.

Immediate-result shortcut (SEP-2663):
- A server MAY return a sync ToolResult for task-supporting tools
  when the operation completes fast enough. Either return a
  CreateTaskResult (with resultType:"task") or a sync
  ToolResult (with resultType:"complete"); both are valid.

resultType:"complete" on non-task responses (SEP-2322):
- Every JSON-RPC response on the tools+tasks surface other than a
  CreateTaskResult MUST carry resultType:"complete". This applies
  to: sync tools/call, tasks/get, tasks/update ack,
  tasks/cancel ack.

Strong consistency / durable create (SEP-2663):
- A server MUST NOT return CreateTaskResult until the task is
  durably created — that is, until a tasks/get for the returned
  taskId would resolve. Issuing tasks/get immediately after the
  CreateTaskResult arrives MUST succeed, not -32602.

Unknown taskId on tasks/get (SEP-2663):
- tasks/get for a taskId the server doesn't recognize MUST return
  JSON-RPC -32602 (InvalidParams). Mirrors the same rule for
  tasks/cancel (clarified upstream in spec commit d963ad0).

Required server fixtures (tools/list MUST include all):
- greet — sync-only.
- slow_compute — task-supporting (seconds: 0 for the immediate
  shortcut path).
- confirm_delete — task-supporting, parks for elicitation.
- failing_job — task-supporting, returns a tool execution error.

  - TasksResultTypeCompleteOnNonTaskResponses: Sync tools/call, tasks/get, tasks/update ack, and tasks/cancel ack MUST all carry resultType:"complete"
    Error: fetch failed

Test SEP-2663 dispatch / envelope rules across the tasks surface.



Removed v1 methods (SEP-2663):
- tasks/result is removed in v2 — the result is inlined on
  tasks/get. Servers MUST reject the method with JSON-RPC -32601.
- tasks/list is removed in v2. Servers MUST reject it with
  -32601.

Server-directed task creation (SEP-2663):
- The client does NOT send a task hint param. The server alone
  decides whether to create a task. A tools/call against a
  task-supporting tool MUST produce CreateTaskResult even with no
  client hint.

Legacy task param tolerated (SEP-2663):
- A v1 client may still send task: { ttl, pollInterval } on
  tools/call. The server MUST tolerate it (no error) AND MUST NOT
  promote a sync-only tool to a task on its presence. The body
  arguments + tool registration are authoritative.

Immediate-result shortcut (SEP-2663):
- A server MAY return a sync ToolResult for task-supporting tools
  when the operation completes fast enough. Either return a
  CreateTaskResult (with resultType:"task") or a sync
  ToolResult (with resultType:"complete"); both are valid.

resultType:"complete" on non-task responses (SEP-2322):
- Every JSON-RPC response on the tools+tasks surface other than a
  CreateTaskResult MUST carry resultType:"complete". This applies
  to: sync tools/call, tasks/get, tasks/update ack,
  tasks/cancel ack.

Strong consistency / durable create (SEP-2663):
- A server MUST NOT return CreateTaskResult until the task is
  durably created — that is, until a tasks/get for the returned
  taskId would resolve. Issuing tasks/get immediately after the
  CreateTaskResult arrives MUST succeed, not -32602.

Unknown taskId on tasks/get (SEP-2663):
- tasks/get for a taskId the server doesn't recognize MUST return
  JSON-RPC -32602 (InvalidParams). Mirrors the same rule for
  tasks/cancel (clarified upstream in spec commit d963ad0).

Required server fixtures (tools/list MUST include all):
- greet — sync-only.
- slow_compute — task-supporting (seconds: 0 for the immediate
  shortcut path).
- confirm_delete — task-supporting, parks for elicitation.
- failing_job — task-supporting, returns a tool execution error.

  - TasksStrongConsistencyImmediateGet: tasks/get issued immediately after CreateTaskResult arrives MUST resolve (server MUST NOT return CreateTaskResult before the task is durably created)
    Error: fetch failed

Test SEP-2663 dispatch / envelope rules across the tasks surface.



Removed v1 methods (SEP-2663):
- tasks/result is removed in v2 — the result is inlined on
  tasks/get. Servers MUST reject the method with JSON-RPC -32601.
- tasks/list is removed in v2. Servers MUST reject it with
  -32601.

Server-directed task creation (SEP-2663):
- The client does NOT send a task hint param. The server alone
  decides whether to create a task. A tools/call against a
  task-supporting tool MUST produce CreateTaskResult even with no
  client hint.

Legacy task param tolerated (SEP-2663):
- A v1 client may still send task: { ttl, pollInterval } on
  tools/call. The server MUST tolerate it (no error) AND MUST NOT
  promote a sync-only tool to a task on its presence. The body
  arguments + tool registration are authoritative.

Immediate-result shortcut (SEP-2663):
- A server MAY return a sync ToolResult for task-supporting tools
  when the operation completes fast enough. Either return a
  CreateTaskResult (with resultType:"task") or a sync
  ToolResult (with resultType:"complete"); both are valid.

resultType:"complete" on non-task responses (SEP-2322):
- Every JSON-RPC response on the tools+tasks surface other than a
  CreateTaskResult MUST carry resultType:"complete". This applies
  to: sync tools/call, tasks/get, tasks/update ack,
  tasks/cancel ack.

Strong consistency / durable create (SEP-2663):
- A server MUST NOT return CreateTaskResult until the task is
  durably created — that is, until a tasks/get for the returned
  taskId would resolve. Issuing tasks/get immediately after the
  CreateTaskResult arrives MUST succeed, not -32602.

Unknown taskId on tasks/get (SEP-2663):
- tasks/get for a taskId the server doesn't recognize MUST return
  JSON-RPC -32602 (InvalidParams). Mirrors the same rule for
  tasks/cancel (clarified upstream in spec commit d963ad0).

Required server fixtures (tools/list MUST include all):
- greet — sync-only.
- slow_compute — task-supporting (seconds: 0 for the immediate
  shortcut path).
- confirm_delete — task-supporting, parks for elicitation.
- failing_job — task-supporting, returns a tool execution error.

  - TasksGetUnknownTaskIdRejected: tasks/get for a taskId the server does not recognize MUST return -32602
    Error: expected -32602; got <missing> (if the server is otherwise compliant, verify it does not validate other dimensions — routing headers, _meta, params shape — before method dispatch)

Test SEP-2663 dispatch / envelope rules across the tasks surface.



Removed v1 methods (SEP-2663):
- tasks/result is removed in v2 — the result is inlined on
  tasks/get. Servers MUST reject the method with JSON-RPC -32601.
- tasks/list is removed in v2. Servers MUST reject it with
  -32601.

Server-directed task creation (SEP-2663):
- The client does NOT send a task hint param. The server alone
  decides whether to create a task. A tools/call against a
  task-supporting tool MUST produce CreateTaskResult even with no
  client hint.

Legacy task param tolerated (SEP-2663):
- A v1 client may still send task: { ttl, pollInterval } on
  tools/call. The server MUST tolerate it (no error) AND MUST NOT
  promote a sync-only tool to a task on its presence. The body
  arguments + tool registration are authoritative.

Immediate-result shortcut (SEP-2663):
- A server MAY return a sync ToolResult for task-supporting tools
  when the operation completes fast enough. Either return a
  CreateTaskResult (with resultType:"task") or a sync
  ToolResult (with resultType:"complete"); both are valid.

resultType:"complete" on non-task responses (SEP-2322):
- Every JSON-RPC response on the tools+tasks surface other than a
  CreateTaskResult MUST carry resultType:"complete". This applies
  to: sync tools/call, tasks/get, tasks/update ack,
  tasks/cancel ack.

Strong consistency / durable create (SEP-2663):
- A server MUST NOT return CreateTaskResult until the task is
  durably created — that is, until a tasks/get for the returned
  taskId would resolve. Issuing tasks/get immediately after the
  CreateTaskResult arrives MUST succeed, not -32602.

Unknown taskId on tasks/get (SEP-2663):
- tasks/get for a taskId the server doesn't recognize MUST return
  JSON-RPC -32602 (InvalidParams). Mirrors the same rule for
  tasks/cancel (clarified upstream in spec commit d963ad0).

Required server fixtures (tools/list MUST include all):
- greet — sync-only.
- slow_compute — task-supporting (seconds: 0 for the immediate
  shortcut path).
- confirm_delete — task-supporting, parks for elicitation.
- failing_job — task-supporting, returns a tool execution error.

## `tasks-required-task-error`

Per SEP-2575 §"Missing Required Capabilities" and SEP-2663 §"Required
Capabilities":

> If a server is unable to service a request to a client that does not
> declare this extension capability without returning CreateTaskResult,
> the server MUST return an error with the code -32021 (Missing
> Required Client Capability), indicating the required extension in the
> error response.

The error data SHOULD include a requiredCapabilities object whose
shape mirrors InitializeRequest.capabilities, e.g.

```json
{
  "requiredCapabilities": {
    "extensions": {
      "io.modelcontextprotocol/tasks": {}
    }
  }
}
`

The scenario calls tools/call for a tool registered with task support
required from a client that did NOT declare the extension. A
conformant server MUST reject with -32021.

Required server fixtures (tools/list MUST include all):
- failing_job — registered with task support declared as required`.
  The tool's payload behavior is irrelevant; only the registration-time
  declaration matters, because the error is returned by the middleware
  before the handler runs.

## `tasks-mrtr-composition`

A tool that gathers input via the SEP-2322 MRTR loop and then escalates
to async on the final round MUST return a CreateTaskResult on that
final round, NOT a sync ToolResult. The composition is what makes
the two surfaces interoperate — clients should not need to choose one
or the other up front.

Spec separation that MUST stay observable:

1. Round 1 (MRTR) carries inputRequests + (optionally) requestState
   and MUST NOT carry taskId.
2. Round 2 (CreateTaskResult) carries taskId + status and MUST
   NOT carry requestState — SEP-2663 removed it from the v2 wire
   shape, so the MRTR phase's requestState does not leak into the
   task envelope and clients don't have to deduplicate across flows.
3. The final task result (inlined on tasks/get once terminal)
   MUST reflect the answer gathered during the MRTR phase, end-to-end.

Required server fixtures (tools/list MUST include all):
- test_tool_with_task — registered with taskSupport=required.
  Round 1: returns InputRequiredResult asking for user_name.
  Round 2: with the elicit response echoed back, escalates to async
  and returns CreateTaskResult. The task's eventual result text MUST
  contain the gathered user_name so the round-trip is observable.

## `input-required-result-basic-elicitation`

Implement a tool named test_input_required_result_elicitation (no arguments required).

Behavior (Round 1): When called without inputResponses, return an InputRequiredResult:

```json
{
  "resultType": "input_required",
  "inputRequests": {
    "user_name": {
      "method": "elicitation/create",
      "params": {
        "message": "What is your name?",
        "requestedSchema": {
          "type": "object",
          "properties": {
            "name": { "type": "string" }
          },
          "required": ["name"]
        }
      }
    }
  }
}
`

Behavior (Round 2): When called with inputResponses containing the key "user_name", return a complete result:

`json
{
  "content": [{ "type": "text", "text": "Hello, <name>!" }]
}
``

## `input-required-result-basic-sampling`

Implement a tool named test_input_required_result_sampling (no arguments required).

Behavior (Round 1): When called without inputResponses, return an InputRequiredResult:

```json
{
  "resultType": "input_required",
  "inputRequests": {
    "capital_question": {
      "method": "sampling/createMessage",
      "params": {
        "messages": [{
          "role": "user",
          "content": { "type": "text", "text": "What is the capital of France?" }
        }],
        "maxTokens": 100
      }
    }
  }
}
`

Behavior (Round 2): When called with inputResponses containing the key "capital_question"`, return a complete result with the sampling response text.

## `input-required-result-basic-list-roots`

Implement a tool named test_input_required_result_list_roots (no arguments required).

Behavior (Round 1): When called without inputResponses, return an InputRequiredResult:

```json
{
  "resultType": "input_required",
  "inputRequests": {
    "client_roots": {
      "method": "roots/list",
      "params": {}
    }
  }
}
`

Behavior (Round 2): When called with inputResponses containing the key "client_roots" (a ListRootsResult with a roots` array), return a complete result that references the provided roots.

## `input-required-result-request-state`

Implement a tool named test_input_required_result_request_state (no arguments required).

Behavior (Round 1): Return an InputRequiredResult with both inputRequests and requestState:

```json
{
  "resultType": "input_required",
  "inputRequests": {
    "confirm": {
      "method": "elicitation/create",
      "params": {
        "message": "Please confirm",
        "requestedSchema": {
          "type": "object",
          "properties": { "ok": { "type": "boolean" } },
          "required": ["ok"]
        }
      }
    }
  },
  "requestState": "<opaque-server-state>"
}
`

Behavior (Round 2): When called with inputResponses AND the echoed requestState`, validate the state and return a complete result. The text content MUST include the word "state-ok" to confirm the server received and validated the requestState.

## `input-required-result-multiple-input-requests`

Implement a tool named test_input_required_result_multiple_inputs (no arguments required).

Behavior (Round 1): Return an InputRequiredResult with multiple inputRequests — elicitation, sampling, and roots/list — plus requestState:

```json
{
  "resultType": "input_required",
  "inputRequests": {
    "user_name": {
      "method": "elicitation/create",
      "params": {
        "message": "What is your name?",
        "requestedSchema": {
          "type": "object",
          "properties": { "name": { "type": "string" } },
          "required": ["name"]
        }
      }
    },
    "greeting": {
      "method": "sampling/createMessage",
      "params": {
        "messages": [{ "role": "user", "content": { "type": "text", "text": "Generate a greeting" } }],
        "maxTokens": 50
      }
    },
    "client_roots": {
      "method": "roots/list",
      "params": {}
    }
  },
  "requestState": "<opaque-server-state>"
}
`

Behavior (Round 2): When called with inputResponses containing ALL keys and the echoed requestState`, return a complete result.

## `input-required-result-multi-round`

Implement a tool named test_input_required_result_multi_round (no arguments required).

Behavior (Round 1): Return an InputRequiredResult with an elicitation request and requestState:

```json
{
  "resultType": "input_required",
  "inputRequests": {
    "step1": {
      "method": "elicitation/create",
      "params": {
        "message": "Step 1: What is your name?",
        "requestedSchema": {
          "type": "object",
          "properties": { "name": { "type": "string" } },
          "required": ["name"]
        }
      }
    }
  },
  "requestState": "<state-round-1>"
}
`

Behavior (Round 2): When called with inputResponses for step1 + requestState, return ANOTHER InputRequiredResult with a new elicitation and updated requestState:

`json
{
  "resultType": "input_required",
  "inputRequests": {
    "step2": {
      "method": "elicitation/create",
      "params": {
        "message": "Step 2: What is your favorite color?",
        "requestedSchema": {
          "type": "object",
          "properties": { "color": { "type": "string" } },
          "required": ["color"]
        }
      }
    }
  },
  "requestState": "<state-round-2>"
}
`

Behavior (Round 3): When called with inputResponses` for step2 + updated requestState, return a complete result.

## `input-required-result-non-tool-request`

Implement a prompt named test_input_required_result_prompt that requires elicitation input.

Behavior (Round 1): When prompts/get is called for test_input_required_result_prompt without inputResponses, return an InputRequiredResult:

```json
{
  "resultType": "input_required",
  "inputRequests": {
    "user_context": {
      "method": "elicitation/create",
      "params": {
        "message": "What context should the prompt use?",
        "requestedSchema": {
          "type": "object",
          "properties": { "context": { "type": "string" } },
          "required": ["context"]
        }
      }
    }
  }
}
`

Behavior (Round 2): When called with inputResponses, return a complete GetPromptResult`.

## `input-required-result-result-type`

Uses the same tool as A1: test_input_required_result_elicitation.

This scenario verifies that the resultType field is explicitly present in the response (not just inferred).

## `input-required-result-tampered-state`

Implement a tool named test_input_required_result_tampered_state (no arguments required).

Behavior (Round 1): When called without inputResponses, return an InputRequiredResult with
integrity-protected requestState (e.g. HMAC-signed).

Behavior (Round 2 - tampered): When called with a modified/tampered requestState, return a
JSON-RPC error (code -32602 or similar) indicating integrity check failure.

## `input-required-result-capability-check`

Implement a tool named test_input_required_result_capabilities (no arguments required).

Behavior: Read client capabilities from _meta['io.modelcontextprotocol/clientCapabilities'].
Only include inputRequests for methods the client supports. For example, if the client declares
sampling: {} but NOT elicitation, only include sampling/createMessage inputRequests.
