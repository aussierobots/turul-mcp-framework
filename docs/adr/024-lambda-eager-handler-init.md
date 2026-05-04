# ADR-024: Lambda Eager Handler Initialisation

**Status:** Accepted
**Date:** 2026-05-04
**Context:** Cold-init amplification under fan-out for `turul-mcp-aws-lambda`-built Lambdas. Filed from gps-trust-space-data-mcp handoff (issue #15).

## Decision

The framework's recommended Lambda startup pattern is **eager handler construction in `main()` before runtime hand-off**, using the existing API:

```rust
#[tokio::main]
async fn main() -> Result<(), Error> {
    let server = LambdaMcpServerBuilder::new()
        ./* configuration */
        .build()
        .await?;
    let handler = server.handler().await?;          // eager — runs in Init phase

    turul_mcp_aws_lambda::run_streaming(handler).await
}
```

For custom dispatch:

```rust
let handler = server.handler().await?;
turul_mcp_aws_lambda::run_streaming_with(move |request| {
    let handler = handler.clone();
    async move {
        if request.uri().path() == "/.well-known/oauth-authorization-server" {
            return Ok(well_known_response());
        }
        handler.handle_streaming(request).await
    }
}).await
```

`LambdaMcpHandler` is `Clone` (cheap — internally `Arc`'d state), so the prebuilt instance is captured by `move` into the dispatch closure and cloned per request.

## What we are NOT adding

We are **not** introducing `LambdaMcpServerBuilder::build_eager()`. Audit confirms `build().await?` followed by `server.handler().await?` already performs the full eager sequence: session storage init, server-state-storage init, tool/resource registration, capability auto-detection, session cleanup task spawn, dynamic-tools sync, and cold-start task recovery. No init work is hidden behind a lazy boundary inside the framework.

A new method would be a second name for the same code path. If a future audit identifies framework-owned init that is lazily deferred (none today), this ADR should be revised before adding the API.

## What changes in this slice

1. **README** (`crates/turul-mcp-aws-lambda/README.md`): the "Custom Dispatch" example is rewritten to capture a prebuilt handler, removing the `HANDLER.get_or_try_init(...)` pattern from the documented happy path.
2. **Examples** (`examples/lambda-mcp-server`, `examples/lambda-mcp-server-streaming`, `examples/middleware-auth-lambda`): `static OnceCell<LambdaMcpHandler>` is removed. Handler is built in `main()` and `move`-captured into the service closure.
3. **Docs**: an explicit recommendation paragraph is added to the README naming the eager pattern as the preferred shape and noting that lazy request-path init remains valid for back-compat but places build cost inside the first invocation `Duration`.

No public API changes. No handler internals changes. No session semantics changes. No new instrumentation.

## Why this matters

`LambdaMcpHandler` build cost (DDB session storage init, DDB server-state-storage init, server build, tool registration) is observed at ~500 ms p50 / ~620 ms p95 in production with 28 tools and DDB backends.

When init runs **inside the first POST** (the lazy `OnceCell::get_or_try_init` pattern), this cost lands in `handler_total_ms` from the caller's perspective. It does **not** appear in Lambda's separate `Init Duration` field, which only covers tokio runtime bootstrap (~50–70 ms).

When init runs in `main()` **before** `lambda_runtime::run()` (the eager pattern), the same cost lands in `Init Duration` instead — billed but not visible to clients, and pre-paid by Provisioned Concurrency pre-warm.

Under N-way parallel fan-out from a single caller, `handler_total = max(parallel_calls)`. The cold container dominates: with ~20% cold rate and 10 parallel calls, P(≥1 cold per fan-out) ≈ 89%, producing a ~500 ms latency floor on every fan-out invocation. The eager pattern eliminates this entirely.

## Tradeoffs

- **Net cost**: identical. Same ~500 ms is spent; it moves from first-POST timeline into Lambda's `Init Duration` field.
- **Cold container UX**: first request's `handler_total` drops by ~500 ms.
- **Provisioned Concurrency synergy**: PC pre-warm runs Init phase. Eager init means PC containers serve their first request with zero cold-init cost. Lazy init means PC containers still pay ~500 ms inside the first POST after pre-warm — defeating much of PC's value.
- **Failure mode shift**: a panic during eager init takes down the container before any traffic, vs lazy where only the first invocation fails and subsequent invocations retry init. This is acceptable: init failures (e.g., DDB unreachable) are not request-recoverable in practice — both modes effectively fail closed; eager fails closed faster and more visibly.

## Verification

- `cargo fmt --all -- --check`
- `cargo test -p turul-mcp-aws-lambda`
- `cargo check --example lambda-mcp-server`
- `cargo check --example lambda-mcp-server-streaming`
- `cargo check --example middleware-auth-lambda`

Caller-side empirical verification (deferred to consumers): CloudWatch `handler_total` p99 for affected tools drops by ~500 ms under fan-out; cold-container churn rate unchanged. Lambda REPORT lines: `Init Duration` rises by ~500 ms; `Duration` drops by the same on cold-container first invocations.

## Status of the originating problem

The space-data-mcp cold-init amplification (10-call fan-out from `current_conditions`) was resolved upstream by collapsing the fan-out to 1 call against `get_historical_current_conditions(at_utc=now)`. Post-collapse readback: n=21, p50=932 ms, p95=992 ms, **0 cold calls** in the 30-min steady-state window. This ADR exists so the analysis and recommended pattern aren't lost when a future caller develops similar amplification.

## Revision triggers

Revise this ADR (and consider adding `build_eager()`) if any of the following become true:

1. A new framework feature defers expensive init behind a lazy boundary inside `LambdaMcpServer::handler()` or below — making the current "eager `build().handler()`" claim no longer true.
2. A common caller pattern emerges where eager init is structurally infeasible (e.g., per-tenant handler construction keyed on first-request data).
3. CloudWatch evidence from a framework consumer shows the documented eager pattern still amplifies under fan-out — indicating hidden lazy work the audit missed.

Until then, the recommendation is: build eagerly, document loudly, no new API.
