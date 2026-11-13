//! The renders of a terrain run (spec §5.11): PNG pictures of the world and its features.
//!
//! Every render is one module with one function that draws a `tiny_skia::Pixmap` from the world,
//! listed once in the crate's table of renders. Drawing runs on `tiny-skia`'s scalar pipeline
//! (no `simd`, float functions from `libm`) and PNGs are written with `png`'s pure-Rust fdeflate
//! compressor and a fixed row filter, so the same world gives the same bytes on every platform.
//!
//! `base` is frame-exact: two pixels per cell, square pixels, no symbols and no text. Labels of
//! the other renders use the design system's fonts (Inter, Spectral), which are embedded and
//! decoded once per process; `skrifa` reads their glyph outlines, and `tiny-skia` fills them
//! like any other shape. [`font_licences`] hands out their SIL OFL 1.1 texts, so the binaries
//! that embed this crate can ship them.

mod base;
mod canvas;
mod farmland;
mod fonts;
mod palette;
mod shade;
#[cfg(test)]
mod test_support;
mod text;

use tiny_skia::Pixmap;

use crate::features::FeatureSet;
use crate::grid::Grid;
use crate::progress::CancelToken;
use crate::world::{ValueType, World};

pub(crate) use self::canvas::encode_png;
pub use self::fonts::{FontError, font_licences};

/// Why a render failed.
#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    /// The run's cancel token was set; the render stopped at its next row.
    #[error("the render was cancelled")]
    Cancelled,
    /// A layer the render draws is absent from the world or has another type.
    #[error("the world has no usable layer `{layer}` for the render")]
    MissingLayer {
        /// The layer name.
        layer: &'static str,
    },
    /// The frame has no pixmap at two pixels per cell: it is empty or too large.
    #[error("a map of {rows} x {cols} cells cannot be drawn at two pixels per cell")]
    Size {
        /// Rows of the frame.
        rows: usize,
        /// Columns of the frame.
        cols: usize,
    },
    /// An embedded font could not be decoded.
    #[error(transparent)]
    Font(#[from] FontError),
    /// The PNG encoder failed.
    #[error("the PNG could not be encoded: {0}")]
    Png(#[from] png::EncodingError),
}

/// What a render draws from: the finished world, the features, the recipe's options for this
/// render (`[render.<name>]`), the run's cancel token and the title line.
pub(crate) struct RenderInput<'a> {
    /// The world after the last stage.
    pub world: &'a World,
    /// The features of the run.
    pub features: &'a FeatureSet,
    /// The recipe's table for this render, if it has one.
    #[cfg_attr(
        test,
        expect(dead_code, reason = "read by the `inworld` render, Task 40")
    )]
    pub options: Option<&'a toml::Table>,
    /// The run's cancel token.
    pub cancel: &'a CancelToken,
    /// The title line (recipe, seed, relief, lakes, Chime walk).
    #[cfg_attr(
        test,
        expect(dead_code, reason = "read by the `truth` render, Task 40")
    )]
    pub title: String,
}

impl<'a> RenderInput<'a> {
    /// [`RenderError::Cancelled`] once the run's token is set.
    pub(crate) fn check_cancel(&self) -> Result<(), RenderError> {
        if self.cancel.is_cancelled() {
            Err(RenderError::Cancelled)
        } else {
            Ok(())
        }
    }

    /// The float raster `layer`.
    pub(crate) fn f64(&self, layer: &'static str) -> Result<&'a Grid<f64>, RenderError> {
        self.world
            .f64(layer)
            .ok_or(RenderError::MissingLayer { layer })
    }

    /// The byte raster `layer`.
    pub(crate) fn u8(&self, layer: &'static str) -> Result<&'a Grid<u8>, RenderError> {
        self.world
            .u8(layer)
            .ok_or(RenderError::MissingLayer { layer })
    }

    /// The mask `layer`.
    pub(crate) fn mask(&self, layer: &'static str) -> Result<&'a Grid<bool>, RenderError> {
        self.world
            .mask(layer)
            .ok_or(RenderError::MissingLayer { layer })
    }

    /// The world value `layer` as a `T`.
    pub(crate) fn value<T: ValueType>(&self, layer: &'static str) -> Result<&'a T, RenderError> {
        self.world
            .value::<T>(layer)
            .ok_or(RenderError::MissingLayer { layer })
    }
}

/// A render: draws a pixmap from its input.
pub(crate) type RenderFn = fn(&RenderInput<'_>) -> Result<Pixmap, RenderError>;

/// Every render by name. A new render is one module and one line here.
pub(crate) const RENDERS: &[(&str, RenderFn)] = &[("base", base::base)];

/// The render `name`, if there is one.
pub(crate) fn render(name: &str) -> Option<RenderFn> {
    RENDERS
        .iter()
        .find(|(id, _)| *id == name)
        .map(|&(_, draw)| draw)
}
