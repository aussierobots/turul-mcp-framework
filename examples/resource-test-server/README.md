# Resource Test Server

An E2E **fixture**, not a tutorial. It registers 18 resources chosen to cover
the `resources/*` surface and its edge cases, and exists so
`tests/resources` (`cargo test -p mcp-resources-tests`, 67 tests) and
`tests/mcp_runtime_capabilities_validation.rs` have one server to point at.

If you are learning how to write a resource, read `examples/resource-server`
(derive macro) or `examples/resources-server` (hand-written traits) instead.

## Spec lane

Pinned to **2025-11-25** in `Cargo.toml` (`protocol-2025-11-25` on every
framework dependency), independent of the workspace 2026-07-28 default: the
suite drives it through the stateful `initialize` →
`notifications/initialized` → `Mcp-Session-Id` handshake.

## What each resource is for

| URI | `mimeType` | Exercises |
|---|---|---|
| `file:///tmp/test.txt` | `text/plain` | Reading from a real temp file |
| `file:///memory/data.json` | `application/json` | In-memory JSON |
| `file:///error/not_found.txt` | `text/plain` | `read()` returning an error |
| `file:///slow/delayed.txt` | `text/plain` | A slow read (timeout behaviour) |
| `file:///template/items/{id}.json` | `application/json` | URI template + variable extraction |
| `file:///empty/content.txt` | `text/plain` | Empty body |
| `file:///large/dataset.json` | `application/json` | Large payload |
| `file:///binary/image.png` | `image/png` | Base64 `blob` contents |
| `file:///session/info.json` | `application/json` | Session context inside `read()` |
| `file:///subscribe/updates.json` | `application/json` | `resources/subscribe` |
| `file:///notify/trigger.json` | `application/json` | `notifications/resources/updated` |
| `file:///multi/contents.txt` | `multipart/mixed` | Several `contents[]` entries with different URIs |
| `file:///paginated/items.json` | `application/json` | Cursor pagination |
| `file:///invalid/bad-chars-and-spaces.txt` | `text/plain` | Awkward characters in a URI |
| `file:///meta/dynamic.json` | `application/json` | `_meta` round-tripping |
| `file:///template/users/{user_id}.json` | `application/json` | A second template shape |
| `file:///template/files/{path}` | `text/plain` | Template with a path segment |
| `file:///complete/all-fields.json` | `application/json` | Every optional `Resource` field populated |

Resources declaring `application/json` build their contents with
`ResourceContent::json()` so `resources/read` reports the same type
`resources/list` advertises. `ResourceContent::text()` always reports
`text/plain`, and the frozen 2025-11-25 protocol crate has no
`with_mime_type()` — which is why `multipart/mixed` on the multi-content
resource still cannot be expressed per entry on this lane.

## Run

```bash
# --port 0 (the default) takes an ephemeral port; the choice is logged
cargo run -p resource-test-server -- --port 8020
```

The E2E harness starts it itself via `TestServerManager::start_resource_server()`
(`tests/shared/src/e2e_utils.rs`), so running it by hand is only for poking at
it with curl.

## Duplicate copy

`tests/resources/bin/main.rs` is a byte-identical copy of `src/main.rs`, built
as the `resource-test-server-e2e` binary. Nothing spawns that binary — the
harness runs `cargo run -p resource-test-server`, i.e. this package. Edit this
file; the copy is dead weight awaiting removal.
