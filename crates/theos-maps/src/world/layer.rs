//! The kinds of layers a world stores.

use super::value::WorldValue;
use crate::grid::Grid;

/// What a layer holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerKind {
    /// Floats, 8 bytes per cell.
    F64,
    /// Bytes.
    U8,
    /// Booleans, one byte per cell in canonical bytes.
    Mask,
    /// Unsigned 32-bit integers.
    U32,
    /// A typed non-raster value, named by its type.
    Value(&'static str),
}

impl std::fmt::Display for LayerKind {
    /// The kind as words for messages: `an f64 raster`, `a mask`, ``a `climate` value``.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::F64 => f.write_str("an f64 raster"),
            Self::U8 => f.write_str("a u8 raster"),
            Self::Mask => f.write_str("a mask"),
            Self::U32 => f.write_str("a u32 raster"),
            Self::Value(type_name) => write!(f, "a `{type_name}` value"),
        }
    }
}

/// A named layer of a world.
#[derive(Debug)]
pub enum Layer {
    /// A float raster.
    F64(Grid<f64>),
    /// A byte raster.
    U8(Grid<u8>),
    /// A boolean raster.
    Mask(Grid<bool>),
    /// A `u32` raster.
    U32(Grid<u32>),
    /// A typed non-raster value.
    Value(Box<dyn WorldValue>),
}

impl Layer {
    /// The kind of this layer.
    #[must_use]
    pub fn kind(&self) -> LayerKind {
        match self {
            Self::F64(_) => LayerKind::F64,
            Self::U8(_) => LayerKind::U8,
            Self::Mask(_) => LayerKind::Mask,
            Self::U32(_) => LayerKind::U32,
            Self::Value(v) => LayerKind::Value(v.type_name()),
        }
    }

    /// The shape `(rows, cols)` of a raster layer; `None` for a value.
    #[must_use]
    pub fn shape(&self) -> Option<(usize, usize)> {
        match self {
            Self::F64(g) => Some((g.rows(), g.cols())),
            Self::U8(g) => Some((g.rows(), g.cols())),
            Self::Mask(g) => Some((g.rows(), g.cols())),
            Self::U32(g) => Some((g.rows(), g.cols())),
            Self::Value(_) => None,
        }
    }
}

/// Bytes one cell takes in memory budgets; a value counts as 8 (ruling L7).
#[must_use]
pub fn bytes_per_cell(kind: LayerKind) -> u64 {
    match kind {
        LayerKind::F64 | LayerKind::Value(_) => 8,
        LayerKind::U8 | LayerKind::Mask => 1,
        LayerKind::U32 => 4,
    }
}

#[cfg(test)]
mod tests {
    use super::LayerKind;

    #[test]
    fn kinds_read_as_words_in_messages() {
        assert_eq!(LayerKind::F64.to_string(), "an f64 raster");
        assert_eq!(LayerKind::U8.to_string(), "a u8 raster");
        assert_eq!(LayerKind::Mask.to_string(), "a mask");
        assert_eq!(LayerKind::U32.to_string(), "a u32 raster");
        assert_eq!(LayerKind::Value("climate").to_string(), "a `climate` value");
    }
}
