use std::collections::BTreeMap;

use super::*;
use crate::golden::GoldenStage;

fn stage(key: &str, layers: &[(&str, &str)]) -> GoldenStage {
    GoldenStage {
        key: key.to_owned(),
        layers: layers
            .iter()
            .map(|&(name, hash)| (name.to_owned(), hash.to_owned()))
            .collect::<BTreeMap<_, _>>(),
    }
}

/// A run of three stages: `grid#0` and `hydrology#0` with one layer, `tectonics#0` with two.
fn golden() -> GoldenRun {
    GoldenRun {
        recipe: "fixture-small".to_owned(),
        seed: 1,
        features: "f0".to_owned(),
        package: "p0".to_owned(),
        stages: vec![
            stage("grid#0", &[("height", "h0")]),
            stage("tectonics#0", &[("height", "h1"), ("uplift", "u1")]),
            stage("hydrology#0", &[("discharge", "d2")]),
        ],
    }
}

#[test]
fn equal_runs_compare_equal() {
    assert_eq!(compare(&golden(), &golden()), Ok(()));
}

#[test]
fn a_changed_layer_names_its_stage_and_layer_with_both_hashes() {
    let mut fresh = golden();
    fresh.stages[1]
        .layers
        .insert("uplift".to_owned(), "u9".to_owned());
    let mismatch = compare(&golden(), &fresh).unwrap_err();
    assert_eq!(
        mismatch,
        GoldenMismatch::Layer {
            stage: "tectonics#0".to_owned(),
            layer: "uplift".to_owned(),
            expected: Some("u1".to_owned()),
            actual: Some("u9".to_owned()),
        }
    );
    assert_eq!(
        mismatch.to_string(),
        "stage `tectonics#0`, layer `uplift`: expected `u1`, got `u9`"
    );
}

#[test]
fn the_first_difference_in_pipeline_order_wins() {
    let mut fresh = golden();
    fresh.package = "p9".to_owned();
    fresh.features = "f9".to_owned();
    fresh.stages[2]
        .layers
        .insert("discharge".to_owned(), "d9".to_owned());
    fresh.stages[1]
        .layers
        .insert("uplift".to_owned(), "u9".to_owned());
    fresh.stages[1]
        .layers
        .insert("height".to_owned(), "h9".to_owned());
    let mismatch = compare(&golden(), &fresh).unwrap_err();
    assert!(
        matches!(&mismatch, GoldenMismatch::Layer { stage, layer, .. }
            if stage == "tectonics#0" && layer == "height"),
        "{mismatch:?}"
    );
}

#[test]
fn a_layer_only_one_run_wrote_is_a_difference() {
    let mut fresh = golden();
    fresh.stages[2]
        .layers
        .insert("lakes".to_owned(), "l9".to_owned());
    let added = compare(&golden(), &fresh).unwrap_err();
    assert_eq!(
        added,
        GoldenMismatch::Layer {
            stage: "hydrology#0".to_owned(),
            layer: "lakes".to_owned(),
            expected: None,
            actual: Some("l9".to_owned()),
        }
    );
    assert_eq!(
        added.to_string(),
        "stage `hydrology#0`, layer `lakes`: expected nothing, got `l9`"
    );
    let removed = compare(&fresh, &golden()).unwrap_err();
    assert!(
        matches!(
            &removed,
            GoldenMismatch::Layer {
                expected: Some(_),
                actual: None,
                ..
            }
        ),
        "{removed:?}"
    );
}

#[test]
fn another_stage_at_a_position_is_a_difference() {
    let mut fresh = golden();
    fresh.stages[1].key = "geology#0".to_owned();
    let mismatch = compare(&golden(), &fresh).unwrap_err();
    assert_eq!(
        mismatch,
        GoldenMismatch::Stage {
            position: 1,
            expected: Some("tectonics#0".to_owned()),
            actual: Some("geology#0".to_owned()),
        }
    );
    assert_eq!(
        mismatch.to_string(),
        "pipeline position 1: expected stage `tectonics#0`, got stage `geology#0`"
    );
}

#[test]
fn a_stage_only_one_run_has_is_a_difference() {
    let mut longer = golden();
    longer.stages.push(stage("travel#0", &[]));
    let extra = compare(&golden(), &longer).unwrap_err();
    assert_eq!(
        extra,
        GoldenMismatch::Stage {
            position: 3,
            expected: None,
            actual: Some("travel#0".to_owned()),
        }
    );
    assert_eq!(
        extra.to_string(),
        "pipeline position 3: expected stage nothing, got stage `travel#0`"
    );
    let missing = compare(&longer, &golden()).unwrap_err();
    assert!(
        matches!(
            &missing,
            GoldenMismatch::Stage {
                position: 3,
                expected: Some(_),
                actual: None
            }
        ),
        "{missing:?}"
    );
}

#[test]
fn features_come_before_the_package() {
    let mut fresh = golden();
    fresh.features = "f9".to_owned();
    fresh.package = "p9".to_owned();
    let mismatch = compare(&golden(), &fresh).unwrap_err();
    assert_eq!(
        mismatch,
        GoldenMismatch::Output {
            output: "features",
            expected: "f0".to_owned(),
            actual: "f9".to_owned(),
        }
    );
    assert_eq!(
        mismatch.to_string(),
        "output `features`: expected f0, got f9"
    );
}

#[test]
fn a_changed_package_alone_is_named() {
    let mut fresh = golden();
    fresh.package = "p9".to_owned();
    assert_eq!(
        compare(&golden(), &fresh),
        Err(GoldenMismatch::Output {
            output: "package",
            expected: "p0".to_owned(),
            actual: "p9".to_owned(),
        })
    );
}

#[test]
fn another_run_is_named_before_any_hash() {
    let mut fresh = golden();
    fresh.seed = 2;
    fresh.package = "p9".to_owned();
    let mismatch = compare(&golden(), &fresh).unwrap_err();
    assert_eq!(
        mismatch,
        GoldenMismatch::Run {
            expected_recipe: "fixture-small".to_owned(),
            expected_seed: 1,
            actual_recipe: "fixture-small".to_owned(),
            actual_seed: 2,
        }
    );
    assert_eq!(
        mismatch.to_string(),
        "the golden run is fixture-small seed 1, the fresh one fixture-small seed 2"
    );
}
