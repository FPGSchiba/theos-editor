//! Booking of a process's height change into the cumulative `erosion` and `deposition` layers.

use crate::grid::Grid;
use crate::kernels::num::max;

/// Adds the thickness each cell lost from `start` to `end` to `erosion` and the thickness it
/// gained to `deposition`; both stay non-negative and cumulative.
pub(super) fn book(
    start: &Grid<f64>,
    end: &Grid<f64>,
    erosion: &mut Grid<f64>,
    deposition: &mut Grid<f64>,
) {
    let cells = start.as_slice().iter().zip(end.as_slice());
    let records = erosion
        .as_mut_slice()
        .iter_mut()
        .zip(deposition.as_mut_slice());
    for ((&before, &after), (eroded, deposited)) in cells.zip(records) {
        *eroded += max(before - after, 0.0);
        *deposited += max(after - before, 0.0);
    }
}
