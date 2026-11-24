//! Lake outlines (spec §5.9): the rings of every lake by marching squares, with its level,
//! whether it is terminal, its outlet and its greatest depth.

use serde::Serialize;

use super::Water;
use crate::grid::Grid;
use crate::kernels::contour::rings;
use crate::kernels::num::min;
use crate::world::MapFrame;

/// The outline of one lake.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LakeShape {
    /// The lake's number, as in `lake_id`.
    pub id: u32,
    /// Closed rings of `[x, y]` metres: exteriors with a positive area, holes with a negative
    /// one.
    pub rings: Vec<Vec<[f64; 2]>>,
    /// The water surface, m.
    pub surface_m: f64,
    /// Whether the lake passes no water on.
    pub terminal: bool,
    /// The centre of the cell the lake spills over; `None` for a terminal lake.
    pub outlet: Option<[f64; 2]>,
    /// The level less the lowest height in the lake, m.
    pub depth_max_m: f64,
}

/// The outlines of the lakes of `water` on the cells of `frame`, in the order of their numbers.
pub(crate) fn outlines(water: &Water, frame: &MapFrame) -> Vec<LakeShape> {
    let height = &water.height;
    water
        .basins
        .iter()
        .filter_map(|basin| {
            let lake = basin.lake.as_ref()?;
            let mut cells = Grid::filled(frame.rows, frame.cols, false);
            let mut lowest = lake.level;
            for &c in &lake.cells {
                cells.as_mut_slice()[c] = true;
                lowest = min(lowest, height.as_slice()[c]);
            }
            let outlet = (!basin.terminal).then(|| frame.index_centre_m(basin.depression.exit));
            Some(LakeShape {
                id: lake.id,
                rings: rings(&cells, frame.cell_m),
                surface_m: lake.level,
                terminal: basin.terminal,
                outlet,
                depth_max_m: lake.level - lowest,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernels::num::to_f64;
    use crate::stages::terrain::hydrology::tests::{
        ARID, CELL_M, N, SEMI_ARID, WET, all_edges, bowl, closed, compute, frame_of, hills,
    };

    /// Whether `p` lies inside the closed ring (even-odd rule).
    fn inside(ring: &[[f64; 2]], p: [f64; 2]) -> bool {
        let mut inside = false;
        for w in ring.windows(2) {
            let (a, b) = (w[0], w[1]);
            if (a[1] > p[1]) != (b[1] > p[1])
                && p[0] < a[0] + (p[1] - a[1]) / (b[1] - a[1]) * (b[0] - a[0])
            {
                inside = !inside;
            }
        }
        inside
    }

    fn centre(cell: usize) -> [f64; 2] {
        [
            (to_f64(cell % N) + 0.5) * CELL_M,
            (to_f64(cell / N) + 0.5) * CELL_M,
        ]
    }

    #[test]
    fn lake_rings_enclose_their_cells() {
        let water = compute(&bowl(), &all_edges(), WET);
        let shapes = outlines(&water, &frame_of(&water.height));
        assert_eq!(shapes.len(), 1);
        let (shape, lake) = (&shapes[0], water.basins[0].lake.as_ref().unwrap());
        assert_eq!(shape.id, lake.id);
        assert_eq!(shape.surface_m.to_bits(), lake.level.to_bits());
        assert_eq!(shape.rings.len(), 1, "one exterior ring, no holes");
        for &cell in &lake.cells {
            assert!(
                shape.rings.iter().any(|r| inside(r, centre(cell))),
                "cell {cell} is outside the outline"
            );
        }
        for cell in 0..N * N {
            if !lake.cells.contains(&cell) {
                assert!(shape.rings.iter().all(|r| !inside(r, centre(cell))));
            }
        }
        let lowest = lake
            .cells
            .iter()
            .map(|&c| water.height.as_slice()[c])
            .fold(f64::INFINITY, f64::min);
        assert!((shape.depth_max_m - (lake.level - lowest)).abs() < 1e-12);
        assert!(shape.depth_max_m > 10.0);
        // The spill cell is the outlet of a lake that fills to spill.
        assert!(!shape.terminal);
        assert_eq!(shape.outlet, Some(centre(water.basins[0].depression.exit)));
    }

    #[test]
    fn terminal_lakes_have_no_outlet() {
        for (height, fixed, weather) in
            [(bowl(), all_edges(), ARID), (hills(), closed(), SEMI_ARID)]
        {
            let water = compute(&height, &fixed, weather);
            let shapes = outlines(&water, &frame_of(&water.height));
            assert!(shapes.iter().any(|s| s.terminal));
            for shape in &shapes {
                let basin = water
                    .basins
                    .iter()
                    .find(|b| b.lake.as_ref().is_some_and(|l| l.id == shape.id))
                    .unwrap();
                assert_eq!(shape.terminal, basin.terminal);
                assert_eq!(shape.outlet.is_none(), shape.terminal);
            }
        }
    }

    #[test]
    fn lakes_come_in_lake_number_order() {
        let water = compute(&hills(), &closed(), SEMI_ARID);
        let shapes = outlines(&water, &frame_of(&water.height));
        let ids: Vec<u32> = water
            .basins
            .iter()
            .filter_map(|b| b.lake.as_ref().map(|l| l.id))
            .collect();
        assert!(ids.len() >= 2, "{ids:?}");
        assert_eq!(shapes.iter().map(|s| s.id).collect::<Vec<_>>(), ids);
        assert!(ids.windows(2).all(|w| w[0] < w[1]));
    }
}
