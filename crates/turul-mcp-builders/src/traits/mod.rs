//! Framework traits for compositional MCP type construction
//!
//! **IMPORTANT**: These are framework features, NOT part of the MCP specification.
//! The MCP specification defines concrete types only.

pub mod tool_traits;
pub mod resource_traits;
pub mod prompt_traits;
pub mod root_traits;
pub mod sampling_traits;
pub mod completion_traits;
pub mod logging_traits;
pub mod notification_traits;
pub mod elicitation_traits;

// Re-export all traits at traits module level
pub use tool_traits::*;
pub use resource_traits::*;
pub use prompt_traits::*;
pub use root_traits::*;
pub use sampling_traits::*;
pub use completion_traits::*;
pub use logging_traits::*;
pub use notification_traits::*;
pub use elicitation_traits::*;
