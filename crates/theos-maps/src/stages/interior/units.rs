//! Lengths of the interior stages: recipes give metres, the cores work in raster cells.
//!
//! Interior outlines have integer vertices in raster cells (spec §5.14), so every length a
//! stage turns into cells must be a whole number of cells, and the cell size itself must lie on
//! the 1/16 m lattice of map geometry. The recipe loader checks both before any stage runs
//! ([`check_cell_size`] through the interior kind's table, [`cells`] in each stage's
//! `check_frame`); the stages convert with the same function when they run.

use crate::kernels::num::{max, round_half_away, to_index};
use crate::stage::ParamError;
use crate::world::MapFrame;

/// The steps of the geometry lattice per metre (P29's `kernels::geometry::lattice`).
const LATTICE_PER_M: f64 = 16.0;

/// How far a quotient may lie from a whole number and still count as one: the rounding of a
/// decimal length divided by a cell size, never a real fraction of a cell.
const TOLERANCE: f64 = 1e-9;

/// `x` as a whole number, when it is one (within [`TOLERANCE`]) and not negative.
fn whole(x: f64) -> Option<u32> {
    let n = round_half_away(x);
    let close = (x - n).abs() <= TOLERANCE * max(n.abs(), 1.0);
    if !(close && n >= 0.0) {
        return None;
    }
    u32::try_from(to_index(n, usize::MAX)).ok()
}

/// Refuses a cell size that is not a multiple of 1/16 m.
///
/// # Errors
/// The reason, for the plan to report at `size_km`.
pub fn check_cell_size(frame: &MapFrame) -> Result<(), String> {
    match whole(frame.cell_m * LATTICE_PER_M) {
        Some(n) if n > 0 => Ok(()),
        _ => Err(format!(
            "an interior's cell size must be a multiple of 1/16 m, got {} m",
            frame.cell_m
        )),
    }
}

/// `metres` as a whole number of the frame's raster cells.
///
/// # Errors
/// [`ParamError`] of `stage` at `parameter` when `metres` is negative or not a whole number of
/// cells.
pub fn cells(
    stage: &str,
    parameter: &str,
    metres: f64,
    frame: &MapFrame,
) -> Result<u32, ParamError> {
    whole(metres / frame.cell_m).ok_or_else(|| {
        ParamError::at(
            stage,
            parameter,
            format!(
                "{metres} m is not a whole number of raster cells of {} m",
                frame.cell_m
            ),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(cell_m: f64) -> MapFrame {
        MapFrame {
            rows: 40,
            cols: 40,
            cell_m,
        }
    }

    #[test]
    fn lengths_must_be_whole_cells() {
        assert_eq!(cells("s", "p", 8.0, &frame(1.0)), Ok(8));
        assert_eq!(cells("s", "p", 0.0, &frame(1.0)), Ok(0));
        assert_eq!(cells("s", "p", 8.0, &frame(0.25)), Ok(32));
        assert_eq!(cells("s", "p", 0.3, &frame(0.1)), Ok(3), "decimal rounding");
        for bad in [15.5, -1.0, f64::NAN] {
            let err = cells("envelope", "pitch_m", bad, &frame(1.0)).unwrap_err();
            assert_eq!(
                (err.stage.as_str(), err.parameter.as_deref()),
                ("envelope", Some("pitch_m"))
            );
        }
    }

    #[test]
    fn the_cell_size_lies_on_the_sixteenth_metre_lattice() {
        for good in [1.0, 0.5, 0.0625, 2.0, 0.1875] {
            assert_eq!(check_cell_size(&frame(good)), Ok(()), "{good}");
        }
        for bad in [0.1, 0.03, 1.01] {
            let err = check_cell_size(&frame(bad)).unwrap_err();
            assert!(err.contains("1/16 m"), "{bad}: {err}");
        }
    }
}
