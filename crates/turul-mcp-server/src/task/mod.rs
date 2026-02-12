//! Task module — MCP task execution, storage integration, and request handlers.
//!
//! This module consolidates all task-related functionality:
//! - [`executor`] — `TaskExecutor` trait and `BoxedTaskWork` type alias
//! - [`handlers`] — MCP request handlers for `tasks/get`, `tasks/list`, `tasks/cancel`, `tasks/result`
//! - [`runtime`] — `TaskRuntime` bridging storage and execution
//! - [`tokio_executor`] — Default `TokioTaskExecutor` using `tokio::spawn`

pub mod executor;
pub mod handlers;
pub mod runtime;
pub mod tokio_executor;
