//! The flag that cancels git commands.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// A flag that cancels the git commands that watch it. Clones share the flag.
#[derive(Clone, Debug, Default)]
pub struct Cancel(Arc<AtomicBool>);

impl Cancel {
    /// A flag that is not set.
    #[must_use]
    pub fn new() -> Cancel {
        Cancel::default()
    }

    /// Sets the flag: every command that watches it, or one of its clones, stops.
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }

    /// Whether the flag is set.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_flag_is_not_set() {
        assert!(!Cancel::new().is_cancelled());
    }

    #[test]
    fn clones_share_the_flag() {
        let cancel = Cancel::new();
        let clone = cancel.clone();
        clone.cancel();
        assert!(cancel.is_cancelled());
    }
}
