//! Compiles a fixture twice, which the scan would never read: findings on lines 3 and 6.

#[path = "../tests/fixtures/not-scanned.rs"]
mod not_scanned;

include!("../tests/fixtures/not-scanned.rs");

// Reading a fixture as data is fine.
const DATA: &str = include_str!("../tests/fixtures/not-scanned.rs");
