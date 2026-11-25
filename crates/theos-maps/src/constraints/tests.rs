use serde_json::json;

use super::*;
use crate::recipe::test_stages::{recipe, registry};
use crate::recipe::{RecipeError, plan};

fn constraints(pairs: &[(&str, [f64; 2])]) -> BTreeMap<String, [f64; 2]> {
    pairs.iter().map(|(p, r)| ((*p).to_owned(), *r)).collect()
}

fn close(value: &Value, expected: f64) -> bool {
    value.as_f64().is_some_and(|v| (v - expected).abs() < 1e-12)
}

#[test]
fn result_keys_follow_the_protocol() {
    assert_eq!(result_key("elevation_m.relief"), "elevation_m_relief");
    assert_eq!(result_key("features.gate-up"), "features_gate_up");
    assert_eq!(result_key("Travel.X"), "travel_x");
}

#[test]
fn evaluate_reports_value_bounds_ok_and_pass() {
    let summary = json!({"lake_pct": 3.0, "elevation_m": {"relief": 900.0}});
    let c = constraints(&[
        ("lake_pct", [1.0, 5.0]),
        ("elevation_m.relief", [0.0, 500.0]),
    ]);
    let out = evaluate(&c, &summary);
    assert!(close(&out["lake_pct"], 3.0));
    assert!(close(&out["lake_pct_min"], 1.0));
    assert!(close(&out["lake_pct_max"], 5.0));
    assert_eq!(out["lake_pct_ok"], json!(true));
    assert!(close(&out["elevation_m_relief"], 900.0));
    assert_eq!(out["elevation_m_relief_ok"], json!(false));
    assert_eq!(out["pass"], json!(false));
    let ok = evaluate(&constraints(&[("lake_pct", [3.0, 3.0])]), &summary);
    assert_eq!(ok["pass"], json!(true));
}

#[test]
fn a_missing_value_fails_without_a_value_key() {
    let out = evaluate(
        &constraints(&[("rivers_pct", [0.0, 9.0])]),
        &json!({"rivers_pct": "x"}),
    );
    assert!(!out.contains_key("rivers_pct"));
    assert_eq!(out["rivers_pct_ok"], json!(false));
    assert!(close(&out["rivers_pct_max"], 9.0));
    assert_eq!(out["pass"], json!(false));
    let absent = evaluate(&constraints(&[("a.b", [0.0, 1.0])]), &json!({}));
    assert_eq!(absent["a_b_ok"], json!(false));
}

#[test]
fn no_constraints_pass() {
    let out = evaluate(&BTreeMap::new(), &json!({}));
    assert_eq!(out.len(), 1);
    assert_eq!(out["pass"], json!(true));
}

fn parameter_of(extra: &str, stages: &[&str]) -> Result<(), Option<String>> {
    match plan(&recipe(extra, stages), &registry(), &serde_json::Map::new()) {
        Ok(_) => Ok(()),
        Err(RecipeError::Invalid { parameter, .. }) => Err(parameter),
        Err(other) => panic!("{other:?}"),
    }
}

#[test]
fn an_unknown_constraint_path_fails_naming_it() {
    let err = parameter_of("[constraints]\n\"nope.x\" = [0.0, 1.0]\n", &["a"]);
    assert_eq!(err, Err(Some("nope.x".to_owned())));
}

#[test]
fn known_paths_pass_planning() {
    let extra = "[constraints]\nlake_pct = [0.0, 1.0]\n\"features.gate-up\" = [0.0, 2.0]\n\
                 \"stages.m.hits\" = [0.0, 2.0]\n";
    assert_eq!(parameter_of(extra, &["m"]), Ok(()));
    let bad_metric = "[constraints]\n\"stages.m.nope\" = [0.0, 2.0]\n";
    assert_eq!(
        parameter_of(bad_metric, &["m"]),
        Err(Some("stages.m.nope".to_owned()))
    );
    let bad_stage = "[constraints]\n\"stages.a.hits\" = [0.0, 2.0]\n";
    assert_eq!(
        parameter_of(bad_stage, &["a"]),
        Err(Some("stages.a.hits".to_owned()))
    );
    let bad_type = "[constraints]\n\"features.dragon\" = [0.0, 2.0]\n";
    assert_eq!(
        parameter_of(bad_type, &["a"]),
        Err(Some("features.dragon".to_owned()))
    );
}

#[test]
fn the_chime_walk_needs_a_travel_stage() {
    let extra = "[constraints]\n\"travel.chime_to_chime_min\" = [0.0, 90.0]\n";
    assert_eq!(
        parameter_of(extra, &["a"]),
        Err(Some("travel.chime_to_chime_min".to_owned()))
    );
    assert_eq!(parameter_of(extra, &["a", "travel"]), Ok(()));
}
