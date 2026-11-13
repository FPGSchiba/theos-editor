//! Shaded relief per cell: 0.3.0's three-light hillshade and its ambient occlusion.
//!
//! Each light is matplotlib's `LightSource(azdeg, altdeg).hillshade(z, vert_exag=1.6, dx, dy)`,
//! as 0.3.0 calls it: the surface normal from numpy's gradient (`kernels::diff::gradient`), its
//! dot product with the direction towards the light, stretched to `[0, 1]` over the grid when
//! the values span more than 1e-6, then clipped to `[0, 1]`. The lights are 0.3.0's: from the
//! north-west 45° high (weight 0.6), from the west-south-west 35° high (0.25) and from the
//! north-north-east 60° high (0.15). 0.3.0 shaded the height upsampled to the pixels; here the
//! cells are shaded and the renders interpolate the result, which keeps the cost per cell.

use crate::grid::Grid;
use crate::kernels::diff::gradient;
use crate::kernels::filters::gaussian;
use crate::kernels::num::{max, min};

/// A light: where it comes from, how high it stands and how much it counts.
struct Light {
    /// Degrees clockwise from north.
    azimuth_deg: f64,
    /// Degrees above the horizon.
    altitude_deg: f64,
    /// Its share of the hillshade.
    weight: f64,
}

/// 0.3.0's three lights; the weights add up to 1.
const LIGHTS: [Light; 3] = [
    Light {
        azimuth_deg: 315.0,
        altitude_deg: 45.0,
        weight: 0.6,
    },
    Light {
        azimuth_deg: 250.0,
        altitude_deg: 35.0,
        weight: 0.25,
    },
    Light {
        azimuth_deg: 20.0,
        altitude_deg: 60.0,
        weight: 0.15,
    },
];

/// The vertical exaggeration of the hillshade (0.3.0).
const EXAGGERATION: f64 = 1.6;

/// The smallest span of one light's values that matplotlib stretches to `[0, 1]`.
const STRETCH_MIN: f64 = 1e-6;

/// The share of light every cell gets regardless of its slope (0.3.0).
const AMBIENT: f64 = 0.30;
/// The share of the hillshade in the brightness (0.3.0).
const DIRECT: f64 = 0.85;

/// The width of the occlusion's Gaussian, cells: 0.3.0's 10 pixels at two pixels per cell.
const OCCLUSION_SIGMA_CELLS: f64 = 5.0;
/// The height below its surroundings at which a cell loses all its light, m (0.3.0).
const OCCLUSION_DEPTH_M: f64 = 400.0;
/// The darkest occlusion factor (0.3.0).
const OCCLUSION_MIN: f64 = 0.75;
/// The brightest occlusion factor (0.3.0).
const OCCLUSION_MAX: f64 = 1.1;

impl Light {
    /// The unit vector towards the light, `[east, north, up]` (matplotlib's `direction`).
    fn direction(&self) -> [f64; 3] {
        let az = (90.0 - self.azimuth_deg).to_radians();
        let alt = self.altitude_deg.to_radians();
        [
            libm::cos(az) * libm::cos(alt),
            libm::sin(az) * libm::cos(alt),
            libm::sin(alt),
        ]
    }

    /// This light's hillshade of the cells with the gradients `d_row` (south) and `d_col`
    /// (east), in `[0, 1]`.
    fn shade(&self, d_row: &[f64], d_col: &[f64]) -> Vec<f64> {
        let [east, north, up] = self.direction();
        // matplotlib's normal is (−∂z/∂east, −∂z/∂north, 1) of the exaggerated surface, and
        // rows run south, so ∂z/∂north = −d_row.
        let mut light: Vec<f64> = d_row
            .iter()
            .zip(d_col)
            .map(|(&gr, &gc)| {
                let (nx, ny) = (-EXAGGERATION * gc, EXAGGERATION * gr);
                (nx * east + ny * north + up) / (nx * nx + ny * ny + 1.0).sqrt()
            })
            .collect();
        let lo = light.iter().fold(f64::INFINITY, |m, &v| min(m, v));
        let hi = light.iter().fold(f64::NEG_INFINITY, |m, &v| max(m, v));
        let span = hi - lo;
        for v in &mut light {
            if span > STRETCH_MIN {
                *v = (*v - lo) / span;
            }
            *v = min(max(*v, 0.0), 1.0);
        }
        light
    }
}

/// The three-light hillshade of `height` on cells of `cell_m` metres, in `[0, 1]`.
pub(crate) fn hillshade(height: &Grid<f64>, cell_m: f64) -> Grid<f64> {
    let (d_row, d_col) = gradient(height, cell_m);
    let mut out = height.map(|_| 0.0);
    for light in &LIGHTS {
        let shade = light.shade(d_row.as_slice(), d_col.as_slice());
        for (o, v) in out.as_mut_slice().iter_mut().zip(shade) {
            *o += light.weight * v;
        }
    }
    out
}

/// The ambient occlusion of `height` (0.3.0): cells below the Gaussian mean of their
/// surroundings get darker, cells above it brighter, `1 + (z − mean) / 400 m` clamped to
/// `[0.75, 1.1]`.
fn occlusion(height: &Grid<f64>, cell_m: f64) -> Grid<f64> {
    let mean = gaussian(height, OCCLUSION_SIGMA_CELLS * cell_m, cell_m);
    let mut out = height.clone();
    for (o, m) in out.as_mut_slice().iter_mut().zip(mean.as_slice()) {
        let factor = 1.0 + (*o - m) / OCCLUSION_DEPTH_M;
        *o = min(max(factor, OCCLUSION_MIN), OCCLUSION_MAX);
    }
    out
}

/// The brightness factor of every cell, `(0.30 + 0.85 · hillshade) · occlusion` (0.3.0).
pub(crate) fn shading(height: &Grid<f64>, cell_m: f64) -> Grid<f64> {
    let mut out = hillshade(height, cell_m);
    let ao = occlusion(height, cell_m);
    for (o, a) in out.as_mut_slice().iter_mut().zip(ao.as_slice()) {
        *o = (AMBIENT + DIRECT * *o) * a;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernels::num::to_f64;

    fn plane(rise_per_row: f64, rise_per_col: f64) -> Grid<f64> {
        let mut g = Grid::filled(16, 16, 0.0);
        for r in 0..16 {
            for c in 0..16_usize {
                g.set(
                    r,
                    c,
                    500.0 + rise_per_row * to_f64(r) + rise_per_col * to_f64(c),
                );
            }
        }
        g
    }

    /// The weighted sines of the light altitudes: what a flat cell receives.
    fn flat_value() -> f64 {
        let sin = |deg: f64| libm::sin(deg.to_radians());
        0.6 * sin(45.0) + 0.25 * sin(35.0) + 0.15 * sin(60.0)
    }

    #[test]
    fn flat_ground_gets_the_weighted_sines_of_the_light_altitudes() {
        let shade = hillshade(&plane(0.0, 0.0), 50.0);
        for v in shade.as_slice() {
            assert!((v - flat_value()).abs() < 1e-12, "{v}");
        }
        // No relief, no occlusion: the factor is 0.30 + 0.85 · hillshade.
        let factor = shading(&plane(0.0, 0.0), 50.0);
        assert!((factor.get(3, 4) - (0.30 + 0.85 * flat_value())).abs() < 1e-12);
    }

    #[test]
    fn slopes_facing_the_north_west_light_are_brighter() {
        // Rows run south: a plane rising to the south faces north.
        let north = hillshade(&plane(20.0, 0.0), 50.0).get(8, 8);
        let south = hillshade(&plane(-20.0, 0.0), 50.0).get(8, 8);
        let west = hillshade(&plane(0.0, 20.0), 50.0).get(8, 8);
        let east = hillshade(&plane(0.0, -20.0), 50.0).get(8, 8);
        assert!(
            north > south && west > east,
            "{north} {south} {west} {east}"
        );
        assert!(north > flat_value() && south < flat_value());
    }

    #[test]
    fn a_ridge_spans_the_full_range_and_hollows_are_darker() {
        let mut ridge = plane(0.0, 0.0);
        for r in 0..16 {
            for c in 0..16_usize {
                let d = to_f64(c.abs_diff(8_usize));
                ridge.set(r, c, 800.0 - 60.0 * d);
            }
        }
        let shade = hillshade(&ridge, 50.0);
        assert!(shade.as_slice().iter().all(|v| (0.0..=1.0).contains(v)));
        // The west flank faces the main light, the east flank away from it.
        assert!(shade.get(8, 4) > shade.get(8, 12));
        // A pit is darker than the ground around it (occlusion below 1).
        let mut pit = plane(0.0, 0.0);
        pit.set(8, 8, 100.0);
        let factor = shading(&pit, 50.0);
        let flat = 0.30 + 0.85 * hillshade(&pit, 50.0).get(1, 1);
        assert!(factor.get(8, 8) < factor.get(1, 1) && factor.get(1, 1) <= flat * 1.1);
    }
}
