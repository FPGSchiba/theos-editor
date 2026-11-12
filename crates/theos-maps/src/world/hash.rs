//! Canonical bytes and hashes of layers (golden hashes, G20).

use theos_map_format::{json, sha256_hex};

use super::layer::Layer;
use crate::kernels::num;

/// Why a layer has no canonical bytes.
#[derive(Debug, thiserror::Error)]
pub enum LayerBytesError {
    /// A float layer holds a NaN or an infinity.
    #[error("cell {cell} is not finite")]
    NonFinite {
        /// The flat index of the first offending cell.
        cell: usize,
    },
    /// The JSON of a value layer could not be written.
    #[error("the value's canonical JSON could not be written")]
    Json(#[from] theos_map_format::FormatError),
}

/// The canonical bytes of `layer`: floats as little-endian with `-0.0` turned into `+0.0`, bytes
/// raw, masks as 0/1, `u32` as little-endian, values as canonical JSON.
///
/// # Errors
/// [`LayerBytesError::NonFinite`] with the first non-finite cell of a float layer;
/// [`LayerBytesError::Json`] when a value's JSON cannot be written, which cannot occur for a
/// `serde_json::Value` today.
pub fn canonical_bytes(layer: &Layer) -> Result<Vec<u8>, LayerBytesError> {
    match layer {
        Layer::F64(grid) => {
            let mut out = Vec::with_capacity(grid.len() * 8);
            for (cell, &x) in grid.as_slice().iter().enumerate() {
                if !x.is_finite() {
                    return Err(LayerBytesError::NonFinite { cell });
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
        Layer::Value(value) => Ok(json::to_canonical_vec(&value.canonical_json())?),
    }
}

/// The SHA-256 of [`canonical_bytes`], as lowercase hex.
///
/// # Errors
/// As [`canonical_bytes`].
pub fn layer_sha256(layer: &Layer) -> Result<String, LayerBytesError> {
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
        assert!(matches!(
            canonical_bytes(&f64_layer(vec![1.0, f64::INFINITY])),
            Err(LayerBytesError::NonFinite { cell: 1 })
        ));
        assert!(matches!(
            layer_sha256(&f64_layer(vec![f64::NAN])),
            Err(LayerBytesError::NonFinite { cell: 0 })
        ));
        let message = layer_sha256(&f64_layer(vec![f64::NAN]))
            .unwrap_err()
            .to_string();
        assert_eq!(message, "cell 0 is not finite");
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
        let value = serde_json::json!({"b": 1, "a": [2, 1]});
        let layer = Layer::Value(Box::new(Json(value)));
        let text = "{
  \"a\": [
    2,
    1
  ],
  \"b\": 1
}
";
        assert_eq!(canonical_bytes(&layer).unwrap(), text.as_bytes());
        assert_eq!(
            layer_sha256(&layer).unwrap(),
            theos_map_format::sha256_hex(text.as_bytes())
        );
    }
}
