//! The world: a frame and a store of named layers.
//!
//! Layers are rasters of the frame's shape or typed non-raster values. Every layer has canonical
//! bytes and a SHA-256 ([`layer_sha256`]) that golden hashes pin.

mod hash;
mod layer;
mod value;

use std::collections::BTreeMap;

pub use hash::{LayerBytesError, canonical_bytes, layer_sha256};
pub use layer::{Layer, LayerKind, bytes_per_cell};
pub use value::{ValueType, WorldValue};

use crate::grid::Grid;
use crate::kernels::num;

/// The geometry of a map: `rows` x `cols` cells of `cell_m` metres (`docs/map-package.md` §2).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MapFrame {
    /// Number of rows (south).
    pub rows: usize,
    /// Number of columns (east).
    pub cols: usize,
    /// Cell edge in metres.
    pub cell_m: f64,
}

impl MapFrame {
    /// The extent `[east, south]` in metres.
    #[must_use]
    pub fn size_m(&self) -> [f64; 2] {
        [
            num::to_f64(self.cols) * self.cell_m,
            num::to_f64(self.rows) * self.cell_m,
        ]
    }

    /// The number of cells.
    #[must_use]
    pub fn cells(&self) -> usize {
        self.rows * self.cols
    }

    /// The area of one cell in square metres.
    #[must_use]
    pub fn cell_area_m2(&self) -> f64 {
        self.cell_m * self.cell_m
    }

    /// The centre `[x, y]` in metres of row `r`, column `c`.
    #[must_use]
    pub fn centre_m(&self, r: usize, c: usize) -> [f64; 2] {
        [
            (num::to_f64(c) + 0.5) * self.cell_m,
            (num::to_f64(r) + 0.5) * self.cell_m,
        ]
    }

    /// The `(row, col)` containing the point `(x, y)` in metres, clamped into the grid.
    #[must_use]
    pub fn cell_at_m(&self, x: f64, y: f64) -> (usize, usize) {
        (
            num::to_index(y / self.cell_m, self.rows),
            num::to_index(x / self.cell_m, self.cols),
        )
    }

    /// The `(row, col)` at the fractions `(fx, fy)` of the extent (G19: `fx` east, `fy` south),
    /// clamped into the grid.
    #[must_use]
    pub fn cell_at_fraction(&self, fx: f64, fy: f64) -> (usize, usize) {
        (
            num::to_index(fy * num::to_f64(self.rows), self.rows),
            num::to_index(fx * num::to_f64(self.cols), self.cols),
        )
    }
}

/// Errors of changing a [`World`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum WorldError {
    /// A raster layer does not have the frame's shape.
    #[error("layer {name:?} is {actual:?} cells, the frame is {expected:?}")]
    Shape {
        /// The layer name.
        name: String,
        /// The frame's `(rows, cols)`.
        expected: (usize, usize),
        /// The layer's `(rows, cols)`.
        actual: (usize, usize),
    },
}

/// A frame and its named layers, ordered by name.
#[derive(Debug)]
pub struct World {
    frame: MapFrame,
    layers: BTreeMap<String, Layer>,
}

impl World {
    /// An empty world over `frame`.
    #[must_use]
    pub fn new(frame: MapFrame) -> Self {
        Self {
            frame,
            layers: BTreeMap::new(),
        }
    }

    /// The frame.
    #[must_use]
    pub fn frame(&self) -> &MapFrame {
        &self.frame
    }

    /// The layer `name`.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&Layer> {
        self.layers.get(name)
    }

    /// Stores `layer` under `name`, replacing any layer of that name.
    ///
    /// # Errors
    /// [`WorldError::Shape`] when a raster layer is not `rows` x `cols` of the frame.
    pub fn insert(&mut self, name: &str, layer: Layer) -> Result<(), WorldError> {
        let expected = (self.frame.rows, self.frame.cols);
        if let Some(actual) = layer.shape().filter(|&shape| shape != expected) {
            return Err(WorldError::Shape {
                name: name.to_owned(),
                expected,
                actual,
            });
        }
        self.layers.insert(name.to_owned(), layer);
        Ok(())
    }

    /// Removes and returns the layer `name`.
    pub fn remove(&mut self, name: &str) -> Option<Layer> {
        self.layers.remove(name)
    }

    /// The float raster `name`; `None` when absent or of another kind.
    #[must_use]
    pub fn f64(&self, name: &str) -> Option<&Grid<f64>> {
        match self.get(name)? {
            Layer::F64(grid) => Some(grid),
            _ => None,
        }
    }

    /// The byte raster `name`; `None` when absent or of another kind.
    #[must_use]
    pub fn u8(&self, name: &str) -> Option<&Grid<u8>> {
        match self.get(name)? {
            Layer::U8(grid) => Some(grid),
            _ => None,
        }
    }

    /// The mask `name`; `None` when absent or of another kind.
    #[must_use]
    pub fn mask(&self, name: &str) -> Option<&Grid<bool>> {
        match self.get(name)? {
            Layer::Mask(grid) => Some(grid),
            _ => None,
        }
    }

    /// The `u32` raster `name`; `None` when absent or of another kind.
    #[must_use]
    pub fn u32(&self, name: &str) -> Option<&Grid<u32>> {
        match self.get(name)? {
            Layer::U32(grid) => Some(grid),
            _ => None,
        }
    }

    /// The value `name` as a `T`; `None` when absent, not a value, or of another type.
    #[must_use]
    pub fn value<T: ValueType>(&self, name: &str) -> Option<&T> {
        match self.get(name)? {
            Layer::Value(value) if value.type_name() == T::NAME => value.as_any().downcast_ref(),
            _ => None,
        }
    }

    /// The layer names in order.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.layers.keys().map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_geometry_follows_the_format() {
        let frame = MapFrame {
            rows: 4,
            cols: 8,
            cell_m: 10.0,
        };
        assert_eq!(frame.size_m(), [80.0, 40.0]);
        assert_eq!(frame.cells(), 32);
        assert!((frame.cell_area_m2() - 100.0).abs() < 1e-12);
        assert_eq!(frame.centre_m(0, 0), [5.0, 5.0]);
        assert_eq!(frame.centre_m(1, 2), [25.0, 15.0]);
        assert_eq!(frame.cell_at_fraction(0.999, 0.0), (0, 7));
        assert_eq!(frame.cell_at_fraction(2.0, -1.0), (0, 7));
        assert_eq!(frame.cell_at_fraction(0.0, 0.999), (3, 0));
        assert_eq!(frame.cell_at_m(25.0, 15.0), (1, 2));
        assert_eq!(frame.cell_at_m(1e9, 1e9), (3, 7));
    }

    fn world() -> World {
        World::new(MapFrame {
            rows: 2,
            cols: 3,
            cell_m: 1.0,
        })
    }

    #[test]
    fn insert_checks_the_raster_shape() {
        let mut w = world();
        let bad = Layer::F64(Grid::filled(3, 3, 0.0));
        assert!(matches!(w.insert("h", bad), Err(WorldError::Shape { .. })));
        assert!(w.get("h").is_none());
        assert!(w.insert("h", Layer::F64(Grid::filled(2, 3, 1.0))).is_ok());
        assert!(w.get("h").is_some());
        assert_eq!(w.names().collect::<Vec<_>>(), ["h"]);
        assert!(w.remove("h").is_some());
        assert!(w.remove("h").is_none());
        assert_eq!(w.frame().cols, 3);
    }

    #[derive(Debug)]
    struct Marker(u32);
    impl WorldValue for Marker {
        fn type_name(&self) -> &'static str {
            "marker"
        }
        fn canonical_json(&self) -> serde_json::Value {
            serde_json::json!({"n": self.0})
        }
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }
    }
    impl ValueType for Marker {
        const NAME: &'static str = "marker";
    }

    #[derive(Debug)]
    struct Other;
    impl WorldValue for Other {
        fn type_name(&self) -> &'static str {
            "other"
        }
        fn canonical_json(&self) -> serde_json::Value {
            serde_json::Value::Null
        }
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }
    }
    impl ValueType for Other {
        const NAME: &'static str = "other";
    }

    #[test]
    fn typed_access_returns_none_for_another_kind() {
        let mut w = world();
        w.insert("f", Layer::F64(Grid::filled(2, 3, 1.0))).unwrap();
        w.insert("u", Layer::U8(Grid::filled(2, 3, 1))).unwrap();
        w.insert("m", Layer::Mask(Grid::filled(2, 3, true)))
            .unwrap();
        w.insert("i", Layer::U32(Grid::filled(2, 3, 7))).unwrap();
        w.insert("v", Layer::Value(Box::new(Marker(3)))).unwrap();
        assert!(w.f64("f").is_some() && w.f64("u").is_none() && w.f64("zz").is_none());
        assert!(w.u8("u").is_some() && w.u8("f").is_none());
        assert!(w.mask("m").is_some() && w.mask("f").is_none());
        assert!(w.u32("i").is_some() && w.u32("f").is_none());
        assert_eq!(w.value::<Marker>("v").map(|m| m.0), Some(3));
        assert!(w.value::<Other>("v").is_none());
        assert!(w.value::<Marker>("f").is_none());
        assert_eq!(
            w.get("v").map(Layer::kind),
            Some(LayerKind::Value("marker"))
        );
        assert_eq!(w.get("f").map(Layer::kind), Some(LayerKind::F64));
        assert_eq!(w.get("u").map(Layer::kind), Some(LayerKind::U8));
        assert_eq!(w.get("m").map(Layer::kind), Some(LayerKind::Mask));
        assert_eq!(w.get("i").map(Layer::kind), Some(LayerKind::U32));
    }

    #[test]
    fn bytes_per_cell_per_kind() {
        assert_eq!(bytes_per_cell(LayerKind::F64), 8);
        assert_eq!(bytes_per_cell(LayerKind::U8), 1);
        assert_eq!(bytes_per_cell(LayerKind::Mask), 1);
        assert_eq!(bytes_per_cell(LayerKind::U32), 4);
        assert_eq!(bytes_per_cell(LayerKind::Value("x")), 8);
    }

    #[test]
    fn value_mutation_through_as_any_mut() {
        let mut m = Marker(1);
        if let Some(x) = WorldValue::as_any_mut(&mut m).downcast_mut::<Marker>() {
            x.0 = 5;
        }
        assert_eq!(m.0, 5);
    }
}
