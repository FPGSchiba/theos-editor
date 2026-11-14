//! The `base` render (spec §5.11): frame-exact at two pixels per cell, square pixels, no
//! symbols and no text. On the Atlas it lies under the pins (data model §8).
//!
//! Drawn in 0.3.0's order (`render_terrain.py`, `_base_rgb`):
//! 1. **Ground**, per pixel, from the cell rasters sampled at the pixel's centre: the rock
//!    colour (`rocks::props`) turns to soil on gentle slopes, then to grass, lush growth and
//!    forest by `vegetation`, to farmland around settlements and to snow by `snow`; the colour
//!    is then multiplied by the shaded relief (`shade::shading`).
//! 2. **Rivers** from `water_features`, as anti-aliased lines: each segment is as wide as the
//!    mean `width_m` of its two ends, and at least `MIN_RIVER_PX`, so thin rivers stay visible.
//! 3. **Lakes** from `lake`, per pixel, flat and last: every pixel of a lake cell is exactly the
//!    lake colour, whatever was drawn under it.
//!
//! The run's cancel token is checked at the start, on every pixel row of the ground and lake
//! passes and before every river line.

#[cfg(test)]
mod tests;

use tiny_skia::Pixmap;

use super::canvas::{self, PX_PER_CELL, sample};
use super::farmland::{self, FarmRow, Farmland};
use super::palette::{self, Rgb, Unit};
use super::shade::shading;
use super::{RenderError, RenderInput};
use crate::features::FeatureSet;
use crate::grid::Grid;
use crate::kernels::diff::slope;
use crate::kernels::num::{max, smoothstep};
use crate::stages::terrain::hydrology::{RiverLine, WaterFeatures};
use crate::world::MapFrame;

/// The narrowest river line, pixels (three quarters of a cell): rivers are a few metres wide,
/// far below a pixel of a map of 50-100 m cells, and must stay visible.
const MIN_RIVER_PX: f64 = 1.5;

/// How strongly farmland covers the ground (0.3.0).
const FARM_STRENGTH: f64 = 0.85;

/// The layers `base` draws.
struct Layers<'a> {
    height: &'a Grid<f64>,
    rock: &'a Grid<u8>,
    vegetation: &'a Grid<f64>,
    snow: &'a Grid<f64>,
    lake: &'a Grid<bool>,
    water: &'a WaterFeatures,
}

impl<'a> Layers<'a> {
    /// The layers of `input`'s world.
    fn read(input: &RenderInput<'a>) -> Result<Self, RenderError> {
        Ok(Self {
            height: input.f64("height")?,
            rock: input.u8("rock")?,
            vegetation: input.f64("vegetation")?,
            snow: input.f64("snow")?,
            lake: input.mask("lake")?,
            water: input.value::<WaterFeatures>("water_features")?,
        })
    }
}

/// The cell fields the ground is coloured from.
struct Ground<'a> {
    /// Rock colour.
    rock: Grid<Unit>,
    /// Slope tangent.
    slope: Grid<f64>,
    /// Brightness factor of the shaded relief.
    shading: Grid<f64>,
    vegetation: &'a Grid<f64>,
    snow: &'a Grid<f64>,
    farmland: Farmland,
}

impl Ground<'_> {
    /// The colour of the pixel in row `y`, column `x`; `farms` are the farms reaching row `y`.
    fn colour(&self, y: usize, x: usize, farms: &FarmRow<'_>) -> Rgb {
        let slope = sample(&self.slope, y, x);
        let farm = farms.share(x) * smoothstep(0.25, 0.08, slope);
        let field = if farm > 0.0 {
            farmland::field(x, y)
        } else {
            [0.0; 3]
        };
        let c = ground_colour(
            sample(&self.rock, y, x),
            slope,
            sample(self.vegetation, y, x),
            sample(self.snow, y, x),
            (farm * FARM_STRENGTH, field),
        );
        palette::rgb(palette::scale(c, sample(&self.shading, y, x)))
    }
}

/// The unshaded ground colour from the rock colour, the slope tangent, the vegetation density,
/// the snow cover and the farmland's share and field colour (0.3.0's thresholds).
fn ground_colour(rock: Unit, slope: f64, vegetation: f64, snow: f64, farm: (f64, Unit)) -> Unit {
    let unit = palette::unit;
    let c = palette::mix(rock, unit(palette::SOIL), smoothstep(0.55, 0.15, slope));
    let c = palette::mix(c, unit(palette::GRASS), smoothstep(0.08, 0.4, vegetation));
    let c = palette::mix(c, unit(palette::LUSH), smoothstep(0.4, 0.65, vegetation));
    let c = palette::mix(c, unit(palette::FOREST), smoothstep(0.62, 0.9, vegetation));
    let c = palette::mix(c, farm.1, farm.0);
    palette::mix(c, unit(palette::SNOW), snow)
}

impl<'a> Ground<'a> {
    /// The ground fields of `layers` on `frame`, with the farmland of `features`.
    fn new(layers: &Layers<'a>, frame: &MapFrame, features: &FeatureSet) -> Self {
        Self {
            rock: layers.rock.map(|code| palette::unit(palette::rock(code))),
            slope: slope(layers.height, frame.cell_m),
            shading: shading(layers.height, frame.cell_m),
            vegetation: layers.vegetation,
            snow: layers.snow,
            farmland: Farmland::new(features, canvas::metres_per_px(frame)),
        }
    }
}

/// Paints every pixel with its ground colour, calling `check` before every row; an error from
/// `check` (the run's cancel) stops the pass.
fn paint_ground(
    pixmap: &mut Pixmap,
    ground: &Ground<'_>,
    mut check: impl FnMut() -> Result<(), RenderError>,
) -> Result<(), RenderError> {
    for (y, row) in canvas::rows_mut(pixmap).enumerate() {
        check()?;
        let farms = ground.farmland.row(y);
        for (x, px) in row.iter_mut().enumerate() {
            canvas::put(px, ground.colour(y, x, &farms));
        }
    }
    Ok(())
}

/// Draws `rivers` (metres) on pixels of `metres_per_px`, calling `check` before every line.
pub(super) fn draw_rivers(
    pixmap: &mut Pixmap,
    rivers: &[RiverLine],
    metres_per_px: f64,
    mut check: impl FnMut() -> Result<(), RenderError>,
) -> Result<(), RenderError> {
    let px = |p: &[f64; 3]| [p[0] / metres_per_px, p[1] / metres_per_px];
    for line in rivers {
        check()?;
        for (ends, widths) in line.points.windows(2).zip(line.width_m.windows(2)) {
            let width = max(
                f64::midpoint(widths[0], widths[1]) / metres_per_px,
                MIN_RIVER_PX,
            );
            canvas::stroke(pixmap, px(&ends[0]), px(&ends[1]), width, palette::RIVER);
        }
    }
    Ok(())
}

/// Paints every pixel of a `lake` cell in the lake colour, calling `check` before every row.
pub(super) fn fill_lakes(
    pixmap: &mut Pixmap,
    lake: &Grid<bool>,
    mut check: impl FnMut() -> Result<(), RenderError>,
) -> Result<(), RenderError> {
    for (y, row) in canvas::rows_mut(pixmap).enumerate() {
        check()?;
        let r = y / PX_PER_CELL;
        for (x, px) in row.iter_mut().enumerate() {
            if lake.get(r, x / PX_PER_CELL) {
                canvas::put(px, palette::LAKE);
            }
        }
    }
    Ok(())
}

/// Draws `base`.
pub(crate) fn base(input: &RenderInput<'_>) -> Result<Pixmap, RenderError> {
    input.check_cancel()?;
    let layers = Layers::read(input)?;
    let frame = *input.world.frame();
    let mut pixmap = canvas::pixmap(&frame)?;
    let ground = Ground::new(&layers, &frame, input.features);
    let check = || input.check_cancel();
    paint_ground(&mut pixmap, &ground, check)?;
    let metres_per_px = canvas::metres_per_px(&frame);
    draw_rivers(&mut pixmap, &layers.water.rivers, metres_per_px, check)?;
    fill_lakes(&mut pixmap, layers.lake, check)?;
    Ok(pixmap)
}
