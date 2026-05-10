# ADR-025: Extract turul-rpc; treat turul-mcp-json-rpc-server as terminal 0.3 shim

**Status**: Accepted
**Date**: 2026-05-10
**Related**: ADR-001 (protocol-alias-usage), ADR-010 (architectural-guidelines), turul-rpc/docs/adr/003 (compatibility contract)

## Context

`turul-mcp-json-rpc-server` is, by construction, a generic JSON-RPC 2.0
implementation. Its source contains no `turul_mcp_*` types — only a doc
comment ever mentioned MCP. Two forces motivate splitting it out of this
workspace:

1. **Discoverability.** The crate is useful outside MCP, but its name buries
   it. Non-MCP consumers do not find it.
2. **Naming.** `turul-mcp-json-rpc-server` advertises an MCP coupling that
   does not exist in the code.

We are **not** doing this to clean up the API surface — that is deferred to
`turul-rpc 0.2` and `turul-mcp-framework 0.4`. The first goal is functional
replication.

## Decision

### Two-line change

1. The generic JSON-RPC 2.0 implementation is published from a sibling
   repository ([turul-rpc](https://github.com/aussierobots/turul-rpc)) as
   four crates at v0.1.0:

   - `turul-rpc-core` — wire types
   - `turul-rpc-jsonrpc` — codec/parser, JSON-RPC 2.0 batch
   - `turul-rpc-server` — async dispatcher, handler trait, session, streaming
   - `turul-rpc` — facade re-exporting the above

   See [turul-rpc/docs/adr/001-crate-boundaries.md][rpc-001] for the split rationale.

2. `turul-mcp-json-rpc-server` becomes a thin re-export shim over
   `turul-rpc`, starting at framework v0.3.39:

   ```rust
   pub use turul_rpc::*;
   pub use turul_rpc::{dispatch, error, error_codes, notification, prelude, request, response, types};
   #[cfg(feature = "async")] pub use turul_rpc::r#async;
   pub use turul_rpc::{JSONRPC_VERSION, JsonRpcDispatcher, JsonRpcHandler, /* ... */};
   ```

   Type identity is preserved through `pub use` chains rooted in the
   turul-rpc crates; downstream code importing
   `turul_mcp_json_rpc_server::*` continues to compile and behave
   identically. See [turul-rpc/docs/adr/003][rpc-003] for the technical
   contract.

### Lifecycle

- **0.3.x line (now and onward)**: `turul-mcp-json-rpc-server` continues to
  ship as the shim. Final shim release is **0.3.39**. Subsequent 0.3.x
  patches happen only if a compatibility-breaking issue is found.
- **0.4.0 (separate slice)**: `turul-mcp-json-rpc-server` is **not
  republished**. Framework workspace removes the dependency and migrates
  every internal `use turul_mcp_json_rpc_server::*` import to
  `use turul_rpc::*`. A separate ADR (ADR-026) will document the
  cleanup-release scope when it lands. Existing 0.3 consumers may continue
  to depend on `turul-mcp-json-rpc-server 0.3.39` indefinitely.

### Compliance corrections shipped with 0.3.39

The shim is **publish-time refactor** for the API surface, but
`turul-rpc 0.1` does close two pre-existing JSON-RPC 2.0 spec gaps that
the original crate had:

1. **Batch processing** is implemented (was a stub with a misleading
   "JSON-RPC 2.0 removed batch support" comment). New API:
   `turul_rpc_jsonrpc::parse_json_rpc_batch`,
   `turul_rpc_server::JsonRpcDispatcher::handle_batch`. Reachable from the
   shim via `turul_mcp_json_rpc_server::dispatch::parse_json_rpc_batch`
   for users who want it; existing code is unaffected.
2. **Strict-id rejection** at the parser is now explicit and tested
   (null id, fractional numeric id → `-32600 Invalid Request`). This was
   already the implicit behavior; tests in
   `turul-rpc-jsonrpc/tests/spec_conformance.rs` lock it in.

Spec compliance is a v0.1 success criterion per
[turul-rpc/docs/adr/002][rpc-002].

## Consequences

**Positive**

- Generic JSON-RPC consumers find `turul-rpc` first.
- The split clarifies the layer boundary already documented in ADR-010
  (handlers → domain errors; dispatcher → protocol).
- Framework 0.4 has a clean cleanup target: drop the shim dep, import
  `turul-rpc` directly, ship one minor.
- JSON-RPC 2.0 batch is now implemented and tested.

**Negative**

- During the 0.3.x line, two crate names refer to the same surface. Docs
  and search results list both. Mitigated by the migration note pointing
  new users to `turul-rpc`.
- Cross-repo coordination cost — `turul-rpc` and `turul-mcp-framework`
  must be released together when `turul-rpc-*` changes. Mitigated by
  the version-pinning discipline in `Cargo.toml` and by the
  `cargo public-api` snapshot guard for the shim.
- The historical `JsonRpcMessage` name collision (response union vs.
  dispatch incoming union) survives in the shim. Cleanup deferred to
  `turul-rpc 0.2` / framework 0.5.

## Alternatives considered

1. **Rename `turul-mcp-json-rpc-server` to `turul-rpc` in place.**
   Rejected: forces every existing consumer to update `Cargo.toml` and
   imports for zero functional benefit. Violates the no-pressure-on-0.3-users
   requirement.
2. **Leave the crate where it is and document its generic use.**
   Rejected: discoverability problem persists; the MCP-coupled name
   continues to mislead.
3. **Bump the shim to 0.4.0 alongside the framework.**
   Rejected. Releasing a `turul-mcp-json-rpc-server 0.4.0` on top of the
   shim signals continued active maintenance and forces every 0.3
   consumer to either pin to `0.3.x` explicitly or follow into 0.4. The
   intent is the opposite — 0.3.39 is the **terminal** shim release;
   0.4.0 of the framework simply stops carrying it forward.
4. **Single `turul-rpc` crate** instead of the four-crate split.
   Rejected at the turul-rpc level (see [rpc-001][rpc-001]) — keeps the
   wire types tied to the async runtime forever.

## Merge blocker — path dependency

The `extract/turul-rpc-shim` branch ships with a sibling-path workspace
dep:

```toml
turul-rpc = { version = "0.1", path = "../turul-rpc/crates/turul-rpc", default-features = false }
```

This **must** be replaced with the crates.io version-only form before
merge to `main`:

```toml
turul-rpc = { version = "0.1", default-features = false }
```

That swap requires `turul-rpc 0.1.0` and its three sibling crates
(`turul-rpc-core`, `turul-rpc-jsonrpc`, `turul-rpc-server`) to be
published to crates.io first. Until then, fresh clones and CI without the
sibling repo at `../turul-rpc` will fail to resolve dependencies. The
branch is staging work; merge is gated on the publish step.

## Verification

The 0.3.39 release passed:

- 24 (original) + 0 (shim has no inline tests) + 6 (`tests/shim_compat.rs`)
  + 1 doctest = baseline preserved with type-identity assertions added.
- 92 + 93 + 76 + 91 + 152 + 229 = 733 lib tests across
  `turul-mcp-protocol-2025-{06-18,11-25}`, `turul-mcp-builders`,
  `turul-mcp-server`, `turul-http-mcp-server`, `turul-mcp-aws-lambda`.
- 86 MCP compliance tests (`tests/mcp_compliance_tests.rs` and the
  consolidated `tests/compliance.rs` binary).
- 30 `streamable_http_e2e` tests.
- 12 `event_dispatcher_persistence` tests.
- `turul-rpc` workspace: 24 inline + 29 spec conformance + 7 batch
  dispatch + 1 doctest = 61 tests.

## References

- [rpc-001]: https://github.com/aussierobots/turul-rpc/blob/main/docs/adr/001-crate-boundaries.md
- [rpc-002]: https://github.com/aussierobots/turul-rpc/blob/main/docs/adr/002-json-rpc-2-compliance.md
- [rpc-003]: https://github.com/aussierobots/turul-rpc/blob/main/docs/adr/003-compatibility-with-turul-mcp-json-rpc-server.md

## Revision log

- 2026-05-10: Initial proposal accepted. Shim landed in framework v0.3.39
  on branch `extract/turul-rpc-shim`.
