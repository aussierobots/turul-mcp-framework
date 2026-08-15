# Icon Showcase

Not a server — a plain binary that builds one of each icon-carrying MCP type
and pretty-prints its JSON. Run it when you want to see the exact wire shape
`icons` serializes to before wiring icons into a real server.

## What it prints

| # | Type | Icon form |
|---|---|---|
| 1 | `Tool` | `Icon::new("https://…")` |
| 2 | `Tool` | `Icon::data_uri("image/svg+xml", "<base64>")` |
| 3 | `Resource` | HTTPS URL |
| 4 | `ResourceTemplate` | HTTPS URL |
| 5 | `Prompt` | HTTPS URL |
| 6 | `Implementation` (server identity) | HTTPS URL |
| 7 | bare `Icon` | `.with_mime_type()`, `.with_sizes()`, `.with_theme()` |

`Icon::data_uri(mime, b64)` builds the `src` for you — the printed output shows
`"src": "data:image/svg+xml;base64,…"` alongside a `mimeType` of
`image/svg+xml`.

`icons` is optional on every one of these types and is omitted from the JSON
when unset, so adding it is never a breaking change for a client.

## Spec lane

MCP **2026-07-28**, which is where `icons` and the `Icon` struct come from.

## Run

```bash
cargo run -p icon-showcase
```

There is no port and no HTTP: the binary prints seven JSON blocks and exits 0.

## Using this in a server

A tool written by hand supplies its icons by implementing `HasIcons`, the way
`examples/calculator-add-manual-server` implements the rest of the trait set.

Macro-authored tools currently cannot: `#[mcp_tool]` and `#[derive(McpTool)]`
have no `icons` attribute, and both unconditionally emit `impl HasIcons for
YourTool {}` (the empty default), so adding your own impl fails to compile with
`E0119: conflicting implementations`. Icons on a macro-authored tool need a
framework change, not a workaround.
