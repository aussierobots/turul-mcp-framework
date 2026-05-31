# Example-Fixture Compliance Harness — Plan

`turul-mcp-protocol-2026-07-28` proves wire-format compliance against the
upstream MCP spec's own canonical JSON examples. The harness has a single
code path shared by `cargo test` (build-time gate) and a runtime CLI binary —
green tests are the proof that the binary is also green on the same host
and pin.

## Components

```
crates/turul-mcp-protocol-2026-07-28/
├── schema/
│   └── EXAMPLES_PIN.md             — pinned upstream commit SHA + date
├── src/
│   ├── compliance/
│   │   ├── fetch.rs                — shallow sparse git clone + idempotent cache
│   │   ├── coverage.rs             — 86-entry CASES table (one per upstream dir)
│   │   └── roundtrip.rs            — semantic diff (sorted keys, skip-if-none aware)
│   └── bin/
│       └── compliance.rs           — CLI with `refresh [--write]` subcommand
└── tests/
    └── upstream_fixtures.rs        — drives roundtrip across CASES + floor test
```

All compliance code is gated behind the `compliance` Cargo feature (default-off).
Downstream consumers of the protocol crate's library surface compile no
fetch/diff code.

## Pinning

A single `const PIN: Pin` in `fetch.rs` is the source of truth. `EXAMPLES_PIN.md`
mirrors it for humans. Pinning to a commit SHA (not a tree SHA) so `git checkout`
works without needing a separate "find the commit that produced this tree" step.

Bumping the pin: `cargo run --bin mcp-compliance-2026-07-28 -- refresh --write`.
The refresh resolves the upstream `main` HEAD, re-runs the full harness against
the candidate pin in a side cache, and either writes the new SHA to both
`fetch.rs` and `EXAMPLES_PIN.md` atomically (with rollback on partial failure)
or refuses to write if any modeled case would regress.

## CASES table

`coverage.rs::CASES` declares one `Case` per upstream example directory. Each
case is either `Kind::NotModeled` (skipped by the harness, counted) or carries
a `parse_and_reserialize: fn(&str) -> Result<Value, String>` that drives the
round-trip through its Rust binding.

A `coverage_table_matches_upstream` test asserts the table is in sync with the
fetched tree: any new upstream dir or any disappeared dir fails the test until
the table is updated.

## Floor

`tests/upstream_fixtures.rs::COVERAGE_FLOOR` asserts `modeled >= N`. The floor
is raised by deliberate PR whenever new cases flip from `NotModeled` to a real
binding. Lowering the floor requires explicit maintainer approval; the test
catches accidental regressions.

## Bidirectional guarantee

The test harness and the CLI binary both call the same
`compliance::roundtrip::run_all(dest)`. The destination differs (tests use
`target/upstream-fixtures/`, the binary uses `$TMPDIR/mcp-compliance-2026-07-28/`)
but the code path is identical. There is no "test only" or "binary only"
behavior — green `cargo test` is the proof the binary is also green on the
same pin.

## Slice 1 (current)

8 modeled cases — `Tool`, `CallToolRequestParams`, `CallToolResult`,
`ListToolsResult`, `Resource`, `Root`, `ListRootsResult`, `ElicitResult`.
All round-trip cleanly against 20 upstream fixtures. Floor = 8.

`ListToolsRequest` is intentionally NotModeled: its upstream fixture is the
full JSON-RPC envelope (`{jsonrpc, id, method, params}`) but our Rust type
models the inner `{method, params}` shape. Modeling it cleanly requires a
typed `JsonRpcRequest<T>` carrier that preserves `RequestMetaObject` keys —
deferred to a later slice.

## Future slices

Bind the inner-shape result types we already model: `CompleteResult`,
`GetPromptResult`, `ReadResourceResult`, `ListResourcesResult`,
`ListPromptsResult`. Bind `EmbeddedResource`, `ResourceLink`, `SamplingMessage`,
`ModelPreferences`. Then tackle the 5 error-envelope dirs (`InternalError`,
`InvalidParamsError`, `MethodNotFoundError`, `ParseError`,
`UnsupportedProtocolVersionError`) via the `JsonRpcError` binding. Finally,
the `*Response` envelope dirs need a generic typed `JsonRpcResponse<T>`.
