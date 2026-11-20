//! Client for [Model Context Protocol](https://modelcontextprotocol.io) servers:
//! connect, list and call tools, read resources, fetch prompts.
//!
//! The client speaks both MCP 2026-07-28 and 2025-11-25 and negotiates the spec
//! per connection. To build a server rather than call one, use
//! [`turul-mcp-server`](https://docs.rs/turul-mcp-server).
//!
//! # Example
//!
//! ```toml
//! [dependencies]
//! turul-mcp-client = "0.4"
//! tokio = { version = "1", features = ["full"] }
//! serde_json = "1"
//! ```
//!
//! ```rust,no_run
//! use turul_mcp_client::{McpClientBuilder, transport::HttpTransport};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = McpClientBuilder::new()
//!         .with_transport(Box::new(HttpTransport::new("http://127.0.0.1:8000/mcp")?))
//!         .build();
//!     client.connect().await?;
//!
//!     let tools = client.list_tools().await?;
//!     println!("{} tools", tools.len());
//!
//!     let result = client
//!         .call_tool("add", serde_json::json!({ "a": 5, "b": 3 }))
//!         .await?;
//!     println!("{result:?}");
//!     Ok(())
//! }
//! ```
//!
//! [`transport::HttpTransport`] sends each request as an independent Streamable HTTP
//! POST. `connect()` probes `server/discover`; a 2025-11-25 server is detected there
//! and handled with the `initialize` handshake. See the [`transport`] module for the
//! other transports.
//!
//! # Features
//!
//! | Feature | Default | Enables |
//! |---|---|---|
//! | `http` | yes | HTTP transport |
//! | `sse` | yes | Server-sent event streams |
//! | `client-bilingual` | yes | Both specs, negotiated at connect |
//! | `client-2026-07-28-only` | no | Only MCP 2026-07-28; use with `--no-default-features` |
//! | `client-2025-11-25-only` | no | Only MCP 2025-11-25; use with `--no-default-features` |
//! | `ext-tasks` | no | The `io.modelcontextprotocol/tasks` extension (2026-07-28 connections) |
//! | `ext-apps` | no | MCP Apps (`io.modelcontextprotocol/ui`) declaration and `_meta.ui` accessors; no rendering |
//!
//! # Where next
//!
//! - [`McpClient`] for resources, prompts and every other operation
//! - [`transport`] for transports
//! - [`turul-mcp-server`](https://docs.rs/turul-mcp-server) to build servers
//! - [Examples](https://github.com/aussierobots/turul-mcp-framework/tree/main/examples)

#![cfg_attr(docsrs, feature(doc_cfg))]
// Spec-coexistence features are mutually exclusive. Bilingual (default) links
// both protocol crates; narrowing to one requires --no-default-features.
#[cfg(any(
    all(feature = "client-bilingual", feature = "client-2025-11-25-only"),
    all(feature = "client-bilingual", feature = "client-2026-07-28-only"),
    all(feature = "client-2025-11-25-only", feature = "client-2026-07-28-only"),
))]
compile_error!(
    "turul-mcp-client: `client-bilingual` (default), `client-2025-11-25-only`, and \
     `client-2026-07-28-only` are mutually exclusive. To narrow:  \
     `cargo build --no-default-features --features http,sse,client-2025-11-25-only`. \
     Bilingual is the default — a bare `cargo build` is enough."
);

#[cfg(not(any(
    feature = "client-bilingual",
    feature = "client-2025-11-25-only",
    feature = "client-2026-07-28-only"
)))]
compile_error!(
    "turul-mcp-client: enable exactly one of `client-bilingual` (default), \
     `client-2025-11-25-only`, or `client-2026-07-28-only`. If you used \
     `--no-default-features`, add one explicitly."
);

// wasm32 (ADR-040): wasm-bindgen hosts only, 2026-07-28 HTTP only.
#[cfg(all(target_arch = "wasm32", not(target_os = "unknown")))]
compile_error!(
    "turul-mcp-client: on wasm32 only `wasm32-unknown-unknown` (a wasm-bindgen host: \
     browser or Node) is supported; see ADR-040."
);

#[cfg(all(
    target_arch = "wasm32",
    any(feature = "client-bilingual", feature = "client-2025-11-25-only")
))]
compile_error!(
    "turul-mcp-client: on wasm32 only the 2026-07-28 lane is supported (ADR-040). Use \
     `default-features = false, features = [\"http\", \"client-2026-07-28-only\"]`."
);

#[cfg(all(target_arch = "wasm32", feature = "oauth-client-credentials"))]
compile_error!(
    "turul-mcp-client: `oauth-client-credentials` is not supported on wasm32 (ADR-040): \
     its token source needs a redirect policy and timeout that fetch cannot express."
);

pub mod client;
pub mod config;
pub mod error;
#[cfg(feature = "ext-apps")]
pub mod ext_apps;
#[cfg(feature = "oauth-client-credentials")]
pub mod oauth_client_credentials;
pub mod prelude;
pub(crate) mod protocol;
mod rt;
pub mod session;
pub mod streaming;
pub mod transport;
pub mod version;

// Re-export main types
#[cfg(feature = "ext-tasks")]
pub use client::ToolCallOutcome;
/// High-level MCP client with session management and automatic reconnection
pub use client::{
    McpClient, McpClientBuilder, NotificationCallback, SubscriptionStream, ToolCallResponse,
};
/// Client configuration types for timeouts, retries, and connection parameters
pub use config::{ClientConfig, RetryConfig, TimeoutConfig};
/// Client-specific error types and result aliases for error handling
pub use error::{McpClientError, McpClientResult};
/// Session management types for tracking connection state and statistics
pub use session::{SessionInfo, SessionManager, SessionState};
/// Per-connection MCP wire-version negotiation
pub use version::McpVersion;

// Re-export transport types
/// Transport layer abstractions for different MCP connection types
pub use transport::{Transport, TransportType};

// Re-export protocol types for convenience
/// Core MCP protocol types and message structures
pub use turul_mcp_protocol_2025_11_25::*;
