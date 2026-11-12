//! Typed access to the declared layers: reads, optional reads, and takes for in-place updates.
//! The rules are on [`StageContext`], under "Reading and taking layers".

use std::any::Any;

use super::StageContext;
use crate::grid::Grid;
use crate::stage::{Access, StageError};
use crate::world::{Layer, LayerKind, ValueType};

/// A cell type of a raster layer, tied to its [`Layer`] variant.
trait Cell: Copy + 'static {
    /// The kind of the layers holding cells of this type.
    const KIND: LayerKind;

    /// The grid of `layer`; `None` when it is of another kind.
    fn grid(layer: &Layer) -> Option<&Grid<Self>>;

    /// The grid of `layer`, by value; `None` when it is of another kind.
    fn into_grid(layer: Layer) -> Option<Grid<Self>>;
}

impl Cell for f64 {
    const KIND: LayerKind = LayerKind::F64;

    fn grid(layer: &Layer) -> Option<&Grid<Self>> {
        match layer {
            Layer::F64(grid) => Some(grid),
            _ => None,
        }
    }

    fn into_grid(layer: Layer) -> Option<Grid<Self>> {
        match layer {
            Layer::F64(grid) => Some(grid),
            _ => None,
        }
    }
}

impl Cell for u8 {
    const KIND: LayerKind = LayerKind::U8;

    fn grid(layer: &Layer) -> Option<&Grid<Self>> {
        match layer {
            Layer::U8(grid) => Some(grid),
            _ => None,
        }
    }

    fn into_grid(layer: Layer) -> Option<Grid<Self>> {
        match layer {
            Layer::U8(grid) => Some(grid),
            _ => None,
        }
    }
}

impl Cell for bool {
    const KIND: LayerKind = LayerKind::Mask;

    fn grid(layer: &Layer) -> Option<&Grid<Self>> {
        match layer {
            Layer::Mask(grid) => Some(grid),
            _ => None,
        }
    }

    fn into_grid(layer: Layer) -> Option<Grid<Self>> {
        match layer {
            Layer::Mask(grid) => Some(grid),
            _ => None,
        }
    }
}

impl Cell for u32 {
    const KIND: LayerKind = LayerKind::U32;

    fn grid(layer: &Layer) -> Option<&Grid<Self>> {
        match layer {
            Layer::U32(grid) => Some(grid),
            _ => None,
        }
    }

    fn into_grid(layer: Layer) -> Option<Grid<Self>> {
        match layer {
            Layer::U32(grid) => Some(grid),
            _ => None,
        }
    }
}

/// The value of `layer` as a `T`; `None` when it holds something else.
fn into_value<T: ValueType>(layer: Layer) -> Option<Box<T>> {
    match layer {
        Layer::Value(value) => {
            let value: Box<dyn Any> = value;
            value.downcast().ok()
        }
        _ => None,
    }
}

impl StageContext<'_> {
    /// The float raster `name`.
    ///
    /// # Errors
    /// See [reading and taking layers](StageContext#reading-and-taking-layers).
    pub fn f64(&self, name: &str) -> Result<&Grid<f64>, StageError> {
        self.raster(name)
    }

    /// The byte raster `name`.
    ///
    /// # Errors
    /// See [reading and taking layers](StageContext#reading-and-taking-layers).
    pub fn u8(&self, name: &str) -> Result<&Grid<u8>, StageError> {
        self.raster(name)
    }

    /// The mask `name`.
    ///
    /// # Errors
    /// See [reading and taking layers](StageContext#reading-and-taking-layers).
    pub fn mask(&self, name: &str) -> Result<&Grid<bool>, StageError> {
        self.raster(name)
    }

    /// The `u32` raster `name`.
    ///
    /// # Errors
    /// See [reading and taking layers](StageContext#reading-and-taking-layers).
    pub fn u32(&self, name: &str) -> Result<&Grid<u32>, StageError> {
        self.raster(name)
    }

    /// The float raster `name`, `None` when absent.
    ///
    /// # Errors
    /// See [reading and taking layers](StageContext#reading-and-taking-layers).
    pub fn f64_opt(&self, name: &str) -> Result<Option<&Grid<f64>>, StageError> {
        self.raster_opt(name)
    }

    /// The byte raster `name`, `None` when absent.
    ///
    /// # Errors
    /// See [reading and taking layers](StageContext#reading-and-taking-layers).
    pub fn u8_opt(&self, name: &str) -> Result<Option<&Grid<u8>>, StageError> {
        self.raster_opt(name)
    }

    /// The mask `name`, `None` when absent.
    ///
    /// # Errors
    /// See [reading and taking layers](StageContext#reading-and-taking-layers).
    pub fn mask_opt(&self, name: &str) -> Result<Option<&Grid<bool>>, StageError> {
        self.raster_opt(name)
    }

    /// The `u32` raster `name`, `None` when absent.
    ///
    /// # Errors
    /// See [reading and taking layers](StageContext#reading-and-taking-layers).
    pub fn u32_opt(&self, name: &str) -> Result<Option<&Grid<u32>>, StageError> {
        self.raster_opt(name)
    }

    /// Takes the float raster `name` out of the world, to write it back updated.
    ///
    /// # Errors
    /// See [reading and taking layers](StageContext#reading-and-taking-layers).
    pub fn take_f64(&mut self, name: &str) -> Result<Grid<f64>, StageError> {
        self.take_raster(name)
    }

    /// Takes the byte raster `name` out of the world, to write it back updated.
    ///
    /// # Errors
    /// See [reading and taking layers](StageContext#reading-and-taking-layers).
    pub fn take_u8(&mut self, name: &str) -> Result<Grid<u8>, StageError> {
        self.take_raster(name)
    }

    /// Takes the mask `name` out of the world, to write it back updated.
    ///
    /// # Errors
    /// See [reading and taking layers](StageContext#reading-and-taking-layers).
    pub fn take_mask(&mut self, name: &str) -> Result<Grid<bool>, StageError> {
        self.take_raster(name)
    }

    /// Takes the `u32` raster `name` out of the world, to write it back updated.
    ///
    /// # Errors
    /// See [reading and taking layers](StageContext#reading-and-taking-layers).
    pub fn take_u32(&mut self, name: &str) -> Result<Grid<u32>, StageError> {
        self.take_raster(name)
    }

    /// The value `name` as a `T`.
    ///
    /// # Errors
    /// See [reading and taking layers](StageContext#reading-and-taking-layers).
    pub fn value<T: ValueType>(&self, name: &str) -> Result<&T, StageError> {
        self.value_opt(name)?.ok_or_else(|| self.missing(name))
    }

    /// The value `name` as a `T`, `None` when absent.
    ///
    /// # Errors
    /// See [reading and taking layers](StageContext#reading-and-taking-layers).
    pub fn value_opt<T: ValueType>(&self, name: &str) -> Result<Option<&T>, StageError> {
        let kind = LayerKind::Value(T::NAME);
        self.check_declared(name, Access::Read, kind)?;
        match self.world.get(name) {
            None => Ok(None),
            Some(layer) => self
                .world
                .value::<T>(name)
                .map(Some)
                .ok_or_else(|| self.wrong_kind(name, kind, layer.kind())),
        }
    }

    /// Takes the value `name` out of the world, to write it back updated.
    ///
    /// # Errors
    /// See [reading and taking layers](StageContext#reading-and-taking-layers).
    pub fn take_value<T: ValueType>(&mut self, name: &str) -> Result<Box<T>, StageError> {
        self.check_declared(name, Access::Write, LayerKind::Value(T::NAME))?;
        self.value::<T>(name)?;
        self.world
            .remove(name)
            .and_then(into_value::<T>)
            .ok_or_else(|| self.missing(name))
    }

    /// The raster `name` of cells `T`.
    fn raster<T: Cell>(&self, name: &str) -> Result<&Grid<T>, StageError> {
        self.raster_opt(name)?.ok_or_else(|| self.missing(name))
    }

    /// The raster `name` of cells `T`, `None` when absent.
    fn raster_opt<T: Cell>(&self, name: &str) -> Result<Option<&Grid<T>>, StageError> {
        self.check_declared(name, Access::Read, T::KIND)?;
        match self.world.get(name) {
            None => Ok(None),
            Some(layer) => T::grid(layer)
                .map(Some)
                .ok_or_else(|| self.wrong_kind(name, T::KIND, layer.kind())),
        }
    }

    /// Takes the raster `name` of cells `T` out of the world; it is checked like a read first,
    /// so a refused take leaves the layer in place.
    fn take_raster<T: Cell>(&mut self, name: &str) -> Result<Grid<T>, StageError> {
        self.check_declared(name, Access::Write, T::KIND)?;
        self.raster::<T>(name)?;
        self.world
            .remove(name)
            .and_then(T::into_grid)
            .ok_or_else(|| self.missing(name))
    }
}
