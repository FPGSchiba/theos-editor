//! A surface filled from seeds and the single steepest receiver of every cell on it (steps 1, 4
//! and 6 of `hydrology`), `flow_dir`, and the discharge accumulated over those receivers.

use crate::grid::{D8, Grid};
use crate::kernels::flood::priority_flood;
use crate::stages::evolution::{cell_index, steepest_receivers};
use crate::stages::terrain::climate::SECONDS_PER_YEAR;

/// The ε of both fills, m (Barnes et al. 2014).
pub(crate) const EPSILON: f64 = 1e-6;

/// A surface filled from seeds, with the single steepest receiver of every cell on it.
///
/// Every cell but a seed has a strictly lower neighbour on the filled surface, so it has a
/// receiver of its own and its water reaches a seed. A seed receives itself.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Drainage {
    /// The filled surface, m.
    pub surface: Grid<f64>,
    /// The cells in the order the fill reached them; receivers come before their donors.
    pub order: Vec<u32>,
    /// The receiver of every cell: its D8 neighbour of steepest descent on `surface` (equal
    /// slopes to the lower index), or itself for a seed.
    pub receivers: Vec<usize>,
}

/// Fills `height` from `seeds` (cell indices, in index order) with [`EPSILON`] and finds the
/// steepest receiver of every cell on cells of `cell_m` metres.
pub(crate) fn drain(height: &Grid<f64>, seeds: &[usize], cell_m: f64) -> Drainage {
    let flooded = priority_flood(height, seeds, EPSILON);
    let mut is_seed = Grid::filled(height.rows(), height.cols(), false);
    for &s in seeds {
        if let Some(cell) = is_seed.as_mut_slice().get_mut(s) {
            *cell = true;
        }
    }
    let receivers = steepest_receivers(&flooded.surface, &is_seed, cell_m)
        .as_slice()
        .iter()
        .map(|&r| cell_index(r))
        .collect();
    Drainage {
        surface: flooded.surface,
        order: flooded.order,
        receivers,
    }
}

impl Drainage {
    /// `input` accumulated over the receivers: every cell holds its own input plus everything
    /// its donors pass on, summed in reverse fill order. A seed keeps what reaches it.
    pub(crate) fn accumulate(&self, input: &Grid<f64>) -> Grid<f64> {
        let mut out = input.clone();
        let flow = out.as_mut_slice();
        for &cell in self.order.iter().rev() {
            let i = cell_index(cell);
            let to = self.receivers[i];
            if to != i {
                flow[to] += flow[i];
            }
        }
        out
    }

    /// The D8 code of every cell's receiver: 1 to 8 for N, NE, E, SE, S, SW, W, NW (the order
    /// of [`D8`]), 0 for a seed (plan ruling L19).
    pub(crate) fn flow_dir(&self) -> Grid<u8> {
        let mut codes = Grid::filled(self.surface.rows(), self.surface.cols(), 0_u8);
        for (i, (&to, code)) in self.receivers.iter().zip(codes.as_mut_slice()).enumerate() {
            if to != i {
                *code = (1_u8..)
                    .zip(D8)
                    .find(|&(_, d)| self.surface.neighbour(i, d) == Some(to))
                    .map_or(0, |(c, _)| c);
            }
        }
        codes
    }
}

/// The routing of step 6: the drainage of the final fill and the discharge over it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Routing {
    /// The routing surface, its fill order and its receivers.
    pub drainage: Drainage,
    /// Runoff accumulated over the receivers, m³/s.
    pub discharge: Grid<f64>,
}

/// Fills `height` from `seeds` and accumulates `runoff` (m³/yr per cell) over the steepest
/// receivers into a discharge in m³/s.
pub(crate) fn route(
    height: &Grid<f64>,
    seeds: &[usize],
    runoff: &Grid<f64>,
    cell_m: f64,
) -> Routing {
    let drainage = drain(height, seeds, cell_m);
    let discharge = drainage.accumulate(runoff).map(|q| q / SECONDS_PER_YEAR);
    Routing {
        drainage,
        discharge,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flow_dir_codes_follow_the_d8_order() {
        // Every cell of the ring drains into the seeded centre.
        let height =
            Grid::from_vec(3, 3, vec![5.0, 4.0, 5.0, 4.0, 0.0, 4.0, 5.0, 4.0, 5.0]).unwrap();
        let drainage = drain(&height, &[4], 1.0);
        assert_eq!(drainage.receivers, [4; 9]);
        assert_eq!(drainage.flow_dir().as_slice(), [4, 5, 6, 3, 0, 7, 2, 1, 8]);
    }

    #[test]
    fn accumulation_passes_flow_to_the_receivers_and_seeds_keep_it() {
        // A pit at cell 2 is filled; everything drains west to the seed at cell 0.
        let height = Grid::from_vec(1, 5, vec![0.0, 3.0, 1.0, 4.0, 6.0]).unwrap();
        let drainage = drain(&height, &[0], 1.0);
        assert_eq!(drainage.receivers, [0, 0, 1, 2, 3]);
        let filled = drainage.surface.as_slice();
        assert!(filled[2] > 3.0 && filled[3] > filled[2]);
        let flow = drainage.accumulate(&Grid::filled(1, 5, 1.0));
        assert_eq!(flow.as_slice(), [5.0, 4.0, 3.0, 2.0, 1.0]);
        let routing = route(&height, &[0], &Grid::filled(1, 5, SECONDS_PER_YEAR), 1.0);
        assert_eq!(routing.discharge.as_slice(), [5.0, 4.0, 3.0, 2.0, 1.0]);
        assert_eq!(routing.drainage.flow_dir().as_slice(), [0, 7, 7, 7, 7]);
    }

    #[test]
    fn seeds_off_the_grid_are_ignored() {
        let height = Grid::from_vec(1, 3, vec![0.0, 1.0, 2.0]).unwrap();
        let drainage = drain(&height, &[0, 99], 1.0);
        assert_eq!(drainage.receivers, [0, 0, 1]);
    }
}
