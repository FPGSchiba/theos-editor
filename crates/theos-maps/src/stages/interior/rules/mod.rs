//! The mission rules. A new rule is one module here plus one line in [`rules`].

use super::mission_core::Rule;

/// Every mission rule, one line per rule.
#[must_use]
pub fn rules() -> Vec<Box<dyn Rule>> {
    Vec::new()
}
