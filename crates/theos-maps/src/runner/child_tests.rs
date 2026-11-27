//! Running child maps: the seeded parent values, their golden pseudo stage and the random
//! streams of the stages that stay when others are switched off.

use serde_json::{Map, Value, json};

use super::{Execution, RunOptions, execute};
use crate::child::test_support::{open, recipe, registry, settlement_recipe};
use crate::child::{ClassFrameValue, ParentFeatures};
use crate::error::RunError;
use crate::progress::CancelToken;
use crate::recipe::{LoadedRecipe, Planning, plan};

fn inputs() -> Map<String, Value> {
    let mut m = Map::new();
    m.insert("feature".to_owned(), json!("town-1"));
    m
}

/// Runs `loaded` as a child of the town of the synthetic parent.
fn run_child(loaded: &LoadedRecipe) -> Result<Execution, RunError> {
    let dir = tempfile::tempdir().unwrap();
    let parent = open(&dir, "town-1");
    let planned = plan(loaded, &registry(), &inputs(), Planning::Child(&parent)).unwrap();
    execute(
        &planned,
        7,
        &registry(),
        &RunOptions::default(),
        Some(&parent),
        &mut |_| {},
        &CancelToken::new(),
    )
}

#[test]
fn seeded_values_are_hashed_as_the_parent_pseudo_stage() {
    let run = run_child(&recipe(&["ph"])).unwrap();
    let first = &run.golden.stages[0];
    assert_eq!(first.key, "parent#0");
    assert_eq!(
        first.layers.keys().map(String::as_str).collect::<Vec<_>>(),
        ["parent.height"]
    );
    assert_eq!(first.layers["parent.height"].len(), 64);
    assert_eq!(run.golden.stages[1].key, "ph#0");
    // The stage copied what the world was seeded with.
    assert_eq!(run.world.f64("h"), run.world.f64("parent.height"));
}

#[test]
fn a_run_that_seeds_nothing_has_no_parent_stage() {
    let run = run_child(&recipe(&["b"])).unwrap();
    assert_eq!(run.golden.stages[0].key, "b#0");
}

#[test]
fn a_measured_parent_layer_is_resampled_smoothly() {
    let run = run_child(&recipe(&["ph"])).unwrap();
    let height = run.world.f64("parent.height").unwrap();
    assert_eq!((height.rows(), height.cols()), (8, 8));
    // The parent's height is a plane, 1.5 per cell east and 6 per cell south; the window starts
    // at (1,600, 1,600), so the first child centre (1,650, 1,650) lies at 1.15 cells.
    let plane = |u: f64, v: f64| 1.5 * (4.0 * v + u) + 0.25;
    assert!((height.get(0, 0) - plane(1.15, 1.15)).abs() < 1e-9);
    assert!((height.get(3, 5) - plane(1.15 + 0.5, 1.15 + 0.3)).abs() < 1e-9);
}

#[test]
fn a_class_parent_layer_keeps_its_codes() {
    let run = run_child(&recipe(&["pw"])).unwrap();
    let water = run.world.f64("parent.water").unwrap();
    assert!(
        water
            .as_slice()
            .iter()
            .all(|&v| v.to_bits() == 0.0_f64.to_bits() || v.to_bits() == 2.0_f64.to_bits())
    );
    assert!(water.as_slice().iter().any(|&v| v > 1.0));
    assert!(water.as_slice().iter().any(|&v| v < 1.0));
}

#[test]
fn parent_features_are_those_touching_the_window_in_the_childs_frame() {
    let run = run_child(&recipe(&["pf"])).unwrap();
    let features = run
        .world
        .value::<ParentFeatures>("parent.features")
        .unwrap();
    let ids: Vec<&str> = features.features.iter().map(|f| f.id.as_str()).collect();
    assert_eq!(ids, ["town-1", "village-1"]);
    assert_eq!(
        serde_json::to_value(&features.features[0].geometry).unwrap(),
        json!({ "Point": [400.0, 400.0] })
    );
    assert_eq!(features.features[1].parent_grid, [4, 4]);
}

#[test]
fn the_class_frame_is_seeded_for_a_settlement() {
    let run = run_child(&settlement_recipe(&["cf"])).unwrap();
    let class = run.world.value::<ClassFrameValue>("class_frame").unwrap();
    assert_eq!(class.class, "town");
    assert_eq!(class.frame.window_m, 1600.0);
    assert_eq!(run.golden.stages[0].key, "parent#0");
}

#[test]
fn switching_a_stage_off_keeps_the_streams_of_the_others() {
    let draw = |stages: &[&str]| {
        let planned = plan(
            &recipe(stages),
            &registry(),
            &inputs(),
            Planning::Standalone,
        )
        .unwrap();
        let run = execute(
            &planned,
            7,
            &registry(),
            &RunOptions::default(),
            None,
            &mut |_| {},
            &CancelToken::new(),
        )
        .unwrap();
        run.reports["b"].values["draw"]
    };
    // `a` runs only for a child, so a standalone run switches it off.
    assert_eq!(draw(&["a", "b"]).to_bits(), draw(&["b"]).to_bits());
}

#[test]
fn a_parent_value_without_a_parent_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let parent = open(&dir, "town-1");
    let planned = plan(
        &recipe(&["ph"]),
        &registry(),
        &inputs(),
        Planning::Child(&parent),
    )
    .unwrap();
    let err = execute(
        &planned,
        7,
        &registry(),
        &RunOptions::default(),
        None,
        &mut |_| {},
        &CancelToken::new(),
    )
    .unwrap_err();
    assert!(matches!(err, RunError::Child(_)), "{err}");
}
