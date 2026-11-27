//! The mission rules. A new rule is one module here plus one line in [`rules`].

use super::mission_core::Rule;

mod close_loop;
mod extend_arc;
mod insert_cycle;
mod leaf;
mod locked_leaf;
mod main_cycle;
mod merge_hall;
mod shortcut;
#[cfg(test)]
mod testkit;

/// Every mission rule, one line per rule.
#[must_use]
pub fn rules() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(main_cycle::MainCycle),
        Box::new(leaf::Leaf),
        Box::new(locked_leaf::LockedLeaf),
        Box::new(insert_cycle::InsertCycle),
        Box::new(extend_arc::ExtendArc),
        Box::new(close_loop::CloseLoop),
        Box::new(shortcut::Shortcut),
        Box::new(merge_hall::MergeHall),
    ]
}
