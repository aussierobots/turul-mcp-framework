//! Regression test: reqwest's `http2` Cargo feature is compiled into `turul-mcp-client`.
//!
//! When `reqwest/http2` is enabled AND a TLS backend with ALPN support is linked (we use
//! `rustls` via `turul-mcp-oauth`), reqwest automatically negotiates HTTP/2 with servers
//! that advertise `h2` via ALPN — collapsing N concurrent requests onto a single TLS
//! connection. Backends that only speak HTTP/1.1 fall back to h1 via ALPN; no breakage.
//!
//! This test does NOT verify wire-level h2 negotiation against a live server — that is
//! validated end-to-end by downstream consumers. What this test DOES verify is that the
//! `http2` Cargo feature is actually compiled in. If the feature is accidentally disabled
//! by a future `Cargo.toml` edit, this test fails to compile.
//!
//! See issue #13 for the full investigation and v0.3.36 release notes.

use reqwest::Client;

/// `reqwest::ClientBuilder::http2_prior_knowledge` is gated by
/// `#[cfg(feature = "http2")]` in reqwest's source. Its presence in our build is proof
/// that the feature is enabled — same code path that enables ALPN-negotiated h2 in
/// production (we do NOT call this method in production; that would force plaintext h2
/// and break h1-only backends).
#[test]
fn http2_feature_compiled_in() {
    // If `reqwest/http2` is disabled, `http2_prior_knowledge` does not exist and this
    // line fails to compile. That is the regression signal we want.
    let _client = Client::builder()
        .http2_prior_knowledge()
        .build()
        .expect("client with http2 prior knowledge should build");
}

/// Sanity: default client (no prior-knowledge) still builds. This is the code path
/// actually used in production via `HttpTransport::with_config` — ALPN chooses between
/// h1 and h2 based on server advertisement.
#[test]
fn default_client_with_alpn_negotiation_builds() {
    let _client = Client::builder()
        .build()
        .expect("default reqwest client should build");
}
