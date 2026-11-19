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
mod contact_sheet;
mod farmland;
mod feats;
mod fonts;
mod geology;
mod inworld;
mod legend;
mod marks;
mod omega;
mod palette;
mod relief;
mod resources;
mod roads;
mod shade;
mod styles;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;
mod text;
mod title;
mod truth;
mod view;

use tiny_skia::Pixmap;

use crate::features::FeatureSet;
use crate::grid::Grid;
use crate::progress::CancelToken;
use crate::stage::LayerSpec;
use crate::stages::terrain::hydrology::WaterFeatures;
use crate::world::{ValueType, World};

pub(crate) use self::canvas::encode_png;
pub use self::contact_sheet::contact_sheet;
pub use self::fonts::{FontError, font_licences};
pub(crate) use self::title::line as title_line;

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
    /// A canvas of this many pixels cannot be made: it is empty or too large.
    #[error("a canvas of {width} x {height} pixels cannot be made")]
    Canvas {
        /// The width in pixels.
        width: usize,
        /// The height in pixels.
        height: usize,
    },
    /// An embedded font could not be decoded.
    #[error(transparent)]
    Font(#[from] FontError),
    /// A contact sheet needs at least one tile and one column.
    #[error("a contact sheet needs at least one tile and one column")]
    NoTiles,
    /// A tile of a contact sheet cannot be read.
    #[error("the tile `{}` cannot be used: {reason}", path.display())]
    Tile {
        /// The tile's file.
        path: std::path::PathBuf,
        /// What is wrong with it.
        reason: String,
    },
    /// The contact sheet cannot be written.
    #[error("the contact sheet cannot be written to `{}`: {source}", path.display())]
    Output {
        /// The output file.
        path: std::path::PathBuf,
        /// The I/O error.
        source: std::io::Error,
    },
    /// A render option of the recipe is not valid.
    #[error("the option `{key}` of render `{render}` is not valid")]
    Option {
        /// The render.
        render: &'static str,
        /// The option.
        key: &'static str,
    },
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
    pub options: Option<&'a toml::Table>,
    /// The run's cancel token.
    pub cancel: &'a CancelToken,
    /// The title line (recipe, seed, relief, lakes, Chime walk).
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

/// The height, which the relief of most renders is shaded from.
const HEIGHT_SPEC: LayerSpec = LayerSpec::f64("height");
/// The lake cells.
const LAKE_SPEC: LayerSpec = LayerSpec::mask("lake");
/// The river lines and lake outlines.
const WATER_FEATURES_SPEC: LayerSpec =
    LayerSpec::value("water_features", <WaterFeatures as ValueType>::NAME);

/// A render in the table of renders.
pub(crate) struct Render {
    /// Its name.
    pub name: &'static str,
    /// How it draws.
    pub draw: RenderFn,
    /// The layers it reads: `run` and `describe` refuse a recipe whose stages do not write them
    /// (ledger ruling P3). Features and roads are optional and not listed.
    pub needs: &'static [LayerSpec],
}

/// Every render by name. A new render is one module and one line here.
pub(crate) const RENDERS: &[Render] = &[
    Render {
        name: "base",
        draw: base::base,
        needs: base::NEEDS,
    },
    Render {
        name: "truth",
        draw: truth::truth,
        needs: truth::NEEDS,
    },
    Render {
        name: "inworld",
        draw: inworld::inworld,
        needs: inworld::NEEDS,
    },
    Render {
        name: "geology",
        draw: geology::geology,
        needs: geology::NEEDS,
    },
    Render {
        name: "resources",
        draw: resources::resources,
        needs: resources::NEEDS,
    },
    Render {
        name: "omega",
        draw: omega::omega,
        needs: omega::NEEDS,
    },
];

/// The render `name`, if there is one.
pub(crate) fn render(name: &str) -> Option<RenderFn> {
    RENDERS.iter().find(|r| r.name == name).map(|r| r.draw)
}

/// The layers the render `name` reads, if there is such a render.
pub(crate) fn needs(name: &str) -> Option<&'static [LayerSpec]> {
    RENDERS.iter().find(|r| r.name == name).map(|r| r.needs)
}
