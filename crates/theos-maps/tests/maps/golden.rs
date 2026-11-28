//! Golden hashes per generator version (CE-77.1, CE-77.2, spec §5.14).
//!
//! The file of [`VERSION`] is recorded by `cargo run -p theos-maps --example bless` in one
//! process and compared here in another, which also proves that runs in two processes give the
//! same package bytes (plan ruling L28).

use std::collections::BTreeMap;

use theos_maps::golden::{
    GOLDEN_RUNS, GoldenFile, GoldenMismatch, GoldenRun, GoldenSpec, GoldenStage, compare,
    golden_path, load, record,
};
use theos_maps::{GENERATOR, VERSION};

type Key<'a> = (
    &'a str,
    u64,
    Vec<(&'a str, &'a str)>,
    Option<(&'a str, u64)>,
);

/// What identifies a recorded run: recipe, seed, inputs and parent.
fn run_key(run: &GoldenRun) -> Key<'_> {
    (
        &run.recipe,
        run.seed,
        run.inputs
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect(),
        run.from.as_ref().map(|p| (p.recipe.as_str(), p.seed)),
    )
}

/// The same key of a [`GoldenSpec`].
fn spec_key(spec: &GoldenSpec) -> Key<'_> {
    (spec.recipe, spec.seed, spec.inputs.to_vec(), spec.from)
}

use crate::support::{TestResult, golden_run};

// The hearth-valley run makes this a map-scale test (`slow_`, about 75–100 s in the test
// profile). A mismatch names, per golden run, the first stage and layer that differ, and the
// fresh hashes go to stderr so a platform's whole record can be compared with the file.
// AC: CE-77.1
#[test]
fn slow_outputs_match_the_golden_hashes_of_this_version() -> TestResult {
    let path = golden_path(VERSION);
    let file = load(&path).unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(file.generator, format!("{GENERATOR}@{VERSION}"));
    let recorded: Vec<_> = file.run.iter().map(run_key).collect();
    let expected: Vec<_> = GOLDEN_RUNS.iter().map(spec_key).collect();
    assert_eq!(
        recorded,
        expected,
        "{} holds other runs than GOLDEN_RUNS; before P08 merges, delete it and run \
         `cargo run -p theos-maps --example bless` (spec ruling 41); once 1.0.0 is on main, \
         raise `VERSION` instead",
        path.display()
    );
    let mut mismatches = Vec::new();
    for (expected, spec) in file.run.iter().zip(GOLDEN_RUNS) {
        let fresh = golden_run(spec)?;
        if let Err(mismatch) = compare(expected, &fresh) {
            eprintln!(
                "fresh hashes of {} seed {}:\n{}",
                fresh.recipe,
                fresh.seed,
                toml::to_string(&fresh)?
            );
            mismatches.push(format!("{} seed {}: {mismatch}", fresh.recipe, fresh.seed));
        }
    }
    assert!(
        mismatches.is_empty(),
        "outputs differ from the golden hashes of {VERSION} ({}):\n{}",
        path.display(),
        mismatches.join("\n")
    );
    Ok(())
}

// The interior runs (a standalone one and a child of a terrain ruin) are recorded and fresh
// runs reproduce them; a changed interior output fails with the stage and layer named.
// AC: CE-80.5
#[test]
fn interior_runs_match_the_golden_hashes() -> TestResult {
    let file = load(&golden_path(VERSION)).unwrap_or_else(|error| panic!("{error}"));
    let interiors: Vec<_> = GOLDEN_RUNS
        .iter()
        .filter(|s| matches!(s.recipe, "choir-hall-ruin" | "ruin-undercroft"))
        .collect();
    assert_eq!(
        interiors.len(),
        2,
        "both interior recipes have a golden run"
    );
    for spec in interiors {
        let expected = file
            .run
            .iter()
            .find(|r| run_key(r) == spec_key(spec))
            .ok_or_else(|| format!("{} seed {} is not recorded", spec.recipe, spec.seed))?;
        let fresh = golden_run(spec)?;
        if let Err(mismatch) = compare(expected, &fresh) {
            return Err(format!("{} seed {}: {mismatch}", spec.recipe, spec.seed).into());
        }
    }
    Ok(())
}

// AC: CE-77.2
#[test]
fn a_changed_output_fails_the_golden_comparison() -> TestResult {
    let golden = golden_run(&GOLDEN_RUNS[0])?;
    let mut changed = golden.clone();
    let stage = changed
        .stages
        .iter_mut()
        .find(|s| s.key == "hydrology#0")
        .ok_or("fixture-small has no hydrology stage")?;
    let hash = stage
        .layers
        .get_mut("discharge")
        .ok_or("hydrology wrote no discharge")?;
    let before = hash.clone();
    *hash = "0".repeat(64);
    assert_ne!(before, *hash);

    let mismatch = compare(&golden, &changed)
        .err()
        .ok_or("the change was not seen")?;
    assert_eq!(
        mismatch,
        GoldenMismatch::Layer {
            stage: "hydrology#0".to_owned(),
            layer: "discharge".to_owned(),
            expected: Some(before),
            actual: Some("0".repeat(64)),
        }
    );
    let message = mismatch.to_string();
    assert!(
        message.contains("stage `hydrology#0`, layer `discharge`"),
        "{message}"
    );

    let mut package = golden.clone();
    package.package = "0".repeat(64);
    assert!(
        matches!(
            compare(&golden, &package),
            Err(GoldenMismatch::Output {
                output: "package",
                ..
            })
        ),
        "a changed package was not seen"
    );
    // The features output too (final review round 4, A2).
    let mut features = golden.clone();
    features.features = "0".repeat(64);
    assert!(
        matches!(
            compare(&golden, &features),
            Err(GoldenMismatch::Output {
                output: "features",
                ..
            })
        ),
        "a changed features output was not seen"
    );
    Ok(())
}

// AC: CE-77.2
#[test]
fn bless_refuses_to_overwrite_an_existing_version() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("golden").join(format!("{VERSION}.toml"));
    let first = GoldenFile {
        generator: format!("{GENERATOR}@{VERSION}"),
        run: vec![GoldenRun {
            recipe: "fixture-small".to_owned(),
            seed: 1,
            inputs: BTreeMap::new(),
            from: None,
            features: "a".repeat(64),
            package: "b".repeat(64),
            stages: vec![GoldenStage {
                key: "grid#0".to_owned(),
                layers: BTreeMap::from([("height".to_owned(), "c".repeat(64))]),
            }],
        }],
    };
    record(&path, &first)?;
    let written = std::fs::read(&path)?;

    let mut second = first.clone();
    second.run[0].package = "d".repeat(64);
    let refused = record(&path, &second);
    assert!(
        matches!(refused, Err(theos_maps::golden::GoldenError::Exists { .. })),
        "{refused:?}"
    );
    assert_eq!(std::fs::read(&path)?, written, "the file changed");
    assert_eq!(load(&path)?, first);
    Ok(())
}
