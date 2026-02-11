//! Prelude module for convenient imports.
//!
//! ```rust,no_run
//! use turul_mcp_task_storage::prelude::*;
//! ```

pub use crate::error::TaskStorageError;
#[cfg(feature = "in-memory")]
pub use crate::in_memory::{InMemoryTaskConfig, InMemoryTaskStorage};
#[cfg(feature = "sqlite")]
pub use crate::sqlite::{SqliteTaskConfig, SqliteTaskStorage};
#[cfg(feature = "postgres")]
pub use crate::postgres::{PostgresTaskConfig, PostgresTaskStorage};
#[cfg(feature = "dynamodb")]
pub use crate::dynamodb::{DynamoDbTaskConfig, DynamoDbTaskStorage};
pub use crate::state_machine::{is_terminal, validate_transition};
pub use crate::traits::{TaskListPage, TaskOutcome, TaskRecord, TaskStorage};
