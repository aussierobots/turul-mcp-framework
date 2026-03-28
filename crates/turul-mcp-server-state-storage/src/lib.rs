//! Server-Global State Storage for MCP Entity Registries
//!
//! Provides pluggable persistence for server-global entity activation state
//! (tools, resources, prompts) across different backends.
//!
//! This crate follows the same pattern as `turul-mcp-session-storage` and
//! `turul-mcp-task-storage`. Session state is client-scoped; server state
//! is instance-global and shared across a cluster.
//!
//! # Backends
//!
//! - `InMemory` — test double (single-process only)
//! - `SQLite` — local durable mode (feature: `sqlite`)
//! - `PostgreSQL` — shared relational deployments (feature: `postgres`)
//! - `DynamoDB` — serverless/AWS deployments (feature: `dynamodb`)

pub mod error;
pub mod traits;

#[cfg(feature = "in-memory")]
pub mod in_memory;

#[cfg(feature = "sqlite")]
pub mod sqlite;

#[cfg(feature = "postgres")]
pub mod postgres;

// Future backends:
// #[cfg(feature = "dynamodb")]
// pub mod dynamodb;

// Re-exports
pub use error::ServerStateError;
pub use traits::{BoxedServerStateStorage, EntityState, RegistrySnapshot, ServerStateStorage};

#[cfg(feature = "in-memory")]
pub use in_memory::InMemoryServerStateStorage;

#[cfg(feature = "sqlite")]
pub use sqlite::{SqliteServerStateConfig, SqliteServerStateStorage};

#[cfg(feature = "postgres")]
pub use postgres::{PostgresServerStateConfig, PostgresServerStateStorage};
