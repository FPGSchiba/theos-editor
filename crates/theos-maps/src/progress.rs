//! Progress events and the cancel token of a run.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::Serialize;

/// One progress event: `done` of `total` steps of `stage` (spec §5.6).
///
/// The runner reports `{stage: <stage id>, done: <index>, total: <pipeline length>}` before every
/// stage; long stages report their own steps under their own name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Progress {
    /// The stage, or the step after the pipeline (`constraints`, `package`, ...).
    pub stage: String,
    /// Steps done.
    pub done: u32,
    /// Steps in all.
    pub total: u32,
}

/// A flag the caller sets to stop a run at its next cancel point. Clones share the flag.
#[derive(Debug, Clone, Default)]
pub struct CancelToken(Arc<AtomicBool>);

impl CancelToken {
    /// A token that is not cancelled.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Asks the run to stop.
    pub fn cancel(&self) {
        // Relaxed is enough: the flag publishes no other data, and a reader that sees the old
        // value stops at the next cancel point instead; the store itself is never lost.
        self.0.store(true, Ordering::Relaxed);
    }

    /// Whether [`CancelToken::cancel`] was called on this token or a clone of it.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::{CancelToken, Progress};

    #[test]
    fn a_cancel_reaches_every_clone() {
        let token = CancelToken::new();
        let clone = token.clone();
        assert!(!token.is_cancelled() && !clone.is_cancelled());
        clone.cancel();
        assert!(token.is_cancelled() && clone.is_cancelled());
        assert!(!CancelToken::default().is_cancelled());
    }

    #[test]
    fn progress_serialises_with_its_field_names() {
        let event = Progress {
            stage: "grid".into(),
            done: 0,
            total: 12,
        };
        assert_eq!(
            serde_json::to_value(&event).unwrap(),
            serde_json::json!({"stage": "grid", "done": 0, "total": 12})
        );
    }
}
