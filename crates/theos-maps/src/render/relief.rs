//! The ground the people's renders share: the pixmap of the frame painted per pixel from the
//! hillshade, and the water of `base` drawn over it.

use tiny_skia::Pixmap;

use super::base::{draw_rivers, fill_lakes};
use super::canvas::{self, sample};
use super::palette::Rgb;
use super::shade::hillshade;
use super::{RenderError, RenderInput};
use crate::grid::Grid;
use crate::stages::terrain::hydrology::WaterFeatures;

/// Paints every pixel of `pixmap` with `colour(y, x, hillshade)`, calling `check` before every
/// row; an error from `check` (the run's cancel) stops the pass.
pub(crate) fn paint_rows(
    pixmap: &mut Pixmap,
    shade: &Grid<f64>,
    mut check: impl FnMut() -> Result<(), RenderError>,
    colour: impl Fn(usize, usize, f64) -> Rgb,
) -> Result<(), RenderError> {
    for (y, row) in canvas::rows_mut(pixmap).enumerate() {
        check()?;
        for (x, px) in row.iter_mut().enumerate() {
            canvas::put(px, colour(y, x, sample(shade, y, x)));
        }
    }
    Ok(())
}

/// The frame-exact pixmap of the world painted by `colour(y, x, hillshade)`, hillshade in
/// `[0, 1]` sampled at the pixel.
pub(crate) fn paint(
    input: &RenderInput<'_>,
    colour: impl Fn(usize, usize, f64) -> Rgb,
) -> Result<Pixmap, RenderError> {
    input.check_cancel()?;
    let height = input.f64("height")?;
    let frame = *input.world.frame();
    let mut pixmap = canvas::pixmap(&frame)?;
    let shade = hillshade(height, frame.cell_m);
    paint_rows(&mut pixmap, &shade, || input.check_cancel(), colour)?;
    Ok(pixmap)
}

/// Draws the rivers and then the lakes of the world over `pixmap`, as `base` does.
pub(crate) fn water(pixmap: &mut Pixmap, input: &RenderInput<'_>) -> Result<(), RenderError> {
    let water = input.value::<WaterFeatures>("water_features")?;
    let lake = input.mask("lake")?;
    let metres_per_px = canvas::metres_per_px(input.world.frame());
    let check = || input.check_cancel();
    draw_rivers(pixmap, &water.rivers, metres_per_px, check)?;
    fill_lakes(pixmap, lake, check)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::test_support::TestWorld;

    #[test]
    fn rows_stop_when_the_check_fails() {
        let mut pixmap = Pixmap::new(4, 4).unwrap();
        let shade = Grid::filled(2, 2, 0.5);
        let mut left = 2;
        let check = || {
            if left == 0 {
                return Err(RenderError::Cancelled);
            }
            left -= 1;
            Ok(())
        };
        let result = paint_rows(&mut pixmap, &shade, check, |_, _, _| [9, 9, 9]);
        assert!(matches!(result, Err(RenderError::Cancelled)));
        let (pixels, _) = pixmap.data().as_chunks::<4>();
        assert_eq!(pixels[0], [9, 9, 9, 255]);
        assert_eq!(pixels[8], [0; 4], "row 2 is never reached");
    }

    #[test]
    fn the_ground_is_painted_from_the_hillshade_and_the_lake_is_flat() {
        let mut w = TestWorld::flat(6, 6, 50.0);
        w.lake(1..3, 1..3);
        let cancel = crate::progress::CancelToken::new();
        let input = w.input(&cancel);
        let mut pixmap = paint(&input, |_, _, shade| {
            let v = crate::kernels::num::to_u8_unit(shade);
            [v, v, v]
        })
        .unwrap();
        water(&mut pixmap, &input).unwrap();
        let image = crate::render::test_support::decode(&canvas::encode_png(&pixmap).unwrap());
        assert_eq!(image.rgb(3, 3), crate::render::palette::LAKE);
        assert_ne!(image.rgb(9, 9), crate::render::palette::LAKE);
    }
}
