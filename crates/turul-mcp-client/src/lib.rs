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
//! - **Multi-transport**: HTTP, Server-Sent Events (SSE), WebSocket (planned), stdio (planned)
//! - **Full Protocol**: Complete MCP 2025-11-25 specification support
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
//! turul-mcp-client = "0.2"
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
//! ```rust,no_run
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use turul_mcp_client::transport::HttpTransport;
//!
//! let transport = HttpTransport::new("http://localhost:8080/mcp")?;
//! # Ok(())
//! # }
//! ```
//!
//! ### SSE Transport (HTTP+SSE)
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
//! ### SSE Transport (Real-time)
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
//! WebSocket and Stdio transports are planned for future releases:
//!
//! ```text
//! // Coming soon:
//! // WebSocketTransport::new("ws://localhost:8080/mcp")
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

pub mod client;
pub mod config;
pub mod error;
pub mod prelude;
pub mod session;
pub mod streaming;
pub mod transport;

// Re-export main types
/// High-level MCP client with session management and automatic reconnection
pub use client::{McpClient, McpClientBuilder};
/// Client configuration types for timeouts, retries, and connection parameters
pub use config::{ClientConfig, RetryConfig, TimeoutConfig};
/// Client-specific error types and result aliases for error handling
pub use error::{McpClientError, McpClientResult};
/// Session management types for tracking connection state and statistics
pub use session::{SessionInfo, SessionManager, SessionState};

// Re-export transport types
/// Transport layer abstractions for different MCP connection types
pub use transport::{Transport, TransportType};

// Re-export protocol types for convenience
/// Core MCP protocol types and message structures
pub use turul_mcp_protocol::*;
