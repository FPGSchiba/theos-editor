//! Step 7 of `hydrology`: the rasters it writes besides `height` and `discharge`.

use super::{Water, WaterThresholds};
use crate::grid::Grid;

/// The lake, river and flow rasters of a [`Water`].
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Rasters {
    /// Lake cells.
    pub lake: Grid<bool>,
    /// The lake's number on lake cells, 0 elsewhere.
    pub lake_id: Grid<u32>,
    /// The lake's level on lake cells, 0 elsewhere, m.
    pub lake_level: Grid<f64>,
    /// Cells outside lakes whose discharge exceeds `river_q`.
    pub river: Grid<bool>,
    /// Cells outside lakes whose discharge exceeds `big_river_q`.
    pub big_river: Grid<bool>,
    /// The lake level on lake cells, else `height`, m.
    pub water_surface: Grid<f64>,
    /// The D8 code of every cell's receiver on the routing surface, 0 at the seeds.
    pub flow_dir: Grid<u8>,
}

/// The rasters of `water` with the river `thresholds`.
pub(crate) fn rasters(water: &Water, thresholds: &WaterThresholds) -> Rasters {
    let height = &water.height;
    let mut lake_id = Grid::filled(height.rows(), height.cols(), 0_u32);
    let mut lake_level = Grid::filled(height.rows(), height.cols(), 0.0);
    let mut water_surface = height.clone();
    for lake in water.basins.iter().filter_map(|b| b.lake.as_ref()) {
        for &c in &lake.cells {
            lake_id.as_mut_slice()[c] = lake.id;
            lake_level.as_mut_slice()[c] = lake.level;
            water_surface.as_mut_slice()[c] = lake.level;
        }
    }
    let lake = lake_id.map(|id| id != 0);
    let mut river = Grid::filled(height.rows(), height.cols(), false);
    let mut big_river = river.clone();
    let cells = water
        .routing
        .discharge
        .as_slice()
        .iter()
        .zip(lake.as_slice())
        .zip(
            river
                .as_mut_slice()
                .iter_mut()
                .zip(big_river.as_mut_slice()),
        );
    for ((&q, &wet), (river, big)) in cells {
        *river = !wet && q > thresholds.river_q;
        *big = !wet && q > thresholds.big_river_q;
    }
    Rasters {
        lake,
        lake_id,
        lake_level,
        river,
        big_river,
        water_surface,
        flow_dir: water.routing.drainage.flow_dir(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stages::terrain::hydrology::discharge::{Drainage, Routing};
    use crate::stages::terrain::hydrology::lakes::{Balance, Basin, Depression, Lake};

    #[test]
    fn rasters_mark_lakes_and_rivers_outside_them() {
        // Cells 1 and 2 hold lake 3 at 4.5 m; discharge rises downstream to cell 0.
        let height = Grid::from_vec(1, 5, vec![1.0, 2.0, 3.0, 6.0, 7.0]).unwrap();
        let basin = Basin {
            depression: Depression {
                cells: vec![1, 2],
                spill_level: 4.5,
                bottom: 1,
                exit: 1,
            },
            balance: Balance::default(),
            terminal: false,
            lake: Some(Lake {
                id: 3,
                cells: vec![1, 2],
                level: 4.5,
            }),
        };
        let drainage = Drainage {
            surface: height.clone(),
            order: vec![0, 1, 2, 3, 4],
            receivers: vec![0, 0, 1, 2, 3],
        };
        let routing = Routing {
            drainage,
            discharge: Grid::from_vec(1, 5, vec![5.0, 4.0, 3.0, 2.0, 1.0]).unwrap(),
        };
        let water = Water {
            height,
            basins: vec![basin],
            filled_pits: 0,
            filled_nested: 0,
            routing,
        };
        let thresholds = WaterThresholds {
            river_q: 1.5,
            big_river_q: 4.5,
        };
        let r = rasters(&water, &thresholds);
        assert_eq!(r.lake.as_slice(), [false, true, true, false, false]);
        assert_eq!(r.lake_id.as_slice(), [0, 3, 3, 0, 0]);
        assert_eq!(r.lake_level.as_slice(), [0.0, 4.5, 4.5, 0.0, 0.0]);
        assert_eq!(r.water_surface.as_slice(), [1.0, 4.5, 4.5, 6.0, 7.0]);
        assert_eq!(r.river.as_slice(), [true, false, false, true, false]);
        assert_eq!(r.big_river.as_slice(), [true, false, false, false, false]);
        assert_eq!(r.flow_dir.as_slice(), [0, 7, 7, 7, 7]);
    }
}
