# conformance-fixture-server

**This is not a usage example. Do not copy it.**

It exists to be driven by upstream's conformance suite,
[`@modelcontextprotocol/conformance`](https://www.npmjs.com/package/@modelcontextprotocol/conformance).
Every tool name, payload key and message string in it is dictated by a
scenario, not chosen for clarity. For how to actually build a server, start
with [`minimal-server`](../minimal-server) or the root
[README Quick Start](../../README.md).

## Why it exists

Everything else that verifies this framework has turul code on both ends of
the wire. 3600-odd internal tests cannot detect a *shared misreading* of the
spec — if the server and the test agree on something wrong, both stay green.
This server is the counterparty: bytes authored by the protocol maintainers,
asserting against us.

It has earned that. Three defects were found by pointing this suite at the
framework and by nothing else:

| Defect | Fixed in |
|---|---|
| Macro-authored tools reported a tool's own failure as a JSON-RPC error instead of `isError: true` | 0.4.2 |
| `resources/read` rejected a resource's own declared mimeType | 0.4.2 |
| **A matching `Host` header defeated Origin validation — DNS rebinding was not blocked** | commit `683b925` |

The third was a real vulnerability, of the same class as the TypeScript SDK's
[GHSA-w48q-cv73-mx4w](https://github.com/modelcontextprotocol/typescript-sdk/security/advisories/GHSA-w48q-cv73-mx4w).

## Running it

```bash
cargo run -p conformance-fixture-server -- --port 8010
```

```bash
npx -y @modelcontextprotocol/conformance@0.2.0-alpha.11 server --requirements 2026-07-28 --url http://127.0.0.1:8010/mcp
```

`--requirements` **replaces** `--suite` and `--spec-version`; passing it
together with `--spec-version` is rejected outright. Add `-o <dir>` to get
`checks.json` per scenario — the pretty output does not include failure
detail, and the JSON does.

## Current result

**37 of 37 scored scenarios pass** (measured 2026-08-15, harness
`0.2.0-alpha.11`). The full run reads 143 passed / 36 failed; every failure is
in the 13 scenarios the harness itself labels *"Not scored for 2026-07-28"*:

- **10 `tasks-*`** — the `io.modelcontextprotocol/tasks` extension (SEP-2663).
  Fixtures not built yet: `greet`, `slow_compute`, `failing_job`,
  `protocol_error_job`, `confirm_delete`, `multi_input`, `test_tool_with_task`.
- **3 `pending`** — `json-schema-2020-12`,
  `http-custom-header-server-validation` (both failing),
  `http-header-validation` (passing).

Because they are unscored, they do not affect conformance — but the tasks ones
are the only external check that exists for our tasks wire format, since no
SDK implements SEP-2663 client-side.

## Working on it

**Fixture names and payload keys are a contract.** Renaming `"user_name"` to
something tidier silently turns a passing scenario into a failing one.

When a scenario fails with `Unknown tool: X`, **`X` is authoritative**. Prefer
it over [`docs/plans/2026-07-28-conformance-fixtures.md`](../../docs/plans/2026-07-28-conformance-fixtures.md),
which was harvested from harness output and is incomplete and wrong in places:
it lists 27 fixtures where the harness references 44, and misnames some — it
says `test_tool_with_logging` where the harness wants `test_logging_tool`.

Nine scenarios print no requirements at all. Their expectations are in the
published package's `dist/index.js`:

```bash
npm pack @modelcontextprotocol/conformance@0.2.0-alpha.11 && tar xzf modelcontextprotocol-conformance-*.tgz
```

That file is the ground truth for every scenario, requirement-printing or not.

## Dependencies beyond the framework

`hmac` / `sha2` / `getrandom`, used by one fixture only. The
`input-required-result-tampered-state` scenario requires the server to detect a
modified `requestState`. The framework passes that value through verbatim by
design — it is attacker-controlled and only the author knows what it means —
and offers no signing helper, so the fixture signs it with an HMAC under a
random per-boot key. That is the pattern a stateless server needs; an
in-process registry of issued states would break under horizontal scaling and
would teach the wrong thing.
