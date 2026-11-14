# ADR-034: Pre-bound listener for the HTTP transport

**Status:** Accepted
**Date:** 2026-11-14
**Crates:** `turul-http-mcp-server` (owner of `PreboundListener` and the bind step), `turul-mcp-server` (builder passthrough)
**Related:** ADR-031 (builder passthrough pattern)

## Context

`HttpMcpServer::run` binds `bind_address` itself. A caller that needs a free
port, which every test that starts a server does, could only ask the kernel
for one, drop that listener and pass the port number in. The port is free
again from the drop until the server binds it, and anything else on the
machine can take it in that gap: another test in the same binary, or another
test process. The loser then fails with "Address already in use", or its
requests reach a different server. Both happened on 2026-11-14 while three
sessions ran the gates on one machine (`mcp_behavioral_compliance`,
`test_rate_limiting_enforced_for_sessionless_ping` and a connection reset).

Two partial fixes existed and are superseded:

- `reserve_port()` in `crates/turul-mcp-server/tests/common/mod.rs`
  serialised the window with a process-wide mutex. That stops collisions
  inside one test binary only.
- Most suites outside that crate bound `127.0.0.1:0`, dropped the listener
  and passed the address, with no protection at all.

## Decision

The HTTP transport can serve on a listener the caller has already bound.

- `turul_http_mcp_server::PreboundListener` is the one type that carries a
  bound `std::net::TcpListener` into a server. `HttpMcpServer::run` takes it
  (once) and serves on it; without one, it binds `bind_address` as before.
- `HttpMcpServerBuilder::listener(std::net::TcpListener)` and
  `McpServerBuilder::listener(std::net::TcpListener)` are the entry points.
  Each sets `bind_address` to the listener's `local_addr()`, so logs,
  `bind_address` readers and URLs built from it agree with the socket.
- Callers that need a free port bind `127.0.0.1:0`, read `local_addr()`, and
  hand the same listener to the builder. The port is never released, so no
  other process or test can take it.

Every in-process test and example that started a server on a dropped port
moves to `.listener(...)` in the same slice, and `reserve_port()` with its
mutex is deleted.

A port handed to a **child process** cannot use `.listener(...)`, because a
listener cannot cross the process boundary. The harness starts such children
with `--port 0` instead; the child binds the port itself and prints
`MCP_LISTEN_ADDR=<addr>` on stdout right after binding, and the harness reads
that line (`TestServerManager` in `tests/shared/src/e2e_utils.rs`, and the
`minimal-server` child in `tests/client_server_upgrade_e2e.rs`). The only
remaining release-then-bind site is
`crates/turul-mcp-client/tests/legacy_0_3_47_discover_probe.rs`, which runs the
frozen 0.3.47 server; that crate is frozen and has no `.listener`.

Rejected: a retry loop on bind failure (hides the race instead of removing
it, and a retry can still land on another server); a cross-process lock file
(new infrastructure for a problem the socket API already solves).

## Consequences

- No release-then-rebind gap remains for callers that use `.listener(...)`.
- `run` consumes the listener: a second `run` on the same server binds
  `bind_address`, which is the address the first run already holds, and
  fails with "Address already in use". That is the correct outcome.
- Lambda is unaffected; it never binds.

## Tests

- `prebound_listener_serves_on_the_given_socket` (`turul-http-mcp-server`):
  the server answers on the listener's port, and `bind_address` reports it.
- `mcp_server_listener_serves_on_the_given_socket` (`turul-mcp-server`): the
  same through `McpServerBuilder::listener`.

## Revision log

- **2026-11-14**: Initial, Proposed.
- **2026-11-14**: Accepted. 45 server and client test files, 9 integration test files and 6 examples migrated; `reserve_port`, `ReservedPort` and `PORT_HANDOFF` in the server test support, and `crates/turul-mcp-server/tests/port_handoff.rs`, deleted. Child-process sites recorded as out of scope.
- **2026-11-14**: Child processes covered: the nine harness-started examples bind `--port 0` themselves and announce `MCP_LISTEN_ADDR=`; `find_available_port`, the harness `PORT_HANDOFF` lock, its start retry and `free_port()` are deleted.
