//! Canonical JSON: what `serde_json`'s pretty printer writes for a value whose object keys are in
//! lexicographic order, followed by one LF (`docs/map-package.md` §3).

use serde_json::{Map, Value};

use crate::error::FormatError;

/// Rebuilds `value` with the keys of every object sorted, recursively.
///
/// Sorting is explicit; it does not rely on `serde_json::Map` being a `BTreeMap`.
#[must_use]
pub fn canonicalise(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut entries: Vec<(String, Value)> = map
                .iter()
                .map(|(key, item)| (key.clone(), canonicalise(item)))
                .collect();
            entries.sort_by(|a, b| a.0.cmp(&b.0));
            let mut sorted = Map::new();
            for (key, item) in entries {
                sorted.insert(key, item);
            }
            Value::Object(sorted)
        }
        Value::Array(items) => Value::Array(items.iter().map(canonicalise).collect()),
        other => other.clone(),
    }
}

/// Writes `value` as canonical JSON: sorted keys, two-space indentation, final LF.
///
/// # Errors
///
/// Returns [`FormatError::Json`] when the value cannot be serialised.
pub fn to_canonical_vec(value: &Value) -> Result<Vec<u8>, FormatError> {
    let mut bytes = Vec::new();
    serde_json::to_writer_pretty(&mut bytes, &canonicalise(value))?;
    bytes.push(b'\n');
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn canonical_json_is_serde_pretty_with_sorted_keys() -> Result<(), Box<dyn std::error::Error>> {
        let v = json!({"b": 1, "a": {"d": [1, 2], "c": 0.5}});
        let bytes = to_canonical_vec(&v)?;
        let expected = format!("{}\n", serde_json::to_string_pretty(&canonicalise(&v))?);
        assert_eq!(bytes, expected.into_bytes());
        let text = String::from_utf8(bytes)?;
        assert!(text.starts_with("{\n  \"a\": {\n    \"c\": 0.5,"));
        assert!(text.ends_with("}\n"));
        Ok(())
    }

    #[test]
    fn numbers_print_in_the_shortest_round_trip_form() -> Result<(), Box<dyn std::error::Error>> {
        for (value, text) in [
            (24000.0, "24000.0"),
            (62.5, "62.5"),
            (0.1, "0.1"),
            (1e-7, "1e-7"),
        ] {
            let bytes = to_canonical_vec(&json!(value))?;
            assert_eq!(String::from_utf8(bytes)?, format!("{text}\n"));
        }
        Ok(())
    }

    #[test]
    fn objects_inside_arrays_are_sorted() -> Result<(), Box<dyn std::error::Error>> {
        let bytes = to_canonical_vec(&json!([{"z": 1, "y": 2}]))?;
        assert_eq!(
            String::from_utf8(bytes)?,
            "[\n  {\n    \"y\": 2,\n    \"z\": 1\n  }\n]\n"
        );
        Ok(())
    }
}
