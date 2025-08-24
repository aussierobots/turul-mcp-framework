//! Declarative Macro Implementations
//!
//! This module contains the implementation of declarative macros for creating
//! MCP tools and resources. These provide a more concise syntax compared to
//! derive macros or manual trait implementations.

pub mod tool;
pub mod resource;
pub mod schema;
pub mod shared;

// Re-export the main implementation functions
pub use tool::tool_declarative_impl;
pub use resource::resource_declarative_impl;
pub use schema::schema_for_impl;