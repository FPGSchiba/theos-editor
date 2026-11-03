//! Synthetic fixture: a crate that hides code from the coverage floor.

#[cfg(not(coverage))]
pub fn hidden() {}
