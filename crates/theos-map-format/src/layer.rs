//! Layer values, their little-endian bytes and hashes (`docs/map-package.md` §4.1–4.2).

use sha2::{Digest, Sha256};

use crate::{DType, FormatError};

/// The raw values of one layer, row by row from north to south.
#[derive(Debug, Clone, PartialEq)]
pub enum LayerData {
    /// 32-bit floats.
    F32(Vec<f32>),
    /// Unsigned 8-bit integers.
    U8(Vec<u8>),
    /// Unsigned 16-bit integers.
    U16(Vec<u16>),
}

/// Identifies a layer in a package: its level (if any) and name. Orders by level, then name.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LayerKey {
    /// The level id; `None` on a map without levels.
    pub level: Option<String>,
    /// The layer name.
    pub name: String,
}

impl LayerData {
    /// The sample type.
    #[must_use]
    pub fn dtype(&self) -> DType {
        match self {
            Self::F32(_) => DType::F32,
            Self::U8(_) => DType::U8,
            Self::U16(_) => DType::U16,
        }
    }

    /// The number of values.
    #[must_use]
    pub fn len(&self) -> usize {
        match self {
            Self::F32(v) => v.len(),
            Self::U8(v) => v.len(),
            Self::U16(v) => v.len(),
        }
    }

    /// True when there are no values.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The little-endian bytes of the values; `-0.0` is written as `+0.0`.
    ///
    /// # Errors
    ///
    /// [`FormatError::NonFinite`] with the index of the first NaN or infinity.
    pub fn to_bytes(&self) -> Result<Vec<u8>, FormatError> {
        match self {
            Self::F32(values) => {
                let mut bytes = Vec::with_capacity(values.len().saturating_mul(4));
                for (index, value) in values.iter().enumerate() {
                    if !value.is_finite() {
                        return Err(FormatError::NonFinite { index });
                    }
                    // Adding +0.0 turns -0.0 into +0.0 and changes nothing else.
                    bytes.extend_from_slice(&(value + 0.0).to_le_bytes());
                }
                Ok(bytes)
            }
            Self::U8(values) => Ok(values.clone()),
            Self::U16(values) => Ok(values.iter().flat_map(|v| v.to_le_bytes()).collect()),
        }
    }

    /// Decodes `cells` values of `dtype` from little-endian `bytes`.
    ///
    /// # Errors
    ///
    /// [`FormatError::LayerSize`] when `bytes` is not `cells` values long, and
    /// [`FormatError::NonFinite`] when a float value is NaN or an infinity.
    pub fn from_bytes(dtype: DType, bytes: &[u8], cells: usize) -> Result<Self, FormatError> {
        let size = match dtype {
            DType::F32 => 4,
            DType::U8 => 1,
            DType::U16 => 2,
        };
        let expected = cells.saturating_mul(size);
        if bytes.len() != expected {
            return Err(FormatError::LayerSize {
                expected,
                found: bytes.len(),
            });
        }
        Ok(match dtype {
            DType::F32 => {
                let values: Vec<f32> = bytes
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .map(|c| f32::from_le_bytes(*c))
                    .collect();
                if let Some(index) = values.iter().position(|v| !v.is_finite()) {
                    return Err(FormatError::NonFinite { index });
                }
                Self::F32(values)
            }
            DType::U8 => Self::U8(bytes.to_vec()),
            DType::U16 => Self::U16(
                bytes
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|c| u16::from_le_bytes(*c))
                    .collect(),
            ),
        })
    }

    /// The smallest and largest decoded value (`offset + raw · scale`), `None` when empty.
    #[must_use]
    pub fn decoded_range(&self, scale: Option<f64>, offset: Option<f64>) -> Option<(f64, f64)> {
        let scale = scale.unwrap_or(1.0);
        let offset = offset.unwrap_or(0.0);
        let raw: Vec<f64> = match self {
            Self::F32(v) => v.iter().map(|&x| f64::from(x)).collect(),
            Self::U8(v) => v.iter().map(|&x| f64::from(x)).collect(),
            Self::U16(v) => v.iter().map(|&x| f64::from(x)).collect(),
        };
        let mut values = raw.into_iter().map(|r| offset + r * scale);
        let first = values.next()?;
        Some(values.fold((first, first), |(lo, hi), v| {
            (if v < lo { v } else { lo }, if v > hi { v } else { hi })
        }))
    }
}

/// The SHA-256 of `bytes` as lower-case hex.
#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut hex = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        hex.push(char::from(DIGITS[usize::from(byte >> 4)]));
        hex.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    hex
}

#[cfg(test)]
mod tests {
    use super::*;

    type R = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn f32_bytes_are_little_endian_and_zero_is_positive() -> R {
        let bytes = LayerData::F32(vec![1.0, -0.0]).to_bytes()?;
        assert_eq!(bytes, [0, 0, 128, 63, 0, 0, 0, 0]);
        Ok(())
    }

    #[test]
    fn u16_bytes_are_little_endian() -> R {
        assert_eq!(LayerData::U16(vec![258]).to_bytes()?, [2, 1]);
        assert_eq!(LayerData::U8(vec![7, 9]).to_bytes()?, [7, 9]);
        Ok(())
    }

    #[test]
    fn nan_and_infinity_are_refused_with_their_index() {
        let nan = LayerData::F32(vec![0.0, f32::NAN]).to_bytes();
        assert!(matches!(nan, Err(FormatError::NonFinite { index: 1 })));
        let inf = LayerData::F32(vec![f32::INFINITY]).to_bytes();
        assert!(matches!(inf, Err(FormatError::NonFinite { index: 0 })));
    }

    #[test]
    fn from_bytes_checks_the_size() {
        let err = LayerData::from_bytes(DType::F32, &[0; 7], 2);
        assert!(matches!(
            err,
            Err(FormatError::LayerSize {
                expected: 8,
                found: 7
            })
        ));
        assert!(err.is_err_and(|e| e.to_string().contains('8') && e.to_string().contains('7')));
    }

    #[test]
    fn from_bytes_round_trips_every_dtype() -> R {
        for data in [
            LayerData::F32(vec![1.5, -2.25, 0.0]),
            LayerData::U8(vec![0, 1, 255]),
            LayerData::U16(vec![0, 258, 65535]),
        ] {
            let back = LayerData::from_bytes(data.dtype(), &data.to_bytes()?, data.len())?;
            assert_eq!(back, data);
        }
        Ok(())
    }

    #[test]
    fn from_bytes_refuses_non_finite_floats() {
        let bytes = f32::NAN.to_le_bytes();
        let err = LayerData::from_bytes(DType::F32, &bytes, 1);
        assert!(matches!(err, Err(FormatError::NonFinite { index: 0 })));
    }

    #[test]
    fn sha256_of_abc() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn decoded_range_applies_scale_and_offset() {
        let (lo, hi) = LayerData::U8(vec![0, 255])
            .decoded_range(Some(1.0 / 255.0), None)
            .unwrap_or((f64::NAN, f64::NAN));
        assert!((lo - 0.0).abs() < 1e-12 && (hi - 1.0).abs() < 1e-12);
        let (lo, hi) = LayerData::U16(vec![10, 20])
            .decoded_range(Some(2.0), Some(1.0))
            .unwrap_or((f64::NAN, f64::NAN));
        assert!((lo - 21.0).abs() < 1e-12 && (hi - 41.0).abs() < 1e-12);
        let (lo, hi) = LayerData::F32(vec![3.0, -1.0, 2.0])
            .decoded_range(None, None)
            .unwrap_or((f64::NAN, f64::NAN));
        assert!((lo + 1.0).abs() < 1e-12 && (hi - 3.0).abs() < 1e-12);
        assert!(
            LayerData::F32(Vec::new())
                .decoded_range(None, None)
                .is_none()
        );
        assert!(LayerData::U8(Vec::new()).is_empty());
    }

    #[test]
    fn layer_keys_order_by_level_then_name() {
        let key = |l: Option<&str>, n: &str| LayerKey {
            level: l.map(str::to_owned),
            name: n.to_owned(),
        };
        let mut keys = vec![
            key(Some("b"), "a"),
            key(Some("a"), "z"),
            key(None, "z"),
            key(Some("a"), "c"),
        ];
        keys.sort();
        assert_eq!(
            keys,
            vec![
                key(None, "z"),
                key(Some("a"), "c"),
                key(Some("a"), "z"),
                key(Some("b"), "a")
            ]
        );
    }
}
