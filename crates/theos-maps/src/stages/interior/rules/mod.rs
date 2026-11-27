//! The mission rules. A new rule is one module here plus one line in [`rules`].

use super::mission_core::Rule;

mod leaf;
mod locked_leaf;
mod main_cycle;
#[cfg(test)]
mod testkit;

/// Every mission rule, one line per rule.
#[must_use]
pub fn rules() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(main_cycle::MainCycle),
        Box::new(leaf::Leaf),
        Box::new(locked_leaf::LockedLeaf),
    ]
}
