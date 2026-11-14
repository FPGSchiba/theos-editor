//! The `omega` render (an extra render, spec §5.11): the detuning field omega over the
//! hillshade, with the ruins marked. Frame-exact, no text.

use tiny_skia::Pixmap;

use super::canvas::sample;
use super::palette::{self, Unit};
use super::styles::poi_mark;
use super::view::View;
use super::{RenderError, RenderInput, feats, relief};
use crate::kernels::num::{max, min, to_u8_unit};

/// The colours of omega 0, 0.5 and 1: blue, yellow, red.
const RAMP: [Unit; 3] = [[0.16, 0.23, 0.42], [0.95, 0.82, 0.42], [0.78, 0.19, 0.17]];
/// How much of the ground shows through the colour: the hillshade grey's share.
const GROUND_SHARE: f64 = 0.35;

/// The colour of omega `t`, clamped to `[0, 1]`.
fn ramp(t: f64) -> Unit {
    let t = min(max(t, 0.0), 1.0);
    if t < 0.5 {
        palette::mix(RAMP[0], RAMP[1], 2.0 * t)
    } else {
        palette::mix(RAMP[1], RAMP[2], 2.0 * t - 1.0)
    }
}

/// Draws `omega`.
pub(crate) fn omega(input: &RenderInput<'_>) -> Result<Pixmap, RenderError> {
    let field = input.f64("omega")?;
    let mut pixmap = relief::paint(input, |y, x, shade| {
        let grey = to_u8_unit(shade);
        let ground = palette::unit([grey; 3]);
        let c = palette::mix(ramp(sample(field, y, x)), ground, GROUND_SHARE);
        palette::rgb(palette::scale(c, 0.55 + 0.45 * shade))
    })?;
    let view = View::new(input.world.frame(), [0.0, 0.0]);
    let ruin = poi_mark("ruin");
    for (f, at) in feats::points(input.features, "poi") {
        input.check_cancel()?;
        if f.ty == "ruin" {
            ruin.draw(&mut pixmap, view.px(at));
        }
    }
    Ok(pixmap)
}
