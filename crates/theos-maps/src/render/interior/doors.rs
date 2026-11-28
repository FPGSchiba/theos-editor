//! Doors of `blueprint`: open ones as a short bar, locked ones in red with the key's name,
//! drops as an arrow from the room they start in.

use tiny_skia::Pixmap;

use super::geo::{self, Cell2, Placement};
use super::skin::Ink;
use crate::kernels::num::to_f32;
use crate::render::canvas::{self, LineStyle};
use crate::render::fonts::Face;
use crate::render::palette::Rgb;
use crate::render::{RenderError, RenderInput, text};
use crate::rooms::{Passage, RoomGraph};
use crate::stages::interior::space_core::{Door, Doors};

/// The width of a door bar, pixels.
const BAR_PX: f64 = 2.5;
/// The shortest door bar, raster cells.
const MIN_BAR_CELLS: f64 = 2.0;
/// Half the length of a drop arrow, pixels.
const ARROW_HALF_PX: f64 = 6.0;
/// The size of a key name, pixels.
const KEY_NAME_PX: f32 = 8.0;

/// The kinds of door that are present, for the legend.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(super) struct Present {
    /// An open door exists.
    pub open: bool,
    /// A locked door exists.
    pub locked: bool,
    /// A drop exists.
    pub drop: bool,
}

/// The rooms at the two ends of a door, by centre.
type Ends = (Option<Cell2>, Option<Cell2>);

/// Draws every door of `doors`; the cancel token is checked before each.
///
/// # Errors
/// [`RenderError::Cancelled`] or a font error.
pub(super) fn draw(
    pixmap: &mut Pixmap,
    input: &RenderInput<'_>,
    place: Placement,
    face: &Face,
    ink: &Ink,
    parts: (&Doors, &[Option<Cell2>]),
) -> Result<Present, RenderError> {
    let (doors, centres) = parts;
    let mut present = Present::default();
    for door in &doors.0 {
        input.check_cancel()?;
        let at = |room: u16| centres.get(usize::from(room)).copied().flatten();
        let ends = (at(door.rooms.0), at(door.rooms.1));
        match door.kind {
            Passage::Open => {
                present.open = true;
                bar(pixmap, place, door, ends, ink.door);
            }
            Passage::Locked(key) => {
                present.locked = true;
                let [x, y] = bar(pixmap, place, door, ends, ink.locked);
                let name = RoomGraph::key_name(key);
                let origin = [to_f32(x) + 4.0, to_f32(y) - 3.0];
                text::draw(pixmap, face, &name, KEY_NAME_PX, origin, ink.locked)?;
            }
            Passage::Drop { from } => {
                present.drop = true;
                let other = if door.rooms.0 == from {
                    door.rooms.1
                } else {
                    door.rooms.0
                };
                drop_arrow(pixmap, place, door, (at(from), at(other)), ink);
            }
        }
    }
    Ok(present)
}

/// Whether the walk through a door goes along the rows (vertically), judged from its rooms.
fn vertical(ends: Ends) -> bool {
    match ends {
        (Some(a), Some(b)) => (a[0] - b[0]).abs() > (a[1] - b[1]).abs(),
        _ => false,
    }
}

/// Draws the bar across a door's gap, along the wall; returns the bar's far end in pixels.
fn bar(pixmap: &mut Pixmap, place: Placement, door: &Door, ends: Ends, colour: Rgb) -> [f64; 2] {
    let (row, col) = (
        f64::from(door.centre.0) + 0.5,
        f64::from(door.centre.1) + 0.5,
    );
    let along = |f: fn(&(u32, u32)) -> u32| -> (f64, f64) {
        let lo = door.cells.iter().map(f).min().map_or(0.0, f64::from);
        let hi = door.cells.iter().map(f).max().map_or(0.0, f64::from) + 1.0;
        let mid = f64::midpoint(lo, hi);
        let half = ((hi - lo) / 2.0).max(MIN_BAR_CELLS / 2.0);
        (mid - half, mid + half)
    };
    let (from, to) = if vertical(ends) {
        let (c0, c1) = along(|c| c.1);
        ([row, c0], [row, c1])
    } else {
        let (r0, r1) = along(|c| c.0);
        ([r0, col], [r1, col])
    };
    let (a, b) = (place.px(from), place.px(to));
    let style = LineStyle {
        width_px: BAR_PX,
        colour,
        dash: None,
    };
    canvas::polyline(pixmap, &[a, b], &style);
    b
}

/// Draws the arrow of a drop, along the axis from the room it starts in to the other.
fn drop_arrow(pixmap: &mut Pixmap, place: Placement, door: &Door, rooms: Ends, ink: &Ink) {
    let centre = place.px([
        f64::from(door.centre.0) + 0.5,
        f64::from(door.centre.1) + 0.5,
    ]);
    // Pixel axes: x east (columns), y south (rows).
    let dir = match rooms {
        (Some(a), Some(b)) => {
            let (dr, dc) = (b[0] - a[0], b[1] - a[1]);
            if dr.abs() > dc.abs() {
                [0.0, dr.signum()]
            } else {
                [dc.signum(), 0.0]
            }
        }
        _ => [1.0, 0.0],
    };
    let tail = [
        centre[0] - dir[0] * ARROW_HALF_PX,
        centre[1] - dir[1] * ARROW_HALF_PX,
    ];
    let tip = [
        centre[0] + dir[0] * ARROW_HALF_PX,
        centre[1] + dir[1] * ARROW_HALF_PX,
    ];
    geo::arrow(pixmap, tail, tip, 1.5, ink.drop);
}
