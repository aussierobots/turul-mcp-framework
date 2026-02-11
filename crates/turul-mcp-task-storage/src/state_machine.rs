//! Task state machine enforcement.
//!
//! Validates transitions per the MCP 2025-11-25 task lifecycle:
//!
//! ```text
//! Working -> InputRequired | Completed | Failed | Cancelled
//! InputRequired -> Working | Completed | Failed | Cancelled
//! Completed/Failed/Cancelled -> ERROR (terminal, no further transitions)
//! ```

use crate::error::TaskStorageError;
use turul_mcp_protocol::TaskStatus;

/// Validate a task status transition per MCP 2025-11-25 lifecycle rules.
///
/// Returns `Ok(())` if the transition is valid, or `Err(TaskStorageError)` if not.
pub fn validate_transition(from: TaskStatus, to: TaskStatus) -> Result<(), TaskStorageError> {
    match from {
        TaskStatus::Working => match to {
            TaskStatus::InputRequired
            | TaskStatus::Completed
            | TaskStatus::Failed
            | TaskStatus::Cancelled => Ok(()),
            TaskStatus::Working => Err(TaskStorageError::InvalidTransition {
                current: from,
                requested: to,
            }),
        },
        TaskStatus::InputRequired => match to {
            TaskStatus::Working
            | TaskStatus::Completed
            | TaskStatus::Failed
            | TaskStatus::Cancelled => Ok(()),
            TaskStatus::InputRequired => Err(TaskStorageError::InvalidTransition {
                current: from,
                requested: to,
            }),
        },
        TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Cancelled => {
            Err(TaskStorageError::TerminalState(from))
        }
    }
}

/// Returns `true` if the status is a terminal state (no further transitions allowed).
pub fn is_terminal(status: TaskStatus) -> bool {
    matches!(
        status,
        TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Cancelled
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_working_transitions() {
        assert!(validate_transition(TaskStatus::Working, TaskStatus::InputRequired).is_ok());
        assert!(validate_transition(TaskStatus::Working, TaskStatus::Completed).is_ok());
        assert!(validate_transition(TaskStatus::Working, TaskStatus::Failed).is_ok());
        assert!(validate_transition(TaskStatus::Working, TaskStatus::Cancelled).is_ok());
    }

    #[test]
    fn test_invalid_working_to_working() {
        assert!(validate_transition(TaskStatus::Working, TaskStatus::Working).is_err());
    }

    #[test]
    fn test_valid_input_required_transitions() {
        assert!(validate_transition(TaskStatus::InputRequired, TaskStatus::Working).is_ok());
        assert!(validate_transition(TaskStatus::InputRequired, TaskStatus::Completed).is_ok());
        assert!(validate_transition(TaskStatus::InputRequired, TaskStatus::Failed).is_ok());
        assert!(validate_transition(TaskStatus::InputRequired, TaskStatus::Cancelled).is_ok());
    }

    #[test]
    fn test_invalid_input_required_to_input_required() {
        assert!(validate_transition(TaskStatus::InputRequired, TaskStatus::InputRequired).is_err());
    }

    #[test]
    fn test_terminal_states_reject_all_transitions() {
        for terminal in [
            TaskStatus::Completed,
            TaskStatus::Failed,
            TaskStatus::Cancelled,
        ] {
            for target in [
                TaskStatus::Working,
                TaskStatus::InputRequired,
                TaskStatus::Completed,
                TaskStatus::Failed,
                TaskStatus::Cancelled,
            ] {
                let result = validate_transition(terminal, target);
                assert!(
                    result.is_err(),
                    "Expected error for {:?} -> {:?}",
                    terminal,
                    target
                );
                match result.unwrap_err() {
                    TaskStorageError::TerminalState(s) => assert_eq!(s, terminal),
                    other => panic!("Expected TerminalState, got: {:?}", other),
                }
            }
        }
    }

    #[test]
    fn test_is_terminal() {
        assert!(!is_terminal(TaskStatus::Working));
        assert!(!is_terminal(TaskStatus::InputRequired));
        assert!(is_terminal(TaskStatus::Completed));
        assert!(is_terminal(TaskStatus::Failed));
        assert!(is_terminal(TaskStatus::Cancelled));
    }
}
