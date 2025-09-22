//! # MCP Client Library
//!
//! A comprehensive Model Context Protocol (MCP) client library with support for multiple
//! transport layers and protocol versions. This library provides both high-level and
//! low-level APIs for interacting with MCP servers.
//!
//! ## Features
//!
//! - **Multi-transport support**: HTTP, SSE, WebSocket, and stdio
//! - **Protocol compliance**: Full MCP 2025-06-18 specification support
//! - **Async/await**: Built on Tokio for high performance
//! - **Session management**: Automatic session handling and recovery
//! - **Streaming support**: Real-time event streaming and progress tracking
//! - **Error handling**: Comprehensive error types and recovery mechanisms
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

pub mod client;
pub mod config;
pub mod error;
pub mod session;
pub mod streaming;
pub mod transport;

// Re-export main types
pub use client::{McpClient, McpClientBuilder};
pub use config::{ClientConfig, RetryConfig, TimeoutConfig};
pub use error::{McpClientError, McpClientResult};
pub use session::{SessionInfo, SessionManager, SessionState};

// Re-export transport types
pub use transport::{Transport, TransportType};

// Re-export protocol types for convenience
pub use turul_mcp_protocol::*;
