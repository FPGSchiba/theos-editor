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
