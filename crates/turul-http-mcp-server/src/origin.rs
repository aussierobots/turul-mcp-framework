//! Origin-header validation (DNS-rebinding protection) for the MCP endpoint.
//!
//! Streamable HTTP §Security: "Servers MUST validate the `Origin` header on
//! all incoming connections to prevent DNS rebinding attacks. If the
//! `Origin` header is present and invalid, servers MUST respond with
//! HTTP 403 Forbidden."

use hyper::HeaderMap;
use std::net::{Ipv4Addr, Ipv6Addr};

/// Validation policy for the `Origin` request header on the MCP endpoint.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum OriginPolicy {
    /// Default. Origin absent → allowed. Origin present → allowed only if
    /// its host is loopback (`localhost`, `127.0.0.0/8`, `[::1]`). Anything
    /// else → HTTP 403.
    ///
    /// The request's `Host` header is **not** consulted: it is
    /// attacker-controlled, and a rebinding attacker sets it to agree with
    /// `Origin`. To serve a browser app from a non-loopback origin, name that
    /// origin with [`OriginPolicy::AllowList`].
    #[default]
    SameOriginOrLoopback,
    /// [`OriginPolicy::SameOriginOrLoopback`] semantics plus an explicit
    /// allowlist of origins (`scheme://host[:port]`). An entry matches when the
    /// scheme, host and port are all equal (RFC 6454 §4-5): scheme and host
    /// compare case-insensitively, and a missing port is the scheme's default.
    /// So `https://app.example` does not admit `http://app.example`. The
    /// literal entry `"null"` admits `Origin: null`.
    AllowList(Vec<String>),
    /// No validation — for deployments that enforce origin upstream
    /// (API Gateway / ALB / reverse proxy) or are not browser-reachable.
    Disabled,
}

/// `(scheme_lowercase, host_lowercase, effective_port)`: the origin triple of
/// RFC 6454 §4-5.
type OriginAuthority = (String, String, u16);

fn default_port(scheme: &str) -> Option<u16> {
    match scheme {
        "http" | "ws" => Some(80),
        "https" | "wss" => Some(443),
        _ => None,
    }
}

/// Parses `scheme://host[:port]` into a normalized authority.
fn parse_origin(origin: &str) -> Option<OriginAuthority> {
    let (scheme, rest) = origin.split_once("://")?;
    let scheme = scheme.to_ascii_lowercase();
    // An origin has no path/query, but be lenient about a trailing slash.
    let authority = rest.strip_suffix('/').unwrap_or(rest);
    if authority.is_empty() {
        return None;
    }
    let (host, port) = split_host_port(authority)?;
    let port = match port {
        Some(p) => p,
        None => default_port(&scheme)?,
    };
    Some((scheme, host, port))
}

/// Splits `host[:port]`, handling bracketed IPv6 (`[::1]:8080`).
fn split_host_port(authority: &str) -> Option<(String, Option<u16>)> {
    if let Some(rest) = authority.strip_prefix('[') {
        let (host, after) = rest.split_once(']')?;
        let port = match after.strip_prefix(':') {
            Some(p) => Some(p.parse().ok()?),
            None if after.is_empty() => None,
            None => return None,
        };
        Some((host.to_ascii_lowercase(), port))
    } else if let Some((host, p)) = authority.rsplit_once(':') {
        if host.is_empty() {
            return None;
        }
        Some((host.to_ascii_lowercase(), Some(p.parse().ok()?)))
    } else {
        Some((authority.to_ascii_lowercase(), None))
    }
}

fn is_loopback_host(host: &str) -> bool {
    if host == "localhost" {
        return true;
    }
    if let Ok(v4) = host.parse::<Ipv4Addr>() {
        return v4.is_loopback();
    }
    if let Ok(v6) = host.parse::<Ipv6Addr>() {
        return v6.is_loopback();
    }
    false
}

/// Validates the request's `Origin` header against `policy`.
///
/// `Ok(())` admits the request; `Err(origin_value)` means the caller MUST
/// respond 403 Forbidden.
pub(crate) fn validate_origin(headers: &HeaderMap, policy: &OriginPolicy) -> Result<(), String> {
    if matches!(policy, OriginPolicy::Disabled) {
        return Ok(());
    }
    let Some(origin) = headers.get(hyper::header::ORIGIN) else {
        return Ok(()); // spec constrains only "present and invalid"
    };
    let Ok(origin) = origin.to_str() else {
        return Err("<non-ascii>".to_string());
    };

    if let OriginPolicy::AllowList(allowed) = policy
        && allowed.iter().any(|a| {
            a == origin
                || matches!(
                    (parse_origin(a), parse_origin(origin)),
                    (Some(x), Some(y)) if x == y
                )
        })
    {
        return Ok(());
    }

    let Some(parsed) = parse_origin(origin) else {
        return Err(origin.to_string()); // includes `Origin: null`
    };
    if is_loopback_host(&parsed.1) {
        return Ok(());
    }
    // Deliberately NOT compared against the request's `Host` header. `Host` is
    // attacker-controlled, and in a DNS-rebinding attack the browser sends
    // `Host` == the attacker's own name (the URL host, rebound to loopback),
    // so `Origin` and `Host` always agree and the check would never fire —
    // admitting exactly the attack this module exists to stop. A legitimate
    // same-origin deployment and a rebinding attack are indistinguishable from
    // these two headers alone, so only server-side knowledge of the expected
    // origin can decide: operators declare it with `OriginPolicy::AllowList`.
    Err(origin.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::header::{HOST, ORIGIN};

    fn headers(origin: Option<&str>, host: Option<&str>) -> HeaderMap {
        let mut h = HeaderMap::new();
        if let Some(o) = origin {
            h.insert(ORIGIN, o.parse().unwrap());
        }
        if let Some(hh) = host {
            h.insert(HOST, hh.parse().unwrap());
        }
        h
    }

    #[test]
    fn absent_origin_is_allowed() {
        let p = OriginPolicy::SameOriginOrLoopback;
        assert!(validate_origin(&headers(None, Some("example.com")), &p).is_ok());
    }

    #[test]
    fn loopback_origins_pass() {
        let p = OriginPolicy::SameOriginOrLoopback;
        for o in [
            "http://localhost",
            "http://localhost:3000",
            "http://127.0.0.1:9999",
            "http://127.8.4.2",
            "http://[::1]:8080",
            "https://LOCALHOST:8443",
        ] {
            assert!(
                validate_origin(&headers(Some(o), Some("example.com")), &p).is_ok(),
                "{o} should pass"
            );
        }
    }

    /// A matching `Host` header MUST NOT admit a non-loopback origin.
    ///
    /// This is the DNS-rebinding case itself: the attacker controls both
    /// headers and sets them consistently, so any rule that trusts their
    /// agreement admits the attack (`Host` + `Origin` both `evil.example.com`
    /// must not yield 200).
    #[test]
    fn matching_host_header_does_not_admit_a_foreign_origin() {
        let p = OriginPolicy::SameOriginOrLoopback;
        for (o, host) in [
            ("http://evil.example.com", "evil.example.com"),
            ("http://app.example:8080", "app.example:8080"),
            ("http://app.example", "app.example"), // 80 vs portless
            ("https://app.example", "app.example"), // 443 vs portless
            ("https://APP.example:443", "app.example:443"),
        ] {
            assert!(
                validate_origin(&headers(Some(o), Some(host)), &p).is_err(),
                "{o} vs matching Host {host} must be rejected — Host is attacker-controlled"
            );
        }
    }

    /// The supported way to serve a browser app from a non-loopback origin.
    #[test]
    fn same_origin_on_a_public_host_is_reachable_via_allowlist() {
        let p = OriginPolicy::AllowList(vec!["https://app.example".into()]);
        assert!(
            validate_origin(
                &headers(Some("https://app.example"), Some("app.example")),
                &p
            )
            .is_ok()
        );
        assert!(
            validate_origin(
                &headers(Some("https://evil.example"), Some("evil.example")),
                &p
            )
            .is_err()
        );
    }

    #[test]
    fn cross_origin_null_and_garbage_are_rejected() {
        let p = OriginPolicy::SameOriginOrLoopback;
        for (o, host) in [
            ("http://attacker.example", "127.0.0.1:8641"),
            ("http://app.example:9000", "app.example:8080"), // port mismatch
            ("null", "127.0.0.1:8641"),
            ("not a url", "127.0.0.1:8641"),
        ] {
            assert!(
                validate_origin(&headers(Some(o), Some(host)), &p).is_err(),
                "{o} vs Host {host} should be rejected"
            );
        }
    }

    #[test]
    fn allowlist_is_additive_and_port_normalized() {
        let p = OriginPolicy::AllowList(vec!["https://app.example".into(), "null".into()]);
        let host = Some("127.0.0.1:8641");
        assert!(validate_origin(&headers(Some("https://app.example"), host), &p).is_ok());
        assert!(validate_origin(&headers(Some("https://app.example:443"), host), &p).is_ok());
        assert!(validate_origin(&headers(Some("null"), host), &p).is_ok());
        // additive: loopback still passes
        assert!(validate_origin(&headers(Some("http://localhost:3000"), host), &p).is_ok());
        // unlisted still rejected
        assert!(validate_origin(&headers(Some("https://other.example"), host), &p).is_err());
    }

    /// An origin is the triple (scheme, host, port) (RFC 6454 §4-5): a listed
    /// `https://` origin does not admit the `http://` page on the same host and
    /// port, which a network attacker can inject script into.
    #[test]
    fn allowlist_compares_the_scheme() {
        let p = OriginPolicy::AllowList(vec!["https://h.example:8650".into()]);
        let host = Some("h.example:8649");
        assert!(validate_origin(&headers(Some("https://h.example:8650"), host), &p).is_ok());
        assert!(validate_origin(&headers(Some("HTTPS://H.example:8650"), host), &p).is_ok());
        assert!(validate_origin(&headers(Some("http://h.example:8650"), host), &p).is_err());
        assert!(validate_origin(&headers(Some("wss://h.example:8650"), host), &p).is_err());

        // Default-port normalisation is per scheme: `https://h` is port 443, and
        // `http://h:443` is still a different origin.
        let p = OriginPolicy::AllowList(vec!["https://h.example".into()]);
        assert!(validate_origin(&headers(Some("https://h.example:443"), host), &p).is_ok());
        assert!(validate_origin(&headers(Some("http://h.example:443"), host), &p).is_err());
        assert!(validate_origin(&headers(Some("http://h.example"), host), &p).is_err());

        // Loopback stays host-only, and `null` stays exact-match only.
        assert!(validate_origin(&headers(Some("http://localhost:8650"), host), &p).is_ok());
        assert!(validate_origin(&headers(Some("null"), host), &p).is_err());
    }

    #[test]
    fn disabled_skips_everything() {
        let p = OriginPolicy::Disabled;
        assert!(validate_origin(&headers(Some("http://attacker.example"), None), &p).is_ok());
    }
}
