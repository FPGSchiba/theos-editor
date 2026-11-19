//! The known area of `inworld`: the land around settlements and main roads that the map's
//! maker walked, as 0.3.0 drew it. A disc around every settlement (a share of the map width by
//! class), a band around every road of class `road` and `main`, blurred and thresholded.

use super::super::roads::network;
use super::super::{RenderError, RenderInput};
use crate::grid::Grid;
use crate::kernels::filters::gaussian;
use crate::kernels::num::{to_f64, to_index};
use crate::stages::terrain::roads::RoadClass;
use crate::world::MapFrame;

/// The radius of the disc around a settlement, as a share of the map width (0.3.0).
fn disc_share(ty: &str) -> f64 {
    match ty {
        "city" => 0.30,
        "town" => 0.25,
        "village" => 0.18,
        _ => 0.10,
    }
}

/// The half width of the band around a road, as a share of the map width (0.3.0).
const BAND_SHARE: f64 = 0.04;
/// The blur that rounds the raw area, cells (0.3.0).
const EDGE_BLUR_CELLS: f64 = 10.0;
/// The blurred value above which a cell is known (0.3.0).
const THRESHOLD: f64 = 0.35;
/// The blur of the paper's shading around the known area, cells (0.3.0).
const PAPER_BLUR_CELLS: f64 = 20.0;

/// The known area.
pub(crate) struct Known {
    /// Whether a cell is known.
    pub mask: Grid<bool>,
    /// The known area blurred widely, `[0, 1]`: the paper tone follows it.
    pub soft: Grid<f64>,
}

/// Sets every cell within `radius` cells of `(row, col)` to 1.
fn stamp(raw: &mut Grid<f64>, row: usize, col: usize, radius: f64) {
    let reach = to_index(radius, usize::MAX).saturating_add(1);
    let (r0, r1) = (row.saturating_sub(reach), (row + reach).min(raw.rows() - 1));
    let (c0, c1) = (col.saturating_sub(reach), (col + reach).min(raw.cols() - 1));
    for r in r0..=r1 {
        for c in c0..=c1 {
            let (dr, dc) = (to_f64(r.abs_diff(row)), to_f64(c.abs_diff(col)));
            if dr * dr + dc * dc <= radius * radius {
                raw.set(r, c, 1.0);
            }
        }
    }
}

/// The known area of the world.
///
/// # Errors
/// [`RenderError::Cancelled`] when the run is cancelled.
pub(crate) fn compute(input: &RenderInput<'_>) -> Result<Known, RenderError> {
    let frame: MapFrame = *input.world.frame();
    let mut raw = Grid::filled(frame.rows, frame.cols, 0.0);
    let width_cells = to_f64(frame.cols);
    for (f, at) in crate::render::feats::points(input.features, "settlement") {
        input.check_cancel()?;
        let (r, c) = frame.cell_at_m(at[0], at[1]);
        stamp(&mut raw, r, c, disc_share(&f.ty) * width_cells);
    }
    if let Some(net) = network(input) {
        for road in net.roads.iter().filter(|r| r.class >= RoadClass::Road) {
            input.check_cancel()?;
            for &cell in &road.cells {
                let i = usize::try_from(cell).unwrap_or(usize::MAX);
                stamp(
                    &mut raw,
                    i / frame.cols,
                    i % frame.cols,
                    BAND_SHARE * width_cells,
                );
            }
        }
    }
    input.check_cancel()?;
    let blurred = gaussian(&raw, EDGE_BLUR_CELLS * frame.cell_m, frame.cell_m);
    let mask = blurred.map(|v| v > THRESHOLD);
    input.check_cancel()?;
    let soft = gaussian(
        &mask.map(|k| if k { 1.0 } else { 0.0 }),
        PAPER_BLUR_CELLS * frame.cell_m,
        frame.cell_m,
    );
    Ok(Known { mask, soft })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::progress::CancelToken;
    use crate::render::test_support::TestWorld;

    #[test]
    fn the_known_area_surrounds_settlements_and_stays_clear_of_the_rest() {
        let mut w = TestWorld::flat(40, 40, 50.0);
        w.settlement("city", [250.0, 250.0]);
        let cancel = CancelToken::new();
        let known = compute(&w.input(&cancel)).unwrap();
        assert!(known.mask.get(5, 5));
        assert!(!known.mask.get(39, 39));
        assert!(known.soft.get(5, 5) > known.soft.get(39, 39));
    }
}
