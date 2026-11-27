//! Planning of child maps: the planning modes and the parent values a plan seeds (spec P29
//! §5.2 items 5 and 6).

use serde_json::{Map, Value, json};

use super::{Planning, plan};
use crate::child::test_support::{open, recipe, registry, settlement_recipe};
use crate::recipe::{LoadedRecipe, RecipeError};
use crate::runner::estimate;

fn inputs() -> Map<String, Value> {
    let mut m = Map::new();
    m.insert("feature".to_owned(), json!("town-1"));
    m
}

fn ids(plan: &super::Plan) -> Vec<&str> {
    plan.stages.iter().map(|s| s.id).collect()
}

fn seeded(plan: &super::Plan) -> Vec<&str> {
    plan.seeded.iter().map(|s| s.name).collect()
}

/// The plan of `loaded` as a child of the town of the synthetic parent.
fn child(loaded: &LoadedRecipe) -> Result<super::Plan, RecipeError> {
    let dir = tempfile::tempdir().unwrap();
    let parent = open(&dir, "town-1");
    plan(loaded, &registry(), &inputs(), Planning::Child(&parent))
}

// AC: CE-28.3
#[test]
fn a_stage_reading_a_parent_value_the_parent_lacks_is_refused() {
    let err = child(&recipe(&["pd"])).unwrap_err();
    match err {
        RecipeError::Invalid {
            stage,
            layer,
            message,
            ..
        } => {
            assert_eq!(stage.as_deref(), Some("pd"));
            assert_eq!(layer.as_deref(), Some("parent.discharge"));
            assert_eq!(
                message,
                "stage `pd` reads `parent.discharge`, which the parent does not have"
            );
        }
        other => panic!("expected Invalid, got {other:?}"),
    }
}

#[test]
fn only_parent_values_a_stage_reads_are_seeded() {
    let planned = child(&recipe(&["ph"])).unwrap();
    assert_eq!(seeded(&planned), ["parent.height"]);
    let both = child(&recipe(&["pw", "ph"])).unwrap();
    assert_eq!(seeded(&both), ["parent.height", "parent.water"]);
    assert_eq!(seeded(&child(&recipe(&["b"])).unwrap()), Vec::<&str>::new());
}

#[test]
fn a_child_plan_knows_its_place_in_the_parent() {
    let planned = child(&recipe(&["pf"])).unwrap();
    assert_eq!(seeded(&planned), ["parent.features"]);
    // 800 m window centred on (2,000, 2,000), snapped to whole 100 m cells.
    assert_eq!(planned.origin_m, Some([1600.0, 1600.0]));
    let parent = planned.parent.unwrap();
    assert_eq!(parent.feature, "town-1");
    assert_eq!(parent.package_sha256.len(), 64);
}

#[test]
fn child_only_stages_are_left_out_of_standalone_plans() {
    let loaded = recipe(&["a", "b"]);
    let alone = plan(&loaded, &registry(), &inputs(), Planning::Standalone).unwrap();
    assert_eq!(ids(&alone), ["b"]);
    assert_eq!(alone.origin_m, None);
    assert!(alone.parent.is_none() && alone.seeded.is_empty());
    assert_eq!(ids(&child(&loaded).unwrap()), ["a", "b"]);
    let described = plan(&loaded, &registry(), &inputs(), Planning::Describe).unwrap();
    assert_eq!(ids(&described), ["a", "b"]);
}

#[test]
fn standalone_only_stages_are_left_out_of_child_plans() {
    let loaded = recipe(&["sa", "b"]);
    let alone = plan(&loaded, &registry(), &inputs(), Planning::Standalone).unwrap();
    assert_eq!(ids(&alone), ["sa", "b"]);
    assert_eq!(ids(&child(&loaded).unwrap()), ["b"]);
    let described = plan(&loaded, &registry(), &inputs(), Planning::Describe).unwrap();
    assert_eq!(ids(&described), ["sa", "b"]);
}

#[test]
fn switching_a_stage_off_keeps_the_other_keys() {
    let with = plan(
        &recipe(&["a", "b", "b"]),
        &registry(),
        &inputs(),
        Planning::Standalone,
    )
    .unwrap();
    let without = plan(
        &recipe(&["b", "b"]),
        &registry(),
        &inputs(),
        Planning::Standalone,
    )
    .unwrap();
    let keys = |p: &super::Plan| p.stages.iter().map(|s| s.key.clone()).collect::<Vec<_>>();
    assert_eq!(keys(&with), ["b#0", "b#1"]);
    assert_eq!(keys(&with), keys(&without));
}

#[test]
fn a_settlement_recipe_without_site_terrain_plans_as_a_child_only() {
    let loaded = settlement_recipe(&["ph"]);
    let planned = child(&loaded).unwrap();
    // The class of the town: 1,600 m window over 4 m cells.
    assert_eq!((planned.frame.rows, planned.frame.cols), (400, 400));
    assert_eq!(
        planned.class_frame.as_ref().map(|c| c.class.as_str()),
        Some("town")
    );
    let err = plan(&loaded, &registry(), &inputs(), Planning::Standalone).unwrap_err();
    assert!(err.to_string().contains("site_terrain"), "{err}");
}

#[test]
fn describe_treats_parent_reads_as_available_and_takes_the_first_parent_type() {
    let loaded = settlement_recipe(&["pd"]);
    let planned = plan(&loaded, &registry(), &Map::new(), Planning::Describe).unwrap();
    assert_eq!(ids(&planned), ["pd"]);
    // The first parent type of the recipe is `town`: 1,600 m over 4 m cells.
    assert_eq!(planned.frame.rows, 400);
    assert!(
        plan(&loaded, &registry(), &Map::new(), Planning::Standalone).is_err(),
        "a standalone plan has no parent values"
    );
}

#[test]
fn the_class_frame_is_seeded_when_a_stage_reads_it() {
    let planned = child(&settlement_recipe(&["cf"])).unwrap();
    assert_eq!(seeded(&planned), ["class_frame"]);
    let err = child(&recipe(&["cf"])).unwrap_err();
    assert!(err.to_string().contains("class_frame"), "{err}");
}

#[test]
fn the_memory_estimate_counts_seeded_values() {
    let planned = child(&recipe(&["ph"])).unwrap();
    // `parent.height` and `h` over 8 x 8 cells of 8 bytes, raised by half.
    let estimated = estimate(&planned);
    assert_eq!(estimated.bytes, (64 * 8 + 64 * 8) * 3 / 2);
    let names: Vec<&str> = estimated.largest.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(names, ["h", "parent.height"]);
    let mut seeded_only = planned;
    seeded_only.stages.clear();
    assert_eq!(estimate(&seeded_only).bytes, 64 * 8 * 3 / 2);
}
