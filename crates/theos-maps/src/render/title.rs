//! The title line of `truth` (spec §5.11): recipe, seed, relief, lakes and the Chime walk,
//! from the metrics module's values (§5.14), so the numbers are defined once.

use serde_json::Value;

/// `<recipe> · seed <seed> · relief <n> m · <n> % lake · Chime walk <n> min`; a part whose
/// metric is absent is left out.
pub(crate) fn line(recipe: &str, seed: u64, metrics: &Value) -> String {
    let number = |path: &str| metrics.get(path).and_then(Value::as_f64);
    let mut parts = vec![recipe.to_owned(), format!("seed {seed}")];
    if let Some(relief) = number("elevation_m.relief") {
        parts.push(format!("relief {relief} m"));
    }
    if let Some(lake) = number("lake_pct") {
        parts.push(format!("{lake} % lake"));
    }
    if let Some(walk) = number("travel.chime_to_chime_min") {
        parts.push(format!("Chime walk {walk} min"));
    }
    parts.join(" \u{b7} ")
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn the_title_names_what_the_metrics_have() {
        let metrics = json!({
            "elevation_m.relief": 1134.0,
            "lake_pct": 6.25,
            "travel.chime_to_chime_min": 12.3,
        });
        assert_eq!(
            line("hearth-valley", 7, &metrics),
            "hearth-valley \u{b7} seed 7 \u{b7} relief 1134 m \u{b7} 6.25 % lake \u{b7} Chime walk 12.3 min"
        );
        assert_eq!(line("r", 1, &json!({})), "r \u{b7} seed 1");
    }
}
