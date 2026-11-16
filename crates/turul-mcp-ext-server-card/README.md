# turul-mcp-ext-server-card

> **Experimental.** MCP Server Cards are an experimental extension upstream:
> "for prototyping and feedback only, and … not an accepted or official MCP
> extension" ([ext-server-card](https://github.com/modelcontextprotocol/ext-server-card),
> pinned at `526201b`). The schema may change before it is accepted. This crate
> follows it on its own 0.1.x line, so expect breaking patch releases.

A Server Card is a static JSON document describing a remote MCP server for
discovery before connecting: identity, remote endpoints and protocol versions
(SEP-2127). This crate holds the wire types, the schema's validation rules, and
the served document (bytes plus `ETag`). Governing decision: ADR-038.

Most users want the `ext-server-card` feature of
[`turul-mcp-server`](https://docs.rs/turul-mcp-server), which serves the card at
`GET <mcp_path>/server-card` on HTTP and Lambda:

```rust,ignore
use turul_mcp_ext_server_card::v1::{Remote, RemoteType, ServerCard};

let card = ServerCard::new("com.example/weather", "Weather forecasts by city")
    .with_remote(Remote::new(RemoteType::StreamableHttp, "https://mcp.example.com/mcp"));

let server = McpServer::builder()
    .name("weather")
    .version("1.2.0")
    .with_server_card(card)
    .build()?;
```

The server fills `version`, `title`, `description`, `websiteUrl`, `icons` and each
remote's `supportedProtocolVersions` from its own `server/discover` data. A card
value that contradicts them fails `build()`.

## What it serves

| Request | Response |
|---|---|
| `GET` / `HEAD` | 200, `Content-Type: application/mcp-server-card+json`, `Cache-Control: public, max-age=3600`, `ETag` |
| matching `If-None-Match` | 304 |
| `OPTIONS` | 204 |
| other methods | 405, `Allow: GET, HEAD, OPTIONS` |

Every response carries the CORS headers the spec requires:
`Access-Control-Allow-Origin: *`, `Allow-Methods: GET`,
`Allow-Headers: Content-Type, If-None-Match`, `Expose-Headers: ETag`.

## Not included

- **AI Catalog** (`/.well-known/ai-catalog.json`): site-wide, so the deployment publishes it.
- **Rate limiting** (SHOULD): use middleware.
- **HTTPS** (MUST in production): terminate TLS in front of the server.
- **Client-side fetching**.

A card is public: never put credentials, tokens or private endpoints in it.
