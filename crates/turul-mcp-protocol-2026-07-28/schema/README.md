# Vendored MCP 2026-07-28 Schema

This directory vendors the upstream MCP specification TypeScript schema **for offline reference**. The Rust types in this crate must serialize to JSON shapes that match `draft-schema.ts` exactly. Tests in `tests/compliance.rs` enforce this.

## Provenance

- **File**: `draft-schema.ts`
- **Upstream source**: <https://github.com/modelcontextprotocol/modelcontextprotocol/blob/271ecc9accafdd9b83a3c869fa67c22953b2af80/schema/2026-07-28/schema.ts>
- **Raw URL used**: `https://raw.githubusercontent.com/modelcontextprotocol/modelcontextprotocol/<commit>/schema/2026-07-28/schema.ts` — fetched by commit, never `main` (see §Regenerating).
- **Vendored on**: 2026-07-29 (re-vendored from the released dated path — `SubscriptionsListenResultMeta` renamed to `…MetaObject`, new `SubscriptionsListenResultResponse`, `@see` anchors repointed; previously 2026-07-28, 2026-07-02, 2026-06-10, 2026-06-07, 2026-05-24)
- **Upstream commit pin**: `271ecc9accafdd9b83a3c869fa67c22953b2af80` — the same commit the example fixtures are pinned to in `EXAMPLES_PIN.md`. Schema and fixtures move together; a re-vendor that leaves them on different commits is a provenance defect. This is the content-bearing commit for `schema/2026-07-28/`, not the release tag `2026-07-28` (`5f5440bb…`), which is a merge commit that the subpath-filtered resolver never returns.
- **Content sha256**: `742750af0bb8c716e7030c4977c992b55d1adc4407e9e66997db5846baedc2cd` (was `c56f0ad2395f9f7109a903a304344a61c65555cb0b2d28c1635cc32497221c87`)
- **Upstream blob sha**: `9b55feeb412bc3ae877f2eac10b5c01ba29a2eed` (`schema/2026-07-28/schema.ts` at the pinned commit)
- **Upstream ETag at vendor time**: `32ea4b21522fe3444693bfc1f5e0a0ce16b1d3a5b5d2542a8ce37ca562333a40` (was `9e61d8fe6b7d645faee95c55913a871efd06d953770a0b4834fd21ad64eb4a65`)
- **Wire protocol version declared in vendored file**: `"2026-07-28"` (`LATEST_PROTOCOL_VERSION`)
- **License**: MIT (matches turul-mcp-framework dual MIT-or-Apache-2.0 licensing)

## Vendored from the released dated path

The spec has finalized. `LATEST_PROTOCOL_VERSION` is the stable date literal
`"2026-07-28"` (was `"DRAFT-2026-v1"` pre-finalization), and the crate's
`MCP_VERSION` / `McpVersion::V2026_07_28` serde rename emit it. The draft literal
is **rejected**, not accepted — `src/version.rs` has no `FromStr` arm and no serde
alias for it, and a negative test asserts the rejection.

This copy is vendored from the immutable `schema/2026-07-28/` upstream directory.
Upstream `schema/draft/` is now the *next* spec cycle's floating pointer and is
**not** what this crate tracks — a pin or drift check resolving against it would
walk onto next-cycle content while still claiming to implement 2026-07-28.
Probing `main` remains correct only because the pinned subpath is the dated
directory, which upstream touches solely to publish errata against the release.

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

The 2026-07-29 re-vendor moved the source from the pre-release `schema/draft/`
path to the released `schema/2026-07-28/` directory. It carried **no wire-format
change** — three deltas only: (1) TypeDoc `@see` anchors repointed from
`/specification/draft/…` to `/specification/2026-07-28/…`; (2)
`SubscriptionsListenResultMeta` renamed `SubscriptionsListenResultMetaObject`;
(3) new `SubscriptionsListenResultResponse extends JSONRPCResultResponse` with
its own example fixture directory, taking the upstream fixture count 87 → 88.
See `docs/adr/027-targeting-mcp-draft-2026-v1.md` for the re-pin trigger and
revision log.

## Regenerating

Fetch by **commit**, never `main` — `main` is mutable, so a `main`-sourced
re-vendor cannot be reproduced later. Use the SHA that
`mcp-compliance-2026-07-28 refresh` reports for the fixtures so both artifacts
land on one commit:

```bash
SHA=271ecc9accafdd9b83a3c869fa67c22953b2af80
curl -fsSL \
  "https://raw.githubusercontent.com/modelcontextprotocol/modelcontextprotocol/$SHA/schema/2026-07-28/schema.ts" \
  -o crates/turul-mcp-protocol-2026-07-28/schema/draft-schema.ts

# Then update the commit/sha256/date above and re-run:
cargo test -p turul-mcp-protocol-2026-07-28
```

If the regenerated schema differs from the version this crate was last built against, expect compliance tests to fail. Update Rust types to match, re-run, repeat.
