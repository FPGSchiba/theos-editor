//! The interior `base` render (spec §5.11): frame-exact at two pixels per cell, floor, walls
//! and cover, blank outside the envelope; no symbols and no text. On the Atlas it lies under
//! the pins.

use tiny_skia::Pixmap;

use super::cells::{self, Masks};
use super::skin::BASE;
use crate::render::{RenderError, RenderInput};
use crate::stage::LayerSpec;

/// The layers `base` reads (ledger ruling P3).
pub(super) const NEEDS: &[LayerSpec] = &[
    LayerSpec::mask("walkable"),
    LayerSpec::mask("walls"),
    LayerSpec::mask("cover"),
    LayerSpec::mask("envelope"),
];

/// Draws `base`.
pub(crate) fn base(input: &RenderInput<'_>) -> Result<Pixmap, RenderError> {
    input.check_cancel()?;
    let masks = Masks::read(input)?;
    cells::paint(input, &masks, &BASE)
}
