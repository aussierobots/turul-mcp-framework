//! **Experimental.** MCP Server Cards (SEP-2127): a static JSON document that
//! describes a remote MCP server for discovery before connecting. Upstream
//! marks the extension experimental and not yet accepted; this crate follows
//! `ext-server-card` at `526201b` on its own 0.1.x line.
//!
//! The crate holds the wire types ([`v1::ServerCard`] and friends), the
//! schema's validation rules ([`v1::ServerCard::validate`]) and the served
//! document ([`v1::ServerCardDocument`]: bytes and `ETag`). Serving the card is
//! `turul-mcp-server`'s `ext-server-card` feature (ADR-038).
//!
//! ```rust
//! use turul_mcp_ext_server_card::v1::{Remote, RemoteType, ServerCard, ServerCardDocument};
//!
//! let card = ServerCard::new("com.example/weather", "Weather forecasts by city")
//!     .with_version("1.2.0")
//!     .with_remote(Remote::new(RemoteType::StreamableHttp, "https://mcp.example.com/mcp"));
//! let doc = ServerCardDocument::new(&card).unwrap();
//! assert!(doc.etag().starts_with('"'));
//! ```

pub mod v1;
