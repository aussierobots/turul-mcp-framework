# Vendored MCP Draft Schema

This directory vendors the upstream MCP specification TypeScript schema **for offline reference**. The Rust types in this crate must serialize to JSON shapes that match `draft-schema.ts` exactly. Tests in `tests/compliance.rs` enforce this.

## Provenance

- **File**: `draft-schema.ts`
- **Upstream source**: <https://github.com/modelcontextprotocol/modelcontextprotocol/blob/main/schema/draft/schema.ts>
- **Raw URL used**: `https://raw.githubusercontent.com/modelcontextprotocol/modelcontextprotocol/<commit>/schema/draft/schema.ts` — fetched by commit, never `main` (see §Regenerating).
- **Vendored on**: 2026-07-28 (re-vendored — `clientInfo` optional, `ResultMetaObject`/`serverInfo`, `DiscoverResult.serverInfo` removed; previously 2026-07-02, 2026-06-10, 2026-06-07, 2026-05-24)
- **Upstream commit pin**: `71e306956a4959c9655e5036be215d41986596e6` — the same commit the example fixtures are pinned to in `EXAMPLES_PIN.md`. Schema and fixtures move together; a re-vendor that leaves them on different commits is a provenance defect.
- **Content sha256**: `c56f0ad2395f9f7109a903a304344a61c65555cb0b2d28c1635cc32497221c87` (was `6e4cba2d17f7156877357762b6b4b63cd790d8973f61ec35ab73cd61ad67017d`)
- **Upstream blob sha**: `110485f68da17d54cb4b9119add86ca958af3a94` (`schema/draft/schema.ts` at the pinned commit)
- **Upstream ETag at vendor time**: `9e61d8fe6b7d645faee95c55913a871efd06d953770a0b4834fd21ad64eb4a65` (was `7ccf61ca2965512cf1ddf995e935a8c197f67967cbb3b0b5793bd1c019426102`)
- **Wire protocol version declared in vendored file**: `"2026-07-28"` (`LATEST_PROTOCOL_VERSION`)
- **License**: MIT (matches turul-mcp-framework dual MIT-or-Apache-2.0 licensing)

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

The 2026-07-28 re-vendor (46 additions / 13 deletions vs the 2026-07-02 cut)
carried four substantive changes, all from upstream #3002 plus a docs-only
ordering clarification: (1) `RequestMetaObject`'s
`io.modelcontextprotocol/clientInfo` became optional, with servers told not to
key behavior or security decisions on it; (2) new `ResultMetaObject extends
MetaObject` carrying optional `io.modelcontextprotocol/serverInfo`, and
`Result._meta` retyped from `MetaObject` to it — so every result gains the key
at once; (3) `DiscoverResult.serverInfo` removed, server identity now riding in
`_meta`; (4) `SubscriptionsListenResultMeta` re-parented to `ResultMetaObject`,
and the `subscriptions/listen` acknowledgment ordering defined per subscription
ID rather than per channel. See `docs/adr/027-targeting-mcp-draft-2026-v1.md`
for the re-pin trigger and revision log.

## Regenerating

Fetch by **commit**, never `main` — `main` is mutable, so a `main`-sourced
re-vendor cannot be reproduced later. Use the SHA that
`mcp-compliance-2026-07-28 refresh` reports for the fixtures so both artifacts
land on one commit:

```bash
SHA=71e306956a4959c9655e5036be215d41986596e6
curl -fsSL \
  "https://raw.githubusercontent.com/modelcontextprotocol/modelcontextprotocol/$SHA/schema/draft/schema.ts" \
  -o crates/turul-mcp-protocol-2026-07-28/schema/draft-schema.ts

# Then update the commit/sha256/date above and re-run:
cargo test -p turul-mcp-protocol-2026-07-28
```

If the regenerated schema differs from the version this crate was last built against, expect compliance tests to fail. Update Rust types to match, re-run, repeat.
