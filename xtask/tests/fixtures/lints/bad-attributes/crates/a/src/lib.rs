//! Attributes that the scan refuses, one finding on each of lines 3, 5, 8 and 12.

#![allow(missing_docs)]

#[allow(dead_code)]
fn silenced() {}

#[expect(dead_code)]
fn unexplained() {}

#[test]
#[ignore]
fn skipped() {}
