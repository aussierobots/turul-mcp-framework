//! Shared storage for the tool-list state of a dynamic MCP server.
//!
//! With the `dynamic-tools` feature of `turul-mcp-server`, tools can be turned
//! on and off while the server runs. This crate stores which tools are active
//! and a fingerprint of the tool set, so several instances (for example AWS
//! Lambda invocations) agree on one tool list. It is not session storage: MCP
//! 2026-07-28 is stateless and keeps nothing per client.
//!
//! Most users want [`turul-mcp-server`](https://docs.rs/turul-mcp-server), which
//! takes a backend through
//! `McpServerBuilder::server_state_storage`. Without one, an in-memory backend
//! is used, which suits a single process.
//!
//! # Example
//!
//! The storage API on its own:
//!
//! ```toml
//! [dependencies]
//! turul-mcp-server-state-storage = "0.4"
//! tokio = { version = "1", features = ["full"] }
//! ```
//!
//! ```rust
//! use turul_mcp_server_state_storage::{InMemoryServerStateStorage, ServerStateStorage};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let storage = InMemoryServerStateStorage::new();
//! storage.set_fingerprint("tools", "abc123".to_string()).await?;
//! assert_eq!(storage.get_fingerprint("tools").await?.as_deref(), Some("abc123"));
//! # Ok(())
//! # }
//! ```
//!
//! # Features
//!
//! | Feature | Default | Backend |
//! |---|---|---|
//! | `in-memory` | yes | [`InMemoryServerStateStorage`]: one process only |
//! | `sqlite` | no | `SqliteServerStateStorage`: a local file |
//! | `postgres` | no | `PostgresServerStateStorage`: shared across instances |
//! | `dynamodb` | no | `DynamoDbServerStateStorage`: AWS serverless |
//!
//! # Where next
//!
//! - [`ServerStateStorage`] for the trait a custom backend implements
//! - [`turul-mcp-server`](https://docs.rs/turul-mcp-server) and its `dynamic-tools` feature
//! - [`dynamic-tools-server`](https://github.com/aussierobots/turul-mcp-framework/tree/main/examples/dynamic-tools-server) example

pub mod error;
pub mod traits;

#[cfg(feature = "in-memory")]
pub mod in_memory;

#[cfg(feature = "sqlite")]
pub mod sqlite;

#[cfg(feature = "postgres")]
pub mod postgres;

#[cfg(feature = "dynamodb")]
pub mod dynamodb;

// Re-exports
pub use error::ServerStateError;
pub use traits::{BoxedServerStateStorage, EntityState, RegistrySnapshot, ServerStateStorage};

#[cfg(feature = "in-memory")]
pub use in_memory::InMemoryServerStateStorage;

#[cfg(feature = "sqlite")]
pub use sqlite::{SqliteServerStateConfig, SqliteServerStateStorage};

#[cfg(feature = "postgres")]
pub use postgres::{PostgresServerStateConfig, PostgresServerStateStorage};

#[cfg(feature = "dynamodb")]
pub use dynamodb::{DynamoDbServerStateConfig, DynamoDbServerStateStorage};
