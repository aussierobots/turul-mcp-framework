//! Per-connection MCP wire-version negotiation for the bilingual client.
//!
//! A single `McpClient` probes the server with `server/discover` at `connect()`
//! and locks one wire spec for its lifetime. [`classify_probe`] is the
//! security-relevant core: a JSON-RPC `-32601` (Method Not Found) is the ONLY
//! signal that downgrades to 2025-11-25. HTTP 4xx and every other JSON-RPC error
//! abort the connect rather than silently downgrade the protocol the caller
//! asked for — a 2026 server behind a broken gateway is not a 2025 server.

use serde::{Deserialize, Serialize};

/// The MCP wire spec a connection speaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum McpVersion {
    /// 2025-11-25 — stateful: `initialize` + `notifications/initialized` + `Mcp-Session-Id`.
    V2025_11_25,
    /// 2026-07-28 — stateless: `server/discover`, per-request `_meta` capability negotiation.
    V2026_07_28,
}

impl McpVersion {
    /// The wire protocol-version string for this spec.
    pub fn as_str(&self) -> &'static str {
        match self {
            McpVersion::V2025_11_25 => "2025-11-25",
            McpVersion::V2026_07_28 => "2026-07-28",
        }
    }
}

impl std::fmt::Display for McpVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Outcome of the `server/discover` probe sent during negotiation.
// Unused in single-spec `client-2025-11-25-only` builds (no probe), hence dead_code there.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DiscoverProbe {
    /// Server returned a valid `DiscoverResult` — it speaks 2026-07-28.
    Discovered,
    /// Server returned a JSON-RPC error response carrying this `error.code`.
    JsonRpcError(i64),
    /// The HTTP request itself failed with this non-2xx status.
    HttpStatus(u16),
}

/// What to do next, given a probe outcome.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ProbeDecision {
    /// Lock the connection to 2026-07-28.
    Use2026,
    /// Send `initialize` and lock the connection to 2025-11-25.
    FallbackTo2025,
    /// Abort negotiation — do NOT downgrade. Carries a human-readable reason.
    Abort(String),
}

/// JSON-RPC "Method not found" — the only error that signals a pre-2026 server
/// (one that lacks the `server/discover` method entirely).
const METHOD_NOT_FOUND: i64 = -32601;

/// Decide the negotiation action from a `server/discover` probe outcome.
///
/// `allow_legacy_gateway_fallback` broadens the fallback trigger to additionally
/// accept HTTP 404/405 — for operators behind gateways that return those codes
/// for unknown methods instead of tunneling the JSON-RPC envelope. It is off by
/// default and weakens protocol-downgrade resistance when enabled.
#[allow(dead_code)]
pub(crate) fn classify_probe(
    probe: DiscoverProbe,
    allow_legacy_gateway_fallback: bool,
) -> ProbeDecision {
    match probe {
        // A valid DiscoverResult is the positive signal for 2026.
        DiscoverProbe::Discovered => ProbeDecision::Use2026,

        // Method Not Found = the server has no `server/discover` → it is older.
        DiscoverProbe::JsonRpcError(METHOD_NOT_FOUND) => ProbeDecision::FallbackTo2025,

        // Any other JSON-RPC error means the server UNDERSTOOD `server/discover`
        // and rejected it for an unrelated reason — not a version signal.
        DiscoverProbe::JsonRpcError(code) => ProbeDecision::Abort(format!(
            "server/discover rejected with JSON-RPC error {code}; the server \
             understood the method and refused it — not a version signal, not a downgrade trigger"
        )),

        // A 2026-07-28 server answers `server/discover` with 200 and needs no
        // session; a 400 means the server bad-requested the method (a 2025-11-25
        // server rejects the sessionless non-initialize request with 400). That is
        // an unambiguous "not stateless 2026" signal, distinct from 401/403 auth or
        // 5xx, so fall back to the 2025 initialize handshake.
        DiscoverProbe::HttpStatus(400) => ProbeDecision::FallbackTo2025,

        // Opt-in escape hatch for gateways that 404/405 unknown methods.
        DiscoverProbe::HttpStatus(status)
            if allow_legacy_gateway_fallback && (status == 404 || status == 405) =>
        {
            ProbeDecision::FallbackTo2025
        }

        // All other HTTP failures are transport/authorization problems, NOT a
        // version signal. Aborting (rather than downgrading) preserves the
        // protocol the caller asked for.
        DiscoverProbe::HttpStatus(status) => ProbeDecision::Abort(format!(
            "server/discover failed with HTTP {status}; transport or authorization \
             failure, not a version signal (no silent downgrade)"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_wire_strings() {
        assert_eq!(McpVersion::V2025_11_25.as_str(), "2025-11-25");
        assert_eq!(McpVersion::V2026_07_28.as_str(), "2026-07-28");
    }

    #[test]
    fn discover_ok_locks_2026() {
        assert_eq!(
            classify_probe(DiscoverProbe::Discovered, false),
            ProbeDecision::Use2026
        );
    }

    #[test]
    fn method_not_found_falls_back_to_2025() {
        assert_eq!(
            classify_probe(DiscoverProbe::JsonRpcError(-32601), false),
            ProbeDecision::FallbackTo2025
        );
    }

    #[test]
    fn other_jsonrpc_errors_abort_without_downgrade() {
        // Parse error, Invalid Request, Invalid Params, Internal Error, and the
        // legacy -32002: the server understood discover — never downgrade.
        for code in [-32700, -32600, -32602, -32603, -32002, 100] {
            assert!(
                matches!(
                    classify_probe(DiscoverProbe::JsonRpcError(code), false),
                    ProbeDecision::Abort(_)
                ),
                "JSON-RPC error {code} must abort, not downgrade"
            );
        }
    }

    #[test]
    fn http_4xx_aborts_by_default_no_downgrade() {
        // 401/403/404/405/429 are transport/auth/gateway signals, not version signals.
        for status in [401, 403, 404, 405, 429] {
            assert!(
                matches!(
                    classify_probe(DiscoverProbe::HttpStatus(status), false),
                    ProbeDecision::Abort(_)
                ),
                "HTTP {status} must abort by default (no silent downgrade)"
            );
        }
    }

    #[test]
    fn http_400_falls_back_to_2025() {
        // A 2026-07-28 server answers server/discover with 200; a 400 means a
        // 2025-11-25 server rejected the sessionless request — fall back to 2025.
        assert_eq!(
            classify_probe(DiscoverProbe::HttpStatus(400), false),
            ProbeDecision::FallbackTo2025
        );
    }

    #[test]
    fn legacy_gateway_hatch_allows_only_404_405_fallback() {
        assert_eq!(
            classify_probe(DiscoverProbe::HttpStatus(404), true),
            ProbeDecision::FallbackTo2025
        );
        assert_eq!(
            classify_probe(DiscoverProbe::HttpStatus(405), true),
            ProbeDecision::FallbackTo2025
        );
        // Auth/server failures still abort even with the hatch enabled (400 is a
        // version signal handled separately — see http_400_falls_back_to_2025).
        for status in [401, 403, 500] {
            assert!(
                matches!(
                    classify_probe(DiscoverProbe::HttpStatus(status), true),
                    ProbeDecision::Abort(_)
                ),
                "HTTP {status} must abort even with the legacy-gateway hatch on"
            );
        }
    }
}
