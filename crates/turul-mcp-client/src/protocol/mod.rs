//! Version-routed request building and response parsing for client operations.
//!
//! `connect()` locks one [`McpVersion`](crate::version::McpVersion) per
//! connection (see `version.rs`); operation methods on `McpClient` dispatch to
//! the matching module here. The 2025-11-25 path is the historical alias-based
//! serialization inline in `client.rs`; the 2026-07-28 path lives in [`v2026_07_28`].

#[cfg(any(feature = "client-bilingual", feature = "client-2026-07-28-only"))]
pub(crate) mod v2026_07_28;
