//! Canonical bytes and hashes of layers (golden hashes, G20).

use theos_map_format::{json, sha256_hex};

use super::layer::Layer;
use crate::kernels::num;

/// A float layer holds a NaN or an infinity, which has no canonical bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("cell {cell} is not finite")]
pub struct NonFinite {
    /// The flat index of the first offending cell.
    pub cell: usize,
}

/// The canonical bytes of `layer`: floats as little-endian with `-0.0` turned into `+0.0`, bytes
/// raw, masks as 0/1, `u32` as little-endian, values as canonical JSON.
///
/// # Errors
/// [`NonFinite`] with the first non-finite cell of a float layer. A value whose JSON cannot be
/// written (which `serde_json::Value` never is) reports cell 0.
pub fn canonical_bytes(layer: &Layer) -> Result<Vec<u8>, NonFinite> {
    match layer {
        Layer::F64(grid) => {
            let mut out = Vec::with_capacity(grid.len() * 8);
            for (cell, &x) in grid.as_slice().iter().enumerate() {
                if !x.is_finite() {
                    return Err(NonFinite { cell });
                }
                out.extend_from_slice(&num::canonical(x).to_le_bytes());
            }
            Ok(out)
        }
        Layer::U8(grid) => Ok(grid.as_slice().to_vec()),
        Layer::Mask(grid) => Ok(grid.as_slice().iter().map(|&b| u8::from(b)).collect()),
        Layer::U32(grid) => Ok(grid
            .as_slice()
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect()),
        Layer::Value(value) => {
            json::to_canonical_vec(&value.canonical_json()).map_err(|_| NonFinite { cell: 0 })
        }
    }
}

/// The SHA-256 of [`canonical_bytes`], as lowercase hex.
///
/// # Errors
/// As [`canonical_bytes`].
pub fn layer_sha256(layer: &Layer) -> Result<String, NonFinite> {
    canonical_bytes(layer).map(|bytes| sha256_hex(&bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::Grid;
    use crate::world::{Layer, WorldValue};

    fn f64_layer(v: Vec<f64>) -> Layer {
        Layer::F64(Grid::from_vec(1, v.len(), v).unwrap())
    }

    #[test]
    fn negative_zero_hashes_as_positive_zero() {
        let a = layer_sha256(&f64_layer(vec![-0.0, 1.0])).unwrap();
        let b = layer_sha256(&f64_layer(vec![0.0, 1.0])).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn a_non_finite_cell_is_reported() {
        assert_eq!(
            canonical_bytes(&f64_layer(vec![1.0, f64::INFINITY])),
            Err(NonFinite { cell: 1 })
        );
        assert_eq!(
            layer_sha256(&f64_layer(vec![f64::NAN])),
            Err(NonFinite { cell: 0 })
        );
    }

    #[test]
    fn integer_kinds_have_fixed_bytes() {
        let u8s = Layer::U8(Grid::from_vec(1, 2, vec![3, 255]).unwrap());
        assert_eq!(canonical_bytes(&u8s).unwrap(), [3, 255]);
        let mask = Layer::Mask(Grid::from_vec(1, 3, vec![true, false, true]).unwrap());
        assert_eq!(canonical_bytes(&mask).unwrap(), [1, 0, 1]);
        let u32s = Layer::U32(Grid::from_vec(1, 1, vec![0x0102_0304]).unwrap());
        assert_eq!(canonical_bytes(&u32s).unwrap(), [4, 3, 2, 1]);
        let f = f64_layer(vec![1.0]);
        assert_eq!(canonical_bytes(&f).unwrap(), 1.0_f64.to_le_bytes());
    }

    #[derive(Debug)]
    struct Json(serde_json::Value);
    impl WorldValue for Json {
        fn type_name(&self) -> &'static str {
            "json"
        }
        fn canonical_json(&self) -> serde_json::Value {
            self.0.clone()
        }
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }
    }

    #[test]
    fn values_hash_by_their_canonical_json() {
        let unsorted: serde_json::Value =
            serde_json::from_str(r#"{"b": 1, "a": {"d": 2, "c": 3}}"#).unwrap();
        let sorted: serde_json::Value =
            serde_json::from_str(r#"{"a": {"c": 3, "d": 2}, "b": 1}"#).unwrap();
        let a = layer_sha256(&Layer::Value(Box::new(Json(unsorted)))).unwrap();
        let b = layer_sha256(&Layer::Value(Box::new(Json(sorted.clone())))).unwrap();
        assert_eq!(a, b);
        let bytes = theos_map_format::json::to_canonical_vec(&sorted).unwrap();
        assert_eq!(a, theos_map_format::sha256_hex(&bytes));
    }
}
