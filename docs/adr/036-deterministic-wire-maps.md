# ADR-036: Wire maps serialize in sorted key order

**Status:** Accepted
**Date:** 2026-11-14
**Crates:** `turul-mcp-protocol-2026-07-28` (owner of `serde_maps::sorted`)
**Related:** ADR-010 §4 (tool output schemas)

## Context

The 2026-07-28 wire types hold JSON objects as `std::collections::HashMap`
fields: tool `inputSchema.properties`, schema extensions flattened beside them,
`_meta` maps, capability maps and others. A `HashMap` iterates in a random
order chosen per map, so serializing the same value twice can produce
different bytes.

The order was stable in this workspace only by accident. Converting a
`HashMap` to `serde_json::Value` sorts keys while serde_json's
`preserve_order` feature is off. Any crate in a consumer's dependency graph
can turn that feature on, and then `tools/list` bytes change from run to run
of the same binary. A downstream server observed exactly this: identical tools
on every run, different bytes on every run. Byte-stable output matters to
anything that hashes, diffs or caches responses, and 2026-07-28 encourages
caching (`ttlMs`, `cacheScope`).

## Decision

Every `HashMap<String, _>` field of a `Serialize` wire type in
`turul-mcp-protocol-2026-07-28` serializes with
`#[serde(serialize_with = "crate::serde_maps::sorted")]` (and the `Option`
form `sorted_opt`), which writes entries in ascending key order. Flattened
maps use the same function. `serde_maps` is the one definition; no other
crate re-implements it.

The public field types stay `HashMap`, so no caller changes. Deserialization
is unchanged.

Rejected:
- Switching the fields to `BTreeMap` — the same effect, but it changes over a
  hundred public field types and every caller that builds them.
- Relying on `serde_json::Value` conversion — that is the accident this ADR
  removes.
- Declaration order (an ordered map) — needs the same type change and gives
  no stronger guarantee than sorting.

The frozen `turul-mcp-protocol-2025-11-25` crate is not changed.

## Tests

- `tool_schema_serializes_identically_every_time`: a schema with many
  properties and extension keys serializes to the same bytes across many
  independently built instances, with keys in sorted order.
- `every_wire_hashmap_field_is_sorted`: a source scan of the crate finds no
  `HashMap<String, …>` field in a `Serialize` type without the attribute.

## Revision log

- **2026-11-14**: Initial, Accepted.
