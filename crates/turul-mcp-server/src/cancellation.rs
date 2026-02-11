//! Cancellation Handle — cooperative cancellation for in-process task execution.
//!
//! Lives in the server crate (not task-storage) because it uses `tokio::sync::watch`
//! which is a runtime-specific primitive that doesn't belong in the storage abstraction.

use tokio::sync::watch;

/// A cooperative cancellation handle for in-process task execution.
///
/// Wraps a `tokio::sync::watch` channel to signal cancellation to a running future.
/// Clone-friendly — both the task executor and the runtime hold copies.
#[derive(Clone)]
pub struct CancellationHandle {
    tx: watch::Sender<bool>,
    rx: watch::Receiver<bool>,
}

impl CancellationHandle {
    /// Create a new (not-yet-cancelled) handle.
    pub fn new() -> Self {
        let (tx, rx) = watch::channel(false);
        Self { tx, rx }
    }

    /// Signal cancellation. Idempotent — multiple calls are safe.
    pub fn cancel(&self) {
        let _ = self.tx.send(true);
    }

    /// Check if cancellation has been requested.
    pub fn is_cancelled(&self) -> bool {
        *self.rx.borrow()
    }

    /// Wait until cancellation is requested.
    ///
    /// Returns immediately if already cancelled.
    pub async fn cancelled(&self) {
        let mut rx = self.rx.clone();
        // If already cancelled, return immediately
        if *rx.borrow() {
            return;
        }
        // Otherwise wait for the signal
        loop {
            if rx.changed().await.is_err() {
                // Sender dropped — treat as cancelled
                return;
            }
            if *rx.borrow() {
                return;
            }
        }
    }
}

impl Default for CancellationHandle {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_new_handle_not_cancelled() {
        let handle = CancellationHandle::new();
        assert!(!handle.is_cancelled());
    }

    #[tokio::test]
    async fn test_cancel_sets_flag() {
        let handle = CancellationHandle::new();
        handle.cancel();
        assert!(handle.is_cancelled());
    }

    #[tokio::test]
    async fn test_cancel_idempotent() {
        let handle = CancellationHandle::new();
        handle.cancel();
        handle.cancel();
        assert!(handle.is_cancelled());
    }

    #[tokio::test]
    async fn test_clone_shares_state() {
        let handle = CancellationHandle::new();
        let clone = handle.clone();
        handle.cancel();
        assert!(clone.is_cancelled());
    }

    #[tokio::test]
    async fn test_cancelled_future_resolves() {
        let handle = CancellationHandle::new();
        let clone = handle.clone();

        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            clone.cancel();
        });

        tokio::time::timeout(std::time::Duration::from_secs(1), handle.cancelled())
            .await
            .expect("cancelled() should resolve within timeout");
    }

    #[tokio::test]
    async fn test_cancelled_future_immediate_if_already_cancelled() {
        let handle = CancellationHandle::new();
        handle.cancel();

        // Should return immediately
        tokio::time::timeout(std::time::Duration::from_millis(10), handle.cancelled())
            .await
            .expect("cancelled() should resolve immediately when already cancelled");
    }
}
