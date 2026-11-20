# ADR-040: `turul-mcp-client` on wasm32 — 2026-07-28 HTTP under wasm-bindgen

**Status:** Accepted
**Date:** 2026-11-20
**Crates:** `turul-mcp-client` (target `wasm32-unknown-unknown`: the crate-private runtime module `rt`, the `Transport` trait's `Send` bound, `HttpTransport` construction, `TransportFactory`)
**Related:** ADR-039 (OAuth Client Credentials, which stays native-only)

## Context

A Rust application compiled to `wasm32-unknown-unknown` (a browser page, or
Node through wasm-bindgen) cannot use `turul-mcp-client` 0.4.17. It has to
hand-write the MCP wire shape over `fetch`: the `MCP-Protocol-Version`,
`Mcp-Method` and `Mcp-Name` headers, `params._meta`, result and error
decoding. That duplicates what `transport/http.rs` and `client.rs` already
own and lets it drift.

Measured on `wasm32-unknown-unknown` (0.4.17, `--no-default-features
--features http,client-2026-07-28-only`):

- `turul-mcp-protocol-2025-11-25`, `turul-mcp-protocol-2026-07-28`,
  `turul-mcp-schema-validation` and `turul-rpc` 0.2.4 check clean.
- The client fails in its dependencies:
  - `tokio` `full`, and `tokio-util` `full` (inherited from the workspace
    entry), enable `net`, which pulls `mio`; `mio` has no wasm backend;
  - `getrandom` 0.4 (through `rand`) needs its `wasm_js` feature;
  - `jsonwebtoken` (feature `oauth-client-credentials`) pulls `getrandom`
    0.2 without `js`, which refuses the target.
- With the dependencies fixed, the client's own code fails at about 40 sites:
  - `tokio::spawn`, `tokio::time::{sleep, timeout, Instant}` and
    `tokio::task::JoinHandle`: there is no tokio runtime;
  - `std::time::Instant` compiles but panics at run time;
  - `reqwest`'s fetch `ClientBuilder` has no `timeout`, pool, HTTP/2 or
    `redirect` options;
  - `Transport` futures must be `Send`; `reqwest`'s fetch futures hold
    `Rc<RefCell<..>>` and are not.

## Decision

1. **Selected by target, not by a cargo feature.** Everything in this ADR is
   under `cfg(target_arch = "wasm32")`. On every other target, the dependency
   lines, resolved features, public types, trait bounds, defaults and
   behaviour are what 0.4.17 ships. No cargo feature is added. Any
   `wasm32` target other than `target_os = "unknown"` (WASI, emscripten)
   fails with a `compile_error!`: the fetch backend and wasm-bindgen need a
   JavaScript host.

2. **Supported feature set:**
   `default-features = false, features = ["http", "client-2026-07-28-only"]`.
   The wire is the 2026-07-28 stateless HTTP POST. On wasm32 a
   `compile_error!` naming this ADR fires for:
   - `client-bilingual` (the default) or `client-2025-11-25-only`: the
     2025-11-25 lane needs a session and a long-lived GET SSE stream;
   - `oauth-client-credentials`: its token source needs a client with its own
     redirect policy and timeout (ADR-039 §6), which fetch cannot express.
     `jsonwebtoken` moves to the native dependency table so that this message,
     not a `getrandom` error, is what the build reports.

   `ext-apps`, `ext-tasks` and `ext-skills` are not refused, but they are not
   checked or supported on wasm32.

3. **Native-only code is removed on wasm32, not ported.**
   - `SseTransport` (the pre-2025-03-26 transport) is not compiled.
     `TransportFactory::from_url` and `create` return
     `TransportError::ConnectionFailed` for an SSE URL or type, and
     `available_transports()` returns `[Http]`. `McpClientBuilder::build()`
     `expect`s the transport, so an SSE URL panics there, as any rejected URL
     already does on native.
   - `HttpTransport::start_event_listener` returns its receiver without
     starting the GET SSE listener. That listener exists only for 2025-11-25,
     which §2 refuses.
   - The `sse` cargo feature has no source effect on any target; it stays inert.

4. **One runtime owner: the crate-private module `rt`.** Every spawn, sleep,
   timeout and clock read compiled on wasm32 goes through it:

   | `rt` item | native | wasm32 |
   |---|---|---|
   | `spawn(fut) -> JoinHandle` | `tokio::spawn` | `wasm_bindgen_futures::spawn_local` of a `futures::future::Abortable` |
   | `JoinHandle::abort()` | tokio's | the `AbortHandle` |
   | `sleep(d)` | `tokio::time::sleep` | `gloo_timers::future::sleep` |
   | `timeout(d, fut) -> Result<T, Elapsed>` | `tokio::time::timeout` | `futures::future::select` against `sleep` |
   | `Instant` (clock) | `std::time::Instant` | `web_time::Instant` |
   | `TimerInstant` (deadlines) | `tokio::time::Instant` | `web_time::Instant` |

   On native every row is the type or function the code uses today, so
   behaviour and public types (`SessionInfo::created_at`, `last_activity`)
   are unchanged. `tokio::sync` and `tokio::io` line reading need no runtime
   and are kept. Sites: `McpClient`'s `Drop` (a `rt::spawn`), its retry
   sleeps and request timeouts, `StreamHandler::start`, and the streaming
   POST reader in `HttpTransport`. `parking_lot` stays: it panics on
   contention on wasm32, and shared code cannot hold a guard across
   `.await` because the native build requires `Send` futures. wasm-only code
   must not either.

5. **`Send` on wasm32 only.** The `Transport` trait and its implementations
   use `#[async_trait]` on native and `#[async_trait(?Send)]` on wasm32. The
   `Send + Sync` supertrait stays on both: `HttpTransport` is `Send + Sync`
   on wasm32 (the fetch `Client` is an `Arc` of headers), and the supertrait
   makes the build prove it. `McpClient`'s methods are inherent `async fn`s;
   their futures are `Send` on native as today, and not `Send` on wasm32,
   where they run on wasm-bindgen's single-threaded executor. A downstream
   `Transport` implementation compiles unchanged on native; to compile on
   wasm32 it needs the same `cfg_attr`.

6. **HTTP options on wasm32.** The fetch backend has no client-level
   timeout, pool, HTTP/2 keepalive or redirect policy.
   - The 30 s transport timeout is set per request (`RequestBuilder::timeout`,
     which aborts the fetch through an `AbortController`) in
     `HttpTransport::dispatch`, which every POST and DELETE passes through,
     on wasm32 only. On native the client-level timeout stays, so a caller's
     `with_client` policy is not overridden. The timer covers the whole
     response, body included, so the streaming POST
     (`send_request_streaming`: `subscriptions/listen`,
     `call_tool_with_progress`) is exempt; `McpClient` already bounds its
     first frame with `timeouts.request`.
   - `ConnectionConfig::pool_settings` is not applied, because the host owns
     connections. This affects tuning only.
   - `follow_redirects = false` cannot be honoured because fetch follows
     redirects. `HttpTransport::with_config` returns
     `TransportError::ConnectionFailed` saying so, instead of ignoring it, and
     `max_redirects` is ignored. `McpClientBuilder::build()` turns that error
     into a panic, as it does for every `with_config` error.
   - No `User-Agent` is set on wasm32, and `ConnectionConfig::user_agent` is
     ignored there; its default names one. A script-set `User-Agent` adds a
     header the server's CORS allow-list does not name, and a browser
     preflight then fails. The host sends its own.

7. **Dependencies.** Native lines move into target tables unchanged.
   - `[target.'cfg(not(target_arch = "wasm32"))'.dependencies]`: the
     existing `tokio` (`full`), `tokio-util` (`workspace`, `codec`, optional),
     `jsonwebtoken` (optional) and `hyper` lines. `hyper` is unused in `src/`
     and `tests/`; moving it keeps it out of the wasm tree at no native cost.
   - `[target.'cfg(target_arch = "wasm32")'.dependencies]`:
     - `tokio = { version = "1", default-features = false, features = ["sync", "io-util"] }`;
     - `tokio-util = { version = "0.7", default-features = false, features = ["codec", "io"], optional = true }`.
       The workspace entry enables `full`, which includes `net`, and
       inheritance can only add features;
     - `web-time`, `wasm-bindgen-futures`, `gloo-timers` (`futures`), and
       `getrandom` with `wasm_js`.
   - The existing dev-dependencies move to the native table. wasm32
     dev-dependencies: `wasm-bindgen-test`, `base64`.
   - Proof of "unchanged": `cargo tree -e features --target <host> -p turul-mcp-client`
     for the default, the `http,client-2026-07-28-only` and the
     `oauth-client-credentials` feature sets is identical before and after.

8. **Not built.**
   - The 2025-11-25 lane, the GET SSE listener and `SseTransport` on wasm32.
   - OAuth Client Credentials on wasm32.
   - WASI, emscripten, or any wasm host other than wasm-bindgen.
   - `Mcp-Param-*` in a browser. The framework's 2026-07-28 CORS allow-list
     (`turul-http-mcp-server` `cors.rs`) permits `MCP-Protocol-Version`,
     `Mcp-Method`, `Mcp-Name` and `Authorization`, with
     `Access-Control-Allow-Origin: *`, and CORS is on by default. It omits
     `Mcp-Param-*`, which the client sends only for tools that declare
     header-bound parameters. A browser call to such a tool fails its
     preflight. This is recorded here, not changed by this ADR.
     A server deployed behind other infrastructure must allow the same headers.

## Tests

- **Fixture:** `crates/turul-mcp-client/examples/wasm_e2e_fixture.rs`, a
  native-only 2026-07-28 server built from the client's existing
  `turul-mcp-server` dev-dependency. It has:
  - a blob resource;
  - a resource that sleeps 2 s;
  - a tool. It must be `call_tool_with_progress`-callable.
  
  It takes `--port`. The 2026-07-28 server rejects a missing or mismatched
  `Mcp-Method`/`Mcp-Name` (400, -32020) and malformed `_meta`, so a
  successful call also checks the wire shape.
- **wasm32 (`tests/wasm.rs`, `#![cfg(target_arch = "wasm32")]`,
  `wasm-bindgen-test`), run twice: under Node ≥ 20 (for `globalThis.crypto`),
  and in headless Firefox (`WASM_BINDGEN_USE_BROWSER=1`, geckodriver).
  The browser run serves the test page from the runner's own port, so every
  request to the fixture is cross-origin and passes a real CORS preflight:**
  - It reads two URLs at compile time: `TURUL_WASM_E2E_URL` (the fixture)
    and `TURUL_WASM_E2E_DEAD_URL` (a port the gate has checked is closed, and
    not on the Fetch bad-port list). When either is unset, the tests that use
    it fail rather than skip.
  - `connect`, then `resources/read` of the blob, returns contents whose base64
    decodes to the fixture's bytes, with its content `_meta` intact. This is
    run once through `HttpTransport::new` and once through `with_config` with
    the default `ConnectionConfig` (no `User-Agent`, so the browser preflight
    passes).
  - `resources/read` of an unknown URI returns `McpClientError::ServerError`
    with code -32602 and the server's message. Decoding `data` is shared code,
    covered natively in `error.rs`.
  - The slow resource, with a 100 ms request timeout and one attempt, returns
    `McpClientError::Timeout` (`rt::timeout` and `rt::sleep`).
  - `call_tool_with_progress` returns the final `structuredContent`, and
    `on_progress` receives exactly one notification carrying the progress,
    total and token (`rt::spawn`, the streaming POST).
  - After a client is dropped and the test yields, a new client still reads
    the blob (the `Drop` `rt::spawn`).
  - The dead URL returns `TransportError::Http`, not a panic.
  - `HttpTransport::with_config` with `follow_redirects: false` returns
    `ConnectionFailed` (§6).
  - 32 draws of `RetryConfig::delay_for_attempt(2)` with jitter all fall in
    range and are not all equal (`rand` through `getrandom` `wasm_js`).
  - `TransportFactory::available_transports()` is `[Http]`, and an SSE URL or
    type gives `ConnectionFailed` (§3).
  - Run as `cargo test --target wasm32-unknown-unknown -p turul-mcp-client
    --no-default-features --features http,client-2026-07-28-only --test wasm`
    with `wasm-bindgen-test-runner` pinned to the `wasm-bindgen` version in
    `Cargo.lock`.
- **Clock routing (native unit tests in `src/rt.rs`):**
  - A scan of the wasm-compiled files in `src/` allows `std::time::Duration`
    and nothing else from `std::time`. The rule is inverted so that a new
    import form cannot slip through. It flags path items, brace lists, `*`,
    `self`, `use std::time;`, `as` renames and `use std::{.. time ..}`.
  - The scan covers code before each file's inline `#[cfg(test)] mod x {`,
    with `//` lines skipped. It asserts that `client.rs`, `session.rs` and
    `transport/http.rs` were scanned. The two native-only files,
    `transport/sse.rs` and `oauth_client_credentials.rs`, are exempt by name.
  - Unit tests pin the flagged and allowed forms, and the test-module cut.
  - The compiler cannot catch this: `std::time::Instant` builds on wasm32 and
    panics only at run time. tokio timers and spawns need no scan, because
    the wasm32 tokio has no `rt` or `time` and the check gate fails on them.
  - Revert-and-fail: the scan was red before the implementation, on
    `session.rs:13` and `transport/http.rs:9`.
- **Gate `scripts/ci-gates.sh wasm` and a `ci.yml` job:**
  - `cargo check --target wasm32-unknown-unknown` of the §2 set;
  - the default feature set, and `oauth-client-credentials`, on wasm32 fail
    with the §2 `compile_error!`, asserted on the message;
  - the fixture started natively, then the wasm32 test under Node and in
    headless Firefox.
- **Native unchanged:** the existing `default`, `opt-in-2025`, `mutex` and
  `docs` gates pass, no existing test assertion is edited, and the §7
  `cargo tree` comparison is empty.

## Consequences

- wasm-bindgen applications get the same `McpClient` and wire code as native
  callers.
- On wasm32 the client's futures are `!Send`, so code that requires `Send`
  futures does not compile there.
- wasm32 adds `web-time`, `wasm-bindgen-futures`, `gloo-timers` and the
  `getrandom` `wasm_js` feature. Native gains no dependency.
- Browser use is tested in headless Firefox. Chrome and Safari are not run.

## Revision log

- **2026-11-20**: Initial. Revised after review:
  - native dependency lines move into target tables unchanged, with a
    feature-tree proof;
  - `web-time` is wasm-only;
  - `jsonwebtoken` is native-only so §2's message is reached;
  - the GET listener and `SseTransport` are removed on wasm32, not ported;
  - `TransportFactory`, redirect, timeout-scope and `User-Agent` behaviour
    are specified;
  - non-`unknown` wasm32 targets are refused;
  - the clock scan is narrowed to what the compiler cannot catch;
  - a 2026-07-28 fixture is added, because `resource-test-server` serves
    2025-11-25 only;
  - wasm tests cover jitter, `Drop`, spawn and redirect refusal;
  - the tests also run in a headless browser, cross-origin, so CORS is
    exercised.
- **2026-11-20**: Test review:
  - a dead-port URL replaces port 9, which browsers block before any connection;
  - `User-Agent` is never set on wasm32;
  - the clock scan inverts its rule;
  - factory, default-config, progress-token and jitter-spread tests are added.
- **2026-11-20**: Accepted after the implementation:
  - the gate passes all 9 checks;
  - all 10 tests pass under Node and in headless Firefox;
  - the native feature trees are identical;
  - revert-and-fail: a script-set `User-Agent` leaves Node green and fails 5
    tests in Firefox on the CORS preflight.
