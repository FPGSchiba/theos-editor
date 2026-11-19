//! The scattered sites, picked by score: watchtowers over the roads, caves in limestone cliffs
//! and camps in the wild. Ported from 0.3.0's `features.py::sites`, its lengths in metres.

use crate::features::{Feature, pick};
use crate::grid::Grid;
use crate::kernels::distance::distance;
use crate::kernels::filters::gaussian;
use crate::kernels::num::max;
use crate::rng::StageRng;
use crate::stages::terrain::rocks::Rock;
use crate::world::MapFrame;

/// Sigma the prominence of a watchtower is measured against, 0.3.0's 10 cells of 62.5 m.
const PROMINENCE_SIGMA_M: f64 = 625.0;
/// A watchtower stands this far from the nearest road at least, m (0.3.0's 0.15 km).
const TOWER_ROAD_MIN_M: f64 = 150.0;
/// A watchtower stands less than this far from the nearest road, m (0.3.0's 0.9 km).
const TOWER_ROAD_MAX_M: f64 = 900.0;
/// The least distance between two watchtowers, m.
const TOWER_SEP_M: f64 = 5000.0;
/// The slope above which limestone counts as a cliff.
const CAVE_SLOPE_MIN: f64 = 0.3;
/// The least distance between two caves, m.
const CAVE_SEP_M: f64 = 3000.0;
/// A camp stands farther than this from every road, m (0.3.0's 0.6 km).
const CAMP_ROAD_MIN_M: f64 = 600.0;
/// The distance to a road on a map without roads, m (0.3.0's 9 km).
const NO_ROAD_M: f64 = 9000.0;
/// Sigma of the slope smoothing of the camp score, 0.3.0's 2 cells of 62.5 m.
const CAMP_SLOPE_SIGMA_M: f64 = 125.0;
/// The slope scale of the camp score.
const CAMP_SLOPE_SCALE: f64 = 0.15;
/// The weight of the noise of the camp score.
const CAMP_NOISE: f64 = 0.2;
/// The least distance between two camps, and from a settlement, m.
const CAMP_SEP_M: f64 = 3000.0;
/// Roads of class road and main carry watchtowers: the codes from this up.
const TOWER_CLASS_MIN: u8 = 3;

/// The shared inputs of the scattered sites.
pub(super) struct Ground<'a> {
    /// Height, m.
    pub height: &'a Grid<f64>,
    /// Slope, rise over run.
    pub slope: &'a Grid<f64>,
    /// Rock codes.
    pub rock: &'a Grid<u8>,
    /// Vegetation density.
    pub vegetation: &'a Grid<f64>,
    /// The `roads` raster, spurs included.
    pub roads: &'a Grid<u8>,
    /// The cells away from the edge.
    pub inside: &'a Grid<bool>,
    /// The frame.
    pub frame: MapFrame,
}

/// Up to `n` watchtowers: the best cells of the prominence `max(height − gauss(height), 0)`
/// where the nearest road of class road or main is between 150 and 900 m away.
pub(super) fn watchtowers(g: &Ground<'_>, n: usize) -> Vec<Feature> {
    let main = g.roads.map(|c| c >= TOWER_CLASS_MIN);
    let Ok((away, _)) = distance(&main, g.frame.cell_m) else {
        return Vec::new();
    };
    let smooth = gaussian(g.height, PROMINENCE_SIGMA_M, g.frame.cell_m);
    let mut score = Grid::filled(g.frame.rows, g.frame.cols, 0.0);
    for i in 0..score.len() {
        let d = away.as_slice()[i];
        if g.inside.as_slice()[i] && d > TOWER_ROAD_MIN_M && d < TOWER_ROAD_MAX_M {
            score.as_mut_slice()[i] = max(g.height.as_slice()[i] - smooth.as_slice()[i], 0.0);
        }
    }
    picks("watchtower", &score, n, TOWER_SEP_M, &[], g)
}

/// Up to `n` caves: the steepest limestone, by `max(slope − 0.3, 0)`.
pub(super) fn caves(g: &Ground<'_>, n: usize) -> Vec<Feature> {
    let limestone = Rock::Limestone.code();
    let mut score = Grid::filled(g.frame.rows, g.frame.cols, 0.0);
    for i in 0..score.len() {
        if g.inside.as_slice()[i] && g.rock.as_slice()[i] == limestone {
            score.as_mut_slice()[i] = max(g.slope.as_slice()[i] - CAVE_SLOPE_MIN, 0.0);
        }
    }
    picks("cave", &score, n, CAVE_SEP_M, &[], g)
}

/// Up to `n` camps: remote ground under cover and gentle, at least 600 m from every road
/// (9 km with no road), 3 km from each other and from every settlement in `settlements` (metres).
/// Draws one noise value per cell in index order.
pub(super) fn camps(
    g: &Ground<'_>,
    n: usize,
    settlements: &[[f64; 2]],
    rng: &mut StageRng,
) -> Vec<Feature> {
    if n == 0 {
        return Vec::new();
    }
    let from_road = distance(&g.roads.map(|c| c > 0), g.frame.cell_m).map_or_else(
        |_| Grid::filled(g.frame.rows, g.frame.cols, NO_ROAD_M),
        |d| d.0,
    );
    let slope = gaussian(g.slope, CAMP_SLOPE_SIGMA_M, g.frame.cell_m);
    let mut score = Grid::filled(g.frame.rows, g.frame.cols, 0.0);
    for i in 0..score.len() {
        let noise = 1.0 + CAMP_NOISE * rng.uniform();
        if g.inside.as_slice()[i] && from_road.as_slice()[i] > CAMP_ROAD_MIN_M {
            let gentle = libm::exp(-slope.as_slice()[i] / CAMP_SLOPE_SCALE);
            score.as_mut_slice()[i] = g.vegetation.as_slice()[i] * gentle * noise;
        }
    }
    picks("camp", &score, n, CAMP_SEP_M, settlements, g)
}

/// The sites of type `ty` at the best `n` cells of `score`, `sep_m` apart and from the points
/// `taken`.
fn picks(
    ty: &str,
    score: &Grid<f64>,
    n: usize,
    sep_m: f64,
    taken: &[[f64; 2]],
    g: &Ground<'_>,
) -> Vec<Feature> {
    pick(score, n, sep_m, taken, sep_m, &g.frame)
        .into_iter()
        .map(|(r, c)| Feature::site(ty, g.frame.centre_m(r, c), []))
        .collect()
}
