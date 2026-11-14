//! The run summary (spec §5.14): the parity metrics nested by path, the feature counts, the
//! metrics of every stage and the warnings.

use std::collections::BTreeMap;

use serde_json::{Map, Value};

use crate::runner::StageFinding;
use crate::stage::Metrics;

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
/// `{"elevation_m": {"relief": ..}}`), then `features` (the count per type), `stages` (each
/// stage id with its metrics) and `findings` (`{stage, message}` per warning). Constraints read
/// it by dotted path (spec §5.6).
pub(crate) fn summary(
    metrics: Value,
    feature_counts: &BTreeMap<String, usize>,
    stage_metrics: &[(String, Metrics)],
    findings: &[StageFinding],
) -> Value {
    let mut out = Map::new();
    if let Value::Object(flat) = metrics {
        for (path, value) in flat {
            insert_nested(&mut out, &path, value);
        }
    }
    let features: Map<String, Value> = feature_counts
        .iter()
        .map(|(kind, &n)| (kind.clone(), Value::from(n)))
        .collect();
    out.insert("features".to_owned(), Value::Object(features));
    let stages: Map<String, Value> = stage_metrics
        .iter()
        .map(|(id, m)| {
            let values = m.0.iter().map(|(k, &v)| (k.clone(), Value::from(v)));
            (id.clone(), Value::Object(values.collect()))
        })
        .collect();
    out.insert("stages".to_owned(), Value::Object(stages));
    let findings = findings
        .iter()
        .map(|f| serde_json::json!({"stage": f.stage, "message": f.message}))
        .collect();
    out.insert("findings".to_owned(), Value::Array(findings));
    Value::Object(out)
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
        let counts = BTreeMap::from([("settlement".to_owned(), 3), ("chime".to_owned(), 2)]);
        let mut hydrology = Metrics::default();
        hydrology.0.insert("lake_count".to_owned(), 2.0);
        let stages = [("hydrology".to_owned(), hydrology)];
        let findings = [StageFinding {
            stage: "travel".to_owned(),
            message: "slow".to_owned(),
        }];
        let s = summary(metrics, &counts, &stages, &findings);
        assert_eq!(s["elevation_m"]["relief"], 1134.0);
        assert_eq!(s["precip_mm_yr"]["median"], 751.0);
        assert_eq!(s["lake_pct"], 6.25);
        assert_eq!(
            s["features"],
            serde_json::json!({"chime": 2, "settlement": 3})
        );
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
}
