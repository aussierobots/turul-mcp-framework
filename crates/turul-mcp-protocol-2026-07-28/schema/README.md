# Vendored MCP Draft Schema

This directory vendors the upstream MCP specification TypeScript schema **for offline reference**. The Rust types in this crate must serialize to JSON shapes that match `draft-schema.ts` exactly. Tests in `tests/compliance.rs` enforce this.

## Provenance

- **File**: `draft-schema.ts`
- **Upstream source**: <https://github.com/modelcontextprotocol/modelcontextprotocol/blob/main/schema/draft/schema.ts>
- **Raw URL used**: <https://raw.githubusercontent.com/modelcontextprotocol/modelcontextprotocol/main/schema/draft/schema.ts>
- **Vendored on**: 2026-07-02 (re-vendored — MCP error-code renumbering, cancellation `requestId` required, `ElicitationCompleteNotification` removal, subscription-stream metadata contracts; previously 2026-06-10, 2026-06-07, 2026-05-24)
- **Upstream ETag at vendor time**: `7ccf61ca2965512cf1ddf995e935a8c197f67967cbb3b0b5793bd1c019426102` (was `fddd24701cf0e60f1acae2934e0c3f2b77e32389b98807ba45cb65413be97bd5` at the 2026-06-10 cut)
- **Content sha256**: `6e4cba2d17f7156877357762b6b4b63cd790d8973f61ec35ab73cd61ad67017d` (was `1bf94a601817ab07fc04058a9ff2e031227f9b9384e198ea7f187e75eb4b9ec6`)
- **Wire protocol version declared in vendored file**: `"2026-07-28"` (line 37, `LATEST_PROTOCOL_VERSION`)
- **License**: MIT (matches turul-mcp-framework dual MIT-or-Apache-2.0 licensing)
- **Upstream commit pin**: `93671a3f2bac3bc11b0eb6327c2d029e272b2871` (`schema/draft/schema.ts`) — verified byte-identical (sha256 unchanged: `6e4cba2d17f7156877357762b6b4b63cd790d8973f61ec35ab73cd61ad67017d`) to the vendored file on 2026-07-13.

## ⚠ DRAFT-PATH WARNING

The upstream `LATEST_PROTOCOL_VERSION` has finalized to the stable date literal
`"2026-07-28"` (was `"DRAFT-2026-v1"` pre-finalization), and the crate's
`MCP_VERSION` / `McpVersion::V2026_07_28` serde rename now emit it (the draft
literal is still accepted on deserialize for back-compat). **The file still lives
under the upstream `schema/draft/` path** and may continue to receive field-level
revisions — re-vendor and regenerate compliance tests as it evolves.

The 2026-06-07 re-vendor carried exactly three substantive (non-comment) changes
from the 2026-05-24 cut: the version-string finalization above; `ResultType`
became an open union (`"complete" | "input_required" | string`); and
`DiscoverResult` now `extends CacheableResult` (`ttlMs`/`cacheScope`). See
`docs/adr/027-targeting-mcp-draft-2026-v1.md` for the re-pin trigger and revision log.

The 2026-07-02 re-vendor (145 additions / 82 deletions vs the 2026-06-10 cut)
carried five substantive changes: (1) MCP error codes renumbered —
`HEADER_MISMATCH` `-32001`→`-32020`, `MISSING_REQUIRED_CLIENT_CAPABILITY`
`-32003`→`-32021`, `UNSUPPORTED_PROTOCOL_VERSION` `-32004`→`-32022`, with the
range formally partitioned (`-32000..-32019` implementation-defined,
`-32020..-32099` spec-reserved, allocated sequentially); (2)
`CancelledNotificationParams.requestId` changed from optional to required,
and the notification is now client→server only (except a stdio-only
server-sent form that closes a `subscriptions/listen` stream);
`ProgressNotification` dropped from `ClientNotification`; (3)
`ElicitationCompleteNotification`/`...Params` and
`ElicitRequestURLParams.elicitationId` removed entirely — completion is
learned by retrying the original MRTR request; (4) new
`NotificationMetaObject` (extends `MetaObject` with optional
`io.modelcontextprotocol/subscriptionId: RequestId`) — every notification
delivered on a `subscriptions/listen` stream MUST carry it;
`NotificationParams._meta` retyped to `NotificationMetaObject`; new
`SubscriptionsListenResult` (required `_meta.subscriptionId`) sent on
graceful stream teardown; (5) `ListRootsRequest.params` retyped from
`RequestParams` to an inline `{ _meta?: MetaObject }` (no longer required
`RequestMetaObject`). See `docs/adr/027-targeting-mcp-draft-2026-v1.md` for
the re-pin trigger and revision log.

## Regenerating

```bash
curl -fsSL \
  https://raw.githubusercontent.com/modelcontextprotocol/modelcontextprotocol/main/schema/draft/schema.ts \
  -o crates/turul-mcp-protocol-2026-07-28/schema/draft-schema.ts

# Then update the ETag/date above and re-run:
cargo test -p turul-mcp-protocol-2026-07-28
```

If the regenerated schema differs from the version this crate was last built against, expect compliance tests to fail. Update Rust types to match, re-run, repeat.
