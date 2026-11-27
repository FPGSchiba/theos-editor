//! Determinism within one platform (spec §5.14): runs in one process give identical bytes.
//!
//! `api::two_runs_into_two_folders_give_identical_outputs` compares every output file (renders,
//! features, package) of two runs of a recipe with every stage. This test adds the hashes of
//! every layer after every stage, and a different run between the two, so state that leaks from
//! one run into the next shows up. Runs in two processes are covered by the golden test, which
//! compares with hashes `bless` recorded in another process (plan ruling L28).

use theos_maps::golden::compare;

use crate::support::{TestResult, golden_run};

#[test]
fn two_runs_in_one_process_give_identical_packages_and_layers() -> TestResult {
    let first = golden_run("fixture-small", 2)?;
    let between = golden_run("fixture-small", 1)?;
    let second = golden_run("fixture-small", 2)?;
    compare(&first, &second)?;
    assert_eq!(first, second);
    assert_ne!(
        first.package, between.package,
        "another seed must give another package, or this test proves nothing"
    );
    Ok(())
}
