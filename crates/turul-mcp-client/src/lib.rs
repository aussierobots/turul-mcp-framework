//! # MCP Client Library
//!
//! **Production-ready Rust client for Model Context Protocol (MCP) servers.**
//!
//! Connect to MCP servers with full protocol compliance, multiple transport options,
//! and automatic session management. Supports both synchronous and streaming operations
//! with comprehensive error handling and recovery mechanisms.
//!
//! [![Crates.io](https://img.shields.io/crates/v/turul-mcp-client.svg)](https://crates.io/crates/turul-mcp-client)
//! [![Documentation](https://docs.rs/turul-mcp-client/badge.svg)](https://docs.rs/turul-mcp-client)
//! [![License](https://img.shields.io/crates/l/turul-mcp-client.svg)](https://github.com/aussierobots/turul-mcp-framework/blob/main/LICENSE)
//!
//! ## Features
//!
//! - **Multi-transport**: HTTP and Server-Sent Events (SSE), with stdio planned
//! - **Bilingual Protocol**: Speaks both MCP 2026-07-28 (stateless core) and
//!   2025-11-25; by default the client negotiates the spec per connection
//! - **High Performance**: Built on Tokio with async/await throughout
//! - **Session Management**: Automatic connection handling and recovery
//! - **Real-time Streaming**: SSE support for progress and notifications
//! - **Error Handling**: Comprehensive error types with automatic retry
//! - **Configurable**: Timeouts, retries, connection pooling
//!
//! ## Installation
//!
//! ```toml
//! [dependencies]
//! turul-mcp-client = "0.3"
//! tokio = { version = "1.0", features = ["full"] }
//! ```
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use turul_mcp_client::{McpClient, McpClientBuilder, transport::HttpTransport};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let transport = HttpTransport::new("http://localhost:8080/mcp")?;
//!     let client = McpClientBuilder::new()
//!         .with_transport(Box::new(transport))
//!         .build();
//!
//!     client.connect().await?;
//!
//!     let tools = client.list_tools().await?;
//!     println!("Available tools: {:?}", tools);
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Transport Types
//!
//! ### HTTP Transport (Streamable HTTP)
//!
//! The Streamable HTTP transport sends each MCP request as an independent HTTP
//! POST. There is no persistent connection. The client negotiates the spec per
//! connection: on a 2026-07-28 connection it is stateless (no session id; each
//! request carries `_meta` and the `MCP-Protocol-Version: 2026-07-28` header). On
//! a connection locked to 2025-11-25, session continuity is maintained via the
//! `Mcp-Session-Id` header the server returns during initialization and the client
//! includes on subsequent requests.
//!
//! `transport.connect()` only marks the transport as logically ready; it performs
//! no network I/O. The first real validation happens when `McpClient::connect()`
//! probes `server/discover`. On a 2026-07-28 server discovery answers statelessly;
//! on a 2025-locked connection the client falls back to the `initialize` POST
//! followed by `notifications/initialized`. If the server is unreachable or rejects
//! the probe, the error surfaces there.
//!
//! ```rust,no_run
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use turul_mcp_client::transport::HttpTransport;
//!
//! let transport = HttpTransport::new("http://localhost:8080/mcp")?;
//! # Ok(())
//! # }
//! ```
//!
//! ### SSE Transport (HTTP+SSE, legacy)
//!
//! For servers using the pre-2025-03-26 SSE-based protocol. Like the HTTP
//! transport, `connect()` is a no-op marker — the SSE subscription is
//! established lazily during message exchange.
//!
//! ```rust,no_run
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use turul_mcp_client::transport::SseTransport;
//!
//! let transport = SseTransport::new("http://localhost:8080/mcp")?;
//! # Ok(())
//! # }
//! ```
//!
//! ### Future Transports
//!
//! Stdio transports are planned for future releases:
//!
//! ```text
//! // Coming soon:
//! // StdioTransport::new("./mcp-server-executable")
//! ```
//!
//! ## Common Operations
//!
//! ### Tool Execution
//!
//! ```rust,no_run
//! # use turul_mcp_client::prelude::*;
//! # async fn example(client: &McpClient) -> Result<(), Box<dyn std::error::Error>> {
//! // List available tools
//! let tools = client.list_tools().await?;
//! println!("Available tools: {:?}", tools);
//!
//! // Execute a tool
//! let result = client.call_tool("calculator", serde_json::json!({
//!     "operation": "add",
//!     "a": 5,
//!     "b": 3
//! })).await?;
//! println!("Result: {:?}", result);
//! # Ok(())
//! # }
//! ```
//!
//! ### Resource Access
//!
//! ```rust,no_run
//! # use turul_mcp_client::prelude::*;
//! # async fn example(client: &McpClient) -> Result<(), Box<dyn std::error::Error>> {
//! // List available resources
//! let resources = client.list_resources().await?;
//!
//! // Discover dynamic URI templates
//! let templates = client.list_resource_templates().await?;
//!
//! // Read a specific resource
//! let content = client.read_resource("file://config.json").await?;
//! println!("Resource content: {:?}", content);
//! # Ok(())
//! # }
//! ```
//!
//! ### Prompt Templates
//!
//! ```rust,no_run
//! # use turul_mcp_client::prelude::*;
//! # async fn example(client: &McpClient) -> Result<(), Box<dyn std::error::Error>> {
//! // List available prompts
//! let prompts = client.list_prompts().await?;
//!
//! // Get a prompt with arguments
//! let prompt = client.get_prompt("code_review", Some(serde_json::json!({
//!     "language": "rust",
//!     "code": "fn main() { println!(\"Hello!\"); }"
//! }))).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Configuration
//!
//! The client supports extensive configuration:
//!
//! ```rust,no_run
//! # use turul_mcp_client::prelude::*;
//!
//! // Create a client with default configuration
//! let client = McpClientBuilder::new()
//!     .build();
//! ```
//!
//! ## Real-time Streaming
//!
//! For real-time notifications and progress updates:
//!
//! ```rust,no_run
//! # use turul_mcp_client::prelude::*;
//! # async fn example(client: &McpClient) -> Result<(), Box<dyn std::error::Error>> {
//! // Get available tools from server
//! let tools = client.list_tools().await?;
//! println!("Available tools: {}", tools.len());
//! # Ok(())
//! # }
//! ```
//!
//! ## Examples
//!
//! **Complete examples available at:**
//! [github.com/aussierobots/turul-mcp-framework/tree/main/examples](https://github.com/aussierobots/turul-mcp-framework/tree/main/examples)
//!
//! - **Basic Client** - Simple tool execution
//! - **Streaming Client** - Real-time notifications
//! - **HTTP Client** - Production HTTP integration
//! - **Retry Logic** - Error handling and recovery
//! - **Monitoring** - Connection health and metrics
//!
//! ## Related Crates
//!
//! - [`turul-mcp-server`](https://crates.io/crates/turul-mcp-server) - Build MCP servers
//! - [`turul-mcp-protocol`](https://crates.io/crates/turul-mcp-protocol) - Protocol types
//! - [`turul-mcp-derive`](https://crates.io/crates/turul-mcp-derive) - Macros for tools/resources

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

pub mod client;
pub mod config;
pub mod error;
pub mod prelude;
pub(crate) mod protocol;
pub mod session;
pub mod streaming;
pub mod transport;
pub mod version;

// Re-export main types
/// High-level MCP client with session management and automatic reconnection
pub use client::{McpClient, McpClientBuilder, NotificationCallback, ToolCallResponse};
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
