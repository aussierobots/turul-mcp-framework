# turul-mcp-json-rpc-server (compatibility shim)

> **Terminal shim.** This crate is a thin re-export of [`turul-rpc`][turul-rpc]
> for backward compatibility with `turul-mcp-framework 0.3.x`. **New code
> should depend on `turul-rpc` directly.**

[turul-rpc]: https://crates.io/crates/turul-rpc

## What this crate is

`turul-mcp-json-rpc-server` was the generic JSON-RPC 2.0 implementation
inside `turul-mcp-framework` through v0.3.38. As of **v0.3.39** the
implementation moved to a sibling repository, [`turul-rpc`][turul-rpc-repo],
which publishes four crates:

- [`turul-rpc-core`](https://crates.io/crates/turul-rpc-core) — wire types
- [`turul-rpc-jsonrpc`](https://crates.io/crates/turul-rpc-jsonrpc) — codec, parser, JSON-RPC 2.0 batch
- [`turul-rpc-server`](https://crates.io/crates/turul-rpc-server) — async dispatcher, handler trait, session, streaming
- [`turul-rpc`](https://crates.io/crates/turul-rpc) — facade re-exporting the above

This crate (`turul-mcp-json-rpc-server`) is now a ~50-line re-export shim.
Every public type, trait, function, and module from prior 0.3.x releases
continues to resolve at the same path with the **same nominal type** —
existing imports compile and behave identically.

[turul-rpc-repo]: https://github.com/aussierobots/turul-rpc

## Lifecycle

| Version | Posture |
|---|---|
| `0.3.38` and earlier | Self-contained JSON-RPC 2.0 implementation |
| `0.3.39` (current) | Re-export shim over `turul-rpc 0.1` |
| `0.3.40+` | Patch shim if a compatibility issue is found |
| `0.4.0` | **Not planned.** `turul-mcp-framework 0.4.0` removes this crate from its dependency tree |

Existing 0.3.x consumers may continue depending on this crate
indefinitely. There is no deprecation warning attached to imports.

## For new code

```toml
[dependencies]
turul-rpc = "0.1"
```

```rust
use turul_rpc::{JsonRpcDispatcher, JsonRpcHandler, RequestParams, SessionContext};
use turul_rpc::error::JsonRpcErrorObject;
use turul_rpc::r#async::ToJsonRpcError;
```

See the [`turul-rpc` README][turul-rpc-readme] for the full quick-start,
two runnable examples (`simple_calculator`, `batch_dispatch`), and the
[`docs/adr/`][adrs] directory for architectural decisions.

[turul-rpc-readme]: https://github.com/aussierobots/turul-rpc#readme
[adrs]: https://github.com/aussierobots/turul-rpc/tree/main/docs/adr

## Compatibility contract

Two test files in this crate enforce the v0.3.38 surface:

- `tests/symbol_coverage.rs` — names every top-level public path from
  the v0.3.38 `cargo public-api` snapshot via `use` statements; fails
  to compile if any path becomes unreachable.
- `tests/shim_compat.rs` — asserts type identity across paths
  (`turul_mcp_json_rpc_server::RequestId == turul_rpc::RequestId ==
  turul_rpc::types::RequestId`).

New `turul-rpc 0.1` APIs that did not exist in v0.3.38 (notably
`parse_json_rpc_batch` and `BatchOrSingle`) live in `turul_rpc::batch`,
a module **not re-exported** by this shim. Users who want batch
processing should depend on `turul-rpc` directly. See [ADR-003][adr-003].

[adr-003]: https://github.com/aussierobots/turul-rpc/blob/main/docs/adr/003-compatibility-with-turul-mcp-json-rpc-server.md

## Compliance posture

`turul-rpc 0.1` (and therefore this shim) implements JSON-RPC 2.0 with
**one documented departure**: incoming requests with `"id": null` are
rejected as `Invalid Request` (`-32600`). The spec permits null id but
discourages it. The strict posture is **inherited from v0.3.38** — this
shim release does not change runtime behaviour. See
`turul-rpc/docs/adr/002-json-rpc-2-compliance.md` for the v0.2 plan to
surface a permissive codec-level type.

## Where to file issues

| Kind | Repository |
|---|---|
| JSON-RPC 2.0 implementation, batch, dispatcher, types | [`turul-rpc`](https://github.com/aussierobots/turul-rpc/issues) |
| Shim compatibility regression in framework 0.3.x | [`turul-mcp-framework`](https://github.com/aussierobots/turul-mcp-framework/issues) |

## License

MIT OR Apache-2.0.
