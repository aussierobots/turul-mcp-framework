# ADR-026: Lambda Streaming Response Empty-Body Envelope Contract

**Status:** Closed — option (c). v0.3.42's `EnsureOneFrame` contract was found insufficient by production wire-level testing; investigation closed with no further runtime change. **APIGW MOCK on OPTIONS is the permanent pattern** for streaming-mode Lambdas. See "Resolution 2026-05-11" below.
**Date:** 2026-05-11
**Crate:** `turul-mcp-aws-lambda`

## Resolution 2026-05-11

After v0.3.42 shipped and was verified against production by the downstream consumer (sd-mcp, v0.7.12, deployed against `sd.aussierobots.com.au`), the empty-body Lambda streaming failure mode was confirmed unchanged. The `EnsureOneFrame` adapter satisfies the framework-internal contract documented below ("`BodyDataStream` yields ≥1 data frame") but does NOT satisfy the actual AWS Lambda Runtime API wire contract — `tx.send_data(Bytes::new())` produces no on-wire content bytes after the metadata + 8-NUL delimiter, hyper reports `IncompleteMessage`, the invocation hangs to timeout, API Gateway emits 502.

A wire-level diagnostic harness was developed on branch `park/wire-level-test-harness` (commit `4937e3a`, retained on origin as archived diagnostic). The harness replicates `lambda_runtime-1.2.0/src/requests.rs:81-137` verbatim and asserts non-empty content bytes after the delimiter. Under v0.3.42's current code, three of four wire tests fail, confirming the framework-level fix is insufficient.

Three resolution paths were considered:

- **(a) Emit non-empty sentinel byte** in `EnsureOneFrame`'s fallback. Trivial code change, but requires real-AWS validation that the sentinel byte stays inside the Lambda streaming envelope rather than leaking to the HTTP response body visible to the client. For 204 No Content responses (which by RFC 7230 §3.3.3 forbid response bodies), a leaked byte may break client expectations. No CI infrastructure exists in turul to validate against real AWS; shipping (a) without external validation risks the same iteration trap as v0.3.42.
- **(b) Reject empty-body streaming responses** at `into_lambda_stream_response`. Honest contract, fully validatable from local tests, but turns silent timeouts into loud build/runtime errors for any consumer using `Empty::new()` or `Full::new(Bytes::new())` bodies through `run_streaming_with`. Cost is borne by every existing consumer with no alternative inside Lambda; they'd be forced to MOCK regardless.
- **(c) Document the limitation; no runtime change.** APIGW MOCK on OPTIONS is already AWS's recommended pattern for streaming-mode Lambdas. The framework-internal contract below stays as documented but is acknowledged as incomplete at the AWS boundary.

**Decision: (c).** Reasons:

1. MOCK is AWS's own recommendation for CORS preflight on streaming Lambdas. The framework is not papering over a missing AWS capability; it is aligning with AWS's documented architecture.
2. Only one consumer (sd-mcp) has hit this bug. The fleet (sv-track, gps-trust-mcp, gps-trust-agent-mcp) will adopt MOCK in their port wave regardless of framework behavior.
3. Without real-AWS CI validation, option (a) would require deploy-iterate-revert cycles against sd-mcp's production Lambda — multi-session iteration cost outweighs the benefit of "Lambda-served preflight."
4. Option (b)'s improved error visibility is real but punitive; it changes a known workaround (MOCK) into a forced workaround without offering anything new.

### Operational consequences

- **v0.3.42 stays published.** Not regressive for non-empty body responses (the entire MCP request flow). Its CHANGELOG and the "Original contract" section below are preserved verbatim as historical record; both overstate the empty-body fix's effectiveness, but yanking or amending would only add confusion.
- **No v0.3.43 planned** for this issue. Future framework releases (for unrelated reasons) should not silently change the empty-body behavior without re-opening this ADR.
- **Fleet pattern: MOCK on all OPTIONS methods** for any consumer deploying turul-mcp-aws-lambda behind API Gateway streaming integration. Same template as sd-mcp v0.7.13.
- **`park/wire-level-test-harness`** branch retained on origin (not merged) as the diagnostic checkpoint. If a future contributor reopens this with real-AWS CI infrastructure, the wire tests there are the regression net any candidate v0.3.43 fix must clear.
- **CLAUDE.md "Test Coverage Discipline" rule 3** (wire-layer coverage requirement) is a permanent gate improvement, retained on `main` regardless of how this specific bug resolved. It exists because v0.3.42's failure mode was tests calibrated to the wrong layer; the rule prevents that recurring.

### Operational status

- sd-mcp: production stable on v0.7.13 with APIGW MOCK on `.well-known/{proxy+}`, `/oauth-protected-resource`, `/oauth-protected-resource/{proxy+}` OPTIONS. Pinned to turul-mcp-aws-lambda 0.3.42 for v0.3.40/v0.3.41 fixes (streaming custom-route CORS, `WWW-Authenticate` exposed by default, builder CORS propagation — all real, all working). OPTIONS does not invoke Lambda.
- sv-track, gps-trust-mcp, gps-trust-agent-mcp: should adopt the same MOCK pattern in their next port wave.

The "Original contract" section below describes what v0.3.42's `EnsureOneFrame` adapter implements. It is true at the framework-internal boundary (`BodyDataStream` yields ≥1 item) but does NOT solve the production failure. Read it as documentation of code that exists, not as a working contract.

---

## Original contract (v0.3.42 — superseded by Resolution above)

## Context

`turul-mcp-aws-lambda` registers Lambda binaries as streaming-only via `lambda_runtime::run` returning `StreamResponse`. The framework's adapter `into_lambda_stream_response<B>(http::Response<B>) -> StreamResponse<BodyDataStream<…>>` converts arbitrary `B: http_body::Body + Unpin + Send + 'static` response bodies into the streaming envelope.

Production failure surfaced 2026-05-11: consumers using `run_streaming_with` with custom dispatch closures that build empty-body responses — most commonly `.well-known` OPTIONS short-circuits constructing `Response<UnsyncBoxBody<Bytes, hyper::Error>>` whose inner body is `http_body_util::Empty::new()` — observed:

- Dispatch closure returns `Ok(response)` immediately (visible in CloudWatch handler-init log lines).
- The Lambda invocation then hangs for the full function timeout (60 s).
- Runtime API records `hyper_util::client::legacy::Error(SendRequest, hyper::Error(IncompleteMessage))`.
- API Gateway emits 502 to the client after the timeout.
- `AWS/Lambda Errors` metric does NOT increment; `Duration` clamps at the function timeout; REPORT shows `Status: timeout`, not `Status: error`.

A/B isolation from the affected consumer's same dispatch closure confirmed: identical response shape with `Full::new(Bytes::from(json))` body (non-empty) succeeds; with `Empty::new()` body fails. The failure is empty-body specific, not header- or status-dependent (204 and 200 both fail; differing CORS header sets fail identically).

Root cause: `http_body_util::BodyDataStream` adapts a `Body` into a `Stream<Item = Bytes>` that yields only `Frame::Data` frames. `Empty::new()` produces zero frames of any kind. The resulting Lambda multipart streaming envelope therefore writes the 8-byte prelude + metadata JSON + trailer separator, then closes the body stream **without ever writing a body chunk**. AWS Lambda's Runtime API client (hyper, from `lambda_runtime`'s perspective) expects at least one chunk before EOF for the streaming framing to terminate cleanly. Without one, the connection closes mid-frame and `hyper` reports `IncompleteMessage`.

This bug is **not a v0.3.39 → v0.3.40 regression** in turul code (the only Lambda commit in that range, `f6438cb`, does not touch `into_lambda_stream_response` or any code path traversed by custom-dispatch empty-body responses) but a pre-existing latent contract violation that consumer code began tripping when they implemented OPTIONS short-circuits in their dispatch closures.

## Decision

The framework guarantees that any `Response<B>` passed to `into_lambda_stream_response` (and by extension to `run_streaming_with` dispatch closures) produces a Lambda streaming response whose `BodyDataStream` yields **at least one data frame**, regardless of whether the underlying body `B` natively produces any data frames.

### Contract (binding on `turul-mcp-aws-lambda` ≥ 0.3.42)

For any input `Response<B>` where `B: http_body::Body<Data = Bytes> + Unpin + Send + 'static`:

1. The output `StreamResponse<BodyDataStream<…>>`'s body stream MUST yield at least one `Item = Result<Bytes, _>` before completion, even when:
   - `B` is `http_body_util::Empty<Bytes>` (zero frames natively), or
   - `B` is `http_body_util::Full<Bytes>` with `Bytes::new()` (zero bytes; depending on `Full`'s implementation may or may not yield a frame), or
   - `B` produces only `Frame::Trailers` and no `Frame::Data`, or
   - any other body shape where `BodyDataStream` would otherwise yield zero items.

2. The framework satisfies this guarantee by wrapping the input body in an internal adapter (`EnsureOneFrame<B>`) that:
   - Forwards the first data frame produced by `B` unchanged, OR
   - If `B` yields `None` before producing any data frame, emits exactly one `Frame::data(Bytes::new())` (zero-length data frame), then terminates.
   - An error from `B` before any frame propagates as-is — Runtime API treats errors as a valid envelope termination.

3. The adapter is invisible to consumers:
   - The `into_lambda_stream_response` function signature is unchanged from a caller's perspective (it is module-private; its return type alias `StreamResult` updates accordingly).
   - `run_streaming_with` and `run_streaming` are unchanged in signature and behavior for bodies that already produce ≥1 data frame.

4. The zero-length data frame is invisible at the HTTP layer:
   - It does not add a `Content-Length` header (the response is already framed as streaming/chunked from the consumer's perspective at the dispatch boundary).
   - It does not change response status, body bytes visible to the client, or response headers.
   - For 204 responses (which by HTTP semantics MUST have no body), the visible-to-client semantics remain "no body" — the zero-length frame exists only within the Lambda Runtime API envelope and is consumed there.

### Out of scope

- Bodies producing `Poll::Pending` indefinitely without yielding a frame are not affected by this contract — they behave the same as before (the dispatch closure is responsible for not constructing such bodies).
- This decision does not affect the buffered (non-streaming) `handle()` code path; that path serializes through `lambda_http::Body` which already handles empty bodies correctly.

## Consequences

### Positive

- Consumers can construct `Response<Empty<Bytes>>` or any zero-data-frame body in dispatch closures (notably `.well-known` OPTIONS short-circuits) without triggering Runtime API `IncompleteMessage`.
- The fix is invisible — no consumer code changes required; pin `turul-mcp-aws-lambda` ≥ 0.3.42 and rebuild.
- Tests validate this ADR directly (`crates/turul-mcp-aws-lambda/src/lib.rs::streaming_completion_tests::test_into_lambda_stream_response_empty_body_yields_data_frame` and `…test_run_streaming_with_empty_body_dispatch_yields_data_frame`). Revert-and-fail check recorded in the commit message confirms the regression net catches the bug.

### Negative

- A zero-length data frame is added to the Lambda Runtime API multipart envelope for empty bodies. This is a single extra ≤16-byte multipart chunk (prelude + zero-length content), invisible at the HTTP layer but visible in CloudWatch billable bytes if introspected at byte-level. Negligible.
- The internal type signature of the body inside `StreamResult` changes from `BodyDataStream<StreamBody>` to `BodyDataStream<EnsureOneFrame<StreamBody>>`. `StreamResult` is module-private; the change is transparent to consumers.

### Neutral

- The adapter is intentionally minimal — three states, ~50 lines including impl. It does not buffer or pre-poll the body, so streaming semantics for non-empty bodies are unchanged (first frame is forwarded as soon as the underlying body yields it).

## Verification

Revert-and-fail proof recorded in commit message: with the fix logic removed and `EnsureOneFrame` definition retained but unused, the empty-body tests fail with `frames: []`; the non-empty negative control still passes. Restoring the fix makes all three tests pass.

Full gates per CLAUDE.md "Test Coverage Discipline":
- ADR exists: this document.
- Production-path coverage: `test_run_streaming_with_empty_body_dispatch_yields_data_frame` exercises `handle_runtime_payload` directly (the layer `run_streaming_with` wraps), using sd-mcp's exact failing response shape.
- Revert-and-fail: confirmed before commit.

## References

- `crates/turul-mcp-aws-lambda/src/lib.rs::EnsureOneFrame` — the adapter.
- `crates/turul-mcp-aws-lambda/src/lib.rs::into_lambda_stream_response` — the wrapping site.
- `crates/turul-mcp-aws-lambda/src/lib.rs::streaming_completion_tests` (3 new tests + 1 negative control).
- AWS Lambda Response Streaming protocol: <https://docs.aws.amazon.com/lambda/latest/dg/configuration-response-streaming.html>
- `http-body` `Frame` and `BodyDataStream` semantics: <https://docs.rs/http-body/>
