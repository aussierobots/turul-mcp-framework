# audit-trail-server (MCP 2026-07-28)

A compliance-style audit trail: append-only events in SQLite, searchable, with
summary / detailed / compliance report generation.

## Spec lane and why

**2026-07-28 — the workspace default lane** (it is in `default-members`, so a
plain `cargo build` covers it). Attribution is deliberately actor-centric:
callers pass an explicit `actor` and every row also records the per-request
correlation id. On the stateless 2026 core there is no client-visible session,
so nothing here keys on cross-request session identity — which is also the
honest design for an audit log, where the actor is a claim you want recorded
explicitly rather than inferred from transport state.

## Run

```bash
cargo run -p audit-trail-server        # port 8009
```

It creates `audit_trail.db` **in the current working directory** on startup.
Run it from a scratch directory if you do not want that file in your repo
checkout.

## Tools

| Tool | Purpose |
|---|---|
| `log_audit_event` | Append one immutable event (`event_type`, `action`, `result`, optional `actor`, `resource`, `metadata`) |
| `search_audit_trail` | Filter by event type / actor / result / time window, with a limit |
| `generate_compliance_report` | `SUMMARY`, `DETAILED`, or `COMPLIANCE` rollups |

## What to expect

```bash
curl -s -X POST http://127.0.0.1:8009/mcp \
  -H 'Content-Type: application/json' -H 'Accept: application/json' \
  -H 'MCP-Protocol-Version: 2026-07-28' \
  -H 'Mcp-Method: tools/call' -H 'Mcp-Name: log_audit_event' \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{
        "name":"log_audit_event",
        "arguments":{"event_type":"ACCESS","action":"login","result":"SUCCESS","actor":"user123"},
        "_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28",
                 "io.modelcontextprotocol/clientCapabilities":{}}}}'
```

```json
{"structuredContent":{"output":{"audit_id":"019fad289dbc71719637fbece8110632",
 "compliance":"LOGGED","immutable":true,"logged":true,
 "timestamp":"2026-07-29T09:15:48.028316480Z"}}}
```

A follow-up `generate_compliance_report` with `report_type: "SUMMARY"` then
answers `{"total_events":1,"unique_actors":1,"success_rate":100.0}`.

Every 2026 request needs `params._meta` carrying
`io.modelcontextprotocol/protocolVersion` and
`io.modelcontextprotocol/clientCapabilities`, plus the `Mcp-Method` header
(and `Mcp-Name` for `tools/call`). The server rejects a body/header
disagreement.

## Note on the word "immutable"

The tool reports `immutable: true` and the compliance report claims
`immutable_records: true`. That describes the *write path* — nothing in this
server updates or deletes a row. It is not enforced by the database: anyone
with the SQLite file can rewrite it. A real deployment enforces that with
append-only storage or an external WORM sink.
