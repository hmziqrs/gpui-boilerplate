//! Cooperative cancellation signal for in-flight query requests.
//!
//! [`QuerySignal`] uses a shared atomic flag so that all clones observe the
//! same cancellation state. The fetcher is expected to check `is_cancelled()`
//! periodically and abort early when possible.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// A cooperative cancellation signal for in-flight query requests.
///
/// Clones share the same underlying flag, so cancelling any clone
/// cancels all of them. This is a runtime-only type — it cannot be
/// serialized because the shared state has no meaningful persisted form.
///
/// # Example
///
/// ```
/// use gpui_query::core::QuerySignal;
///
/// let signal = QuerySignal::new();
/// let clone = signal.clone();
///
/// assert!(!signal.is_cancelled());
/// assert!(!clone.is_cancelled());
///
/// signal.cancel();
/// assert!(signal.is_cancelled());
/// assert!(clone.is_cancelled());
/// ```
#[derive(Debug, Clone)]
pub struct QuerySignal {
    cancelled: Arc<AtomicBool>,
}

impl PartialEq for QuerySignal {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.cancelled, &other.cancelled)
    }
}

impl Eq for QuerySignal {}

impl QuerySignal {
    /// Create a new, non-cancelled signal.
    pub fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Signal cancellation. All clones sharing this flag will observe
    /// `is_cancelled() == true` after this call.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    /// Check whether cancellation has been signalled.
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

impl Default for QuerySignal {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_signal_is_not_cancelled() {
        let signal = QuerySignal::new();
        assert!(!signal.is_cancelled());
    }

    #[test]
    fn cancel_sets_flag() {
        let signal = QuerySignal::new();
        signal.cancel();
        assert!(signal.is_cancelled());
    }

    #[test]
    fn clone_shares_state() {
        let signal = QuerySignal::new();
        let clone = signal.clone();

        signal.cancel();

        assert!(signal.is_cancelled());
        assert!(clone.is_cancelled());
    }

    #[test]
    fn multiple_clones_share_state() {
        let signal = QuerySignal::new();
        let c1 = signal.clone();
        let c2 = signal.clone();
        let c3 = signal.clone();

        c1.cancel();

        assert!(signal.is_cancelled());
        assert!(c1.is_cancelled());
        assert!(c2.is_cancelled());
        assert!(c3.is_cancelled());
    }

    #[test]
    fn default_is_not_cancelled() {
        let signal = QuerySignal::default();
        assert!(!signal.is_cancelled());
    }
}
