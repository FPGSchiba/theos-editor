//! The cells of an interior as pixels: floor, walls and cover at two pixels per cell, shared by
//! `base` and the blueprints.

use tiny_skia::Pixmap;

use super::skin::Ground;
use crate::grid::Grid;
use crate::render::canvas::{self, PX_PER_CELL};
use crate::render::{RenderError, RenderInput};

/// The mask layers of the cells.
pub(super) struct Masks<'a> {
    walkable: &'a Grid<bool>,
    walls: &'a Grid<bool>,
    cover: &'a Grid<bool>,
    envelope: &'a Grid<bool>,
}

impl<'a> Masks<'a> {
    /// The masks of `input`'s world.
    pub(super) fn read(input: &RenderInput<'a>) -> Result<Self, RenderError> {
        Ok(Self {
            walkable: input.mask("walkable")?,
            walls: input.mask("walls")?,
            cover: input.mask("cover")?,
            envelope: input.mask("envelope")?,
        })
    }

    /// The colour of cell `(row, col)`; `None` leaves the pixel as it is.
    fn colour(
        &self,
        ground: &Ground,
        row: usize,
        col: usize,
    ) -> Option<crate::render::palette::Rgb> {
        if self.walls.get(row, col) {
            Some(ground.wall)
        } else if self.cover.get(row, col) {
            Some(ground.cover)
        } else if self.walkable.get(row, col) {
            Some(ground.floor)
        } else if self.envelope.get(row, col) {
            Some(ground.solid)
        } else {
            ground.outside
        }
    }
}

/// A transparent pixmap of the frame with every cell painted, the cancel token checked before
/// every pixel row.
///
/// # Errors
/// [`RenderError::Size`] for a frame without a pixmap; [`RenderError::Cancelled`].
pub(super) fn paint(
    input: &RenderInput<'_>,
    masks: &Masks<'_>,
    ground: &Ground,
) -> Result<Pixmap, RenderError> {
    let mut pixmap = canvas::pixmap(input.world.frame())?;
    for (y, row) in canvas::rows_mut(&mut pixmap).enumerate() {
        input.check_cancel()?;
        for (x, px) in row.iter_mut().enumerate() {
            let colour = masks.colour(ground, y / PX_PER_CELL, x / PX_PER_CELL);
            if let Some(colour) = colour {
                canvas::put(px, colour);
            }
        }
    }
    Ok(pixmap)
}
