//! # Session Storage Prelude
//!
//! This module provides convenient re-exports of the most commonly used types
//! from the session storage library.
//!
//! ```rust
//! use turul_mcp_session_storage::prelude::*;
//! ```

// Core trait and types
pub use crate::traits::{
    SessionStorage, SessionStorageError, SessionInfo, SseEvent,
    BoxedSessionStorage, SessionStorageBuilder,
};

// In-memory implementation (always available)
pub use crate::in_memory::{InMemoryConfig, InMemoryError, InMemorySessionStorage, InMemoryStats};

// Optional implementations
#[cfg(feature = "sqlite")]
pub use crate::sqlite::{SqliteConfig, SqliteError, SqliteSessionStorage};

#[cfg(feature = "postgres")]
pub use crate::postgres::{PostgresConfig, PostgresError, PostgresSessionStorage};

#[cfg(feature = "dynamodb")]
pub use crate::dynamodb::{DynamoDbConfig, DynamoDbError, DynamoDbSessionStorage};

// Convenience functions
pub use crate::{
    create_default_storage, create_memory_storage, StorageResult,
};

#[cfg(feature = "sqlite")]
pub use crate::{create_sqlite_storage, create_sqlite_storage_with_config};

#[cfg(feature = "postgres")]
pub use crate::{create_postgres_storage, create_postgres_storage_with_config};

#[cfg(feature = "dynamodb")]
pub use crate::{create_dynamodb_storage, create_dynamodb_storage_with_config};