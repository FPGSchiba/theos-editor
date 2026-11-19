//! Canonical JSON: what `serde_json`'s pretty printer writes for a value whose object keys are in
//! lexicographic order and whose `-0.0` is `+0.0`, followed by one LF (`docs/map-package.md` §3,
//! spec §5.4). [`to_checked_value`] is the one checked conversion into it: it refuses NaN and
//! infinities, which `serde_json` would write as `null`.

use serde::Serialize;
use serde_json::{Map, Number, Value};

use crate::error::FormatError;
use crate::finite;

/// Rebuilds `value` with the keys of every object sorted, recursively, and every `-0.0` turned
/// into `+0.0`.
///
/// Sorting is explicit; it does not rely on `serde_json::Map` being a `BTreeMap`.
#[must_use]
pub fn canonicalise(value: &Value) -> Value {
    match value {
        Value::Number(n) if n.as_f64().is_some_and(|x| x == 0.0 && x.is_sign_negative()) => {
            Number::from_f64(0.0).map_or_else(|| value.clone(), Value::Number)
        }
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

/// `value` as a canonical JSON value (sorted keys, `-0.0` as `+0.0`), refusing NaN and
/// infinities anywhere in it.
///
/// # Errors
///
/// [`FormatError::JsonNotFinite`] when `value` holds a NaN or an infinity;
/// [`FormatError::Json`] when it cannot be turned into JSON (a map with keys that are not
/// strings).
pub fn to_checked_value<T: Serialize + ?Sized>(value: &T) -> Result<Value, FormatError> {
    if !finite::is_finite(value) {
        return Err(FormatError::JsonNotFinite);
    }
    Ok(canonicalise(&serde_json::to_value(value)?))
}

/// Writes `value` as canonical JSON: sorted keys, `-0.0` as `+0.0`, two-space indentation, final
/// LF.
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
    fn the_checked_conversion_refuses_non_finite_numbers() -> Result<(), Box<dyn std::error::Error>>
    {
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let err = to_checked_value(&vec![1.0, bad]).err();
            assert!(
                matches!(err, Some(FormatError::JsonNotFinite)),
                "{bad}: {err:?}"
            );
        }
        let value = to_checked_value(&std::collections::BTreeMap::from([("b", -0.0), ("a", 2.5)]))?;
        assert_eq!(serde_json::to_string(&value)?, r#"{"a":2.5,"b":0.0}"#);
        let keys = std::collections::BTreeMap::from([((1, 2), 0.5)]);
        assert!(matches!(to_checked_value(&keys), Err(FormatError::Json(_))));
        Ok(())
    }

    // Spec §5.4 and §5.13: -0.0 is written as +0.0.
    #[test]
    fn negative_zero_is_written_as_positive_zero() -> Result<(), Box<dyn std::error::Error>> {
        let bytes = to_canonical_vec(&json!({"a": [-0.0, 1.5], "b": -0.0}))?;
        assert_eq!(
            String::from_utf8(bytes)?,
            "{
  \"a\": [
    0.0,
    1.5
  ],
  \"b\": 0.0
}
"
        );
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
