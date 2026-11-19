//! The run summary (spec §5.14): the parity metrics nested by path, the feature counts, the
//! metrics of every stage and the warnings.

use std::collections::BTreeMap;

use serde_json::{Map, Value};
use theos_map_format::FormatError;
use theos_map_format::json::to_checked_value;

use crate::features::TYPES;
use crate::runner::StageFinding;
use crate::stage::Metrics;

/// The summary's object of feature counts, `features.<type>` to a constraint.
pub(crate) const FEATURES: &str = "features";
/// The summary's object of stage metrics, `stages.<id>.<metric>` to a constraint.
pub(crate) const STAGES: &str = "stages";
/// The summary's list of warnings.
const FINDINGS: &str = "findings";

/// Inserts `value` under the dotted `path` of `root`, creating the objects on the way.
fn insert_nested(root: &mut Map<String, Value>, path: &str, value: Value) {
    match path.split_once('.') {
        None => {
            root.insert(path.to_owned(), value);
        }
        Some((head, rest)) => {
            let entry = root
                .entry(head.to_owned())
                .or_insert_with(|| Value::Object(Map::new()));
            if let Value::Object(inner) = entry {
                insert_nested(inner, rest, value);
            }
        }
    }
}

/// The run summary: the parity `metrics` (keys are dotted paths, so `elevation_m.relief` becomes
/// `{"elevation_m": {"relief": ..}}`), then `features` (the count of every type of
/// [`TYPES`], 0 included), `stages` (each
/// stage id with its metrics) and `findings` (`{stage, message}` per warning). Constraints read
/// it by dotted path (spec §5.6). The stage metrics go through the checked conversion, as every
/// number bound for JSON does.
///
/// # Errors
/// [`FormatError::JsonNotFinite`] when a stage metric is NaN or an infinity (spec §5.13).
pub(crate) fn summary(
    metrics: Value,
    feature_counts: &BTreeMap<String, usize>,
    stage_metrics: &[(String, Metrics)],
    findings: &[StageFinding],
) -> Result<Value, FormatError> {
    let mut out = Map::new();
    if let Value::Object(flat) = metrics {
        for (path, value) in flat {
            insert_nested(&mut out, &path, value);
        }
    }
    // Every type of `TYPES` with 0 first (ledger ruling, Task 11): a count constraint must see 0.
    let mut features: Map<String, Value> = TYPES
        .iter()
        .map(|(_, ty)| ((*ty).to_owned(), Value::from(0)))
        .collect();
    for (ty, &n) in feature_counts {
        features.insert(ty.clone(), Value::from(n));
    }
    out.insert(FEATURES.to_owned(), Value::Object(features));
    let stages = stage_metrics
        .iter()
        .map(|(id, m)| Ok((id.clone(), to_checked_value(&m.0)?)))
        .collect::<Result<Map<String, Value>, FormatError>>()?;
    out.insert(STAGES.to_owned(), Value::Object(stages));
    let findings = findings
        .iter()
        .map(|f| serde_json::json!({"stage": f.stage, "message": f.message}))
        .collect();
    out.insert(FINDINGS.to_owned(), Value::Array(findings));
    Ok(Value::Object(out))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_summary_nests_dotted_paths() {
        let metrics = serde_json::json!({
            "elevation_m.relief": 1134.0,
            "lake_pct": 6.25,
            "precip_mm_yr.median": 751.0,
        });
        let counts = BTreeMap::from([("town".to_owned(), 3), ("chime".to_owned(), 2)]);
        let mut hydrology = Metrics::default();
        hydrology.0.insert("lake_count".to_owned(), 2.0);
        let stages = [("hydrology".to_owned(), hydrology)];
        let findings = [StageFinding {
            stage: "travel".to_owned(),
            message: "slow".to_owned(),
        }];
        let s = summary(metrics, &counts, &stages, &findings).unwrap();
        assert_eq!(s["elevation_m"]["relief"], 1134.0);
        assert_eq!(s["precip_mm_yr"]["median"], 751.0);
        assert_eq!(s["lake_pct"], 6.25);
        assert_eq!(s["features"]["chime"], 2);
        assert_eq!(s["features"]["town"], 3);
        assert_eq!(s["stages"]["hydrology"]["lake_count"], 2.0);
        assert_eq!(
            s["findings"],
            serde_json::json!([{"stage": "travel", "message": "slow"}])
        );
        assert!(s.get("elevation_m.relief").is_none());
        let keys: Vec<&str> = s.as_object().unwrap().keys().map(String::as_str).collect();
        assert_eq!(
            keys,
            [
                "elevation_m",
                "features",
                "findings",
                "lake_pct",
                "precip_mm_yr",
                "stages"
            ]
        );
        // the dotted paths are what constraints read
        let c = BTreeMap::from([("elevation_m.relief".to_owned(), [1000.0, 1200.0])]);
        assert_eq!(crate::constraints::evaluate(&c, &s)["pass"], true);
    }

    // Spec §5.13: the summary goes into the manifest, which holds no NaN or infinity;
    // `Value::from` would turn one into `null` without a word.
    #[test]
    fn a_non_finite_stage_metric_is_refused() {
        for bad in [f64::NAN, f64::INFINITY] {
            let mut m = Metrics::default();
            m.0.insert("depth".to_owned(), bad);
            let stages = [("level".to_owned(), m)];
            let result = summary(serde_json::json!({}), &BTreeMap::new(), &stages, &[]);
            assert!(
                matches!(result, Err(FormatError::JsonNotFinite)),
                "{bad}: {result:?}"
            );
        }
        let mut m = Metrics::default();
        m.0.insert("depth".to_owned(), -0.0);
        let stages = [("level".to_owned(), m)];
        let s = summary(serde_json::json!({}), &BTreeMap::new(), &stages, &[]).unwrap();
        assert_eq!(
            serde_json::to_string(&s["stages"]).unwrap(),
            r#"{"level":{"depth":0.0}}"#
        );
    }

    /// Ledger ruling (Task 11): `features` lists every type of `TYPES`, 0 included, so a count
    /// constraint sees 0 on a map without that type instead of a missing value.
    #[test]
    fn every_feature_type_is_counted_with_zero_included() {
        let counts = BTreeMap::from([("chime".to_owned(), 2)]);
        let s = summary(serde_json::json!({}), &counts, &[], &[]).unwrap();
        let features = s["features"].as_object().unwrap();
        let types: Vec<&str> = crate::features::TYPES.iter().map(|(_, t)| *t).collect();
        let mut sorted = types.clone();
        sorted.sort_unstable();
        assert_eq!(
            features.keys().map(String::as_str).collect::<Vec<_>>(),
            sorted
        );
        for ty in types {
            let expected = if ty == "chime" { 2 } else { 0 };
            assert_eq!(features[ty], expected, "{ty}");
        }
        let c = BTreeMap::from([("features.gate-up".to_owned(), [0.0, 2.0])]);
        let results = crate::constraints::evaluate(&c, &s);
        assert_eq!(results["features_gate_up"], 0.0);
        assert_eq!(results["features_gate_up_ok"], true);
        assert_eq!(results["pass"], true);
    }
}
