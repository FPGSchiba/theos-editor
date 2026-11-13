//! The suitability fields of the ten resources, 0.3.0's `features.py::resources` before the
//! placing.
//!
//! With `flat = exp(-slope / 0.04)`, `near_water` the cells less than 0.6 km from a river or lake,
//! `hard` granite, quartzite or basalt, `contact` the soft rock (limestone, sandstone, shale)
//! within the contact distance of granite, and `P` the precipitation in mm/yr:
//!
//! ```text
//! iron-ore           0.8 [quartzite] + 0.7 contact + 0.35 [basalt] [slope > 0.1]
//! copper-ore         contact + 0.3 [basalt]
//! silver-ore         hard ridge [height > its 70th percentile]
//! stone              [limestone or granite] [0.08 < slope < 0.5]
//! clay               [alluvium or shale] flat near_water
//! timber             gauss([vegetation > 0.75])
//! peat-reeds         flat near_water [P > 900] [not lake]
//! salt               [next to a terminal lake, not in it] or [P < 400 and flat > 0.5]
//! resonance-crystal  hard prominence (0.3 + ruins_near + omega)
//! herbs              [-1 < T < 7] [0.25 < vegetation < 0.75]
//! ```
//!
//! `ridge = clip(curvature / (p99(curvature) + 1e-9), 0, 1)` with the curvature the negated
//! Laplacian of the height smoothed over 125 m, in 0.3.0's units of height per cell squared;
//! `prominence = clip((height - gauss(height)) / 150, 0, 1)` with the smoothing over 500 m;
//! `ruins_near` the largest `exp(-d^2 / (2 s^2))` over the ruins with `s` 2.5 km.
//!
//! The lengths of 0.3.0 (cells of 62.5 m) are metres here: curvature smoothing 125 m (2 cells),
//! prominence smoothing 500 m (8 cells), contact distance 350 m (0.35 km, at least 2 cells), timber
//! smoothing 187.5 m (3 cells), salt shore 125 m (2 cells). A distance in cells is
//! [`metres_to_cells`], which rounds where 0.3.0 truncated.

use std::collections::BTreeSet;

use crate::grid::Grid;
use crate::kernels::KernelError;
use crate::kernels::diff::laplace;
use crate::kernels::distance::distance;
use crate::kernels::filters::gaussian;
use crate::kernels::morphology::{Structure, dilate};
use crate::kernels::num::{max, min};
use crate::kernels::stats::{metres_to_cells, percentile};
use crate::stages::terrain::hydrology::WaterFeatures;
use crate::stages::terrain::rocks::Rock;
use crate::world::MapFrame;

/// The resource types in the order 0.3.0 placed them.
pub const NAMES: [&str; 10] = [
    "iron-ore",
    "copper-ore",
    "silver-ore",
    "stone",
    "clay",
    "timber",
    "peat-reeds",
    "salt",
    "resonance-crystal",
    "herbs",
];

/// Sigma of the height smoothing of the curvature, 0.3.0's 2 cells of 62.5 m.
const CURVATURE_SIGMA_M: f64 = 125.0;
/// Sigma of the height smoothing of the prominence, 0.3.0's 8 cells.
const PROMINENCE_SIGMA_M: f64 = 500.0;
/// How far the contact zone around granite reaches, m (0.3.0's 0.35 km).
const CONTACT_M: f64 = 350.0;
/// The least number of dilation steps of the contact zone.
const CONTACT_MIN_CELLS: usize = 2;
/// Sigma of the timber smoothing, 0.3.0's 3 cells.
const TIMBER_SIGMA_M: f64 = 187.5;
/// How far a terminal lake's salt shore reaches, m (0.3.0's 2 cells).
const SALT_SHORE_M: f64 = 125.0;
/// Within this distance of water the ground is wet, m.
const NEAR_WATER_M: f64 = 600.0;
/// Sigma of the influence of a ruin on resonance crystal, m.
const RUINS_SIGMA_M: f64 = 2500.0;

/// The inputs of the fields.
pub struct Inputs<'a> {
    /// Height, m.
    pub height: &'a Grid<f64>,
    /// Slope tangent.
    pub slope: &'a Grid<f64>,
    /// Rock codes.
    pub rock: &'a Grid<u8>,
    /// River cells.
    pub river: &'a Grid<bool>,
    /// Lake cells.
    pub lake: &'a Grid<bool>,
    /// Cells of lakes that pass no water on.
    pub terminal: &'a Grid<bool>,
    /// Vegetation density.
    pub vegetation: &'a Grid<f64>,
    /// Temperature, degrees Celsius.
    pub temperature: &'a Grid<f64>,
    /// Precipitation, mm/yr.
    pub precipitation: &'a Grid<f64>,
    /// The ruins, metres.
    pub ruins: &'a [[f64; 2]],
    /// The detuning field; absent counts as 0.
    pub omega: Option<&'a Grid<f64>>,
}

/// The cells of the lakes of `water` that are terminal, from the raster of lake numbers (0 is no
/// lake).
#[must_use]
pub fn terminal_mask(lake_id: &Grid<u32>, water: &WaterFeatures) -> Grid<bool> {
    let terminal: BTreeSet<u32> = water
        .lakes
        .iter()
        .filter(|l| l.terminal)
        .map(|l| l.id)
        .collect();
    lake_id.map(|id| id != 0 && terminal.contains(&id))
}

/// The largest `exp(-d^2 / (2 s^2))` over `ruins`, per cell; 0 without ruins.
fn ruins_near(frame: &MapFrame, ruins: &[[f64; 2]]) -> Grid<f64> {
    let mut near = Grid::filled(frame.rows, frame.cols, 0.0);
    let two_s2 = 2.0 * RUINS_SIGMA_M * RUINS_SIGMA_M;
    for i in 0..near.len() {
        let (r, c) = near.cell(i);
        let [x, y] = frame.centre_m(r, c);
        for at in ruins {
            let (dx, dy) = (x - at[0], y - at[1]);
            near.as_mut_slice()[i] =
                max(near.as_slice()[i], libm::exp(-(dx * dx + dy * dy) / two_s2));
        }
    }
    near
}

/// Whether each cell is less than 0.6 km from a river or lake; all false without water.
fn near_water(inputs: &Inputs<'_>, frame: &MapFrame) -> Result<Grid<bool>, KernelError> {
    let mut water = inputs.river.clone();
    for (w, &l) in water.as_mut_slice().iter_mut().zip(inputs.lake.as_slice()) {
        *w = *w || l;
    }
    if !water.as_slice().iter().any(|&w| w) {
        return Ok(Grid::filled(frame.rows, frame.cols, false));
    }
    let (dist, _) = distance(&water, frame.cell_m)?;
    Ok(dist.map(|m| m < NEAR_WATER_M))
}

/// Soft rock next to granite: the granite dilated, without granite, on limestone, sandstone or
/// shale.
fn contact_zone(rock: &Grid<u8>, frame: &MapFrame) -> Grid<bool> {
    let is = |code: u8| Rock::from_code(code);
    let granite = rock.map(|c| is(c) == Some(Rock::Granite));
    let steps = metres_to_cells(CONTACT_M, frame.cell_m).max(CONTACT_MIN_CELLS);
    let mut zone = dilate(&granite, Structure::Cross, frame.cell_m, steps);
    for (i, z) in zone.as_mut_slice().iter_mut().enumerate() {
        let soft = matches!(
            is(rock.as_slice()[i]),
            Some(Rock::Limestone | Rock::Sandstone | Rock::Shale)
        );
        *z = *z && !granite.as_slice()[i] && soft;
    }
    zone
}

/// The ridge measure `clip(curvature / (p99 + 1e-9), 0, 1)`.
fn ridge(height: &Grid<f64>, frame: &MapFrame) -> Result<Grid<f64>, KernelError> {
    let smooth = gaussian(height, CURVATURE_SIGMA_M, frame.cell_m);
    // `laplace` divides by the cell area; 0.3.0's units are height per cell squared.
    let area = frame.cell_m * frame.cell_m;
    let curvature = laplace(&smooth, frame.cell_m).map(|v| -v * area);
    let p99 = percentile(curvature.as_slice(), 99.0)?;
    Ok(curvature.map(|v| min(max(v / (p99 + 1e-9), 0.0), 1.0)))
}

/// The prominence `clip((height - gauss(height)) / 150, 0, 1)`.
fn prominence(height: &Grid<f64>, frame: &MapFrame) -> Grid<f64> {
    let smooth = gaussian(height, PROMINENCE_SIGMA_M, frame.cell_m);
    let mut out = height.clone();
    for (v, s) in out.as_mut_slice().iter_mut().zip(smooth.as_slice()) {
        *v = min(max((*v - s) / 150.0, 0.0), 1.0);
    }
    out
}

/// The share of a flag: 1 or 0.
fn flag(b: bool) -> f64 {
    if b { 1.0 } else { 0.0 }
}

/// The ten fields in the order of [`NAMES`].
///
/// # Errors
/// A kernel error when the height is empty.
pub fn compute(
    inputs: &Inputs<'_>,
    frame: &MapFrame,
) -> Result<Vec<(&'static str, Grid<f64>)>, KernelError> {
    let contact = contact_zone(inputs.rock, frame);
    let ridge = ridge(inputs.height, frame)?;
    let prominence = prominence(inputs.height, frame);
    let near = near_water(inputs, frame)?;
    let ruins = ruins_near(frame, inputs.ruins);
    let high = percentile(inputs.height.as_slice(), 70.0)?;
    let timber = gaussian(
        &inputs.vegetation.map(|v| flag(v > 0.75)),
        TIMBER_SIGMA_M,
        frame.cell_m,
    );
    let shore = dilate(
        inputs.terminal,
        Structure::Cross,
        frame.cell_m,
        metres_to_cells(SALT_SHORE_M, frame.cell_m),
    );
    let mut out: Vec<Grid<f64>> = (0..NAMES.len())
        .map(|_| Grid::filled(frame.rows, frame.cols, 0.0))
        .collect();
    for i in 0..frame.cells() {
        let rock = Rock::from_code(inputs.rock.as_slice()[i]);
        let is = |r: Rock| rock == Some(r);
        let hard = rock.is_some_and(Rock::is_hard);
        let slope = inputs.slope.as_slice()[i];
        let flat = libm::exp(-slope / 0.04);
        let near = near.as_slice()[i];
        let precipitation = inputs.precipitation.as_slice()[i];
        let veg = inputs.vegetation.as_slice()[i];
        let temperature = inputs.temperature.as_slice()[i];
        let contact = flag(contact.as_slice()[i]);
        let omega = inputs.omega.map_or(0.0, |o| o.as_slice()[i]);
        let wetland = flat * flag(near) * flag(precipitation > 900.0);
        let values = [
            0.8 * flag(is(Rock::Quartzite))
                + 0.7 * contact
                + 0.35 * flag(is(Rock::Basalt) && slope > 0.1),
            contact + 0.3 * flag(is(Rock::Basalt)),
            flag(hard) * ridge.as_slice()[i] * flag(inputs.height.as_slice()[i] > high),
            flag(is(Rock::Limestone) || is(Rock::Granite)) * flag(slope > 0.08 && slope < 0.5),
            flag(is(Rock::Alluvium) || is(Rock::Shale)) * flat * flag(near),
            timber.as_slice()[i],
            wetland * flag(!inputs.lake.as_slice()[i]),
            flag(
                (shore.as_slice()[i] && !inputs.terminal.as_slice()[i])
                    || (precipitation < 400.0 && flat > 0.5),
            ),
            flag(hard) * prominence.as_slice()[i] * (0.3 + ruins.as_slice()[i] + omega),
            flag(temperature > -1.0 && temperature < 7.0) * flag(veg > 0.25 && veg < 0.75),
        ];
        for (grid, v) in out.iter_mut().zip(values) {
            grid.as_mut_slice()[i] = v;
        }
    }
    Ok(NAMES.into_iter().zip(out).collect())
}
