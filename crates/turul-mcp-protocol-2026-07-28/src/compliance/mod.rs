//! Wire-format compliance harness against upstream MCP example fixtures.
//!
//! Three sub-modules:
//! - [`fetch`] — shallow sparse `git clone` of the pinned upstream tree, with
//!   an idempotent local cache.
//! - [`coverage`] — the table of 86 upstream example directories mapped to
//!   their Rust binding (or [`coverage::Kind::NotModeled`] for entries we
//!   haven't bound yet).
//! - [`roundtrip`] — parse / re-serialize / semantic-diff each modeled case
//!   against every `*.json` fixture in its directory, returning a structured
//!   [`roundtrip::Report`] consumed by both the test harness and the CLI.

pub mod coverage;
pub mod fetch;
pub mod roundtrip;
