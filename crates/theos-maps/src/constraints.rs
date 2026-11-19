//! Constraint results with the protocol's keys (`docs/generators.md` §3).

use std::collections::BTreeMap;

use serde_json::{Map, Value};

use crate::features::TYPES;
use crate::metrics::{CHIME_WALK, PATHS};
use crate::recipe::PlannedStage;
use crate::stage::TypedStage;
use crate::stages::terrain::travel::Travel;
use crate::summary;

/// The result key of a summary path: lower case, every character outside `[a-z0-9]` an `_`.
#[must_use]
pub fn result_key(path: &str) -> String {
    path.to_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_lowercase() || c.is_ascii_digit() {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn lookup(summary: &Value, path: &str) -> Option<f64> {
    path.split('.')
        .try_fold(summary, |value, segment| value.get(segment))?
        .as_f64()
}

/// Checks each constraint against the summary. For a path with key `k` the map holds `k` (when
/// the summary has a number there), `k_min`, `k_max` and `k_ok`; `pass` is true when every
/// constraint holds. A missing or non-numeric value fails its constraint.
///
/// Every number it writes is finite, as JSON needs: `k` comes from a JSON number of the summary,
/// and the bounds are a plan's, which check 5 of `plan` keeps finite.
#[must_use]
pub(crate) fn evaluate(
    constraints: &BTreeMap<String, [f64; 2]>,
    summary: &Value,
) -> Map<String, Value> {
    let mut out = Map::new();
    let mut pass = true;
    for (path, [min, max]) in constraints {
        let key = result_key(path);
        let value = lookup(summary, path);
        let ok = value.is_some_and(|v| *min <= v && v <= *max);
        pass &= ok;
        if let Some(v) = value {
            out.insert(key.clone(), Value::from(v));
        }
        out.insert(format!("{key}_min"), Value::from(*min));
        out.insert(format!("{key}_max"), Value::from(*max));
        out.insert(format!("{key}_ok"), Value::from(ok));
    }
    out.insert("pass".to_owned(), Value::from(pass));
    out
}

/// Whether a constraint may name `path` in a recipe with these planned stages.
pub(crate) fn is_known_path(path: &str, stages: &[PlannedStage]) -> bool {
    if path == CHIME_WALK {
        return stages.iter().any(|s| s.id == Travel::ID);
    }
    if PATHS.contains(&path) {
        return true;
    }
    let under = |key: &str| {
        path.strip_prefix(key)
            .and_then(|rest| rest.strip_prefix('.'))
    };
    if let Some(kind) = under(summary::FEATURES) {
        return TYPES.iter().any(|(_, t)| *t == kind);
    }
    if let Some((id, metric)) = under(summary::STAGES).and_then(|rest| rest.split_once('.')) {
        return stages
            .iter()
            .any(|s| s.id == id && s.metric_names.contains(&metric));
    }
    false
}

#[cfg(test)]
mod tests;
