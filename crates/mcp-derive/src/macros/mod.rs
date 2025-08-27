//! Declarative Macro Implementations
//!
//! This module contains the implementation of declarative macros for creating
//! MCP tools and resources. These provide a more concise syntax compared to
//! derive macros or manual trait implementations.

pub mod tool;
pub mod resource;
pub mod schema;
pub mod prompt;
pub mod sampling;
pub mod notification;
pub mod completion;
pub mod elicitation;
pub mod roots;
pub mod logging;
pub mod shared;

// Re-export the main implementation functions
pub use tool::tool_declarative_impl;
pub use resource::resource_declarative_impl;
pub use schema::schema_for_impl;
pub use prompt::prompt_declarative_impl;
pub use sampling::sampling_declarative_impl;
pub use notification::notification_declarative_impl;
pub use completion::completion_declarative_impl;
pub use elicitation::elicitation_declarative_impl;
pub use roots::roots_declarative_impl;
pub use logging::logging_declarative_impl;