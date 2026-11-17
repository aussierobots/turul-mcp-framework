//! Declarative Macro Implementations
//!
//! Implements the declarative macros (`tool!`, `resource!`, `prompt!`, `completion!`, `elicitation!`,
//! `logging!`, `notification!`, `roots!`, `sampling!`) and `schema_for!`. They provide a more concise syntax than
//! derive macros or manual trait implementations.

pub mod completion;
pub mod elicitation;
pub mod logging;
pub mod notification;
pub mod prompt;
pub mod resource;
pub mod roots;
pub mod sampling;
pub mod schema;
pub mod shared;
pub mod tool;

// Re-export the main implementation functions
pub use completion::completion_declarative_impl;
pub use elicitation::elicitation_declarative_impl;
pub use logging::logging_declarative_impl;
pub use notification::notification_declarative_impl;
pub use prompt::prompt_declarative_impl;
pub use resource::resource_declarative_impl;
pub use roots::roots_declarative_impl;
pub use sampling::sampling_declarative_impl;
pub use schema::schema_for_impl;
pub use tool::tool_declarative_impl;
