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
//! use mcp_client::{McpClient, McpClientBuilder, transport::HttpTransport};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = McpClientBuilder::new()
//!         .with_transport(HttpTransport::new("http://localhost:8080/mcp")?)
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
//! ### HTTP Transport (Streamable HTTP 2025-03-26+)
//! 
//! ```rust,no_run
//! use mcp_client::transport::HttpTransport;
//! 
//! let transport = HttpTransport::new("http://localhost:8080/mcp")?;
//! ```
//!
//! ### SSE Transport (HTTP+SSE 2024-11-05)
//!
//! ```rust,no_run
//! use mcp_client::transport::SseTransport;
//! 
//! let transport = SseTransport::new("http://localhost:8080/mcp")?;
//! ```
//!
//! ### WebSocket Transport
//!
//! ```rust,no_run
//! use mcp_client::transport::WebSocketTransport;
//! 
//! let transport = WebSocketTransport::new("ws://localhost:8080/mcp")?;
//! ```
//!
//! ### Stdio Transport
//!
//! ```rust,no_run
//! use mcp_client::transport::StdioTransport;
//! 
//! let transport = StdioTransport::new("./mcp-server-executable")?;
//! ```

pub mod client;
pub mod transport;
pub mod session;
pub mod error;
pub mod streaming;
pub mod config;

// Re-export main types
pub use client::{McpClient, McpClientBuilder};
pub use error::{McpClientError, McpClientResult};
pub use session::{SessionManager, SessionInfo, SessionState};
pub use config::{ClientConfig, RetryConfig, TimeoutConfig};

// Re-export transport types
pub use transport::{Transport, TransportType};

// Re-export protocol types for convenience
pub use mcp_protocol_2025_06_18::*;